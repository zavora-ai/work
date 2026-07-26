# Implementation Plan: Zavora Work Studio

## Overview

Implementation is sequenced so that the user experience is proven before breadth is added. Phase 1 delivers one complete proactive loop — Daily newsletter — chosen specifically because it requires **zero net-new capability work** (`mcp-news` is real and keyless, `mcp-email` is real). Every hour of Phase 1 therefore goes into the experience rather than the plumbing, which makes it a genuine test of the product thesis. If that loop does not feel good, no amount of connector breadth will save the product, and we will know it in Phase 1 rather than after Phase 4.

Order: foundations and guardrails → one vertical slice → trust loop hardening → Documents surface → remaining proactive templates and new connectors → privacy and release gates.

Two guardrails are built in Phase 0 rather than retrofitted, because they are the specific defences the prior attempt lacked: the vocabulary lint and the renderer payload lint. Both fail the build.

Repository: `work.zavora.ai`. The Core is a standalone Rust binary; the Shell is Electron; the Capability_Layer is consumed as path or version dependencies. One exception is planned and scoped: `zavora-slide-layout` and `zavora-docx-html` gain stable identifier emission, contributed upstream rather than forked, because in-app editing cannot exist without it (task 13.6). This is now done, and smaller than planned: `zavora-docx-html` already emitted `data-p` per block, so only the slide renderer needed the addition. It took the same shape — `SvgOptions { identify }` mirroring `HtmlOptions { editable }` — and is off by default, so all 741 of `zavora-slide`'s tests, including its byte-identical corpus gate, still pass. A scene item also had to say which *shape* it came from, because one shape is drawn as several items and only the shape is editable.

`journeys.md` is the acceptance frame for every interface task: a surface is done when the journey that exercises it reads correctly end to end with the copy in `mockups/`. Tasks below cite the journey they satisfy where one applies.

## Progress

Verified by `make ci` — formatting, clippy with `-D warnings`, 174 Core tests, 20 Shell tests, and both guardrails. `make adk-check` additionally verifies the ADK-Rust adapter and all three specialists against their real capability servers (187 Core tests with the `adk` feature). The upstream identifier work is verified by `zavora-slide`'s own 741 tests:

| Task | State | Evidence |
|---|---|---|
| 13.6 Upstream identifiers | Done | `SvgOptions { identify }` and `ItemSource` in `zavora-slide`; 741 upstream tests pass, corpus gate included |
| 13.7 `studio-decks` | Done | 8 tests: every drawn element resolves to a shape, and survives JSON to the renderer |
| 13.8 `studio-docs` | Done | 8 tests: editable view with one identifier per block, outline from the file's own headings |
| 13.9 Presentation catalogue | Done | 71 operations classified with the drift guard; `catalogue_slides.rs` |
| 13.10 Document catalogue | Done | 88 operations — the live server had grown 3 the static list missed |
| 13.11 Artefact integration | Done | `tests/artefacts.rs`: 4 tests against the real document and presentation servers; 4 skips prove they ran |
| 13.12 Three specialists | Done | `document_agent`, `presentation_agent` beside `spreadsheet_agent`; instruction guards on length and vocabulary |
| 13.13 Endpoints | Done | `/document` and `/deck` over authenticated loopback; 20 Shell tests including 401 and 422 |
| 13.14 Workspaces | Done | Document and deck draw real files; sidebar and canvas derived from one model |
| 1.1 Workspace and Core binary | Partial | `core/` workspace with `studio-jobs`, `studio-store`, `studio-strings`, `studio-lint`, `studio-core`. ADK-Rust dependencies deferred to task 3.5 so the engine invariants are proven in isolation first; remaining crates land with the subsystems that need them. |
| 1.2 Electron shell skeleton | Done | `shell/` builds main, preload and renderer. Renderer has `contextIsolation: true`, `nodeIntegration: false`, `sandbox: true`, a strict CSP with `connect-src 'none'`, navigation and window-open both refused. The Shell mints a 32-byte token from the OS CSPRNG, passes it by environment, redacts it from anything bound for a log, and never hands it to the renderer — which reaches the Core only through the typed preload bridge. The handshake is tested against the real Core binary: correct token 200, and missing, wrong and non-Bearer credentials all 401. |
| 1.3 Encrypted store and migrations | Done | `0001_init.sql` creates all 12 tables; migration is idempotent; `activity_log` append-only triggers. At-rest encryption is behind the `sqlcipher` feature, verified in the packaged artefact by task 17.4. |
| 1.4 Property 16 test | Done | `property_16_activity_log_is_append_only` — `UPDATE` and `DELETE` both rejected, log intact afterwards, and rejection confirmed from a separate process. |
| 1.5 Loopback channel and event stream | Done | Core binds `127.0.0.1:0`, requires a bearer token the Shell mints at spawn and passes by environment; verified over real HTTP that missing, wrong and non-Bearer credentials all get 401 and only the correct token gets 200, and that the token is never printed or written to disk. Events carry a monotonic sequence and replay exactly what a reconnecting renderer missed; a resume point older than retained history forces a refetch rather than a silent gap. A test scans every event on the stream through the payload guardrail. |
| 1.6 Vocabulary lint | Done | `vocab-lint` exits 0 on 101 clean strings and 1 with violations; `property_11_catalogue_is_clean` fails the test suite; wired into `make ci`, the CI workflow and the pre-commit hook. |
| 1.7 Renderer payload lint | Done | `studio-api` defines every renderer-facing view type; `property_12_no_payload_leaks_a_technical_identifier` scans field names *and* string values, since a well-named field carrying `"gpt-5-mini"` leaks just as effectively. `DiagnosticsPayload` is exempt, and a test proves the exemption does real work rather than covering an already-clean payload. |
| 3.3 Side-effect gate | Done | `studio-gate` decides every operation from an authored classification table, and `studio-runner::GateHandler` implements ADK-Rust's `ToolConfirmationHandler` against it — verified compiling and passing against the real `adk-core` via `make adk-check`. The adapter passes `auto_approve = true` on every call, so Property 2 is proven through the adapter as well as under it. |
| 3.6 Run exclusivity | Done | `studio-runner::lease` makes exclusivity a primary-key guarantee rather than a check-then-act. `property_5_no_two_runs_of_a_job_overlap` drives 40 contended attempts, leaves half the runs in flight, and asserts via a self-join that no two runs of one Job have overlapping intervals. A lease whose run stopped reporting is reclaimed, so a crash cannot lock a Job out of working forever. |
| 3.4 Gate property tests | Done | `property_1_a_dry_run_performs_no_external_effect` over four kind/state combinations and both external operations; `property_2_auto_approve_never_changes_a_decision` exhaustive over 5 operations x 3 modes x 7 kind/state pairs; `property_17_irreversible_actions_carry_no_window`. |
| 6.1 / 6.2 Tier routing and failover | Done | `studio-router`: three tiers, each an ordered chain whose tail is failover, defaulting entirely to OpenAI on first run. One global cost-versus-quality preference shifts which tier is consulted and never asks the User to name a model. `property_14` proves failover returns the *same result* to the User and records the failure in the Activity_Log rather than surfacing it. |
| 6.3 / 6.4 Spend accounting | Done | Every call metered whoever made it. `property_15` asserts the ledger equals usage across all three surfaces with per-Job attribution. A failed attempt is not billed. The daily limit pauses proactive work and provably never stops the User's own. Currency formatting never renders a fraction of a cent. |
| 7.1 / 7.2 Durable scheduling and missed runs | Done | `studio-schedule` holds the User's form as authoritative and derives cron for execution only — a test asserts the human form never contains a cron field. Missed occurrences are counted across a sleeping laptop: a digest runs once on waking, a monitor lets four stale checks go, and a very long absence is bounded rather than spinning. |
| 4.1 Durable decision queue | Done | `studio-tray` over the store, all four classes, resolve-once semantics, and consolidation so three Jobs failing on one account raise one item. |
| 4.2 Durability and taxonomy tests | Done | `property_3_the_tray_survives_the_process_ending` writes with one `Store`, drops it, reopens, and asserts the same unresolved set — then proves an abandoned transaction leaves nothing. `property_18` and `property_22` cover the Finding and consolidation invariants. |
| 4.4 Manifest fidelity test | Done | `property_20_manifest_holds_exactly_what_was_suppressed` — every suppressed operation is held, an excluded row is not performed, and an out-of-range exclusion is refused. |
| 5.1 Steering notes | Done | `studio-steering` with per-Job and global notes, scope narrowing by Artefact kind, derived notes held unconfirmed, rewording promoting recency, and contradiction surfacing. |
| 5.2 Steering property tests | Done | Properties 6, 7, 21, 29 and 30, including that a global note added *after* a per-Job note still does not overtake it. |
| 3.1 Job model and state machine | Done | `studio-jobs`: `JobKind`, `JobState`, closed transition sets, read-only activation path. |
| 3.2 Property 4 test | Done | `property_4_transition_set_is_closed_per_kind` is exhaustive over every `(kind, from, to)` triple; `property_4_rejection_does_not_mutate` proves rejection leaves state untouched. |

### All three artefacts, end to end

Driven in the built application against a live model, not only in tests.

| What | State | Evidence |
|---|---|---|
| Ask in words — document | Done | "Add a clause 10 headed Governing law…" produced clause 10 and the Kenya sentence; the page reloaded on its own after 72s |
| Ask in words — deck | Done | "Add a final slide that says Thank you" produced a third slide, title read back from the file |
| Editing by hand — all three | Done | one `/edit` path per kind: a cell, a paragraph by block identifier, a shape's text run. Each goes through the same gate and capability as an agent's change |
| Memory panel — all three | Done | notes and proposals per file, refetched after every exchange |
| Views reload themselves | Done | a change by asking or by hand refetches what is drawn, the work list and the folder |
| What a specialist may reach | Core done, no interface yet | `/capabilities` with on, off, remove and allocate; turning Spreadsheets off stops the specialist on the next request and back on restores it |

Two bugs found by driving it, both of which reported success and lost the work:

- **`set_title` on a newly added slide.** Reports `status: success, message: "Set title"` and the
  slide is left with no shapes at all — the title is written to the build model and dropped when
  an opened package is saved. The presentation specialist is now told to put the words in as a
  text box instead, which survives, and to read the slide back before saying what it did. The
  engine bug is unfixed and belongs upstream in `zavora-slide`.
- **`open_presentation` rejects `read_only`** where the other two servers accept or ignore it, so
  one shape of opening argument worked for two kinds and failed for the third.

### The spreadsheet MVP, end to end

Driven in the built application rather than only in tests: the folder created on first run
and counted from disk, a file opened by clicking it, a cell changed by hand, a column asked
for in words landing in the right rows with the User's own edit intact, notes kept per file,
then the application quit and relaunched with the work, the conversation and the notes still
there.

| Task | State | Evidence |
|---|---|---|
| Real home folder | Done | `studio-artefacts::home`, 5 tests; `Documents › Work Studio` created on first run, which makes two claims the interface already made true |
| Real folder listing | Done | `/files`; "1 file, 0 folders on your Mac" counted from disk; a file it cannot open is shown but not offered |
| Persistence | Done | `keeper.rs`, 9 tests including one that reopens the store as after a restart; migration `0002_thread_turns` |
| Your work | Done | `/threads`; the sidebar lists pieces of work the User has done, and clicking one reopens its file |
| Conversation kept | Done | turns stored per piece of work and loaded when it is reopened |
| Steering, real | Done | `/steering`; every note shows where it came from; anything derived is asked as a question and influences nothing until accepted |
| Editing by hand | Done | `/edit` through the same gate and the same server as an agent's change; a typed number stays a number |
| Honest figures | Done | the Dashboard reports unavailable rather than inventing; the route switcher no longer appears in the desktop application |

Three bugs this found, each of which silently lost the User's work:

- **`zavora-xlsx` discarded any change to a cell that already had a value.** The parsed
  original was inserted over the authored write, so adding a cell worked and changing one did
  not, with a success message either way. It had already bitten us unnoticed: an earlier run
  reported renaming a header and the file still held the old text. One line; the engine's own
  567 tests still pass.
- **A typed value was sent as text**, so `1999` became a string and every formula referring
  to it broke.
- **The specialist was told only the path**, so it assumed the table began at row 1 when it
  began at row 5 and wrote a column of formulas against empty cells. It is now told what is
  actually in the file.

### Built, but not yet real

A checkbox is a poor record for a screen that exists and reads correctly but shows
invented data. These tasks are not complete, and saying only "open" would understate them
just as marking them done would overstate them. Each is drawn, reachable, and fed from
`shell/src/renderer/fixtures.ts` rather than from the store.

| Task | Drawn | What it still shows | What it needs |
|---|---|---|---|
| 9.2 Dashboard | Yes | `5 / 3 / 11 / $0.62`, all invented | Counts from the store; spend from `spend_ledger`, which now carries real money. A figure we do not have must be reported unavailable, never as zero |
| 9.3 In Tray | Yes | Three fixture items | `tray_items` |
| 9.5 Out Tray | Yes | Three fixture deliveries | `deliveries`, which is why 4.5 is a prerequisite |
| 9.6 Job detail | Yes | Fixture run history and steering notes | `job_runs` and `steering_notes` |
| 9.4 Kickoff | Yes | A fixture manifest, so approving approves nothing | The manifest the gate already produces |
| 12.5 New work | Yes | The "describe it" box does nothing | Task 12.4, the intent router |
| 15.1 Settings | Yes | Accounts, spend limit, folders, tiers — none persist or take effect | Writes, and something that reads them |
| 15.5 Diagnostics | Yes | Fixture activity and build detail | `activity_log`, which exists and is append-only |
| 12.8 / 13.8 / 13.9 editing clients | Yes | The User can select but not type: no cell, paragraph or shape is editable by hand | The one edit path both authors share (Property 23) |

Two statements the interface makes are currently untrue, which ranks above any of the
above: it says "Your files live in Documents › Work Studio on your Mac" and "Folders here
are real folders on your Mac", while that folder does not exist and the 14 files listed are
invented.

The store is the common thread. All 12 tables exist and are tested, and the Core's HTTP
surface writes to none of them, so closing the app forgets everything that happened in it.


Also verified ahead of their tasks: Property 17 (an irreversible delivery cannot claim a reversal window), the four-class tray constraint, one-off Jobs carrying no schedule, and steering scope matching ownership — all enforced by schema `CHECK` constraints rather than application code.

Both guardrails from task 2's checkpoint are now demonstrably working: planting the four labels the original hand-drawn mockup used makes `vocab-lint` exit 1 and fails `property_11`; removing them returns it to `101 strings checked, all clean`.

Phase 0 is complete: task 2's checkpoint conditions are all met — the workspace and shell build, the vocabulary lint fails on a planted term and passes when removed, the payload lint rejects a planted model identifier, and the Core cannot be reached without the bearer token.

Eleven of the 31 correctness properties are now covered by tests: 1, 2, 3, 4, 6, 7, 11, 12, 16, 17, 18, 20, 21, 22, 29 and 30 — sixteen, in fact. All were implementable without ADK-Rust because they are properties of the engine rather than of execution.

Two defects were found and fixed by the tests rather than by review. Tray consolidation originally matched a substring of the item's prose, and SQLite's case-insensitive `LIKE` made the account "X" collide with the word "expired"; the fault now has its own `cause` column and is matched by equality. A `get` query bound no parameter and was rewritten as a single statement.

ADK-Rust is now wired, behind a default-off `adk` feature on `studio-runner`. The decision rules do not depend on it, so the invariants stay testable without the sibling checkout, and `make adk-check` proves the adapter against the real crate. Locally that check takes 5.8s because adk-rust is already built; a cold CI build would be slow and needs a warm cache before this joins the default pipeline.

**A design change this iteration.** An operation absent from the classification table was being *performed* in a `live` Job with an "I don't know how to take this back" fallback. A failing test caught it. That contradicts design principle 4 — autonomy requires reversibility — since Work Studio could neither describe the action nor undo it. `SuppressReason::Unclassified` now refuses such an operation in every state and every mode, and the User is asked instead. Requirement 18.6 and the design's gate section should be updated to say so.

Nineteen of the 32 correctness properties now have passing tests: 1, 2, 3, 4, 5, 6, 7, 11, 12, 14, 15, 16, 17, 18, 20, 21, 22, 29 and 30.

**A defect the tests caught.** The missed-run counter advanced its cursor but held the day boundary fixed at today, so every occurrence between the last run and this morning was invisible — a laptop asleep from Monday to Friday reported one missed newsletter instead of four. `align_day` now advances the boundary and weekday with the cursor using Euclidean division, since Rust's `%` is negative for times before the reference day. The two helper functions that had been standing in for this were doing nothing, which is what made the bug easy to miss on reading.

Next: task 3.5 (the run pipeline, which now has every part it composes), then 9.x (the product surface, whose acceptance frame is the journeys) and 10.x (the newsletter vertical slice).

## Tasks

- [ ] 1. Workspace and guardrail foundations
  - [x] 1.1 Create the Cargo workspace and Core binary skeleton
    - Create `core/` Cargo workspace with crates `studio-core` (binary), `studio-jobs`, `studio-tray`, `studio-artefacts`, `studio-router`, `studio-connectors`, `studio-store`
    - Add `adk-runner`, `adk-agent`, `adk-core`, `adk-session`, `adk-artifact`, `adk-tool` (mcp feature), `adk-skill`, `adk-graph`, `adk-guardrail` as dependencies
    - Enable the `standard` feature tier on `adk-rust` so OpenAI is available; verify Gemini-only `minimal` default is not in effect
    - Verify `cargo check` passes with an empty `main`
    - _Requirements: 14.2, 18.1_
  - [x] 1.2 Create the Electron shell skeleton with no Node integration in the renderer
    - Create `shell/` with main process, preload bridge, and a React + Vite renderer
    - Configure `contextIsolation: true`, `nodeIntegration: false`, and a strict content security policy
    - Implement Core process supervision: spawn on an OS-assigned port, mint a per-process bearer token, terminate on quit, restart on unexpected exit
    - Verify the renderer can reach a Core health endpoint only with the token
    - _Requirements: 16.2, 18.2_
  - [x] 1.3 Implement the encrypted local store and migrations
    - Implement `studio-store` over SQLite with at-rest encryption and a key held in the OS keychain
    - Write the initial migration covering `jobs`, `job_runs`, `tray_items`, `steering_notes`, `artefacts`, `artefact_changes`, `deliveries`, `connectors`, `activity_log`, `spend_ledger` per the design data model
    - Add a database trigger or store-level guard rejecting `UPDATE` and `DELETE` on `activity_log`
    - _Requirements: 3.4, 7.6, 16.1, 16.4_
  - [x]* 1.4 Write property test: Activity log is append-only
    - **Property 16: Activity log append-only**
    - For any sequence of store operations, no `activity_log` row is ever updated or deleted
    - **Validates: Requirement 7.6**
  - [x] 1.5 Implement the ordered, resumable event stream
    - Implement a single Core→renderer event stream with monotonic sequence numbers and resume-from-sequence
    - Implement renderer reconnection that replays missed events, so a reload never loses tray items or in-progress work
    - _Requirements: 3.4, 6.3, 19.5_
  - [x] 1.6 Build the vocabulary lint and wire it into the build
    - Extract all User-visible strings into a single catalogue module; forbid inline literals in components via lint rule
    - Implement the prohibited-term scanner over the catalogue using the Requirement 1.1 term list
    - Fail the build on any match; add the check to CI and to the local pre-commit hook
    - **Property 11: Vocabulary containment** — no string in the User-visible catalogue matches the prohibited-term list
    - _Requirements: 1.1, 1.2_
  - [x] 1.7 Build the renderer payload lint
    - Define the renderer-facing view types (`JobView`, tray item views, artefact views) with no agent, model, provider, server or tool fields
    - Implement a schema assertion over every non-diagnostic Core→renderer payload rejecting such identifiers
    - **Property 12: Renderer concept containment** — no non-diagnostic payload crossing to the renderer carries an agent, model, provider, server or tool identifier
    - _Requirements: 1.6, 3.4, 14.7_

- [ ] 2. Checkpoint — guardrails demonstrably work
  - Confirm `cargo check` and the renderer build both pass
  - Confirm the vocabulary lint fails the build when a prohibited term is deliberately introduced, then passes when removed
  - Confirm the renderer payload lint fails when a model identifier is deliberately added to a view type
  - Confirm the Core cannot be reached without the bearer token
  - Ask the user to review the guardrail behaviour before product work begins

- [ ] 3. Job engine core
  - [x] 3.1 Implement the Job model and closed state machine
    - Implement `JobKind` (`scheduled` / `one_off`) and `JobState` with a transition function that accepts only the enumerated transitions for that kind and rejects all others without mutating state
    - Implement Job creation from a template (`scheduled`) and from a User intent in New work (`one_off`), and Job update for purpose, schedule and steering
    - Implement the `read_only` determination — a Job whose composition contains no `external_effect` tool and produces no Artefact — and the `draft`→`live` activation path that skips Kickoff for such Jobs
    - Implement `out_tray_policy` (`always` / `on_change`), defaulting monitoring templates to `on_change`, and `output_folder` for Jobs that write Artefacts
    - _Requirements: 3.1, 3.2, 3.3, 3.7, 4.4, 5.7, 7.7, 12.10_
  - [x]* 3.2 Write property tests: state machine and read-only exemption
    - **Property 4: Closed transition set per kind** — every applied transition belongs to the enumerated set for that Job's kind, and no `scheduled` state is reachable by a `one_off` Job or vice versa. **Validates: Requirement 3.7**
    - **Property 19: Read-only jobs never gate** — for any Read_Only_Job, no `kickoff` tray item is created and its composition contains no `external_effect` tool. **Validates: Requirement 5.7**
  - [x] 3.3 Implement the side-effect gate
    - Implement the tool classification table (`read` / `local_write` / `external_effect`) authored in Work Studio, keyed by server and tool name
    - Implement the gate as an ADK-Rust `ToolConfirmationHandler`: permit `read` always, permit `local_write` with a change-log entry, permit `external_effect` only when the Job is `live`
    - In `kickoff_dry_run` mode, serialise the intended external action into the review payload and suppress execution
    - Emit an `IntendedActionManifest` for Jobs whose output is actions rather than a document: per-row verb, plain-language description, affected count, reversibility, optional inspectable content handle
    - Parse but ignore any `autoApprove` flag present in Capability_Layer MCP configuration
    - Produce a `Reversibility` descriptor with an optional expiry window for every permitted `external_effect`
    - _Requirements: 5.2, 5.8, 7.3, 7.4, 7.8, 18.3, 18.4_
  - [x]* 3.4 Write property tests: Side-effect gate invariants
    - **Property 1: No unauthorised external effect** — for any run in `kickoff_dry_run`, the set of performed `external_effect` operations is empty. **Validates: Requirements 5.2, 18.3**
    - **Property 2: Auto-approval is never authorisation** — gate decisions are identical with and without `autoApprove` present. **Validates: Requirement 18.4**
    - **Property 17: Reversal honesty** — no `irreversible` delivery offers reversal and every offered reversal has a `reversible` or `partial` descriptor. **Validates: Requirements 7.3, 7.4**
    - **Property 32: Nothing unclassified is ever performed** — an operation absent from the table is refused in every state and mode. **Validates: Requirements 18.7, 18.8**
  - [x] 3.5 Implement the run pipeline
    - Implement the seven pipeline stages: acquire lease, assemble context, resolve tier, execute via `Runner`, gate side effects, record in one transaction, route result
    - Implement the per-Job lease so runs of the same Job never overlap
    - Record `job_runs`, Artefacts, Deliveries, Spend and Activity_Log entries transactionally
    - _Requirements: 3.4, 9.4, 15.3_
  - [x]* 3.6 Write property test: Run exclusivity
    - **Property 5: Run exclusivity**
    - For any Job under concurrent trigger pressure, no two Job_Runs have overlapping start and finish intervals
    - **Validates: Requirement 9.4**

- [ ] 4. Tray subsystem
  - [x] 4.1 Implement the durable decision queue
    - Implement enqueue, list, and resolve for the four tray classes — `kickoff`, `escalation`, `finding`, `attention` — with durable persistence
    - Implement resolution recording without expiry, auto-approval, or silent discard
    - Implement `finding` dismissal that does not alter the raising Job's state
    - Implement consolidation so a Connector fault affecting n Jobs raises exactly one `attention` item
    - _Requirements: 6.1, 6.3, 6.4, 6.10, 13.8_
  - [x]* 4.2 Write durability and taxonomy tests
    - **Property 3: Tray durability** — kill the Core mid-write at each pipeline stage; assert the unresolved tray item set after restart equals the set before termination. **Validates: Requirements 3.4, 6.3**
    - **Property 18: Finding is never a fault** — no Job that raised a `finding` has its state altered by it. **Validates: Requirements 6.1, 6.10**
    - **Property 22: Consolidated connector faults** — exactly one `attention` item per Connector fault. **Validates: Requirement 13.8**
  - [ ] 4.3 Implement Kickoff_Review resolution paths
    - Implement `approved` → `live`; `rejected` with comment → `draft` plus a Steering_Note; `approved_with_edits` → `live` plus a candidate note; `approved_with_exclusions` → perform retained rows plus a candidate note per exclusion; `approved_once` → perform the batch and remain in `draft`
    - _Requirements: 5.3, 5.4, 5.5, 5.9, 5.10_
  - [x]* 4.4 Write property test: Manifest fidelity
    - **Property 20: Manifest fidelity**
    - The set of actions performed on approval equals the set of non-excluded rows shown to the User, and no action outside the manifest is performed
    - **Validates: Requirements 5.8, 5.9**
  - [ ] 4.5 Implement Out Tray and delivery records
    - Implement reverse-chronological delivery listing filterable by Job, with reversal action where the descriptor permits it and an unexpired window
    - Implement withdrawal of the reversal action when its window lapses, retaining a statement that it can no longer be undone
    - Implement quiet-run collapsing for Jobs with `out_tray_policy = on_change`
    - Implement non-blocking inline steering, presented as a separate action from reversal
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9_

- [ ] 5. Steering store
  - [x] 5.1 Implement Steering_Notes
    - Implement ordered, per-Job notes with origin, confirmed flag, active flag and sequence; support add, reword, deactivate, delete
    - Implement Global_Steering_Notes with a scope of `everything` or a single Artefact kind, stored in the same table with a null Job reference
    - Implement the candidate-note pattern for the three derived origins — edit-and-approve, manifest exclusion, escalation choice — each confirmed before it is applied and never stored silently
    - Implement resolution order: matching global notes first, then per-Job notes, most recent last, so per-Job notes win without a separate precedence rule
    - Implement conflict detection surfaced in the list rather than resolved silently
    - _Requirements: 8.1, 8.2, 8.3, 8.5, 8.6, 8.7, 8.9, 8.10, 5.4, 5.9, 6.9_
  - [x]* 5.2 Write property tests: Steering invariants
    - **Property 6: Steering visibility** — every stored preference influencing a run appears in the User-visible list. **Validates: Requirement 8.4**
    - **Property 7: Steering recency** — for conflicting notes, the greater sequence governs the output. **Validates: Requirement 8.6**
    - **Property 21: No unconfirmed derived preference** — no note with `confirmed = 0` influences any run. **Validates: Requirements 5.4, 8.4**
    - **Property 28: Steering precedence** — where a per-Job and a global note conflict, output reflects the per-Job note. **Validates: Requirement 8.9**
    - **Property 29: Global steering visibility** — every global preference influencing a run appears in the Settings list with its scope. **Validates: Requirements 8.8, 8.10**

- [ ] 6. Model router and spend accounting
  - [x] 6.1 Implement Quality_Tier routing with failover
    - Port the ordered-chain model, `provider/model` factory and `FallbackOutcome` distinction from `adk-gateway` (`config.rs` `CategoryConfig`, `model_factory.rs`, `fallback_chain.rs`) into `studio-router`, discarding channels, RBAC and JWT
    - Implement the three tiers with OpenAI defaults and a single global cost-versus-quality preference
    - Implement failover on failure or rate limit, recording the failover in the Activity_Log without surfacing it as an error
    - _Requirements: 14.1, 14.2, 14.3, 14.4, 14.5_
  - [x]* 6.2 Write property test: Failover transparency
    - **Property 14: Failover transparency**
    - A unit of work completed via failover produces the same User-visible outcome class as one completed on the primary, and is recorded in the Activity_Log
    - **Validates: Requirements 14.4, 14.5**
  - [x] 6.3 Implement spend accounting across all model usage
    - Wrap every model call regardless of caller so proactive, documents and internal classification usage are all attributed
    - Implement daily aggregation, per-Job attribution and the daily limit with proactive pause on breach
    - _Requirements: 15.1, 15.2, 15.3, 15.4, 15.5_
  - [x]* 6.4 Write property test: Spend completeness
    - **Property 15: Spend completeness**
    - The sum of `spend_ledger` entries for a period equals total model usage cost for that period across all surfaces
    - **Validates: Requirement 15.3**

- [ ] 7. Proactive engine
  - [x] 7.1 Implement durable scheduling
    - Implement User-term schedules (time-of-day, weekdays, interval) as authoritative, with a derived cron form never displayed
    - Implement SQLite-backed schedule and run-history persistence rather than relying on ADK-Rust's in-memory cron stores
    - Implement next-run computation in the User's local time zone
    - _Requirements: 9.1, 9.2, 9.7, 18.5_
  - [x] 7.2 Implement missed-run and failure policy
    - Implement `run_once_on_wake` and `skip_to_next` evaluation on start and on host wake
    - Implement transient retry with backoff to the Job limit; no retry for user-resolvable failures; pause after three consecutive identical failures
    - _Requirements: 9.3, 9.5, 9.6, 17.6_
  - [ ] 7.3 Implement manual run
    - Implement run-now that does not alter the schedule
    - _Requirements: 9.8_
  - [ ]* 7.4 Write durability test: Missed-run policy across sleep and restart
    - Simulate host sleep spanning one or more scheduled times and assert each policy behaves as declared
    - **Validates: Requirements 9.2, 9.3**

- [ ] 8. Checkpoint — engine complete, no product surface yet
  - Confirm all property tests from tasks 3–7 pass
  - Confirm a scripted Job can be created, dry-run, approved, scheduled, executed, recorded and steered entirely through the Core with no interface
  - Ask the user to confirm the engine semantics before interface work begins

- [ ] 9. Product surface — Dashboard and trays
  - [ ] 9.1 Implement the design system and shell chrome
    - Implement typography, colour roles, spacing and card primitives per `mockups/mockups.html`
    - Implement the persistent left panel: **New work**, **Dashboard**, the *Your work* thread list, and **Settings** at the foot; assert in a test that no other top-level destination exists
    - Implement thread status indicators as colour plus a distinguishable glyph shape — working, scheduled, needs you, done, paused — with hover and focus both revealing the concrete next fact, and the same string as the item's accessible name
    - Implement collapsible left and right rails with a focus mode that collapses both, keeping status glyphs visible in the collapsed left strip
    - Ensure Job_State and tray class are never conveyed by colour alone; each tray class has a distinct icon shape, label and edge treatment
    - _Requirements: 1.5, 6.2, 21.1, 21.2, 21.3, 21.4_
    - _Journey: J11_
  - [ ] 9.2 Implement the Dashboard
    - Implement the metric strip: working for you, waiting on you, done today, cost today
    - Implement the Job card grid with purpose, state pill and next run in human terms; a Connector-caused pause names the account on the pill
    - Apply the sub-cent money formatting rule
    - _Requirements: 3.2, 9.7, 13.8, 15.1, 15.7_
    - _Journey: J1, J5_
  - [ ] 9.3 Implement the In Tray
    - Implement all four item classes — `kickoff`, `escalation`, `finding`, `attention` — with distinct, non-colour-dependent treatments so that a first-time approval is not confusable with a fault and a Finding is not presented as a fault
    - Implement inline choice actions for escalations, and dismissal for Findings
    - Implement non-modal presentation, Dashboard count, and assistive-technology announcement without focus theft
    - Implement escalation presentation stating what was attempted, what was uncertain, and the available choices
    - _Requirements: 6.1, 6.2, 6.5, 6.7, 6.8, 6.10, 21.5_
    - _Journey: J4, J5, J9_
  - [ ] 9.4 Implement the Kickoff_Review view in both shapes
    - Output shape: present the full output the Job would have delivered, framed as reviewing work rather than granting permission
    - Manifest shape: present the `IntendedActionManifest` with per-row description, affected count, reversibility marker, drill-in to inspectable content, and per-row exclusion
    - Implement all five resolutions including `approved_once` and `approved_with_exclusions`, with candidate-note confirmation microcopy
    - _Requirements: 5.3, 5.4, 5.5, 5.6, 5.8, 5.9, 5.10, 5.11_
    - _Journey: J1, J3_
  - [ ] 9.5 Implement the Out Tray
    - Implement the reverse-chronological feed with per-Job filtering, reversal where permitted and unexpired, explicit plain marking where not, quiet-run collapsing, and inline non-blocking steering presented separately from reversal
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.7, 7.8, 7.9_
    - _Journey: J2, J8_
  - [ ] 9.6 Implement Job detail with the steering list
    - Implement purpose, human schedule, state with only valid actions, run history as plain sentences, produced files, per-Job spend, and the editable Steering_Notes list
    - Implement the re-baseline offer when purpose or schedule changes materially
    - _Requirements: 3.2, 6.6, 8.3, 15.2_
  - [ ] 9.7 Implement failure presentation
    - Implement the four error classes: recovered (silent), user_actionable (named account plus one next action), job_failed (one plain sentence), internal (reassuring, no detail)
    - _Requirements: 17.1, 17.2, 17.3, 17.4_

- [ ] 10. Vertical slice — Daily newsletter, end to end
  - [ ] 10.1 Mount the newsletter capability servers
    - Mount `mcp-news` over stdio for ingestion (real and keyless) and `mcp-email` for delivery
    - Classify every mounted tool in the side-effect table; `send_email` is `external_effect`
    - _Requirements: 18.1, 18.3_
  - [ ] 10.2 Implement the Daily newsletter Job_Template
    - Implement the template with working defaults for sources, tone, send time and recipient, requiring no User input beyond activation
    - Compose the Job with the appropriate Quality_Tier per step: `fast` for source filtering, `balanced` for drafting
    - _Requirements: 4.2, 4.3, 14.1_
  - [ ] 10.3 Implement the Email Connector with consent language
    - Implement connect, scope statement in plain language, health check, and Vault-backed credential storage returning no credential value to the renderer
    - Implement expiry and revocation handling that sets dependent Jobs to `needs_attention` with one consolidated item
    - _Requirements: 13.2, 13.3, 13.4, 13.5, 13.6_
  - [ ]* 10.4 Write credential containment test
    - **Property 13: Credential containment**
    - No credential value appears in any renderer payload, log, Activity_Log entry or export bundle
    - **Validates: Requirements 13.4, 13.5**
  - [ ] 10.5 Implement first-run flow
    - Implement single-credential first run defaulting to OpenAI, with no Connector, schedule, model or storage configuration required
    - Implement key-acquisition assistance: a link to the provider's key creation page and a plain-language statement of what the key is and what it costs
    - Land the User on a Dashboard presenting the template library in an activatable state, never an empty Job list
    - Request a Connector only at the moment of activation, stating what the Job will do with it
    - State when the first scheduled execution will occur on activation, in the User's local time zone
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.7, 2.8, 9.9_
    - _Journey: J1_
  - [ ] 10.6 Acceptance-test the complete loop
    - Activate the template; observe first output produced with zero external effect; review in In Tray; approve; observe the scheduled live run send; find it in Out Tray; attach "too long"; observe the next run reflect it
    - Measure time from first launch to first reviewed output against the 10-minute target
    - _Requirements: 2.6, 5.1, 5.2, 5.3, 7.1, 8.2_

- [ ] 11. Checkpoint — the thesis test
  - Demonstrate the full Phase 1 loop to the user on a clean machine
  - Confirm the vocabulary lint, renderer payload lint and surface inventory test all pass on the shipped surface
  - Ask the user whether the experience justifies proceeding to breadth, and capture any redirection before Phase 3 begins

- [ ] 12. Artefact clients and the Documents surface — spreadsheet first
  - [x] 12.1 Implement the artefact store and change log
    - Implement Artefacts as ordinary files in a User-chosen visible folder defaulting to a conventional documents location, named from the User's intent
    - Implement the ordered author-attributed change log generalising `docx-agent-app/src/oplog.rs` across all three types, with revert-to-sequence
    - Implement content-hash and mtime recording with external edit detection including the editing application where detectable, and the move/rename/delete detection path
    - Implement Artefact derivation links so provenance ("made from Q3 revenue model.xlsx") is a stored relationship rather than display text
    - _Requirements: 11.4, 11.5, 11.6, 12.1, 12.2, 12.3, 12.4, 12.5_
    - _Journey: J6_
  - [x]* 12.2 Write property test: External edit preservation
    - **Property 10: External edit preservation**
    - If an Artefact changed on disk since the last recorded hash, no Studio edit discards those changes
    - **Validates: Requirement 11.6**
  - [x] 12.3 Implement the Spreadsheet agent
    - Mount `worksheet-mcp` and load spreadsheet instructions through `adk-skill`'s `SkillInjector`, seeded from `zavora-cli/.skills/xlsx.md`
    - Implement the conversation-plus-live-preview layout using the excel-agent-app pattern as the UX reference, with direct grid edits round-tripping through the same tool surface
    - _Requirements: 10.5, 11.1_
  - [ ] 12.4 Implement the intent router
    - Implement `fast`-tier classification returning Artefact type and confidence; below threshold ask exactly one outcome-framed question, never naming an agent or engine
    - _Requirements: 10.1, 10.2, 10.3_
  - [x] 12.5 Implement the New work entry state
    - Implement the single intent field, the file drop target, the recurring-work template cards, and the recent-threads list showing last author, editing application where known, and derivation provenance
    - _Requirements: 10.6, 10.7_
    - _Journey: J6_
  - [ ] 12.5b Implement the Documents Repository
    - Implement the Repository as a view over the real folder: folders shown are folders on disk, and create / rename / move apply to disk
    - Implement kind chips (Documents, Decks, Spreadsheets, PDFs) as filters over the current folder, never as folders
    - Implement per-row metadata: changed when and by whom, *Used in* links back to the Jobs that touched the Artefact, derivation source, and version count
    - Implement the scheduled-Job output folder relationship shown on the folder row
    - Implement "Show in Finder" and external-change reconciliation per Requirement 12.5
    - _Requirements: 12.6, 12.7, 12.8, 12.9, 12.10_
  - [ ]* 12.5c Write property tests: Repository fidelity
    - **Property 30: Repository mirrors disk** — every folder shown exists on disk and every folder operation is applied there. **Validates: Requirements 12.6, 12.7, 12.9**
    - **Property 31: No app-only taxonomy** — no Artefact appears under a container absent from disk; kinds never render as folders. **Validates: Requirement 12.7**
  - [x] 12.6 Implement the shared Artefact_Client shell
    - Implement the common shell used by all three clients: selection model, contextual toolbar, change badges with author attribution and per-change reversal, version history panel, and a secondary "open elsewhere" action that is not the primary editing route
    - Implement the optimistic local model and reconciliation: local application first, Core render authoritative on divergence
    - Implement the `Edit_Operation` dispatcher so User edits traverse the same MCP tool surface as Artefact_Agent edits, producing one change log
    - Implement the rule that no direct edit is offered unless it can be expressed as an `Edit_Operation`
    - Implement keyboard-only operation for every direct edit offered
    - _Requirements: 22.3, 22.4, 22.5, 22.6, 22.7, 22.9, 22.11_
  - [x]* 12.7 Write property tests: single edit path
    - **Property 23: Single edit path** — every change, User- or Studio-originated, appears in the change log as an authored `Edit_Operation` and reverts by the same mechanism. **Validates: Requirements 11.4, 22.4**
    - **Property 24: No unloggable edit** — no client offers a direct edit whose effect cannot be expressed as an `Edit_Operation`. **Validates: Requirement 22.5**
  - [x] 12.8 Port the spreadsheet editing client
    - Port `XlsxPreview.tsx`, `useSpreadsheet.ts`, `Ribbon`, `FormattingToolbar`, `ChartPicker`, `PivotWizard`, `ConditionalFormatPanel`, `ValidationBuilder`, `CommentPanel`, `NamedRangeManager`, `ProtectionDialog`, `SheetInspector` and `ChartRenderer` from `excel-agent-app/frontend` onto the shared shell
    - Repoint `toolApi.ts` at the Core's `Edit_Operation` dispatcher; strip `LoginPage`, `AdminDashboard`, `WorkspaceSidebar` and all Postgres/JWT assumptions
    - Verify formula recalculation happens in app via `zavora-xlsx`'s formula engine, with no external application required
    - _Requirements: 22.1, 22.2, 22.3, 22.7_
    - _Journey: J11_
  - [ ]* 12.9 Write property tests: Artefact fidelity for spreadsheets
    - **Property 8: Unedited round-trip** and **Property 9: Edit locality**, using the existing `zavora-xlsx` corpus fixtures
    - **Validates: Requirements 11.2, 11.3**

- [ ] 13. Document and presentation agents and their editing clients
  - [x] 13.1 Implement the Document agent with a fidelity probe
    - Mount `docx-mcp`; load instructions from the existing `docx` and `doc-coauthoring` skills
    - Implement the pre-edit fidelity probe for User-supplied `.docx`: open, save to a temporary file, compare structural inventory; on detected loss, inform the User, offer to work on a copy, and offer to describe the changes instead of making them
    - _Requirements: 11.1, 11.2, 11.7, 11.8_
    - _Journey: J7_
  - [ ]* 13.2 Write fidelity corpus test for user-supplied documents
    - Test against real-world documents exercising the features `zavora-docx/WORKPLAN.md` lists as unimplemented — footnotes, hyperlinks, bookmarks, comments, watermarks, track changes, form fields, protection
    - Assert the probe warns and offers a copy rather than silently dropping content
    - **Validates: Requirement 11.7**
  - [x] 13.3 Implement the Presentation agent
    - Mount `mcp_slides`; load instructions from the existing `pptx` skill
    - Use `render_slide` for the live preview and `lint_design` plus contrast QA before presenting a deck as complete, stating any fix it made rather than applying it silently
    - _Requirements: 10.5, 10.8, 11.1_
    - _Journey: J6_
  - [ ] 13.4 Implement multi-artefact composition
    - Compose Artefact_Agents within a single User-visible task using a `SequentialAgent`, presenting one result
    - _Requirements: 10.4_
  - [x] 13.5 Implement open-from-disk and version history views
    - Implement opening an existing Artefact into the surface and continuing work
    - Implement the plain-language change history view with revert
    - _Requirements: 10.6, 11.5_
  - [x] 13.6 Contribute Render_Node identifiers upstream to the two engines
    - **Already satisfied for documents, and was never a blocker.** `zavora-docx-html`
      emits `data-p="{body-index}"` on every block when `HtmlOptions { editable: true }`,
      and `Document::to_editable_html()` is the entry point. That index is exactly the one
      `update_paragraph_text` accepts, so selection, attribution and per-block edits all
      work today. Verified by running the engine: `<p data-p="0">8. Termination</p>`.
      `Document::page_layout()` additionally gives page geometry and rendered
      header/footer HTML for a paginated view.
    - Still to do for **presentations**: `zavora-slide-layout::to_svg` emits no element
      identifiers, so click-to-edit and change attribution on a slide remain blocked.
    - Add stable `data-node-id` emission to `zavora-slide-layout::to_svg` for every shape, text body, table cell and chart element
    - Add stable `data-node-id` emission to `zavora-docx-html` for every paragraph, run, table cell and list item
    - Guarantee identifier stability across re-renders for unchanged nodes so selection survives an edit and change attribution is computable by node identity
    - Contribute both upstream rather than forking; verify against each engine's existing corpus tests
    - _Requirements: 22.3, 22.6_
    - _Note: this is a prerequisite for 13.7 and 13.8, not an optimisation. Verified absent — grepping both emitters for `id=`, `data-*`, `shape_id` and `element_id` returns nothing._
  - [ ]* 13.7 Write property test: Render node stability
    - **Property 25: Render node stability** — for any Artefact rendered, edited and re-rendered, identifiers of unchanged nodes are unchanged
    - **Property 26: Render fidelity** — the rendered view contains every content node the document model contains
    - **Validates: Requirements 22.1, 22.2, 22.3, 22.6**
  - [x] 13.8 Implement the document editing client
    - Render from `zavora-docx-html::to_html_fragment` plus `css::generate_base_css`, mounted on the shared shell
    - Implement L1 direct editing: text entry and editing, inline and paragraph formatting, lists, tables, and comments, with selection driven by `data-node-id`
    - Implement change badges attributing Work_Studio's own edits with per-change reversal
    - Apply the fidelity guard: block any direct edit affecting content the engine cannot round-trip, with an explanation instead of a lossy write
    - _Requirements: 22.1, 22.2, 22.3, 22.6, 22.10, 22.11_
    - _Journey: J11_
  - [x] 13.9 Implement the presentation editing client
    - Render each slide from `zavora-slide-layout::to_svg` with a thumbnail strip, mounted on the shared shell
    - Implement L1 direct editing: select, move, resize and delete shapes; edit text in place; reorder and duplicate slides; apply theme colours — with hit-testing driven by `data-node-id`
    - Run `lint_design` and contrast QA on change and report any fix made rather than applying it silently
    - _Requirements: 10.8, 22.1, 22.2, 22.3, 22.11_
  - [ ]* 13.10 Write test: lossless direct editing
    - **Property 27: Lossless direct editing** — no direct edit is performed where the engine cannot round-trip the affected content
    - **Validates: Requirements 11.2, 22.10**
  - [ ] 13.11 Verify no external office application is required
    - Confirm every L0 and L1 operation in the ladder is achievable in app for all three types, and that "open elsewhere" appears only as a secondary action
    - _Requirements: 22.1, 22.8, 22.9_

- [ ] 14. Remaining proactive templates and connectors
  - [ ] 14.1 Implement the Calendar Connector
    - Mount `mcp-calendar`; implement consent language and health checks per the Connector model
    - _Requirements: 13.1, 13.2, 13.3, 13.6, 13.7_
  - [ ] 14.2 Implement `sysmon-mcp` as a new headless capability
    - Implement a local system-health MCP server exposing CPU, memory, disk free, battery, uptime and backup status, since `mcp-observability` requires a cloud APM backend and `computer-use-mcp` lacks these metrics
    - Classify all tools as `read`
    - _Requirements: 18.6_
  - [ ] 14.3 Implement `x-mcp` as a new headless capability
    - Implement an X/Twitter MCP server with real API endpoints, post and read-own-timeline scopes, and a deletion path to support reversal, since `mcp-cms` and `mcp-marketing` post to generic paths that do not resolve against the real API
    - Classify posting as `external_effect` with a `reversible` descriptor
    - _Requirements: 13.1, 18.6, 7.3_
  - [ ] 14.4 Implement the remaining eight Job_Templates
    - Inbox triage, computer health monitor, news and competitor monitor, website availability monitor, meeting preparation, expense and invoice capture, morning digest, weekly report roll-up
    - Assign Quality_Tiers per the design table; declare Connector requirements and missed-run policy for each
    - _Requirements: 4.1, 4.2, 4.3, 4.5, 14.1_
  - [ ] 14.5 Implement the social posting template on `x-mcp`
    - Implement drafting at `balanced` tier with Kickoff_Review suppression of the post and Out Tray reversal after going live
    - _Requirements: 4.1, 5.2, 7.3_

- [ ] 19. What a specialist knows — earned knowledge and authored competence

  The product's defensibility is here rather than in the editing, which will not stay a
  differentiator. The signal is already being recorded and discarded: the change log holds an
  author per edit, so the difference between what Work Studio produced and what the User then
  changed is an observed preference. This phase turns that into something the specialists draw
  on, and pairs it with know-how we author once.

  Ordered so the visible half works first. The steering list already claims to be everything
  Work Studio goes on, and today it is fixture text — so the first task is the one that makes
  an existing claim true.

  - [x] 19.1 Make the steering list real, end to end
    - Serve Steering_Notes from the store over the loopback channel: read, add, reword,
      deactivate, delete, for both per-thread and global notes.
    - Replace the fixture list in the Job detail and Settings panes with the real one.
    - Resolve notes in the Core when a run starts, never accept them from the renderer, so
      nothing influences a run that the User cannot see.
    - Test: a note added in the interface changes the next run's instruction; a deactivated
      one does not; per-thread beats global.
    - _Requirements: 8.1, 8.2, 8.3, 8.7, 8.8, 8.9, 8.10_
    - _Properties: 28, 29_

  - [x] 19.2 Record provenance for every note
    - Each note carries where it came from in the User's terms: the Artefact and when, or
      that the User said it directly.
    - Show it in both lists, as the design already draws it.
    - Test: no note can be presented without a provenance the interface can render.
    - _Requirements: 8.11, 8.14_
    - _Properties: 35_

  - [ ] 19.3 Derive observed preferences from the change log
    - Find, in `artefact_changes`, places where Work Studio made a change and the User then
      changed the same thing, and describe the difference as a candidate preference.
    - Derive only from changes whose author is the User, and never from Artefact content.
    - Scope each candidate to the Artefact kind and document class it was seen in.
    - Test: a candidate is produced from a real pair of edits; no candidate is ever produced
      from document text, asserted by feeding a document containing instructions.
    - _Requirements: 8.11, 8.13, 8.16_
    - _Properties: 34, 37_

  - [ ] 19.4 Ask before acting on anything derived
    - Present a candidate as a proposal in the User's words — "You have shortened my summaries
      three times. Shall I keep them under 150 words?" — with accept, reword and dismiss.
    - A candidate influences nothing until accepted; on acceptance it becomes an ordinary
      note in the list.
    - Test: a pending candidate provably changes no run; an accepted one changes the next.
    - _Requirements: 8.4, 8.12_
    - _Properties: 33_

  - [ ] 19.5 Standing and forgetting
    - A recurring correction raises a note's standing; a contradicting one stops it applying
      and says so in the list rather than removing it silently.
    - Test: after a contradicting correction, no subsequent run reflects the note, and the
      list shows why.
    - _Requirements: 8.15_
    - _Properties: 36_

  - [ ] 19.6 Learn from what went wrong
    - Feed reversals, refusals by the gate, and rejected Kickoff_Reviews in as their own kind
      of note, so the same mistake is not repeated.
    - Test: a reversed delivery produces a note that is visible and scoped.
    - _Requirements: 8.11_

  - [ ] 19.7 Keep a thread's account
    - Summarise a continuing thread so returning to it does not begin again; present it as
      something the User can read and delete.
    - Test: a thread reopened after the process ended carries its account.
    - _Requirements: 8.17_

  - [ ] 19.8 Externalise the persona
    - Move each specialist's instruction out of program code into editable content, assembled
      per run, and show it in Settings.
    - Test: editing the content changes the next run's instruction; the User's notes still
      come last.
    - _Requirements: 23.7, 23.5_
    - _Properties: 39_

  - [ ] 19.9 Authored competence, disclosed in two levels
    - Load `SKILL.md` packs from disk with `adk-skill`; carry one catalogue line per pack in
      the instruction and load a body only when the work calls for it.
    - Show each specialist's packs in Settings with a way to turn one off.
    - Say which body of know-how was followed, in the User's terms.
    - Test: a pack added on disk appears without a code change; the instruction grows by one
      line per pack, not by a body; a disabled pack is not offered.
    - _Requirements: 23.1, 23.2, 23.3, 23.6_

  - [ ] 19.10 Know-how cannot widen authority
    - Assert that no enabled pack changes the set of operations a specialist may perform.
    - Test: with every pack enabled, the operation set equals the authored classification
      table exactly.
    - _Requirements: 23.4_
    - _Properties: 38_

  - [ ] 19.11 Similarity search over notes, locally
    - Retrieve notes by meaning rather than by keyword, computed over the notes themselves and
      never over Artefact content, on the User's own computer.
    - Keyword retrieval is the first implementation; this replaces it behind the same
      interface when there is enough remembered for keyword to feel thin.
    - Test: nothing derived from a note leaves the computer; retrieval respects scope.
    - _Requirements: 8.18, 16.1_
    - _Properties: 37, 40_

- [ ] 15. Privacy, Settings and data control
  - [ ] 15.1 Implement Settings as a single screen with six sections
    - Implement **General**: AI key with provider status and replace/add, the cost-versus-quality nudge, launch at login, file location, daily limit with today's usage, and export/delete actions
    - Implement **How I should work**: the Global_Steering_Notes list with scope chips, provenance line per note, an add field with scope selector, and a plain statement that per-thread instructions win
    - Implement **Accounts**, **Files**, **Spending** and **Privacy** sections
    - Confine every provider and model identifier to Settings and diagnostics; a single technical-details link sits at the foot and is referenced from nowhere else
    - _Requirements: 1.6, 8.7, 8.8, 8.9, 14.7, 15.4, 16.6_
    - _Mockups: 21-settings, 22-global-steering_
  - [ ] 15.2 Implement the privacy statement view
    - State in plain language which accounts are connected, what leaves the device, and where it goes, presented as directional flow rows rather than prose
    - Source every provider-specific data-handling claim from a maintained per-provider statement rather than fixed interface copy
    - _Requirements: 16.5, 16.8_
    - _Journey: J10_
  - [ ] 15.3 Implement export-all and delete-all
    - Implement a single export action producing open formats and a single delete action removing all local data and credentials
    - _Requirements: 16.6_
  - [ ] 15.4 Implement local model routing
    - Allow the User to route any tier to a locally hosted model without disabling other capability
    - _Requirements: 16.7_
  - [ ] 15.5 Implement the diagnostics view
    - Surface underlying technical detail for support, reachable only from Settings and referenced from no primary surface
    - _Requirements: 1.6, 17.5_
  - [ ] 15.6 Verify no-telemetry posture
    - Assert no analytics, telemetry or crash reporting transmits User content; any diagnostic transmission is opt-in and off by default
    - _Requirements: 16.2, 16.3_

- [ ] 16. Accessibility and performance
  - [ ] 16.1 Complete keyboard operability and accessible semantics
    - Full keyboard traversal of all five surfaces; accessible names, roles and states for all interactive elements
    - _Requirements: 21.1, 21.2_
  - [ ]* 16.2 Write accessibility test suite
    - Automated contrast checks against WCAG 2.2 AA, keyboard-only traversal tests, and accessible-name coverage assertions
    - **Validates: Requirements 21.1, 21.2, 21.3, 21.4**
  - [ ] 16.3 Meet responsiveness and resource budgets
    - Cold start under 3 s and idle resident memory under 250 MB on the reference machine; visible feedback within 100 ms; no interface blocking on Capability_Layer calls; progress streamed in User terms
    - _Requirements: 19.1, 19.2, 19.3, 19.4, 19.5_

- [ ] 17. Trust gates and release
  - [ ] 17.1 Implement code signing and notarization
    - macOS signing and notarization, Windows signing; verify first launch presents no operating system security warning
    - _Requirements: 20.1_
  - [ ] 17.2 Implement the authenticated update channel
    - Authenticated delivery with integrity verification before applying
    - _Requirements: 20.2_
  - [ ] 17.3 Publish the software bill of materials
    - Generate and publish an SBOM per release; ensure CI provisions the sibling path dependencies the Capability_Layer requires
    - _Requirements: 20.3_
  - [ ] 17.4 Verify at-rest encryption in the released build
    - Confirm Requirement 16.4 holds in the packaged artefact, not only in development
    - _Requirements: 16.4, 20.4_
  - [ ] 17.5 Release gate
    - Block release until tasks 17.1 through 17.4 are satisfied
    - _Requirements: 20.5_

- [ ] 18. Final checkpoint — full acceptance
  - Walk all ten journeys in `journeys.md` end to end on a clean machine, confirming each reads as written
  - Confirm all property tests, enforcement lints, durability tests, fidelity tests, accessibility tests and performance budgets pass
  - Confirm all ten Job_Templates activate, produce a Kickoff_Review where applicable, go live, appear in the Out Tray, and respond to steering
  - Confirm read-only templates go live without a Kickoff_Review and raise Findings rather than faults
  - Confirm the left panel still contains exactly New work, Dashboard, the thread list and Settings, and that no developer-facing surface has re-entered the product
  - Ask the user for release sign-off
