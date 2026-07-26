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
  capabilities: () => ipcRenderer.invoke("core:capabilities"),
  addCapability: (body: {
    label: string;
    command: string;
    args?: string[];
    env?: Record<string, string>;
    agents?: string[];
  }) => ipcRenderer.invoke("core:addCapability", body),
  capabilityAction: (body: { id: string; action: string; agents?: string[] }) =>
    ipcRenderer.invoke("core:capabilityAction", body),
  edit: (body: { path: string; sheet: string; cell: string; value: string; thread?: string }) =>
    ipcRenderer.invoke("core:edit", body),
  files: (within?: string) => ipcRenderer.invoke("core:files", within),
  newFolder: (body: { name: string; within?: string }) =>
    ipcRenderer.invoke("core:newFolder", body),
  threads: () => ipcRenderer.invoke("core:threads"),
  thread: (id: string) => ipcRenderer.invoke("core:thread", id),
  steering: (id?: string) => ipcRenderer.invoke("core:steering", id),
  addNote: (body: { note: string; thread?: string }) => ipcRenderer.invoke("core:addNote", body),
  noteAction: (body: { id: string; action: string; text?: string }) =>
    ipcRenderer.invoke("core:noteAction", body),
  ask: (request: { asked: string; path: string; thread?: string }) =>
    ipcRenderer.invoke("core:ask", request),
  sheet: (path: string) => ipcRenderer.invoke("core:sheet", path),
  document: (path: string) => ipcRenderer.invoke("core:document", path),
  deck: (path: string) => ipcRenderer.invoke("core:deck", path),
});
