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
  start: (body: unknown) => ipcRenderer.invoke("core:start", body),
  standings: () => ipcRenderer.invoke("core:standings"),
  tray: () => ipcRenderer.invoke("core:tray"),
  trayAct: (decision: unknown) => ipcRenderer.invoke("core:trayAct", decision),
  deliveries: () => ipcRenderer.invoke("core:deliveries"),
  overview: () => ipcRenderer.invoke("core:overview"),
  activity: () => ipcRenderer.invoke("core:activity"),
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
  presentBegin: (body: { voice?: string; about?: string }) =>
    ipcRenderer.invoke("core:presentBegin", body),
  presentSay: (body: { words: string }) => ipcRenderer.invoke("core:presentSay", body),
  presentHush: () => ipcRenderer.invoke("core:presentHush"),
  presentEnd: () => ipcRenderer.invoke("core:presentEnd"),
  presentHeard: () => ipcRenderer.invoke("core:presentHeard"),
  talk: (path: string) => ipcRenderer.invoke("core:talk", path),
  speak: (body: { words: string; voice?: string }) => ipcRenderer.invoke("core:speak", body),
  sheetAct: (body: Record<string, unknown>) => ipcRenderer.invoke("core:sheetAct", body),
  redo: (body: { path: string; thread?: string }) => ipcRenderer.invoke("core:redo", body),
  undo: (body: { path: string; thread?: string }) => ipcRenderer.invoke("core:undo", body),
  format: (body: {
    path: string;
    sheet: string;
    range: string;
    how: Record<string, unknown>;
    thread?: string;
  }) => ipcRenderer.invoke("core:format", body),
  edit: (body: {
    path: string;
    sheet: string;
    cell: string;
    value: string;
    thread?: string;
    more?: { cell: string; value: string }[];
  }) =>
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
