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

import { app, BrowserWindow, dialog, ipcMain, session, shell } from "electron";
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
          "img-src 'self' data:",
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

app.whenReady().then(async () => {
  applyContentSecurityPolicy();

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
