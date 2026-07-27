/**
 * Shell to Core handshake.
 *
 * This is the seam where the two halves of the product meet, so it is tested
 * against the real Core binary rather than a stub. It proves the whole isolation
 * story end to end: the Shell mints a token, the Core accepts only that token, and
 * nothing else on the machine can reach the port.
 *
 * Skipped when the Core has not been built, so a renderer-only checkout still
 * passes. Build it with `cargo build -p studio-core` from `core/`.
 */

import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { after, before, describe, test } from "node:test";

import { startCore, stopCore, type CoreHandle } from "../src/main/core.ts";

const BINARY = resolve(import.meta.dirname, "../../core/target/debug/studio-core");
const available = existsSync(BINARY);

describe("shell to core handshake", { skip: available ? false : "Core not built" }, () => {
  let dataDir: string;
  let core: CoreHandle;

  before(async () => {
    dataDir = await mkdtemp(join(tmpdir(), "zws-handshake-"));
    core = await startCore({ binary: BINARY, dataDir });
  });

  after(async () => {
    if (core) stopCore(core);
    if (dataDir) await rm(dataDir, { recursive: true, force: true });
  });

  test("the Core comes up on a loopback port the Shell can read", () => {
    assert.ok(core.port > 0, "expected a port");
    assert.ok(core.token.length === 64, "expected a 32-byte token");
  });

  test("the correct token is accepted", async () => {
    const response = await fetch(`http://127.0.0.1:${core.port}/health`, {
      headers: { Authorization: `Bearer ${core.token}` },
    });
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { ready: true });
  });

  test("no token, a wrong token, and a wrong scheme are all refused", async () => {
    const base = `http://127.0.0.1:${core.port}/health`;

    const none = await fetch(base);
    assert.equal(none.status, 401, "a request with no credential must be refused");

    const wrong = await fetch(base, { headers: { Authorization: "Bearer deadbeef" } });
    assert.equal(wrong.status, 401, "a wrong token must be refused");

    const scheme = await fetch(base, { headers: { Authorization: core.token } });
    assert.equal(scheme.status, 401, "the Bearer scheme is required");
  });

  test("the event stream is resumable from a sequence", async () => {
    const response = await fetch(`http://127.0.0.1:${core.port}/events?since=0`, {
      headers: { Authorization: `Bearer ${core.token}` },
    });
    assert.equal(response.status, 200);
    const body = (await response.json()) as {
      events: unknown[];
      refetchRequired: boolean;
      latestSeq: number;
    };
    assert.ok(Array.isArray(body.events));
    assert.equal(body.refetchRequired, false);
    assert.equal(typeof body.latestSeq, "number");
  });

  test("the token is never written to the data directory", async () => {
    const entries = await readdir(dataDir);
    for (const entry of entries) {
      const path = join(dataDir, entry);
      let contents: string;
      try {
        contents = await readFile(path, "latin1");
      } catch {
        continue; // a directory, or something we cannot read as a file
      }
      assert.ok(
        !contents.includes(core.token),
        `${entry} contains the bearer token; it must exist only in memory`,
      );
    }
  });

  test("only the port is persisted, so a restart can rebind cleanly", async () => {
    const port = await readFile(join(dataDir, "port"), "utf8");
    assert.equal(Number.parseInt(port.trim(), 10), core.port);
  });
});

/**
 * A reused data directory must not make the Shell connect to a dead port.
 *
 * The first version of the supervisor read whatever `port` file it found, so on
 * the second launch it connected to the previous Core's port and every request
 * failed with ECONNREFUSED — which reads like the Core failing to start. The
 * original handshake test could not catch it because it used a fresh temporary
 * directory, where no stale file can exist.
 */
describe("a reused data directory", { skip: available ? false : "Core not built" }, () => {
  let dataDir: string;

  before(async () => {
    dataDir = await mkdtemp(join(tmpdir(), "zws-restart-"));
  });

  after(async () => {
    if (dataDir) await rm(dataDir, { recursive: true, force: true });
  });

  test("a second launch in the same directory reaches the new Core, not the old port", async () => {
    const first = await startCore({ binary: BINARY, dataDir });
    const firstPort = first.port;
    stopCore(first);
    await new Promise((r) => setTimeout(r, 400));

    // The stale file is still on disk at this point unless the supervisor clears it.
    const second = await startCore({ binary: BINARY, dataDir });
    try {
      assert.notEqual(second.port, firstPort, "the new Core should bind a fresh port");

      const response = await fetch(`http://127.0.0.1:${second.port}/health`, {
        headers: { Authorization: `Bearer ${second.token}` },
      });
      assert.equal(response.status, 200, "the Shell must reach the Core it just started");

      const reported = await readFile(join(dataDir, "port"), "utf8");
      assert.equal(
        Number.parseInt(reported.trim(), 10),
        second.port,
        "the port file must describe the running Core",
      );
    } finally {
      stopCore(second);
    }
  });
});

/**
 * The spreadsheet path, end to end.
 *
 * The Core reads a real `.xlsx` with `zavora-xlsx` and answers with a grid the renderer
 * can draw. This is the seam that replaces browser-side parsing, so it is worth proving
 * against a real file rather than a fixture.
 */
describe("reading a spreadsheet", { skip: available ? false : "Core not built" }, () => {
  let dataDir: string;
  let core: CoreHandle;

  before(async () => {
    dataDir = await mkdtemp(join(tmpdir(), "zws-sheet-"));
    core = await startCore({ binary: BINARY, dataDir });
  });

  after(async () => {
    if (core) stopCore(core);
    if (dataDir) await rm(dataDir, { recursive: true, force: true });
  });

  const ask = (path: string) =>
    fetch(`http://127.0.0.1:${core.port}/sheet?path=${encodeURIComponent(path)}`, {
      headers: { Authorization: `Bearer ${core.token}` },
    });

  test("a real spreadsheet comes back already formatted", async () => {
    const response = await ask("/tmp/zws-demo.xlsx");
    assert.equal(response.status, 200);
    const model = (await response.json()) as {
      fileName: string;
      sheets: { name: string; firstRow: number; rows: { display: string; formula?: string }[][] }[];
    };
    assert.equal(model.fileName, "zws-demo.xlsx");
    const sheet = model.sheets[0]!;
    assert.equal(sheet.name, "Summary");
    // A sheet starts at A1, as every spreadsheet does; the data's own position survives inside
    // it, which is what these row offsets check.
    assert.equal(sheet.firstRow, 0, "a sheet starts at A1");
    assert.equal(sheet.rows[0]![0]!.display, "", "A1 is there, and empty");
    assert.equal(sheet.rows[4]![0]!.display, "Month", "the heading is still on row 5");
    assert.equal(
      sheet.rows[5]![3]!.formula,
      "=C6*1.12",
      "a formula cell must carry its formula for the formula bar",
    );
    assert.equal(
      sheet.rows[5]![3]!.display,
      "5555200",
      "and its value already formatted, so the renderer never calculates",
    );
  });

  test("a file that is not a spreadsheet is refused in the User's words", async () => {
    const path = join(dataDir, "not-a-sheet.xlsx");
    await writeFile(path, "this is not a spreadsheet");
    const response = await ask(path);
    assert.equal(response.status, 422);
    const body = (await response.json()) as { problem: string };
    assert.match(body.problem, /could not be opened/);
    assert.doesNotMatch(
      body.problem,
      /zip|EOCD|error:/i,
      "the underlying cause belongs in diagnostics, not in front of the User",
    );
  });

  test("the spreadsheet endpoint needs the token too", async () => {
    const response = await fetch(`http://127.0.0.1:${core.port}/sheet?path=/tmp/zws-demo.xlsx`);
    assert.equal(response.status, 401);
  });
});

describe("reading a document", { skip: available ? false : "Core not built" }, () => {
  let dataDir: string;
  let core: CoreHandle;

  before(async () => {
    dataDir = await mkdtemp(join(tmpdir(), "zws-doc-"));
    core = await startCore({ binary: BINARY, dataDir });
  });

  after(async () => {
    if (core) stopCore(core);
    if (dataDir) await rm(dataDir, { recursive: true, force: true });
  });

  const ask = (path: string) =>
    fetch(`http://127.0.0.1:${core.port}/document?path=${encodeURIComponent(path)}`, {
      headers: { Authorization: `Bearer ${core.token}` },
    });

  test("a real document comes back editable, with an identifier per block", async () => {
    const response = await ask("/tmp/zws-demo.docx");
    assert.equal(response.status, 200);
    const model = (await response.json()) as {
      fileName: string;
      html: string;
      blockCount: number;
      outline: { text: string; level: number }[];
    };
    assert.equal(model.fileName, "zws-demo.docx");
    assert.ok(model.blockCount >= 3, "every paragraph must be present");
    assert.equal(
      (model.html.match(/data-p=/g) ?? []).length,
      model.blockCount,
      "every block must be addressable, or a change cannot be attributed",
    );
    assert.ok(
      model.outline.some((entry) => entry.text.includes("Termination")),
      "the headings must come through for the outline",
    );
  });

  test("a file that is not a document is refused in the User's words", async () => {
    const path = join(dataDir, "not-a-doc.docx");
    await writeFile(path, "this is not a document");
    const response = await ask(path);
    assert.equal(response.status, 422);
    const body = (await response.json()) as { problem: string };
    assert.ok(!/zip|EOCD|error/i.test(body.problem), `leaked detail: ${body.problem}`);
  });

  test("the document endpoint needs the token too", async () => {
    const response = await fetch(
      `http://127.0.0.1:${core.port}/document?path=/tmp/zws-demo.docx`,
    );
    assert.equal(response.status, 401);
  });
});

describe("reading a deck", { skip: available ? false : "Core not built" }, () => {
  let dataDir: string;
  let core: CoreHandle;

  before(async () => {
    dataDir = await mkdtemp(join(tmpdir(), "zws-deck-"));
    core = await startCore({ binary: BINARY, dataDir });
  });

  after(async () => {
    if (core) stopCore(core);
    if (dataDir) await rm(dataDir, { recursive: true, force: true });
  });

  const ask = (path: string) =>
    fetch(`http://127.0.0.1:${core.port}/deck?path=${encodeURIComponent(path)}`, {
      headers: { Authorization: `Bearer ${core.token}` },
    });

  test("a real deck comes back drawn, and every element says what it refers to", async () => {
    const response = await ask("/tmp/zws-demo.pptx");
    assert.equal(response.status, 200);
    const model = (await response.json()) as {
      fileName: string;
      slides: {
        number: number;
        title: string;
        svg: string;
        itemCount: number;
        targets: ({ refers_to: string; position?: number } | null)[];
      }[];
    };
    assert.equal(model.fileName, "zws-demo.pptx");
    assert.equal(model.slides.length, 2);
    assert.equal(model.slides[0]!.number, 1, "the User counts slides from one");
    assert.ok(model.slides[0]!.svg.startsWith("<svg"), "a slide must arrive drawn");
    assert.equal(
      model.slides[0]!.targets.length,
      model.slides[0]!.itemCount,
      "every drawn element must resolve to something, or a click means nothing",
    );
    const first = model.slides[0]!.targets.find((t) => t !== null);
    assert.ok(first, "a slide with a text box must have something changeable on it");
  });

  test("a file that is not a deck is refused in the User's words", async () => {
    const path = join(dataDir, "not-a-deck.pptx");
    await writeFile(path, "this is not a presentation");
    const response = await ask(path);
    assert.equal(response.status, 422);
    const body = (await response.json()) as { problem: string };
    assert.ok(!/zip|EOCD/i.test(body.problem), `leaked detail: ${body.problem}`);
  });

  test("the deck endpoint needs the token too", async () => {
    const response = await fetch(`http://127.0.0.1:${core.port}/deck?path=/tmp/zws-demo.pptx`);
    assert.equal(response.status, 401);
  });
});
