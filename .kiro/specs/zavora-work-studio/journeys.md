# User Journeys: Zavora Work Studio

## Purpose

This document validates `requirements.md` and `design.md` against concrete, named situations. Each journey is a walkthrough of real screens with the actual words the User would read. Journeys are only worth writing if they change the design, so each one ends with what it exposed, and the amendments they forced are collected at the end and applied back into the spec.

Screens referenced here are in `mockups/`, generated from `mockups/mockups.html`.

| Screen | File | Journeys |
|---|---|---|
| S1 Dashboard | `mockups/01-dashboard.png` | J1, J4, J5 |
| S2 First run | `mockups/02-first-run.png` | J1 |
| S3 Recurring work library + consent | `mockups/03-recurring-library.png` | J1, J3 |
| S4 First draft — newsletter | `mockups/04-first-draft-newsletter.png` | J1, J2 |
| S5 First draft — intended actions | `mockups/05-first-draft-actions.png` | J3 |
| S6 Waiting on you | `mockups/06-waiting-on-you.png` | J4, J5, J9 |
| S7 Thread detail + steering | `mockups/07-task-detail-steering.png` | J2 |
| S8 Done for you | `mockups/08-done-for-you.png` | J2, J8 |
| S9 New work | `mockups/09-new-work.png` | J6 |
| S11 Honest limits | `mockups/11-honest-limits.png` | J7 |
| S12 What leaves this computer | `mockups/12-what-leaves.png` | J10 |
| S15 Document workspace | `mockups/15-document-workspace.png` | J11 |
| S16 Spreadsheet workspace | `mockups/16-spreadsheet-workspace.png` | J11 |
| S17 Deck workspace | `mockups/17-deck-workspace.png` | J6, J11 |
| S18 Details pane | `mockups/18-details-panel.png` | J6, J11 |
| S19 Focus mode | `mockups/19-focus-mode.png` | J11 |
| S20 Documents repository | `mockups/20-documents-repository.png` | J6, J12 |
| S21 Settings | `mockups/21-settings.png` | J1, J10, J12 |
| S22 Global steering | `mockups/22-global-steering.png` | J12 |

### The cast

- **Naomi**, 41, finance director at a mid-size Nairobi logistics firm. Confident with Excel, has never used a terminal, has never heard of an API key and will need to be walked to one. Evaluates software by whether it saves her Thursday.
- **David**, 36, founder of a four-person SaaS company. Technical enough to be suspicious, impatient with configuration, posts on X for the company.

---

## J1 — Naomi's first ten minutes

**Situation.** Naomi installs Work Studio on a Friday afternoon after a colleague mentioned it. She has ten minutes before a call. Nothing is configured. She has no idea what the product does beyond "AI that does work".

**Screens.** S2 → S3 → S4

1. **First launch (S2).** One panel: *"Welcome to Zavora Work Studio. Your work stays on this computer. Paste one key to get started — you can change it later."* One field, labelled *OpenAI key*, with the reassurance *"Stored in your Mac keychain. Never sent anywhere except OpenAI."* A quiet secondary link, *Use a different provider*.

   Naomi does not have a key. She follows the field's helper link, creates one, pastes it. This is the single hardest moment in the product and the spec should stop pretending otherwise — see amendment **A13**.

2. **Choosing work (S3).** She lands not on an empty dashboard but on *"What should I take off your hands?"* with *"Pick one. I'll do it once and show you before anything leaves this computer."* Nine cards. Three are marked **Ready**; the others say **Needs Gmail** or **Needs Calendar** in plain grey text.

   She picks **Daily newsletter** — *"A short brief from your sources, in your inbox each morning"* — because it says Ready and requires nothing of her.

3. **Waiting.** Progress is stated in her terms: *"Reading your sources… found 34 items"*, then *"Picking what matters"*, then *"Writing it up"*. Elapsed: 40 seconds.

4. **First draft (S4).** A full newsletter, formatted as she'd receive it. Above it: *"This is what I'd have emailed you"* and *"Nothing was sent. Change anything you like — I'll remember it."* Three buttons: **Looks good — send it daily at 7:00 am**, **Edit it first**, **Not quite — tell it why**.

   She reads it. It's decent but long. She clicks **Looks good**.

5. **Confirmation.** *"Done. I'll send this every weekday at 7:00 am. The first one arrives Monday morning."*

**Elapsed:** 6 minutes 20 seconds, of which four were spent obtaining the OpenAI key.

**Requirements exercised.** 2.1–2.7, 4.1–4.3, 5.1, 5.2, 5.3, 5.6, 19.3.

**What it exposed.**
- The 10-minute target in R2.6 is met, but *only because the newsletter template needs no Connector*. The measurement is honest only if it names that condition, which it does.
- The primary obstacle to first value is not our software — it is obtaining a provider key. The spec treats this as a single field and says nothing about helping. → **A13**
- The confirmation copy must state when the first live run happens, because "daily at 7:00 am" activated on a Friday afternoon means Monday, and silence here reads as a bug. → **A12**

---

## J2 — Going live, then steering it back

**Situation.** Monday's brief arrives. It is too long. Naomi does not want to reconfigure anything; she wants to complain once and have it stick.

**Screens.** S4 (edit path) → S8 → S7

1. On Friday she had chosen **Looks good** rather than **Edit it first**. Monday's brief lands in her inbox at 7:00 am and appears in **Done for you** on the Dashboard: *"Sent your Monday brief — 3 sources, 6 minute read"*.

2. She opens the item and clicks **Steer** (S8). An inline field, no dialog. She types *"Too long — keep it under 400 words."* Below it: *"Saved. This applies from your next brief."* Nothing blocks; the item stays in place.

3. **Tuesday.** The brief is 380 words. In **Done for you**: *"Sent your Tuesday brief — 3 sources, 2 minute read"*.

4. **Task detail (S7).** She opens **Daily newsletter** out of curiosity. Left column, *What it's done*, is four plain sentences — including *"Sent Friday's brief — you said it was too long"*. Right column, **What I've learned from you**, is three editable notes:
   - *Keep it under 400 words.* — footnoted *You shortened Friday's draft · 2 days ago*
   - *Don't include crypto prices.* — *You told me on Friday*
   - *Lead with anything about the EU AI Act.* — *You told me last week*

   Under them: *"This is everything I go on. Change or delete any of it."*

**Requirements exercised.** 7.1, 7.2, 7.5, 8.1–8.6, 3.2.

**What it exposed.**
- The steering list is the single most trust-building surface in the product, and it works precisely because R8.4 forbids invisible preferences. Had we stored an embedding or a learned profile, this screen would be a lie.
- An **edit-and-approve** derived note needs confirmable microcopy, not silent storage. The correct pattern is *"I noticed you cut it to 380 words. Should I always keep it under 400?"* with **Yes, remember that** / **No, just this once**. The spec requires confirmation (R5.4) but does not specify the pattern. → **A3b**
- Task detail shows *"4p a day"* as the per-task cost. Sub-cent amounts need a formatting rule or the Dashboard will show `$0.004`. → **A9**

---

## J3 — Handing over the inbox

**Situation.** Naomi is convinced enough to try something that touches her real work. Inbox triage is the scariest thing in the library.

**Screens.** S3 (consent panel) → S5

1. She clicks **Try it** on **Inbox triage**. Before any authorisation, a panel (S3, right): *"Connect Gmail for Inbox triage. So it can do this job, it will:"*
   - ✓ Read the messages in your inbox
   - ✓ Add labels and archive what you don't need
   - ✓ Write draft replies — and leave them as drafts

   Then: *"It will never send a message without you. Only Inbox triage uses this."* Buttons **Connect Gmail** / **Not now**.

2. She connects. The first run executes with the side-effect gate suppressing everything.

3. **First draft, as actions (S5).** *"Here's what I'd have done to your inbox"* / *"Your inbox is untouched. Uncheck anything you'd rather I left alone."* A four-row manifest:

   | | | |
   |---|---|---|
   | Archive | 18 newsletters and receipts you've never opened | reversible |
   | Label | 9 messages as **Needs reply** — 3 of them are from clients | reversible |
   | Draft | 4 replies, left unsent in your drafts folder | you send them |
   | Leave alone | 11 messages I wasn't confident about | — |

   Below, an offer: *"Want to read one of the drafts? Reply to Achieng about the Thursday deadline"* → **Open draft**.

   Buttons: **Do this every 2 hours** / **Just this once** / **Not quite — tell it why**. Footer: *"Everything here can be undone from Done for you for 30 days."*

4. She reads the Achieng draft, finds it acceptable, and clicks **Do this every 2 hours**.

**Requirements exercised.** 5.1, 5.2, 5.6, 13.2, 13.3, 18.3.

**What it exposed. This is the journey that broke the design.**
- A Kickoff_Review is not always a document. For most of the ten templates it is a **manifest of intended actions**, which the spec mentioned in one clause of the data model and never designed. It needs: a per-row plain-language description, a reversibility marker per row, a drill-in to inspect any produced content, and per-row opt-out. → **A3**
- Per-row opt-out is a **new resolution type**. `approved_with_exclusions` is neither approve nor reject, and each excluded row should become a candidate Steering_Note (*"don't archive receipts"*). → **A3c**
- **Just this once** is a third resolution the spec does not have: perform this batch, stay in `draft`. Users will want it and its absence forces a false binary. → **A3d**
- *"undone… for 30 days"* asserts a reversal **window**, but the data model only has a reversibility flag. Gmail archive is reversible indefinitely; an X post is reversible until deleted; a sent email never was. The affordance must expire honestly. → **A5**

---

## J4 — The monitor that has nothing to say

**Situation.** Naomi also activated **Computer health**. It runs every two hours. For nine days it finds nothing. On day ten her startup disk hits 94%.

**Screens.** S3 → S1 → S6

1. She clicks **Try it** on Computer health. Under the current spec, R5.1 requires a dry run and a Kickoff_Review. So the product produces… *"Everything looks fine. Disk 61% used, backups current, memory healthy."* and asks her to approve it.

   This is absurd. Approving "nothing is wrong" teaches her that reviews are noise, which is the precise habit that destroys the trust model. **The spec is wrong here.** → **A2**

2. **Corrected behaviour.** Read-only tasks — those whose runs can produce no external effect and no Artefact — go **live on activation** with the confirmation *"Watching now. I'll only interrupt you when something needs you."* What she reviews later is the first *finding*, not an empty baseline.

3. Nine quiet days. **Done for you** would otherwise accumulate twelve *"all clear"* entries a day — 108 items of noise burying the three things she cares about. S8 therefore shows one collapsed line: *"Checked your computer — all clear · quiet runs are hidden"*. → **A4**

4. **Day ten (S1, S6).** In **Waiting on you**, an item that is neither an approval nor a fault:

   > **WORTH KNOWING** — *Your startup disk is 94% full*
   > Nothing is broken yet. 18 GB sits in Downloads from before April. Computer health.
   > **See what's big** · **Got it**

**Requirements exercised.** 6.1, 6.2, 9.1, 21.4.

**What it exposed.**
- The three-class tray taxonomy is incomplete. This item requires no decision from the product's perspective and nothing is broken — it is a **finding**. Forcing it into `attention` would say "Work Studio is broken" when Work Studio is working perfectly. Forcing it into `escalation` would demand a decision that isn't ours to make. A fourth class is required. → **A1**
- Every monitoring template in the library — computer health, website availability, news monitor — produces findings, so this is one third of the product, not an edge case.
- The **Got it** action is a dismissal that resolves without any state change to the Job. The resolution vocabulary needs it.

---

## J5 — Gmail expires on a Thursday

**Situation.** Naomi changes her Google password. Two tasks depend on Gmail. She learns about it from the app, not from silence.

**Screens.** S6 → S1

1. The next triage run fails on authorisation. The failure classifier marks it `user_actionable`, not `job_failed`, so there is no retry storm.

2. **One** consolidated In Tray item, not two:

   > **NEEDS FIXING** — *Gmail needs reconnecting*
   > Your sign-in expired on Thursday. Inbox triage and Morning digest are paused until you reconnect — I've stopped trying.
   > **Reconnect Gmail**

3. On the Dashboard (S1), both dependent tasks show a `Paused · Gmail` pill rather than a scary red failure, because the tasks are not broken — the account is.

4. She reconnects. Both tasks return to `live` without a fresh Kickoff_Review, because nothing about what they do has changed.

**Requirements exercised.** 13.6, 13.7, 17.1, 17.3, 17.6, 9.6.

**What it exposed.**
- Consolidation is load-bearing. Three Gmail tasks failing must never produce three items; the fault is one account, so the item is one. R13.6 says this; the visual design must reflect it, and the Dashboard pill must name the cause (`Paused · Gmail`), which the requirement does not currently ask for. → **A14**
- Returning to `live` after reconnection must *not* re-trigger Kickoff. The transition set permits `needs_attention`→`live`, so this is correct as written — worth recording that it was checked.

---

## J6 — "A board deck from last quarter's numbers"

**Situation.** David has a board meeting Tuesday. The numbers are in a spreadsheet Work Studio helped him build last month.

**Screens.** S9 → S10

1. **New work (S9).** *"What do you need?"* with one field: *Describe it — "a board deck from last quarter's numbers"*, and *"Or drop a file here to keep working on it"*. Beneath it, *Or hand over something that runs on its own* — three cards — and then **Pick up where you left off**:
   - `Q3 revenue model.xlsx` — *You and I both edited it · 20 minutes ago* · 12 versions
   - `Board deck — July.pptx` — *I made this yesterday from Q3 revenue model.xlsx* · 4 versions
   - `Partnership agreement — draft 3.docx` — *You changed this in Word · 2 days ago* · 7 versions

   Footer: *"All of these live in Documents › Work Studio on your Mac."*

   There is no "new document / new deck / new spreadsheet" choice, and no specialist to pick. He types his sentence.

2. **At work (S10).** Conversation left, the actual slide right.

   > **You:** Make a board deck from last quarter's numbers
   > *I found **Q3 revenue model.xlsx** in your documents. I'll build 8 slides from the summary tab — revenue, margin, cash runway and the regional split. Starting now.*
   > **You:** Put the regional split on its own slide with a chart
   > *Done — slide 5 now has a stacked bar by region. I also fixed the contrast on the axis labels so it reads on a projector.*

   Right pane: slide 5 rendered as SVG with the chart selected and handles showing, eight thumbnails beneath, a contextual toolbar (**Text · Shape · Chart · Colours · History**), and the invitation *"Click anything on the slide to change it yourself."* Footer: *"Saved to Documents › Work Studio — show the file."*

3. He clicks the chart and drags it wider. He double-clicks the title and rewrites it. Neither action leaves the app, and both appear in the same change history as the agent's work. When he does want the file itself, *show the file* reveals it in Finder — a secondary action, not the route to editing.

**Requirements exercised.** 10.1–10.6, 11.1, 11.4, 11.6, 12.1–12.3.

**What it exposed.**
- Two engines were used in one task — spreadsheet read, presentation write — and the User never learned this. That is the Documents-as-one-surface decision paying off. Confirms the choice against the original three-agent sidebar.
- *"I made this yesterday from Q3 revenue model.xlsx"* is **provenance**, and the data model has no derivation link between Artefacts. It is also the most reassuring sentence on the screen. → **A7**
- R10 never specified a home state for Documents. Recents plus a drop target plus provenance is that state, and the drop target is how R10.6 is actually reached. → **A6**
- *"I also fixed the contrast on the axis labels"* is an unrequested improvement, reported plainly. Worth a design note: `mcp_slides`' `lint_design` and contrast QA should run before presenting a deck as done, and their fixes should be stated, not silent.

---

## J7 — The contract it refuses to break

**Situation.** David asks Work Studio to edit a contract his lawyer marked up in Word. This is the case the audit flagged as the highest reuse risk in the entire product.

**Screen.** S11

1. > **You:** Tighten the termination clause and add a 60-day notice period

2. The fidelity probe runs before any edit: open, save to a temporary file, compare structural inventory. It finds 23 tracked changes and 14 comments that `zavora-docx` can read but cannot write back.

3. > *Before I touch this — your file has **tracked changes and 14 comments** from your lawyer that I can't keep. If I edit this copy, they'd be lost.*

   Right pane, amber panel: **I'd lose your lawyer's markup** — *"This document has 23 tracked changes and 14 comments. I can read them, but I can't write them back yet — so saving over this file would strip them out."*

   Below: **Work on a copy instead** — *"I'll make Partnership agreement — draft 4.docx with your changes, and leave draft 3 exactly as it is with all the markup."* Buttons **Work on a copy** / **Just tell me what to change**. Footer: *"I check every file this way before I edit it."*

4. He takes the copy. Draft 3 is untouched.

**Requirements exercised.** 11.2, 11.7, 17.1.

**What it exposed.**
- An engine limitation, surfaced in the User's terms, reads as **carefulness rather than weakness**. This is the strongest argument for shipping the probe in Phase 3 rather than deferring it: it converts `zavora-docx`'s known gaps from a silent data-loss risk into a trust-building moment.
- **Just tell me what to change** is a genuine second option — advice instead of editing — and it costs almost nothing to implement. It should be in the spec.
- *"can't write them back **yet**"* is the correct register: honest, not apologetic, and it ages well when the engine improves.

---

## J8 — Posting to X, and taking it back

**Situation.** David's social posting task has been live a week. This morning's post is off-key.

**Screen.** S8

1. **Done for you:** *"Posted to X: 'We shipped local-first document editing…' · 9:02 am · Social posting · 14 likes so far"*, with **Take it down** and **Steer**.

2. He clicks **Steer** and types *"Too promotional — write like a person next time."* → *"Saved. This applies from your next post."* The post stays up; steering is not deletion.

3. Immediately below, *"Sent your morning digest"* offers no reversal at all — instead, greyed text: **Can't be unsent**. No button that would fail.

**Requirements exercised.** 7.2, 7.3, 7.4, 7.5, 8.1.

**What it exposed.**
- Honesty about irreversibility is more valuable than a reversal button that sometimes works. The greyed *Can't be unsent* label is a deliberate anti-feature.
- **Steer** and **Take it down** are frequently confused intents. Separating them — one changes the future, one changes the past — is the correct division and should be stated in the design.
- The reversal affordance needs an expiry, per **A5**: a post deleted from X after three months of engagement is a different act than undoing it at 9:03 am, and the interface should stop offering it silently at some point rather than fail later.

---

## J9 — The £480 charge it wasn't sure about

**Situation.** Naomi's expense capture task finds what might be a duplicate. It is not broken, and it will not guess.

**Screen.** S6

> **NEEDS YOUR CALL** — *Two of these look like the same invoice*
> I was filing receipts and found a £480 charge twice, three days apart. Expense capture.
> **File both** · **File one**

**Requirements exercised.** 6.1, 6.5.

**What it exposed.**
- R6.5 requires an escalation to state what was attempted, what was uncertain and the choices. Rendering those choices as **inline buttons** rather than an open text field is what makes it a two-second decision instead of a task. The spec has a `choices` column but never says they render inline. → **A11**
- Choosing **File one** is also implicit steering (*"when you see the same amount twice within a week, keep one"*). Offering that as a confirmable derived note is the same pattern as J2 and J3. Escalation resolutions should be able to produce candidate notes. → **A3c**
- Because a mid-run escalation blocks a long-running task, this is where `adk-graph` interrupts plus `SqliteCheckpointer` earn their place: the run resumes after her decision rather than restarting from the top.

---

## J10 — What leaves this computer

**Situation.** David's co-founder asks whether this thing is uploading their contracts. He needs an answer in one screen.

**Screen.** S12

1. *"What leaves this computer"* / *"Everything else — your files, your tasks, what I've learned about you — stays here."*

2. Two flow rows, not prose:
   - Your words and files → **OpenAI** — to write and reason
   - Newsletters and posts → **Gmail**, **X** — because you asked them to go there

3. **Connected accounts**, each with what it does and how many tasks use it, each with **Disconnect**.

4. **Export everything** / **Delete everything on this computer**, and a lock indicator: *Locked on this disk*.

**Requirements exercised.** 16.1, 16.2, 16.5, 16.6, 13.7, 20.4.

**What it exposed.**
- A directional flow diagram answers the question that prose does not. Two rows beat two paragraphs.
- The claim *"Nothing is kept by them for training on business accounts"* is a factual assertion about a third party that could become false. Provider-specific claims must be sourced from a maintained statement, not hardcoded copy. → **A15**
- *Locked on this disk* is the only place at-rest encryption becomes visible to the User, which is a reason to treat R16.4 as a product feature rather than a compliance chore.

---

## J11 — Naomi finishes the work herself, without leaving

**Situation.** Naomi has the supplier agreement and the Q3 model open on different days. In both cases the agent did the construction and she wants to make small changes herself. She does not have Word or Excel open, and she is not going to open them.

**Screens.** S13 → S14

### The document

1. She asked for a 60-day notice period. The reply: *"Added it as clause 8.3 and updated the cross-reference in 12.1 so they agree. Both are marked in the margin."*

2. **S13.** The right pane is the real document, rendered from the file. Page 3 of 7. Clause 8.3 sits in a highlighted block with a badge reading **I added this**, and the header shows *2 changes by me* beside **History**. A modest toolbar: **B · I · Heading · List · Table · Comment**.

3. She clicks into clause 9.1 and edits the wording directly. The caret behaves. Her change and the agent's change appear in the same history, distinguished by author. She reverts the cross-reference edit in 12.1 with one action on its badge, leaving 8.3 in place.

4. Nowhere does the screen suggest opening Word. *Show the file* exists, quietly.

### The spreadsheet

5. She asked for a 12% growth case. The reply: *"Added column D with 12% applied to each month, and extended the total row. The chart picked it up."*

6. **S14.** Grid, formula bar showing `D7 = C7*1.12`, cell D7 selected, the new column tinted as the agent's work, sheet tabs, and the chart below already reflecting both series. Toolbar: **Format · Chart · Pivot · Rules · History**.

7. She types over D8 to test a manual figure. The total recalculates immediately — *"Formulas recalculate here, not in Excel."*

8. She wants a waterfall chart, which the toolbar does not offer. She asks for it in the conversation instead and gets it. This is the ladder working as designed: what the client cannot do directly, the agent does on request.

**Requirements exercised.** 22.1–22.11, 11.4, 11.5, 10.5.

**What it exposed.**
- The verification behind this journey changed the design. The engines render to **HTML** (`zavora-docx-html::to_html_fragment` + `generate_base_css`) and **SVG** (`zavora-slide-layout::to_svg`), not page images as an earlier draft assumed. Live web output is selectable, hit-testable, zoomable and screen-reader accessible; page images are none of those.
- **The blocking gap is one small engine change.** Neither emitter outputs any node identifier — grepping both for `id=`, `data-*`, `shape_id` and `element_id` returns nothing. Without stable `data-node-id` attributes there is no way to know what the User clicked and no way to attribute or revert an individual agent change. Everything in this journey depends on that one addition. → task 13.6
- **One edit path is the load-bearing invariant.** Her edit and the agent's edit must both be `Edit_Operation`s through the same tool surface, or the shared history in step 3 is impossible. `excel-agent-app` already works this way, which is why its client is portable rather than merely inspirational.
- **Step 8 is the scope guard, not a limitation.** A client that offered every chart type would be an office suite. Offering the common operations directly and routing the rest through the conversation is both cheaper and a better experience — and it is the reason R22.8 explicitly disclaims authoring parity.
- The spreadsheet client is roughly 85% built and the presentation client is 0% built, so sequencing spreadsheet first in task 12 and presentation last in task 13 follows the evidence rather than the product hierarchy.

---


## J12 — David stops repeating himself

**Situation.** David has corrected the brand colours on three separate decks. Each time it stuck to that deck and nowhere else. On the fourth deck he loses patience.

**Screens.** S22 → S20

1. He opens **Settings › How I should work** and finds five notes already there, each with a scope chip and where it came from:

   | Scope | Note | Origin |
   |---|---|---|
   | Everything | Write plainly. No exclamation marks, no "excited to share". | *You told me on 12 July* |
   | Decks | Use our brand colours — deep green and sand. Never the default blue. | *You told me on 14 July* |
   | Decks | Put the ask on the last slide, not the first. | *You reordered two decks · 18 July* |
   | Spreadsheets | Kenyan shillings, thousands separator, no decimals. | *You reformatted a sheet · 19 July* |
   | Documents | Our legal entity is Zavora Logistics Ltd, not Zavora Ltd. | *You corrected me twice* |

   At the foot: *"If a single piece of work has its own instructions, those win over these."*

2. He adds *"Never use stock photography"* with scope **Decks**. Every future deck picks it up; the thread he is in right now picks it up on its next change.

3. Later he needs last quarter's board pack and goes to **Documents › All files** (S20). Breadcrumb reads *Documents › Work Studio — 14 files, 2 folders on your Mac*. Two real folders — *Board packs*, and *Expenses* annotated *"6 files · filled by Expense capture"*. Kind chips across the top filter rather than navigate. Each row shows who changed it and when, which threads used it, what it was made from, and a version count.

4. `Q3 revenue model.xlsx` shows **Used in** two threads — *Q3 revenue model* and *Board deck — July* — so he clicks through from the file to the deck conversation that consumed it.

5. Footer: *"Folders here are real folders on your Mac. Move or rename them anywhere and I'll keep up."*

**Requirements exercised.** 8.7–8.10, 12.6–12.10, 15.4, 16.6.

**What it exposed.**
- **Steering needed two scopes, not one.** Everything before this journey assumed per-thread notes. *"Use our brand colours"* is a house style, not a property of one deck, and forcing it to be per-thread means retyping it forever. Global notes with an Artefact-kind scope fix it, and precedence falls out of the existing recency rule — global notes are assembled first, so per-thread notes naturally win.
- **Organisation must live on disk.** The Repository is a *view*, not a store: folders are real folders, and kinds are filters rather than folders. Anything else produces a taxonomy that vanishes the moment the User opens Finder, which contradicts R12.1 outright.
- **A file and a thread are different axes.** One thread produces several files; one file is used by several threads. *Used in* is what makes derivation navigable rather than merely descriptive, and it is the first place the lineage stored for A7 does visible work.
- **Scheduled work needs a home folder.** *"6 files · filled by Expense capture"* only reads sensibly if a Job can own an output folder — otherwise proactive output scatters through the User's documents.

---


## What these journeys changed

Twenty-three amendments. Ten are material design changes; the rest are gaps in specification detail.

| # | Amendment | Severity | From | Applied to |
|---|---|---|---|---|
| **A1** | Add a fourth tray class `finding` — the task worked and found something worth knowing. Distinct from `attention` (task is broken) and `escalation` (task needs a decision). | **Material** | J4 | R6.1, R6.2; design tray table; Property 18 |
| **A2** | Read-only tasks are exempt from Kickoff_Review and go live on activation. Reviewing "nothing is wrong" trains the User to ignore reviews. | **Material** | J4 | R5.7; design Job lifecycle |
| **A3** | Kickoff_Review has two payload shapes: an output document, or an **intended-action manifest** with per-row description, reversibility marker, drill-in and per-row opt-out. | **Material** | J3 | R5.8; design Kickoff payload section |
| **A3b** | Derived Steering_Notes are confirmed with a specified microcopy pattern, never stored silently. | Detail | J2 | R5.4 note; design steering section |
| **A3c** | Excluded manifest rows and escalation choices both produce **candidate** Steering_Notes for confirmation. | Detail | J3, J9 | design steering section |
| **A3d** | Add the resolution `approved_once` — perform this batch, stay in `draft`. | Detail | J3 | R5.9; `tray_items.resolution` |
| **A4** | Add `out_tray_policy: always \| on_change`. Monitoring tasks must not flood the Out Tray with quiet runs. | **Material** | J4 | R7.7; `jobs.out_tray_policy` |
| **A5** | Reversal has a window, not just a flag. Add `reversal_expires_at`; withdraw the affordance honestly when it lapses. | Detail | J3, J8 | R7.8; `deliveries.reversal_expires_at` |
| **A6** | Specify a New work entry state: intent field, drop target, recurring-work templates, and recent threads with provenance. This is how R10.6 is reached. | Detail | J6 | R10.7; design Navigation |
| **A7** | Add Artefact derivation links so provenance ("made from Q3 revenue model.xlsx") is real. | Detail | J6 | `artefacts.derived_from` |
| **A9** | Money formatting rule for sub-cent amounts; never render `$0.004`. | Detail | J2 | R15.7 |
| **A11** | Escalation choices render as inline actions, not a free-text field. | Detail | J9 | R6.8 |
| **A12** | Activation confirmation states when the first live run occurs, in the User's time zone. | Detail | J1 | R9.9 |
| **A13** | First run must actively help obtain a provider key — the largest obstacle to first value, and not our software. | Detail | J1 | R2.8 |
| **A14** | A task paused by a Connector fault shows the cause on its Dashboard pill (`Paused · Gmail`). | Detail | J5 | R13.8 |
| **A15** | Provider-specific privacy claims come from a maintained statement, not hardcoded copy. | Detail | J10 | R16.8 |
| **A16** | Artefact views are in-app **editors**, not previews, for all three types. Opening an external office application becomes a secondary action. | **Material** | J11 | R22 (new), R10.5; design In-App Artefact Clients |
| **A17** | Rendering substrate corrected from page images to **HTML** and **SVG**, both verified present in the engines. | **Material** | J11 | design In-App Artefact Clients |
| **A18** | `zavora-slide-layout` and `zavora-docx-html` must emit stable `data-node-id` attributes. Verified absent; blocks all in-app editing of documents and decks. | **Material** | J11 | task 13.6; Property 25 |
| **A19** | An editing capability ladder with **L3 authoring parity as an explicit non-goal**, and unsupported operations routed to the agent instead. | **Material** | J11 | R22.8; design ladder table |
| **A20** | Unify document threads and proactive tasks as one `Job` with `kind: scheduled \| one_off`, shown in a single list distinguished only by state. | **Material** | J6, J11, J12 | R3.2, R3.7; design Job lifecycle; `jobs.kind` |
| **A21** | Navigation becomes New work · Dashboard · thread list · Documents · Settings. Threads *are* the destination, so "Documents" and "Proactive tasks" cease to be work destinations. | **Material** | J6, J11 | design Navigation; tasks 9.1 |
| **A22** | Add **Global_Steering_Notes** scoped to everything or one Artefact kind, held in Settings, with per-thread notes winning. | **Material** | J12 | R8.7–8.10; design Steering; Properties 29–30 |
| **A23** | Add the **Repository** as a view over the real folder — folders are real, kinds are filters, threads are a second axis, and scheduled Jobs own an output folder. | **Material** | J12 | R12.6–12.10; design Repository; Properties 31–32 |

### What the journeys confirmed

Three contested decisions survived contact with concrete situations and should now be considered settled:

1. **Documents as one surface** rather than three agent entries (J6). Two engines served one request and the User never learned there were two. Had the sidebar offered "Slides agent", David would have had to know his numbers were in a spreadsheet before he could pick correctly.
2. **Steering rather than gating** (J2). The learned-preferences list is the product's most trust-building screen, and it only works because R8.4 forbids invisible preferences.
3. **The vocabulary lint** (all journeys). The original hand-drawn mockup contained *"DocX agent"*, *"Slides agent"*, *"Spreadsheet agent"*, *"social agent"*, *"digest agent"*, *"Model spend today"*, *"Runs today"* and *"First run, awaiting review"* — eight violations in one screen, by the person who wrote the rule. A build-time lint is not bureaucracy; it is the only thing that would have caught them.
