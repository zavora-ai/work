/**
 * Main process.
 *
 * Security posture, all of it deliberate:
 * - the renderer has no Node integration and no direct file system access;
 * - context isolation is on, so the renderer sees only the typed preload bridge;
 * - a strict content security policy is applied to every response;
 * - navigation away from the bundled renderer is refused, as is opening a window;
 * - the bearer token stays in this process. The renderer asks for data; it never
 *   holds the credential.
 */

import { app, BrowserWindow, dialog, ipcMain, net, protocol, session, shell } from "electron";
import { join } from "node:path";

import {
  clearPort,
  resolveCoreBinary,
  startCore,
  stopCore,
  type CoreHandle,
} from "./core.ts";

const isDev = !app.isPackaged;
let core: CoreHandle | undefined;
let window: BrowserWindow | undefined;

/** Ask the Core for something on the renderer's behalf, attaching the token here. */
async function coreFetch(path: string): Promise<unknown> {
  if (!core) throw new Error("the Core is not running");
  const response = await fetch(`http://127.0.0.1:${core.port}${path}`, {
    headers: { Authorization: `Bearer ${core.token}` },
  });
  if (!response.ok) {
    throw new Error(`the Core refused the request (${response.status})`);
  }
  return response.json();
}

/// Asking the Core to do something.
///
/// Kept apart from `coreFetch` because a problem here is not a fault to be thrown: the
/// Core answers a refusal in the User's own words, and the interface should show those
/// words rather than a status code.
async function corePost(path: string, body: unknown): Promise<unknown> {
  if (!core) throw new Error("the Core is not running");
  const response = await fetch(`http://127.0.0.1:${core.port}${path}`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${core.token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  return response.json();
}

/**
 * Serve the pictures inside a document, on request from the page.
 *
 * The Core no longer carries every image in the model — a 216MB document inlined to a 288MB
 * payload, which the window cannot hold — so the view references them and something has to fetch
 * them. That something is this process, because it is the only one holding the token: the renderer
 * asks for `zws-media://picture?path=…&id=rId7` and never sees a credential, which is the same
 * arrangement as every other call it makes.
 */
function servePictures(): void {
  protocol.handle("zws-media", async (request) => {
    if (!core) return new Response("", { status: 503 });

    const asked = new URL(request.url);
    const path = asked.searchParams.get("path");
    const id = asked.searchParams.get("id");
    if (!path || !id) return new Response("", { status: 400 });

    const from = new URL(`http://127.0.0.1:${core.port}/media`);
    from.searchParams.set("path", path);
    from.searchParams.set("id", id);

    // `net.fetch` rather than the global one so this goes through Electron's own stack, and the
    // token is added here rather than anywhere the page can read it.
    return net.fetch(from.toString(), {
      headers: { Authorization: `Bearer ${core.token}` },
    });
  });
}

function applyContentSecurityPolicy(): void {
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        "Content-Security-Policy": [
          "default-src 'self'",
          "script-src 'self'",
          // Vite injects styles at dev time; production uses extracted CSS.
          `style-src 'self'${isDev ? " 'unsafe-inline'" : ""}`,
          // `zws-media:` is this process serving the pictures inside the User's document. Still
          // no remote origin: the scheme resolves to the Core on loopback.
          "img-src 'self' data: zws-media:",
          // The presenter's own voice, made by the Core and handed straight to the window. Still
          // nothing remote: the sound arrives as bytes in the answer, not as a URL to fetch.
          "media-src 'self' data:",
          "font-src 'self'",
          // The renderer never talks to the network. It talks to the bridge.
          "connect-src 'none'",
          "object-src 'none'",
          "frame-ancestors 'none'",
        ].join("; "),
      },
    });
  });
}

/**
 * Which workspace a file named at launch belongs to.
 *
 * Empty for anything this app cannot open, so an unknown file is simply not opened rather
 * than opening a workspace that has nothing to show.
 */
export function openingQuery(path: string | undefined): Record<string, string> {
  if (!path) return {};
  const extension = path.slice(path.lastIndexOf(".") + 1).toLowerCase();
  if (extension === "xlsx") return { sheet: path };
  if (extension === "docx") return { document: path };
  if (extension === "pptx") return { deck: path };
  return {};
}

function createWindow(): BrowserWindow {
  const win = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 1024,
    minHeight: 700,
    titleBarStyle: "hiddenInset",
    backgroundColor: "#f7f6f3",
    show: false,
    webPreferences: {
      // A sandboxed preload must be CommonJS, so it is authored as .cts.
      preload: join(import.meta.dirname, "../preload/index.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      webSecurity: true,
    },
  });

  // Refuse navigation away from the bundled renderer, and refuse new windows.
  win.webContents.on("will-navigate", (event, url) => {
    const allowed = isDev && url.startsWith("http://localhost:");
    if (!allowed) event.preventDefault();
  });
  win.webContents.setWindowOpenHandler(({ url }) => {
    // A real external link is handed to the OS browser, never opened in-app.
    if (url.startsWith("https://")) void shell.openExternal(url);
    return { action: "deny" };
  });

  win.once("ready-to-show", () => win.show());
  return win;
}

// Declared before the app is ready, which is the only time it can be: without this the scheme is
// treated as neither secure nor able to carry a stream, and the pictures do not load.
protocol.registerSchemesAsPrivileged([
  { scheme: "zws-media", privileges: { standard: true, secure: true, supportFetchAPI: true, stream: true } },
]);

app.whenReady().then(async () => {
  applyContentSecurityPolicy();
  servePictures();

  ipcMain.handle("core:health", () => coreFetch("/health"));
  // Opening one of the User's own files.
  //
  // The picker belongs here because the renderer has no filesystem access and should not
  // gain any: it receives a path the User chose and nothing else. Without this the app
  // could only ever show its own sample, which is what it did.
  ipcMain.handle("shell:openFile", async () => {
    const result = await dialog.showOpenDialog({
      title: "Open a file",
      properties: ["openFile"],
      filters: [
        { name: "Documents, decks and spreadsheets", extensions: ["docx", "pptx", "xlsx"] },
        { name: "Documents", extensions: ["docx"] },
        { name: "Decks", extensions: ["pptx"] },
        { name: "Spreadsheets", extensions: ["xlsx"] },
      ],
    });
    if (result.canceled || result.filePaths.length === 0) return undefined;
    return result.filePaths[0];
  });

  ipcMain.handle("core:start", (_event, body: unknown) => corePost("/start", body));
  ipcMain.handle("core:standings", () => coreFetch("/standings"));
  ipcMain.handle("core:tray", () => coreFetch("/tray"));
  ipcMain.handle("core:trayAct", (_event, decision: unknown) =>
    corePost("/tray/act", decision),
  );
  ipcMain.handle("core:deliveries", () => coreFetch("/deliveries"));

  ipcMain.handle("core:overview", () => coreFetch("/overview"));
  ipcMain.handle("core:activity", () => coreFetch("/activity"));

  ipcMain.handle("core:capabilities", () => coreFetch("/capabilities"));
  ipcMain.handle("core:addCapability", (_e, body: unknown) => corePost("/capabilities", body));
  ipcMain.handle("core:capabilityAction", (_e, body: unknown) =>
    corePost("/capabilities/act", body),
  );

  ipcMain.handle("core:edit", (_event, body: unknown) => corePost("/edit", body));
  // Presenting aloud. The audio comes back as bytes rather than a URL, because a recording of the
  // User's own words should not be addressable by anything else on the machine.
  ipcMain.handle("core:talk", (_event, path: string) =>
    coreFetch(`/talk?path=${encodeURIComponent(path)}`),
  );
  ipcMain.handle("core:speak", async (_event, body: unknown) => {
    if (!core) throw new Error("the Core is not running");
    const answer = await fetch(`http://127.0.0.1:${core.port}/speak`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${core.token}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    });
    if (!answer.ok) return { problem: await answer.text() };
    const sound = await answer.arrayBuffer();
    return {
      wav: Buffer.from(sound).toString("base64"),
      lastsMs: Number(answer.headers.get("x-lasts-ms") ?? 0),
    };
  });

  // Presenting live. The session is the Core's; this only carries the asking and the answers.
  ipcMain.handle("core:presentBegin", (_event, body: unknown) => corePost("/present/begin", body));
  ipcMain.handle("core:presentSay", (_event, body: unknown) => corePost("/present/say", body));
  ipcMain.handle("core:presentHush", () => corePost("/present/hush", {}));
  ipcMain.handle("core:presentEnd", () => corePost("/present/end", {}));
  ipcMain.handle("core:presentHeard", () => coreFetch("/present/heard"));

  ipcMain.handle("core:sheetAct", (_event, body: unknown) => corePost("/sheet/act", body));
  ipcMain.handle("core:redo", (_event, body: unknown) => corePost("/redo", body));
  ipcMain.handle("core:undo", (_event, body: unknown) => corePost("/undo", body));
  ipcMain.handle("core:format", (_event, body: unknown) => corePost("/format", body));

  ipcMain.handle("core:files", (_event, within: unknown) =>
    coreFetch(
      typeof within === "string" && within
        ? `/files?within=${encodeURIComponent(within)}`
        : "/files",
    ),
  );

  ipcMain.handle("core:newFolder", (_event, body: unknown) => corePost("/folder", body));

  ipcMain.handle("core:threads", () => coreFetch("/threads"));

  ipcMain.handle("core:thread", (_event, id: unknown) => {
    if (typeof id !== "string") throw new Error("a piece of work is needed");
    return coreFetch(`/thread?thread=${encodeURIComponent(id)}`);
  });

  ipcMain.handle("core:steering", (_event, id: unknown) =>
    coreFetch(typeof id === "string" && id ? `/steering?thread=${encodeURIComponent(id)}` : "/steering"),
  );

  ipcMain.handle("core:addNote", (_event, body: unknown) => corePost("/steering", body));

  ipcMain.handle("core:noteAction", (_event, body: unknown) => corePost("/steering/act", body));

  ipcMain.handle("core:ask", (_event, request: unknown) => {
    if (typeof request !== "object" || request === null) {
      throw new Error("a request is needed");
    }
    return corePost("/ask", request);
  });

  ipcMain.handle("core:sheet", (_event, path: unknown) => {
    if (typeof path !== "string" || path.length === 0) {
      throw new Error("that file could not be opened");
    }
    return coreFetch(`/sheet?path=${encodeURIComponent(path)}`);
  });

  ipcMain.handle("core:document", (_event, path: unknown) => {
    if (typeof path !== "string" || path.length === 0) {
      throw new Error("that file could not be opened");
    }
    return coreFetch(`/document?path=${encodeURIComponent(path)}`);
  });

  ipcMain.handle("core:deck", (_event, path: unknown) => {
    if (typeof path !== "string" || path.length === 0) {
      throw new Error("that file could not be opened");
    }
    return coreFetch(`/deck?path=${encodeURIComponent(path)}`);
  });

  ipcMain.handle("core:events", (_event, since: unknown) => {
    const seq = typeof since === "number" && Number.isFinite(since) ? since : 0;
    return coreFetch(`/events?since=${Math.max(0, Math.trunc(seq))}`);
  });

  const dataDir = join(app.getPath("userData"), "data");
  dataDirForShutdown = dataDir;
  try {
    core = await startCore({
      binary: resolveCoreBinary({
        override: process.env.ZWS_CORE_BINARY,
        isPackaged: app.isPackaged,
        resourcesPath: process.resourcesPath,
        shellDir: join(import.meta.dirname, "../.."),
      }),
      dataDir,
    });
  } catch (error) {
    // Failure is reported in the User's terms, never as a spawn error.
    console.error("[shell] the Core did not start:", error);
  }

  window = createWindow();
  // A file named at launch opens straight into its workspace. The renderer reads it from
  // the address, which is the same route the review harness uses, so there is one way in
  // rather than two.
  const opening = openingQuery(process.env.ZWS_OPEN);
  if (isDev && process.env.VITE_DEV_SERVER_URL) {
    const url = new URL(process.env.VITE_DEV_SERVER_URL);
    for (const [key, value] of Object.entries(opening)) url.searchParams.set(key, value);
    await window.loadURL(url.toString());
  } else {
    await window.loadFile(join(import.meta.dirname, "../renderer/index.html"), {
      query: opening,
    });
  }
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit();
});

let dataDirForShutdown: string | undefined;

app.on("before-quit", () => {
  if (core) stopCore(core);
  if (dataDirForShutdown) void clearPort(dataDirForShutdown);
});
