/**
 * The preload bridge.
 *
 * This is the entire capability surface of the renderer. It cannot read files,
 * cannot spawn processes, cannot reach the network, and never sees the bearer
 * token — the main process attaches that on the renderer's behalf.
 *
 * Authored as CommonJS (`.cts`) because a sandboxed preload must be CommonJS, and
 * the sandbox is not something worth giving up for module-format convenience.
 *
 * The contract it satisfies is declared once in `src/shared/bridge.ts`.
 */

import electron = require("electron");

const { contextBridge, ipcRenderer } = electron;

contextBridge.exposeInMainWorld("studio", {
  health: () => ipcRenderer.invoke("core:health"),
  events: (since: number) => ipcRenderer.invoke("core:events", since),
  openFile: () => ipcRenderer.invoke("shell:openFile"),
  sheet: (path: string) => ipcRenderer.invoke("core:sheet", path),
  document: (path: string) => ipcRenderer.invoke("core:document", path),
  deck: (path: string) => ipcRenderer.invoke("core:deck", path),
});
