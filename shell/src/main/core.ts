/**
 * Core supervisor.
 *
 * The Shell owns the Core process lifecycle. It mints a bearer token per Core
 * process, passes it by environment at spawn, and never writes it to disk or
 * hands it to the renderer directly — the renderer reaches the Core only through
 * the preload bridge, which attaches the token on its behalf.
 *
 * The Core binds an OS-assigned loopback port and writes it to a file in its data
 * directory. Only the port is written; the token never is.
 */

import { spawn, type ChildProcess } from "node:child_process";
import { randomBytes } from "node:crypto";
import { readFile, rm } from "node:fs/promises";
import { join } from "node:path";

export interface CoreHandle {
  port: number;
  token: string;
  process: ChildProcess;
}

export interface SpawnOptions {
  /** Path to the Core binary. */
  binary: string;
  /** Where the Core keeps its store. */
  dataDir: string;
  /** How long to wait for the Core to report its port. */
  readyTimeoutMs?: number;
}

/** A fresh token for one Core process, from the OS CSPRNG. */
export function mintToken(): string {
  return randomBytes(32).toString("hex");
}

/**
 * Environment for the Core. Nothing here is logged: the token would leak into
 * crash reports and terminal scrollback.
 */
export function coreEnv(token: string, dataDir: string): Record<string, string> {
  return {
    ZWS_TOKEN: token,
    ZWS_DATA_DIR: dataDir,
    ZWS_SERVE: "1",
  };
}

/** Redact anything token-shaped before it can reach a log. */
export function redact(text: string, token: string): string {
  if (!token) return text;
  return text.split(token).join("«token»");
}

async function readPort(dataDir: string, timeoutMs: number): Promise<number> {
  const path = join(dataDir, "port");
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;
  while (Date.now() < deadline) {
    try {
      const raw = await readFile(path, "utf8");
      const port = Number.parseInt(raw.trim(), 10);
      if (Number.isInteger(port) && port > 0) return port;
    } catch (error) {
      lastError = error;
    }
    await new Promise((r) => setTimeout(r, 50));
  }
  throw new Error(
    `the Core did not report a port within ${timeoutMs}ms${
      lastError ? ` (${String(lastError)})` : ""
    }`,
  );
}

export async function startCore(options: SpawnOptions): Promise<CoreHandle> {
  const token = mintToken();

  // Remove any port left by a previous Core before spawning this one. Without
  // this the Shell reads the stale file, connects to a port nothing is listening
  // on, and every request fails with ECONNREFUSED — which looks like the Core
  // failing to start rather than what it is. The handshake test did not catch it
  // because it uses a fresh directory each run, where no stale file can exist.
  await rm(join(options.dataDir, "port"), { force: true });

  const child = spawn(options.binary, [], {
    env: { ...process.env, ...coreEnv(token, options.dataDir) },
    stdio: ["ignore", "pipe", "pipe"],
  });

  // A failure to spawn arrives as an event, not as a thrown error. Without this
  // listener Node treats it as an uncaught exception, which in Electron is a fatal
  // dialog and an app that quits — so a missing Core binary took the whole product
  // down instead of being reported. The caller's `try` cannot catch that, because it
  // happens after `spawn` has already returned.
  const spawnFailed = new Promise<never>((_resolve, reject) => {
    child.once("error", (error: NodeJS.ErrnoException) => {
      reject(
        new Error(
          error.code === "ENOENT"
            ? `Work Studio could not start its engine: nothing was found at ${options.binary}`
            : `Work Studio could not start its engine: ${error.message}`,
        ),
      );
    });
  });

  // Core output is useful for support but must never carry the token.
  child.stdout?.on("data", (b: Buffer) =>
    console.log("[core]", redact(b.toString().trimEnd(), token)),
  );
  child.stderr?.on("data", (b: Buffer) =>
    console.error("[core]", redact(b.toString().trimEnd(), token)),
  );

  // Whichever happens first: the Core reports its port, or it fails to start.
  const port = await Promise.race([
    readPort(options.dataDir, options.readyTimeoutMs ?? 10_000),
    spawnFailed,
  ]);
  return { port, token, process: child };
}

/**
 * Where the Core binary is.
 *
 * Packaged, it sits beside the app's other resources. Running from a checkout there is
 * no such directory — `resourcesPath` points inside Electron itself — so the build
 * output is used instead. Getting this wrong meant the app could not start at all
 * outside a package, which no test noticed because every test passes an explicit path.
 */
export function resolveCoreBinary(env: {
  override?: string;
  isPackaged: boolean;
  resourcesPath: string;
  /** The Shell directory, from which the Core's build output is found. */
  shellDir: string;
}): string {
  if (env.override) return env.override;
  if (env.isPackaged) return join(env.resourcesPath, "studio-core");
  return join(env.shellDir, "..", "core", "target", "debug", "studio-core");
}

export function stopCore(handle: CoreHandle): void {
  if (!handle.process.killed) handle.process.kill("SIGTERM");
}

/** Clear the reported port. Called on shutdown so nothing stale is left behind. */
export async function clearPort(dataDir: string): Promise<void> {
  await rm(join(dataDir, "port"), { force: true });
}
