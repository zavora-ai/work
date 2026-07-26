# Zavora Work Studio

A privacy-first desktop application that gives non-technical professionals a team of
AI agents that do real work: three artefact specialists that create and co-edit
documents, decks and spreadsheets in the app, and proactive work that runs to a
schedule under a one-time approval and a light steering loop.

The specification is the source of truth: `.kiro/specs/zavora-work-studio/`
— `requirements.md`, `design.md`, `tasks.md`, `journeys.md`, and rendered screens in
`mockups/`.

## Layout

| Path | What it is |
|---|---|
| `core/` | The Core — a standalone Rust binary. Owns all state, all execution, and the only writer of local data. |
| `core/crates/studio-jobs` | Job identity and the closed lifecycle state machine. |
| `core/crates/studio-gate` | The side-effect gate: the only place an external action is authorised. |
| `core/crates/studio-tray` | The durable decision queue and its four item classes. |
| `core/crates/studio-steering` | What the User has told Work Studio, per thread and globally. |
| `core/crates/studio-runner` | Run exclusivity, and the adapter that answers a runtime with the gate. |
| `core/crates/studio-router` | Quality tiers, failover, and spend accounting. |
| `core/crates/studio-schedule` | Schedules in the User's terms, and missed-run policy. |
| `core/crates/studio-sheets` | Reads a spreadsheet with `zavora-xlsx` into a grid the interface can draw. |
| `core/crates/studio-docs` | Reads a document into an editable view, one identifier per block. |
| `core/crates/studio-decks` | Draws a deck, and says what each drawn element refers to. |
| `core/crates/studio-artefacts` | The User's files, and the one change log both they and Work Studio write to. |
| `core/crates/studio-store` | Encrypted local SQLite store and migrations. |
| `core/crates/studio-strings` | Every User-visible string, with its surface scope. |
| `core/crates/studio-lint` | The vocabulary guardrail and its `vocab-lint` binary. |
| `core/crates/studio-api` | Renderer-facing view types and the payload guardrail. |
| `core/crates/studio-core` | The Core binary. |
| `core/crates/studio-core/src/api.rs` | The authenticated loopback channel and event stream. |
| `shell/` | Electron shell. Supervises the Core, holds the bearer token, and renders the product surface. |

The Core has no dependency on Electron, so the shell is replaceable without
touching product logic.

## Commands

```sh
make test        # every Core test
make guardrails  # the build-failing guardrails
make ci          # what CI runs: fmt, clippy, tests, guardrails
make run         # run the Core
make hooks       # install the pre-commit hook
```

## The guardrails

Two checks fail the build rather than producing a warning. They exist because the
previous in-house attempt (`adk-desktop`) drifted from a product into a development
environment one honest-looking label at a time.

**Vocabulary** — Requirement 1.1 forbids a list of technical terms in any
User-visible string; Requirement 1.2 requires the build to fail on one. Every
string lives in `studio-strings` with a scope, and `vocab-lint` scans them:

```
$ make lint
vocab-lint: 235 strings checked, all clean
vocab-lint: 211 Shell strings checked, and they match the Core's
```

It checks two catalogues, because there are two. The Shell cannot read a Rust
constant, so it mirrors the strings in `shell/src/shared/strings.ts` — and a mirror
is where a second, unchecked copy of the product's words can grow. It did: the
Shell had 127 strings the Core had never seen, so the rule had never been applied to
them, and one of them said "provider" on a first-run screen. The lint now applies the
rule to both and fails if a key is in one and not the other, or carries different
words. A string the interface renders that the rule has not seen is a hole in the
guardrail, not a detail.

The rule is scoped, not blanket. Settings may name a provider because Requirement
14.7 confines provider and model identifiers there; the diagnostics view exists to
hold technical detail. Everywhere else the prohibition is absolute. "Run" is
matched only in its noun senses ("11 runs today", "last run"), never as a verb
("Run now").

**Renderer payload** — no non-diagnostic payload crossing to the renderer may
carry an agent, model, provider, server or tool identifier. `studio-api` holds every
renderer-facing type, and the check scans field names *and* string values, because a
well-named field carrying `"gpt-5-mini"` leaks just as effectively as one called
`model`. The diagnostics payload is exempt, and a test proves that exemption is doing
real work rather than covering an already-clean payload.

## State of the build

Verified working:

- Job kinds and the closed transition set per kind, exhaustive over every
  `(kind, from, to)` triple — Correctness Property 4.
- Read-only Jobs skip Kickoff review — Requirement 5.7.
- The local store, its migration, and an Activity_Log that is append-only by
  database trigger rather than by convention — Correctness Property 16. Rejection
  holds even from another process.
- Schema constraints for Job kind and state, one-off Jobs carrying no schedule,
  steering scope matching ownership, the four tray classes, and irreversible
  deliveries never claiming a reversal window — Correctness Property 17.
- The vocabulary guardrail, including a test that it catches the violations
  the first hand-drawn mockup actually contained.
- The renderer payload guardrail over every view type, including nested and
  arrayed leaks reported with a path to the offending field.
- The loopback channel: the Core binds an OS-assigned port on `127.0.0.1` and
  rejects any request without the bearer token the Shell mints at spawn. Verified
  over real HTTP, including that the token is never printed or written to disk.
- An ordered, resumable event stream, so a renderer reload replays exactly what it
  missed and a resume point older than retained history forces a refetch rather
  than a silent gap.

- The Shell, with no Node integration in the renderer, a strict content security
  policy, and navigation refused. Its only capability surface is a typed preload
  bridge, so widening the renderer's power shows up in a diff.
- The Shell to Core handshake, tested against the real binary rather than a stub,
  because that seam is where a credential mistake would hide.
- The side-effect gate. A dry run performs no external action, an unclassified
  operation is treated as externally visible rather than waved through, and an
  `autoApprove` flag declared in configuration provably changes no decision —
  checked exhaustively across operations, modes and Job states.
- The tray: four classes, resolve-once semantics, one item per failing account
  however many Jobs depend on it, and unresolved items that survive the process
  ending while an abandoned transaction leaves nothing behind.
- Run exclusivity as a database guarantee: 40 contended attempts, half left in
  flight, and no two executions of one piece of work ever overlap. A lease left by
  a process that died is reclaimed rather than locking the work out.
- The ADK-Rust confirmation adapter, compiled and tested against the real crate,
  so there is exactly one place in the product where an external action is
  authorised.
- Smart routing: three quality tiers, each an ordered chain whose tail is failover.
  Work served by a fallback returns the same result to the User and the failure is
  recorded rather than reported. Every call is metered whoever made it, and the
  daily limit pauses proactive work while never stopping the User's own.
- The spreadsheet specialist end to end without a model: all 93 operations classified,
  the real capability server started over stdio, a write passing the gate into the file
  on disk, and the Core reading back what was written. A test asserts the server's own
  operation list is fully covered by the classification, so a new operation fails the
  build rather than being refused at run time.
- One change path for both authors. A cell the User typed and a column an agent added
  are the same kind of thing, which is why one history shows both and undo works the
  same either way.
- A spreadsheet read by the Core and drawn by the renderer without a second parser,
  so the number on screen is the number in the file. Real row and column positions
  survive, and a formula cell shows its formula rather than only its result.
- Schedules held in the User's own words, with cron derived for execution and never
  displayed. A laptop asleep from Monday to Friday makes a digest run once on
  waking and a monitor let four stale checks go.
- Steering, per thread and globally, with global notes narrowed by Artefact kind
  and per-thread notes winning by construction rather than by a special rule.
  Nothing influences a run unless the User can see and edit it.

- All three specialists, each against its own real capability server. Every operation
  classified — 93 spreadsheet, 88 document, 71 presentation — with a test holding the
  server's own list so a new one fails the build rather than being refused at run time.
  That guard earned its keep twice: it caught an operation this catalogue had invented,
  and three the document server had grown since the list was written.
- A deck the User can point at. Slides were the one artefact whose drawing named nothing,
  so a click could not be traced. The renderer now attributes every drawn element to the
  shape it came from, which matters because one shape is drawn more than once — a filled
  box and its text are two elements of one thing. The attribution survives a save, a
  reopen and an edit to a different shape.
- Documents and decks read by the Core and drawn by the renderer, with the sidebar and the
  canvas derived from one model so they cannot disagree about the file that is open.

`make ci` runs formatting, clippy with `-D warnings`, 178 Core tests, 20 Shell tests
and both guardrails. `make adk-check` adds 16 more that need the sibling checkouts: the
ADK-Rust adapter and all three specialists against their real capability servers. CI runs
it as a separate `specialists` job, which matters more than it sounds — the tests in
`make ci` compare each catalogue against a list written down beside it, so they catch a
catalogue that invents an operation but not a server that grows one. Only the job with the
servers built compares against what a server actually exposes, and those tests skip rather
than fail when a server is missing, so the job also asserts that nothing skipped.

Two sibling checkouts are required: `zavora-xlsx` beside this repository for
`studio-sheets`, and `adk-rust` for `make adk-check`. CI must provision both.

Not yet started: the connectors, which need credentials this checkout does not have.
