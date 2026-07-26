# Requirements Document

## Introduction

Zavora Work Studio is a privacy-first desktop application that gives non-technical knowledge workers a team of AI agents that do real work on their behalf. It is built on ADK-Rust and delivered as a signed desktop binary so that documents, credentials, schedules and history stay on the user's machine.

The product rests on three pillars:

1. **End-user productivity, not developer tooling.** The user is a non-technical professional. No agent, model, protocol, or infrastructure concept is ever exposed in the interface.
2. **Standard artefact agents.** Specialist agents for word-processing documents, presentations and spreadsheets that create and iteratively co-edit real files with the user.
3. **Proactive work.** Jobs that run on a schedule without being asked — daily newsletter, social posting, inbox triage, system monitoring and other recurring work — governed by a one-time human approval followed by a lightweight steering loop.

This specification is deliberately written UX-first. A prior in-house attempt (`adk-desktop`) achieved strong engineering depth but surfaced developer concepts — terminals, git worktrees, sandbox permission modes, agent graphs, a 51 KB MCP configuration panel — and shipped none of the ten target end-user jobs. Work Studio is a ground-up rebuild of the entire product surface on top of the proven headless capability layer. Technology polish is explicitly sequenced after the user loves the product.

### Scope of this spec

Version 1 of Zavora Work Studio: the desktop shell, the Job abstraction, the In Tray / Out Tray approval and steering loop, the Proactive engine with its template library, the unified Documents surface backed by three artefact agents, silent multi-provider model routing defaulting to OpenAI, and the trust gates required to ship a privacy product.

### Out of scope for v1

Multi-user collaboration and real-time co-presence; mobile and web clients; a marketplace for third-party jobs; team/enterprise administration, RBAC or multi-tenancy; user-authored agents, prompt editing, or graph authoring surfaces; voice interaction; any developer-facing surface (terminals, sandboxes, worktrees, tracing UIs, protocol inspectors).

## Glossary

- **Work_Studio**: The Zavora Work Studio desktop application as a whole.
- **User**: A non-technical knowledge worker operating Work_Studio. The only human role in v1.
- **Job**: The single unit of work the User understands and manipulates. One thread in the interface. A Job has a plain-language purpose, a **kind** — `scheduled` or `one_off` — an optional schedule, a state, a history of Job_Runs, produced Artefacts, and an accumulated set of Steering_Notes. Proactive work is a `scheduled` Job; work the User starts from New work is a `one_off` Job. Jobs are the only container of work exposed to the User.
- **Repository**: The User-facing view of every Artefact Work_Studio has produced or edited. It is a view *over* the real folder on disk, never a separate store: folders shown are real folders, and kinds (Documents, Decks, Spreadsheets, PDFs) are filters over them rather than folders of their own.
- **Global_Steering_Note**: A Steering_Note that applies across all Jobs rather than to one, optionally scoped to an Artefact kind. Held in Settings, visible and editable on the same terms as per-Job notes. Per-Job notes take precedence.
- **Job_Template**: A pre-built, ready-to-activate Job definition shipped with the product (e.g. "Daily newsletter", "Inbox triage"). Templates remove the empty state and the configuration ritual from first run.
- **Job_Run**: A single execution of a Job, with a start time, an outcome, produced Artefacts, and a Delivery record.
- **Job_State**: One of `draft`, `awaiting_kickoff`, `live`, `paused`, `needs_attention`, `retired`.
- **Kickoff_Review**: The one-time human review of a Job's first output. Approving a Kickoff_Review transitions the Job to `live`, after which subsequent runs execute without blocking on the User.
- **In_Tray**: The queue of items the User must see: Kickoff_Reviews, escalations from Jobs that were uncertain, Findings, and Jobs in `needs_attention`.
- **Finding**: An item raised by a Job that executed successfully and discovered something the User should know, where nothing is broken and no decision is owed to the Job. Monitoring Jobs produce Findings as their normal output.
- **Intended_Action_Manifest**: The reviewable form of a Kickoff_Review for a Job whose output is a set of actions rather than a document. Each row states in plain language what would be done, how many items it affects, and whether it can be undone, and each row can be excluded by the User.
- **Read_Only_Job**: A Job whose executions can produce no external effect and no Artefact. Monitoring Jobs are Read_Only_Jobs.
- **Out_Tray**: The reverse-chronological record of completed proactive work, shown for visibility rather than approval, with reversal offered where the underlying action is reversible.
- **Steering_Note**: A short natural-language correction the User attaches to a Job or a Job_Run ("too long", "drop the crypto section"). Steering_Notes are durable, User-visible, User-editable, and injected into subsequent Job_Runs of that Job.
- **Artefact**: A real file produced or edited by Work_Studio — a `.docx`, `.pptx`, `.xlsx` or `.pdf` — stored in a User-visible folder on the local file system.
- **Artefact_Agent**: One of three specialist agents (Document, Presentation, Spreadsheet) that create and iteratively edit Artefacts. Artefact_Agents are an internal division of labour and are not named or selected by the User.
- **Artefact_Client**: The in-app view of an Artefact. It renders the Artefact faithfully from the file itself and accepts direct User edits. There is one Artefact_Client per Artefact type, all sharing a common shell, selection model and edit path.
- **Render_Node**: An element of a rendered Artefact that carries a stable identifier traceable to a specific node of the underlying document model. Render_Nodes are what make selection, direct editing and change attribution possible.
- **Edit_Operation**: A single change to an Artefact, expressed in the same operation vocabulary the Artefact_Agent uses. Both User edits and Work_Studio edits are Edit_Operations, which is why both appear in one change history and both can be reverted the same way.
- **Documents_Surface**: The single User-facing surface for creating and editing Artefacts. It accepts a plain-language intent and silently routes to the correct Artefact_Agent.
- **Proactive_Engine**: The subsystem that schedules, triggers, executes, retries and records Job_Runs for Jobs that have a schedule.
- **Capability_Layer**: The reused headless subsystems that carry no User interface: the document engines (`zavora-docx`, `zavora-slide`, `zavora-xlsx`), the MCP servers, and the ADK-Rust crates.
- **Connector**: A User-facing representation of an external account Work_Studio can act through (Email, Calendar, X). One Connector maps to one or more Capability_Layer servers plus a credential held in the Vault.
- **Model_Router**: The subsystem that selects a language model for each unit of work from a Quality_Tier policy, without User involvement, and fails over to an alternate provider when the primary is unavailable.
- **Quality_Tier**: An ordered, provider-agnostic classification of work — `fast`, `balanced`, `best` — that the Model_Router resolves to a concrete provider and model.
- **Vault**: The local, OS-keychain-backed store for all credentials (model provider keys and Connector tokens).
- **Activity_Log**: The append-only, local record of every action Work_Studio took on the User's behalf, including which Connector was used and what was sent.
- **Trust_Gate**: A release requirement whose absence would contradict the product's privacy proposition — code signing, notarization, an authenticated update channel, and encryption of local data at rest.
- **Spend**: The User-visible cumulative cost of model usage over a period.

## Requirements

### Requirement 1: User-Facing Vocabulary and Concept Boundary

**User Story:** As a User with no technical background, I want the application to speak only about my work and never about its own machinery, so that I can use it without learning any new concepts.

#### Acceptance Criteria

1. THE Work_Studio interface SHALL NOT render any of the following terms in any User-visible string: agent, model, provider, LLM, token, prompt, MCP, server, tool, tool call, session, run (as a noun for execution), invocation, artefact (as a system term), checkpoint, graph, sandbox, protocol, API, JSON, schema, or crate. _Note: "AI" is permitted as a User-facing word where it names the thing the User is paying for, as in "Your AI key"._
2. THE Work_Studio build SHALL fail IF any string in the User-visible string catalogue matches the prohibited-term list defined in Requirement 1.1.
3. WHEN Work_Studio must refer to an external account, THE interface SHALL name the account in the User's terms (for example "Gmail", "Google Calendar", "X") and never the underlying Connector implementation.
4. WHEN Work_Studio must refer to a file it produced, THE interface SHALL refer to it by its file name and type as the User would ("Q3 board deck.pptx"), not as a system object.
5. THE Work_Studio interface SHALL NOT expose any surface for authoring, inspecting or editing agent definitions, prompts, orchestration graphs, or server configuration.
6. WHERE a technical detail is required for support or debugging, THE Work_Studio interface SHALL confine it to a single diagnostics view reachable only from Settings, and SHALL NOT reference it from any primary surface.

### Requirement 2: First-Run Experience

**User Story:** As a new User, I want to get real value within my first few minutes without configuring anything, so that I understand what the product does before I invest any effort.

#### Acceptance Criteria

1. THE Work_Studio first-run flow SHALL require exactly one credential from the User: a model provider key, defaulting to OpenAI.
2. THE Work_Studio first-run flow SHALL NOT require the User to configure any Connector, schedule, model, or storage location before reaching a working state.
3. WHEN first-run completes, THE Work_Studio Dashboard SHALL present the Job_Template library in an activatable state and SHALL NOT present an empty Job list.
4. WHEN a User activates their first Job_Template, THE Work_Studio SHALL produce a first output for Kickoff_Review without requiring any further configuration IF that template's required Connectors are already satisfied or the template requires none.
5. IF a Job_Template requires a Connector that is not yet configured, THEN THE Work_Studio SHALL request only that Connector, at the moment of activation, in the User's language, and SHALL state what the Job will do with it.
6. THE Work_Studio SHALL reach a state where the User has reviewed a real first output within 10 minutes of first launch, measured on a Job_Template requiring no Connector.
7. THE Work_Studio first-run flow SHALL NOT present a tour, tutorial, or multi-screen onboarding wizard as a precondition to use.
8. WHERE the User does not already hold a model provider key, THE Work_Studio first-run flow SHALL provide direct assistance in obtaining one, including a link to the provider's key creation page and a plain-language statement of what the key is and what it will cost. _Rationale: journey J1 measured key acquisition as four of the six minutes to first value; it is the largest single obstacle and it is not our software._

### Requirement 3: The Job Abstraction

**User Story:** As a User, I want everything the product does for me to appear as a single, consistent kind of thing I can read, pause and correct, so that I always know what is happening on my behalf.

#### Acceptance Criteria

1. THE Work_Studio SHALL represent every unit of ongoing work as a Job, and SHALL expose no other container of work to the User.
2. THE Work_Studio SHALL support two Job kinds — `scheduled` for recurring proactive work and `one_off` for work the User starts directly — and SHALL present both in a single unified list distinguished only by state, not by kind. _Rationale: separating them would create two kinds of work in the User's mind and duplicate history, steering, reversal and spend attribution for each._
3. THE Work_Studio SHALL expose exactly the following Job attributes to the User: plain-language purpose, schedule in human terms where one exists, Job_State, last outcome, next scheduled time where one exists, produced Artefacts and Deliveries, and accumulated Steering_Notes.
4. THE Work_Studio SHALL NOT expose to the User any of the following Job attributes: agent identity, model or Quality_Tier selection, server or tool identifiers, run counts, token counts, retry policy, or execution topology.
5. THE Work_Studio SHALL persist every Job and its complete Job_Run history to local storage such that all Jobs, Job_States, Steering_Notes and pending In_Tray items survive application restart and unexpected termination.
6. WHEN a Job's underlying execution fails in a way the User must know about, THE Work_Studio SHALL set the Job to `needs_attention` and place a single item in the In_Tray.
7. THE Work_Studio SHALL support Job_State transitions only along the following paths. For `scheduled` Jobs: `draft`→`awaiting_kickoff`, `awaiting_kickoff`→`live`, `awaiting_kickoff`→`draft`, `live`→`paused`, `live`→`needs_attention`, `paused`→`live`, `paused`→`awaiting_kickoff`, `needs_attention`→`live`, `needs_attention`→`paused`. For `one_off` Jobs: `active`→`finished`, `active`→`needs_attention`, `needs_attention`→`active`, `finished`→`active`. From any state: →`retired`.
8. THE Work_Studio SHALL indicate every Job's state in the unified list using colour together with a distinguishable glyph shape, and SHALL make the state's concrete meaning available as text on hover, on keyboard focus, and as the item's accessible name.

### Requirement 4: Job Template Library

**User Story:** As a User, I want a set of ready-made jobs covering the work I actually repeat, so that I can start by choosing rather than by building.

#### Acceptance Criteria

1. THE Work_Studio SHALL ship with at least the following ten Job_Templates: daily newsletter, social posting, inbox triage, computer health monitor, news and competitor monitor, website availability monitor, meeting preparation, expense and invoice capture, morning digest, and weekly report roll-up.
2. FOR EACH Job_Template, THE Work_Studio SHALL declare the Connectors it requires and SHALL present that requirement to the User in plain language before activation.
3. WHEN a User activates a Job_Template, THE Work_Studio SHALL create a Job in `draft` state pre-populated with a working configuration, and SHALL NOT require the User to supply any value that has a defensible default.
4. THE Work_Studio SHALL allow a User to edit a Job's purpose, schedule and Steering_Notes after activation without re-running Kickoff_Review, except as required by Requirement 6.6.
5. THE Work_Studio SHALL allow the User to activate the same Job_Template more than once as independent Jobs.

### Requirement 5: Kickoff Review

**User Story:** As a User, I want to review a job's first real output once before it starts working on its own, so that I can trust it without approving every future action.

#### Acceptance Criteria

1. WHEN a Job in `draft` is started, THE Work_Studio SHALL execute one Job_Run in a mode where no external side effect is performed, and SHALL place the resulting output in the In_Tray as a Kickoff_Review.
2. WHILE a Job is in `awaiting_kickoff`, THE Work_Studio SHALL NOT send, post, delete, or otherwise cause any externally visible effect on the User's behalf for that Job.
3. WHEN a User approves a Kickoff_Review, THE Work_Studio SHALL transition the Job to `live` and SHALL schedule subsequent Job_Runs to execute without blocking on the User.
4. WHEN a User edits the output during Kickoff_Review and then approves, THE Work_Studio SHALL derive a Steering_Note from the difference between the produced and the approved output and SHALL attach it to the Job.
5. WHEN a User rejects a Kickoff_Review with a comment, THE Work_Studio SHALL return the Job to `draft`, attach the comment as a Steering_Note, and offer to produce a new first output.
6. THE Work_Studio SHALL present a Kickoff_Review as a review of the User's work product, not as a permission dialog, and SHALL show the full output the Job would have delivered.
7. WHERE a Job is a Read_Only_Job, THE Work_Studio SHALL exempt it from Kickoff_Review and SHALL transition it directly from `draft` to `live` on activation, confirming to the User that it is now watching and will interrupt only when something needs them. _Rationale: journey J4 established that asking a User to approve "nothing is wrong" trains them to treat reviews as noise, which destroys the trust model the Kickoff_Review exists to build._
8. WHERE a Job's output is a set of actions rather than a document, THE Work_Studio SHALL present the Kickoff_Review as an Intended_Action_Manifest in which each row states what would be done, how many items it affects, and whether it can be undone, and each row can be excluded by the User.
9. WHEN a User excludes one or more rows of an Intended_Action_Manifest and approves, THE Work_Studio SHALL perform only the retained rows and SHALL offer a candidate Steering_Note derived from each exclusion for the User to confirm.
10. THE Work_Studio SHALL offer a resolution that performs the reviewed work once and leaves the Job in `draft`, distinct from approving it for future execution.
11. WHEN a Kickoff_Review contains produced content that is not fully shown in the manifest, THE Work_Studio SHALL allow the User to open and read that content before resolving.

### Requirement 6: In Tray

**User Story:** As a User, I want one place that tells me what needs my decision, so that nothing waits on me invisibly and nothing interrupts me unnecessarily.

#### Acceptance Criteria

1. THE Work_Studio SHALL present an In_Tray containing exactly four classes of item: Kickoff_Reviews, escalations from Jobs that could not proceed confidently, Findings from Jobs that executed successfully and discovered something the User should know, and Jobs in `needs_attention`.
2. THE Work_Studio SHALL visually distinguish all four In_Tray classes from one another by label, icon shape and edge treatment, such that a first-time approval is not confusable with a fault, and a Finding is not presented as a fault. _Rationale: journey J4 established that presenting a successful monitoring result as `needs_attention` misreports Work_Studio as broken when it is working correctly._
3. THE Work_Studio SHALL persist all In_Tray items durably, such that no item is lost across application restart, unexpected termination, or host reboot.
4. WHEN an In_Tray item has been pending longer than its Job's configured patience window, THE Work_Studio SHALL NOT silently discard, expire, or auto-approve it.
5. WHEN a Job escalates for a decision, THE Work_Studio SHALL state in plain language what it was trying to do, what it was unsure about, and what the available choices are.
6. WHEN a User changes a Job's purpose or schedule in a way that materially alters what it will produce, THE Work_Studio SHALL offer to return the Job to `awaiting_kickoff` for a fresh Kickoff_Review.
7. THE Work_Studio SHALL show the In_Tray count on the Dashboard and SHALL NOT use modal interruption for In_Tray items.
8. WHEN an escalation offers the User a choice, THE Work_Studio SHALL render the available choices as inline actions on the item, and SHALL NOT require the User to compose a free-text response in order to resolve it.
9. WHEN a User resolves an escalation by choosing an option, THE Work_Studio SHALL offer a candidate Steering_Note derived from that choice for the User to confirm.
10. THE Work_Studio SHALL allow a Finding to be dismissed without altering the state of the Job that raised it.

### Requirement 7: Out Tray

**User Story:** As a User, I want to see everything that was done on my behalf and be able to undo it where possible, so that autonomy never means loss of control.

#### Acceptance Criteria

1. THE Work_Studio SHALL present an Out_Tray listing completed Job_Runs in reverse-chronological order, filterable by Job.
2. FOR EACH Out_Tray item, THE Work_Studio SHALL show what was done, when, which Connector or file it affected, and the resulting Artefact or Delivery in reviewable form.
3. WHERE the underlying action is reversible, THE Work_Studio SHALL offer a reversal action on the Out_Tray item and SHALL state the limits of that reversal in plain language.
4. WHERE the underlying action is not reversible, THE Work_Studio SHALL indicate this on the item rather than offering a reversal that cannot be honoured.
5. THE Work_Studio SHALL allow the User to attach a Steering_Note to any Out_Tray item without blocking, and SHALL confirm that the note will apply to the Job's next execution.
6. THE Work_Studio SHALL record every Out_Tray item in the Activity_Log, and the Activity_Log SHALL be append-only.
7. THE Work_Studio SHALL support a per-Job Out_Tray policy of either recording every Job_Run or recording only Job_Runs in which something changed, and SHALL default monitoring Jobs to the latter. _Rationale: journey J4 established that a two-hourly monitor recording every quiet run would place 108 items a week into the Out_Tray and bury the items the User cares about._
8. WHERE a reversal is only possible for a limited period, THE Work_Studio SHALL record that period, SHALL state it to the User at the time of the action, and SHALL withdraw the reversal action when the period lapses rather than offer a reversal that will fail.
9. THE Work_Studio SHALL present reversal and steering as distinct actions on an Out_Tray item, one affecting what was already done and the other affecting future executions.

### Requirement 8: Steering and Preference Memory

**User Story:** As a User, I want my corrections to stick without me having to repeat them, and I want to see and change what the product believes I want, so that I can steer it instead of re-approving it.

#### Acceptance Criteria

1. THE Work_Studio SHALL store Steering_Notes per Job as durable, ordered, natural-language records.
2. WHEN a Job_Run executes, THE Work_Studio SHALL make all of that Job's active Steering_Notes available to the execution that produces the output.
3. THE Work_Studio SHALL present a Job's Steering_Notes to the User as an editable list, and SHALL allow the User to add, reword, deactivate and delete individual notes.
4. THE Work_Studio SHALL NOT infer or store a User preference that is not visible to the User in the Steering_Notes list.
5. WHEN a Steering_Note is added, THE Work_Studio SHALL state which Job it applies to and when it will first take effect.
6. WHERE two Steering_Notes conflict, THE Work_Studio SHALL prefer the more recent note and SHALL surface the conflict in the Steering_Notes list.
7. THE Work_Studio SHALL support Global_Steering_Notes that apply across all Jobs, and SHALL allow each to be scoped either to everything or to a single Artefact kind.
8. THE Work_Studio SHALL present Global_Steering_Notes in Settings as an editable list on the same terms as per-Job notes, showing for each note its scope and where it came from.
9. WHERE a per-Job Steering_Note and a Global_Steering_Note conflict, THE Work_Studio SHALL prefer the per-Job note, and SHALL state this precedence to the User in the Global_Steering_Note list.
10. THE Work_Studio SHALL apply the same visibility rule to Global_Steering_Notes as to per-Job notes: no global preference influences any Job_Run unless it appears in that list.

### Requirement 9: Proactive Scheduling and Execution

**User Story:** As a User, I want jobs to run reliably at the times I expect even if my computer was asleep or the application was closed, so that I can depend on them.

#### Acceptance Criteria

1. THE Proactive_Engine SHALL support recurring schedules expressed in the User's terms, including time-of-day, selected weekdays, and fixed intervals.
2. THE Proactive_Engine SHALL persist every schedule and every Job_Run outcome durably, such that scheduling state survives application restart and host reboot.
3. WHEN a scheduled time is missed because the application was not running or the host was asleep, THE Proactive_Engine SHALL apply the Job's declared missed-run policy of either running once on next availability or skipping to the next occurrence.
4. THE Proactive_Engine SHALL execute Job_Runs of the same Job serially and SHALL NOT start a Job_Run while a prior Job_Run of that Job is in progress.
5. WHEN a Job_Run fails for a transient reason, THE Proactive_Engine SHALL retry with backoff up to the Job's declared limit before setting the Job to `needs_attention`.
6. WHEN a Job_Run fails for a reason the User must resolve, THE Proactive_Engine SHALL NOT retry and SHALL set the Job to `needs_attention` immediately.
7. THE Proactive_Engine SHALL display the next scheduled time for every `live` Job on the Dashboard in the User's local time zone.
8. THE Work_Studio SHALL allow the User to run any Job immediately without altering its schedule.
9. WHEN a Job becomes `live`, THE Work_Studio SHALL state when its first scheduled execution will occur, expressed in the User's local time zone as a day and time. _Rationale: journey J1 activated a 7:00 am daily Job on a Friday afternoon; without this statement the two-day silence before Monday reads as a fault._

### Requirement 10: Documents Surface

**User Story:** As a User, I want to describe the document I need and get it, without choosing which specialist to ask, so that I think about my work and not about the product's structure.

#### Acceptance Criteria

1. THE Work_Studio SHALL present a single Documents_Surface for creating and editing Artefacts and SHALL NOT present separate entry points per Artefact type or per Artefact_Agent.
2. WHEN a User states an intent in plain language, THE Documents_Surface SHALL select the Artefact type and the responsible Artefact_Agent without asking the User to choose.
3. IF the intended Artefact type is genuinely ambiguous, THEN THE Documents_Surface SHALL ask a single question in outcome terms (for example "a document or a deck?") and SHALL NOT expose agent or engine names.
4. WHEN a task requires more than one Artefact type, THE Documents_Surface SHALL coordinate the required Artefact_Agents within one User-visible task and SHALL present one result.
5. THE Documents_Surface SHALL show the Artefact being worked on in an Artefact_Client alongside the conversation, updating the visible representation after each change, and SHALL accept direct User edits in that client as specified in Requirement 22.
6. THE Work_Studio SHALL allow the User to open an existing Artefact from disk into the Documents_Surface and continue working on it.
7. THE Documents_Surface SHALL present a home state containing a single intent field, a drop target for existing files, and a list of recent Artefacts showing for each one who last changed it and, where applicable, which Artefact it was derived from.
8. WHEN Work_Studio makes an improvement to an Artefact that the User did not request, THE Work_Studio SHALL state what it changed and why in plain language rather than make the change silently.

### Requirement 11: Artefact Creation and Iterative Co-Editing

**User Story:** As a User, I want to work on a real document over multiple turns, editing it myself and having the product edit it too, without either of us destroying the other's work.

#### Acceptance Criteria

1. THE Work_Studio SHALL produce Artefacts in the native formats `.docx`, `.pptx` and `.xlsx`, and SHALL produce files that open without repair prompts in Microsoft Office and LibreOffice.
2. WHEN Work_Studio edits an existing Artefact, THE Work_Studio SHALL preserve all content and formatting it did not intend to change.
3. WHEN Work_Studio opens and saves an Artefact without making a change, THE resulting file SHALL be materially equivalent to the original.
4. THE Work_Studio SHALL record every change to an Artefact as an ordered entry attributed to either the User or the Work_Studio.
5. THE Work_Studio SHALL allow the User to view the change history of an Artefact and to revert the Artefact to any prior point in that history.
6. WHEN the User has edited an Artefact outside Work_Studio since the last change, THE Work_Studio SHALL detect this before its next edit and SHALL incorporate the User's changes rather than overwrite them.
7. IF an Artefact cannot be edited without loss of content or formatting, THEN THE Work_Studio SHALL inform the User before proceeding and SHALL offer to work on a copy.
8. WHERE Work_Studio has declined to edit an Artefact, THE Work_Studio SHALL offer to describe the changes it would make instead of making them.

### Requirement 12: Artefact Storage and File Ownership

**User Story:** As a User, I want the files the product makes to be real files I can find, open and email, so that my work is not trapped inside an application.

#### Acceptance Criteria

1. THE Work_Studio SHALL store every Artefact as an ordinary file in a User-visible folder on the local file system.
2. THE Work_Studio SHALL allow the User to choose the location of that folder and SHALL default it to a conventional documents location.
3. THE Work_Studio SHALL name Artefact files from the User's stated intent, using names a User would recognise.
4. THE Work_Studio SHALL maintain its version history alongside the Artefact without requiring the Artefact file itself to be a proprietary format.
5. IF an Artefact file is moved, renamed or deleted outside Work_Studio, THEN THE Work_Studio SHALL detect this, SHALL NOT recreate the file silently, and SHALL inform the User in plain language.
6. THE Work_Studio SHALL present a Repository listing every Artefact it has produced or edited, and that Repository SHALL be a view over the real folder on disk rather than a separate store.
7. THE Repository SHALL present folders that exist on disk as folders, and SHALL present Artefact kinds as filters rather than as folders, so that no organisation visible in Work_Studio is absent when the User opens the folder in their file manager. _Rationale: an app-only taxonomy disappears the moment the User looks at their own disk, which contradicts Requirement 12.1._
8. THE Repository SHALL show for each Artefact when it last changed, who changed it, which Jobs have used it, what it was derived from, and how many versions exist.
9. THE Work_Studio SHALL allow the User to create, rename and move folders from the Repository, and SHALL apply those operations to the real folder on disk.
10. WHERE a `scheduled` Job writes Artefacts, THE Work_Studio SHALL allow the User to choose the folder it writes to and SHALL show that relationship in the Repository.

### Requirement 13: Connectors

**User Story:** As a User, I want to connect my email, calendar and social account with the least possible ceremony, and know exactly what each connection allows, so that I can grant access confidently.

#### Acceptance Criteria

1. THE Work_Studio SHALL support Email, Calendar and X Connectors in v1.
2. WHEN requesting a Connector, THE Work_Studio SHALL state in plain language what it will read, what it will write, and which Jobs will use it.
3. THE Work_Studio SHALL request only the access scopes required by the Jobs the User has activated, and SHALL request additional scopes only at the moment a Job needs them.
4. THE Work_Studio SHALL store all Connector credentials in the Vault and SHALL NOT store any credential in application configuration files, logs, or the Activity_Log.
5. THE Work_Studio SHALL NOT return any credential value to the presentation layer.
6. WHEN a Connector credential expires or is revoked, THE Work_Studio SHALL set every dependent `live` Job to `needs_attention` with a single In_Tray item stating which account needs reconnecting.
7. THE Work_Studio SHALL allow the User to disconnect a Connector, and WHEN a Connector is disconnected THE Work_Studio SHALL pause every dependent Job rather than allow it to fail repeatedly.
8. WHEN a Job is paused or requires attention because of a Connector, THE Work_Studio SHALL name the responsible account in the Job's state indicator on the Dashboard, and SHALL raise exactly one In_Tray item for the account regardless of how many Jobs depend on it.

### Requirement 14: Model Routing and Quality Tiers

**User Story:** As a User, I want the product to pick the right level of intelligence for each task by itself, and to keep working when a provider has an outage, so that I never think about models.

#### Acceptance Criteria

1. THE Model_Router SHALL select a model for every unit of work from a Quality_Tier policy without User involvement.
2. THE Work_Studio SHALL default to OpenAI as the provider on first run.
3. THE Work_Studio SHALL allow the User to supply credentials for additional providers and to express a single global preference between cost and quality, and SHALL NOT require any per-Job model selection.
4. WHEN the provider selected for a Quality_Tier fails or is rate-limited, THE Model_Router SHALL fail over to the next configured option for that tier and SHALL complete the unit of work.
5. WHEN a unit of work completes only after failover, THE Work_Studio SHALL record the failover in the Activity_Log and SHALL NOT surface it as a User-facing error.
6. IF no configured provider can serve a Quality_Tier, THEN THE Work_Studio SHALL report the failure in plain language naming the account that needs attention, and SHALL set affected Jobs to `needs_attention`.
7. THE Work_Studio SHALL confine all provider and model identifiers to Settings and the diagnostics view.

### Requirement 15: Spend Visibility

**User Story:** As a User, I want to see what the product is costing me, so that autonomous work does not produce a surprise bill.

#### Acceptance Criteria

1. THE Work_Studio SHALL display cumulative Spend for the current day on the Dashboard.
2. THE Work_Studio SHALL attribute Spend to individual Jobs and SHALL show per-Job Spend on the Job's detail view.
3. THE Work_Studio SHALL account for Spend across all model usage, including proactive Job_Runs, Documents_Surface work, and internal classification work.
4. THE Work_Studio SHALL allow the User to set a daily Spend limit.
5. WHEN a daily Spend limit is reached, THE Work_Studio SHALL pause proactive Job_Runs, SHALL place one item in the In_Tray, and SHALL NOT interrupt work the User is actively doing in the Documents_Surface.
6. THE Work_Studio SHALL express Spend in currency and SHALL NOT express it in tokens or request counts on any primary surface.
7. WHERE a Spend amount is smaller than the currency's smallest conventional unit, THE Work_Studio SHALL round it to a legible form or express it as a rate over a longer period, and SHALL NOT render amounts with more than two decimal places.

### Requirement 16: Privacy and Local-First Operation

**User Story:** As a User, I want my documents, credentials and history to stay on my machine, so that using AI at work does not mean exporting my work.

#### Acceptance Criteria

1. THE Work_Studio SHALL store all Jobs, Job_Runs, Steering_Notes, Artefacts, Activity_Log entries and credentials on the local device.
2. THE Work_Studio SHALL NOT transmit User content to any endpoint other than the model provider selected by the Model_Router and the Connectors the User has explicitly configured.
3. THE Work_Studio SHALL NOT include telemetry, analytics, or crash reporting that transmits User content, and WHERE any diagnostic transmission exists it SHALL be opt-in and disabled by default.
4. THE Work_Studio SHALL encrypt local Job, Job_Run, Steering_Note and Activity_Log storage at rest.
5. THE Work_Studio SHALL provide a single view that states, in plain language, which accounts are connected, what leaves the device, and where it goes.
6. THE Work_Studio SHALL provide a single action that exports all of the User's data in open formats, and a single action that deletes all local data and credentials.
7. WHERE a unit of work can be served by a locally hosted model, THE Work_Studio SHALL permit the User to route that work locally without disabling any other capability.
8. WHERE THE Work_Studio states a claim about a third party's handling of User data, THE claim SHALL be sourced from a maintained per-provider statement rather than fixed interface copy, so that it can be corrected without a release.

### Requirement 17: Failure Legibility

**User Story:** As a User, I want to be told what went wrong in terms of my work and what to do about it, so that failures do not require technical interpretation.

#### Acceptance Criteria

1. WHEN any operation fails, THE Work_Studio SHALL present a message that names the affected Job or Artefact, states the consequence for the User, and states the next action the User can take.
2. THE Work_Studio SHALL NOT present provider errors, protocol errors, status codes, stack traces, or identifiers on any primary surface.
3. WHEN a failure is caused by an expired or revoked Connector, THE Work_Studio SHALL name the account and offer reconnection as the next action.
4. WHEN a failure is transient and recovered automatically, THE Work_Studio SHALL NOT present it to the User and SHALL record it in the Activity_Log.
5. THE Work_Studio SHALL make the underlying technical detail of any failure available in the diagnostics view for support purposes.
6. WHEN a Job has failed for the same reason on three consecutive Job_Runs, THE Work_Studio SHALL pause it and SHALL state in the In_Tray item that it has stopped trying.

### Requirement 18: Capability Reuse Boundary

**User Story:** As the product team, we want to reuse proven engine and connector code without inheriting a developer-shaped product surface, so that we gain the years of capability work without repeating the prior attempt's mistake.

#### Acceptance Criteria

1. THE Work_Studio SHALL consume the Capability_Layer only through headless interfaces, and no Capability_Layer component SHALL contribute any User-visible interface.
2. THE Work_Studio SHALL NOT depend on the `adk-desktop` presentation layer, and SHALL NOT reproduce its terminal, worktree, sandbox-mode, plugin-host, graph-authoring or protocol-configuration surfaces.
3. THE Work_Studio SHALL preserve the Capability_Layer's approval semantics such that no externally visible action occurs without either a Kickoff_Review approval or a `live` Job authorisation.
4. THE Work_Studio SHALL NOT honour any auto-approval flag declared in Capability_Layer configuration as authorisation for an externally visible action.
5. WHERE Capability_Layer functionality is required but its persistence is in-memory only, THE Work_Studio SHALL supply durable persistence rather than accept loss on restart.
6. WHERE a required capability does not exist in the Capability_Layer, THE Work_Studio SHALL implement it as a headless component conforming to the same boundary.
7. WHERE an operation is not present in Work_Studio's own classification of side effects, THE Work_Studio SHALL NOT perform it in any Job state, and SHALL raise it to the User for a decision instead. _Rationale: an unclassified operation cannot be described to the User in plain language and cannot be offered for reversal, so performing it would breach both Requirement 7.2 and Requirement 17.1. Discovered when a test found such an operation being performed in a `live` Job behind an "I don't know how to take this back" fallback._
8. THE Work_Studio SHALL derive its classification of side effects from its own authored table and SHALL NOT derive it from metadata declared by a Capability_Layer component, so that a component cannot widen what Work_Studio will do on the User's behalf.

### Requirement 19: Responsiveness and Resource Budgets

**User Story:** As a User, I want the application to feel immediate and stay out of my way while it works in the background, so that it is pleasant to keep open all day.

#### Acceptance Criteria

1. WHEN launched, THE Work_Studio SHALL present an interactive Dashboard within 3 seconds on hardware with 8 GB RAM and a 4-core CPU.
2. WHEN the User performs any navigation or interface action, THE Work_Studio SHALL provide visible feedback within 100 milliseconds.
3. WHEN a Job_Run or Documents_Surface task is in progress, THE Work_Studio SHALL stream progress in the User's terms and SHALL NOT present an indeterminate wait longer than 3 seconds without explanation.
4. WHILE idle with proactive Jobs scheduled, THE Work_Studio SHALL consume less than 250 MB of resident memory and negligible CPU.
5. THE Work_Studio SHALL keep the interface responsive during Job_Runs and Artefact operations, and SHALL NOT block the interface on any Capability_Layer call.

### Requirement 20: Trust Gates for Release

**User Story:** As a User evaluating a privacy-first product, I want the first launch to feel legitimate and every update to be verifiable, so that the product's claims match my experience of installing it.

#### Acceptance Criteria

1. THE Work_Studio SHALL be code-signed and notarized for macOS and signed for Windows, such that first launch presents no operating system security warning.
2. THE Work_Studio SHALL deliver updates over an authenticated channel and SHALL verify update integrity before applying.
3. THE Work_Studio SHALL publish a software bill of materials for each release.
4. THE Work_Studio SHALL satisfy Requirement 16.4 in every released build.
5. THE Work_Studio SHALL NOT be released to end users until Requirements 20.1 through 20.4 are satisfied.

### Requirement 21: Accessibility

**User Story:** As a User who relies on assistive technology or keyboard navigation, I want full access to my jobs and documents, so that the product is usable regardless of how I work.

#### Acceptance Criteria

1. THE Work_Studio SHALL make every primary surface — Dashboard, In_Tray, Out_Tray, Job detail, Documents_Surface, Settings — fully operable by keyboard.
2. THE Work_Studio SHALL expose accessible names, roles and states for all interactive elements to platform accessibility APIs.
3. THE Work_Studio SHALL meet WCAG 2.2 AA contrast ratios for all text and interface elements.
4. THE Work_Studio SHALL NOT convey Job_State or In_Tray item class by colour alone.
5. WHEN an In_Tray item arrives, THE Work_Studio SHALL announce it to assistive technology without stealing focus.

### Requirement 22: In-App Artefact Viewing and Editing

**User Story:** As a User, I want to see and change my documents, decks and spreadsheets inside Work Studio, so that I do not need a second office suite open to do my own work.

#### Acceptance Criteria

1. THE Work_Studio SHALL render every supported Artefact type faithfully within an Artefact_Client, such that the User is never required to open another application in order to see what the Artefact contains.
2. THE Artefact_Client SHALL derive its rendering from the Artefact file itself rather than from a separate summary or approximation, such that what the User sees reflects what the file contains.
3. THE Artefact_Client SHALL accept direct User edits for each type, covering at minimum: editing and formatting text; changing cell values and formulas; inserting, moving, resizing and deleting objects; and reordering slides, rows and columns.
4. THE Work_Studio SHALL apply every direct User edit through the same Edit_Operation vocabulary it uses for its own changes, such that User edits and Work_Studio edits appear in one change history and are reverted by the same mechanism.
5. THE Artefact_Client SHALL NOT offer a direct edit that cannot be recorded as an Edit_Operation in the change history.
6. THE Artefact_Client SHALL indicate which parts of the Artefact Work_Studio itself changed, attributed and reversible, and SHALL distinguish them from the User's own changes.
7. WHEN the User makes a direct edit, THE Artefact_Client SHALL reflect it visually within 100 milliseconds, and WHERE a recalculation or re-layout is required THE Artefact_Client SHALL complete it without the User leaving the surface.
8. THE Work_Studio SHALL NOT be required to reach authoring parity with commercial office suites. WHERE the User needs an operation the Artefact_Client does not offer directly, THE Work_Studio SHALL offer to make that change on the User's behalf. _Rationale: the division of labour is that Work_Studio constructs and the User inspects and nudges; pursuing full parity would rebuild an office suite the User already declined to open._
9. THE Work_Studio SHALL keep opening an Artefact in an external application available as a secondary action, and SHALL NOT present it as the primary route to editing.
10. WHEN a direct User edit would affect content the underlying engine cannot round-trip, THE Artefact_Client SHALL prevent the edit and explain why, rather than perform it lossily.
11. THE Artefact_Client SHALL support keyboard-only editing for every operation it offers directly.
