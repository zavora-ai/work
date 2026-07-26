/**
 * Supervisor tests.
 *
 * These cover the parts of the Shell that hold a credential, because that is
 * where a mistake is expensive and invisible.
 */

import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import {
  coreEnv,
  mintToken,
  redact,
  resolveCoreBinary,
  startCore,
} from "../src/main/core.ts";

test("a minted token is long, hex, and different every time", () => {
  const a = mintToken();
  const b = mintToken();
  assert.notEqual(a, b, "tokens must not repeat");
  assert.match(a, /^[0-9a-f]{64}$/, "expected 32 bytes of hex");
});

test("the Core receives the token by environment and nothing else sensitive", () => {
  const token = mintToken();
  const env = coreEnv(token, "/tmp/zws");
  assert.equal(env.ZWS_TOKEN, token);
  assert.equal(env.ZWS_DATA_DIR, "/tmp/zws");
  assert.equal(env.ZWS_SERVE, "1");
  assert.deepEqual(
    Object.keys(env).sort(),
    ["ZWS_DATA_DIR", "ZWS_SERVE", "ZWS_TOKEN"],
    "the Core's environment should be exactly what it needs",
  );
});

test("redaction removes the token from anything bound for a log", () => {
  const token = mintToken();
  const line = `Listening on 127.0.0.1:5544 with ${token} attached`;
  const safe = redact(line, token);
  assert.ok(!safe.includes(token), "the token must not survive redaction");
  assert.ok(safe.includes("«token»"), "the redaction should be visible");
  assert.ok(safe.includes("127.0.0.1:5544"), "the rest of the line should survive");
});

test("redaction handles repeated occurrences and an empty token", () => {
  const token = "abc123";
  assert.equal(redact(`${token} and ${token}`, token), "«token» and «token»");
  assert.equal(redact("nothing to do", ""), "nothing to do");
});

// ---- starting up ----
//
// These exist because the app did not start at all: `npm start` resolved the Core to a
// path inside Electron's own bundle, the spawn failed with ENOENT, and because nothing
// listened for the child's `error` event Electron turned it into a fatal dialog and quit.
// Every existing test passed an explicit binary path, so none of them went near it.

test("a missing engine is reported in the User's words, not as a crash", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "zws-missing-"));
  try {
    await assert.rejects(
      () => startCore({ binary: join(dataDir, "not-here"), dataDir, readyTimeoutMs: 3000 }),
      (error: Error) => {
        assert.match(error.message, /could not start its engine/);
        assert.ok(!/ENOENT|spawn/i.test(error.message), `leaked detail: ${error.message}`);
        return true;
      },
      "the failure must arrive as a rejected promise the caller can handle",
    );
  } finally {
    await rm(dataDir, { recursive: true, force: true });
  }
});

test("the engine is looked for where it actually is", () => {
  // Packaged: beside the app's other resources.
  assert.equal(
    resolveCoreBinary({
      isPackaged: true,
      resourcesPath: "/Apps/Work Studio.app/Contents/Resources",
      shellDir: "/Apps/Work Studio.app/Contents/shell",
    }),
    join("/Apps/Work Studio.app/Contents/Resources", "studio-core"),
  );

  // From a checkout: the build output, never inside Electron itself. This is the case
  // that was broken.
  const fromCheckout = resolveCoreBinary({
    isPackaged: false,
    resourcesPath: "/repo/shell/node_modules/electron/dist/Electron.app/Contents/Resources",
    shellDir: "/repo/shell",
  });
  assert.ok(
    !fromCheckout.includes("node_modules"),
    `the Core is not inside Electron: ${fromCheckout}`,
  );
  assert.equal(fromCheckout, join("/repo", "core", "target", "debug", "studio-core"));

  // An explicit path always wins, which is what the tests rely on.
  assert.equal(
    resolveCoreBinary({
      override: "/somewhere/studio-core",
      isPackaged: true,
      resourcesPath: "/r",
      shellDir: "/s",
    }),
    "/somewhere/studio-core",
  );
});

test("the engine really is where the resolver says, in this checkout", async () => {
  const resolved = resolveCoreBinary({
    isPackaged: false,
    resourcesPath: "/irrelevant",
    shellDir: join(import.meta.dirname, ".."),
  });
  // If this fails, `npm start` cannot work: build the Core first.
  assert.ok(
    existsSync(resolved),
    `the Core binary should be at ${resolved} — run \`cargo build -p studio-core\``,
  );
});
