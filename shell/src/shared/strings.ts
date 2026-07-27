/**
 * Every string the User can read.
 *
 * Mirrors `core/crates/studio-strings` so that the same vocabulary rule can be
 * enforced on both sides (Requirements 1.1, 1.2). Components must reference a key
 * here rather than writing a literal, which is what lets `npm run lint:vocab`
 * check the whole product surface in one place.
 *
 * Scope decides how strictly the rule applies. It is absolute on `primary`
 * surfaces. `settings` may name a provider, because Requirement 14.7 confines
 * provider and model identifiers there. `diagnostics` exists to hold technical
 * detail for support (Requirement 17.5).
 */

export type Scope = "primary" | "settings" | "diagnostics";

export interface Entry {
  text: string;
  scope: Scope;
}

const p = (text: string): Entry => ({ text, scope: "primary" });
const s = (text: string): Entry => ({ text, scope: "settings" });
const d = (text: string): Entry => ({ text, scope: "diagnostics" });

export const CATALOGUE = {
  // ---- navigation ----
  "nav.new_work": p("New work"),
  "nav.dashboard": p("Dashboard"),
  "nav.your_work": p("Your work"),
  "nav.documents": p("Documents"),
  "nav.settings": p("Settings"),
  "nav.skip": p("Skip to what's on this screen"),
  "nav.collapse": p("Collapse this panel"),
  "nav.expand": p("Show this panel"),

  // ---- thread status ----
  "status.working": p("Working now"),
  "status.scheduled": p("Waiting for its time"),
  "status.needs_you": p("Needs you"),
  "status.done": p("Finished"),
  "status.paused": p("Paused"),

  // ---- dashboard ----
  "dash.metric.working": p("Working for you"),
  "dash.metric.waiting": p("Waiting on you"),
  "dash.metric.done": p("Done today"),
  "dash.metric.cost": p("Cost today"),
  "dash.waiting_heading": p("Waiting on you"),
  "dash.done_heading": p("Done for you"),
  "dash.running_heading": p("Running on their own"),
  "dash.nothing_waiting": p("Nothing needs you right now."),

  // ---- tray ----
  "tray.kickoff.label": p("First draft to check"),
  "tray.escalation.label": p("Needs your call"),
  "tray.finding.label": p("Worth knowing"),
  "tray.attention.label": p("Needs fixing"),
  "tray.read_it": p("Read it"),
  "tray.got_it": p("Got it"),
  "tray.nothing_expires": p("Nothing here expires and nothing gets decided for you."),
  "tray.heading": p("Waiting on you"),

  // ---- first draft review ----
  "kickoff.output.title": p("This is what I'd have emailed you"),
  "kickoff.output.sub": p("Nothing was sent. Change anything you like — I'll remember it."),
  "kickoff.manifest.title": p("Here's what I'd have done to your inbox"),
  "kickoff.manifest.sub": p("Your inbox is untouched. Uncheck anything you'd rather I left alone."),
  "kickoff.approve_daily": p("Looks good — send it daily at 7:00 am"),
  "kickoff.approve_recurring": p("Do this every 2 hours"),
  "kickoff.edit_first": p("Edit it first"),
  "kickoff.once": p("Just this once"),
  "kickoff.reject": p("Not quite — tell it why"),
  "kickoff.can_undo": p("Everything here can be undone from Done for you for 30 days."),
  "kickoff.open_draft": p("Open draft"),
  "kickoff.read_one": p("Want to read one of the drafts?"),
  "kickoff.manifest.count": p("42 messages this morning"),
  "kickoff.manifest.reversible": p("reversible"),
  "kickoff.manifest.you_send": p("you send them"),

  // ---- out tray ----
  "out.heading": p("Done for you"),
  "out.undo": p("Undo"),
  "out.undo_all": p("Undo all"),
  "out.take_down": p("Take it down"),
  "out.steer": p("Steer"),
  "out.see_list": p("See list"),
  "out.open_file": p("Open file"),
  "out.cannot_unsend": p("Can't be unsent"),
  "out.steer_saved": p("Saved. This applies from your next one."),
  "out.steer_placeholder": p("Tell it what to change next time…"),

  // ---- thread detail ----
  "thread.what_its_done": p("What it's done"),
  "thread.learned": p("What I've learned from you"),
  "thread.everything_i_go_on": p("This is everything I go on. Change or delete any of it."),
  "thread.new_note": p("Tell it something new…"),
  "thread.pause": p("Pause"),
  "thread.resume": p("Resume"),
  "thread.run_now": p("Run now"),
  "thread.edit": p("Edit"),

  // ---- new work ----
  "new.title": p("What do you need?"),
  "new.placeholder": p('Describe it — "a board deck from last quarter\'s numbers"'),
  "new.drop": p("Or drop a file here to keep working on it."),
  "new.recurring": p("Or hand over something that runs on its own"),
  "new.resume": p("Pick up where you left off"),
  "new.files_live": p("Your files live in Documents › Work Studio on your Mac."),
  "new.see_all": p("See all 10"),
  "new.try_it": p("Try it"),
  "new.ready": p("Ready"),

  // ---- recurring library + consent ----
  "library.title": p("What should I take off your hands?"),
  "library.sub": p("Pick one. I'll do it once and show you before anything leaves this computer."),
  "library.more": p("More to choose from"),
  "consent.title": p("Connect Gmail for Inbox triage"),
  "consent.intro": p("So it can do this job, it will:"),
  "consent.read": p("Read the messages in your inbox"),
  "consent.label": p("Add labels and archive what you don't need"),
  "consent.draft": p("Write draft replies — and leave them as drafts"),
  "consent.never_send": p("It will never send a message without you. Only Inbox triage uses this."),
  "consent.connect": p("Connect Gmail"),
  "consent.not_now": p("Not now"),

  // ---- documents ----
  "new.open_a_file": p("Open a file"),
  "dash.not_measured": p("not measured"),
  "tray.dismiss": p("Dismiss"),
  "settings.nothing_told_yet": p("You have not told me anything yet."),
  "thread.not_found": p("That piece of work is not here"),
  "thread.nothing_said_yet": p("Nothing has been said about this yet."),
  "thread.nothing_told_yet": p("You have not told me anything about this yet."),
  "thread.you": p("you"),
  "thread.me": p("Work Studio"),
  "new.could_not_start": p("I could not make a start on that."),
  "new.make_a_start": p("Make a start"),
  "new.starting": p("Making a start…"),
  "sheet.add": p("Add a sheet"),
  "doc.page": p("Page"),
  "present.heading": p("Presenting"),
  "present.nothing": p("There are no slides to present."),
  "present.back": p("Back a slide"),
  "present.forward": p("On a slide"),
  "present.notes": p("Your notes"),
  "present.leave": p("Stop presenting"),
  "deck.present": p("Present"),
  "sheet.rename": p("Rename this sheet"),
  "sheet.delete": p("Delete this sheet"),
  "sheet.delete_sure": p("Delete this sheet and everything on it?"),
  "out.cannot_undo": p("cannot be undone"),
  "out.already_undone": p("undone"),
  "diag.nothing_yet": d("Nothing recorded yet."),
  "caps.title": s("What each specialist can reach"),
  "caps.intro": s("Turn one off and nothing will use it. Give one to a specialist and it may use it for your work."),
  "caps.none": s("Nothing yet."),
  "caps.add_one": s("Add one"),
  "caps.add": s("Add"),
  "caps.remove": s("Remove"),
  "caps.turn_on": s("Turn on"),
  "caps.turn_off": s("Turn off"),
  "caps.used_by": s("Used by"),
  "caps.nobody": s("nobody yet"),
  "caps.needs": s("Needs:"),
  "caps.per_agent": s("What each one has been given"),
  "caps.nothing_given": s("Nothing yet — it cannot do this kind of work."),
  "caps.name_placeholder": s("What is it called?"),
  "caps.command_placeholder": s("What should Work Studio run?"),
  "caps.then_allocate": s("Once added, choose which specialists may use it."),
  "steer.new_placeholder": p("Tell it something new\u2026"),
  "steer.everything_i_go_on": p("This is everything I go on. Change or delete any of it."),
  "steer.nothing_yet": p("Nothing yet. Tell it how you like things done, or correct it and it will notice."),
  "steer.yes_do_that": p("Yes, do that"),
  "steer.reword": p("Say it differently"),
  "steer.no_thanks": p("No, forget it"),
  "steer.keep_this": p("Keep this"),
  "steer.forget": p("Forget"),
  "details.edit": p("Edit"),
  "common.never_mind": p("Never mind"),
  "nav.no_work_yet": p("Nothing yet. Open a file or describe what you need."),
  "repo.back_to_top": p("Back to all files"),
  "repo.new_folder_name": p("What should the folder be called?"),
  "repo.nothing_yet": p("Nothing here yet"),
  "repo.nothing_of_that_kind": p("Nothing of that kind here"),
  "repo.where_files_live": p("Files you and I make together live here, as ordinary files you can open anywhere."),
  "repo.col.size": p("Size"),
  "kickoff.not_done": p("What I left alone"),
  "doc.blocks": p("paragraphs"),
  "doc.selected_block": p("This paragraph is selected"),
  "doc.no_headings": p("No headings yet"),
  "deck.slides": p("slides"),
  "deck.selected_shape": p("This is selected"),
  "common.loading": p("Opening\u2026"),
  "doc.click_to_change": p("Click anything on the slide to change it yourself."),
  "doc.type_to_edit": p("Type anywhere to edit."),
  "doc.recalc_here": p("Formulas recalculate here, not in Excel."),
  "doc.changes_by_me": p("changes by me"),
  "doc.i_added_this": p("I added this"),
  "doc.ask_change": p("Ask for a change…"),
  "doc.in_this_document": p("In this document"),
  "doc.sheets": p("Sheets"),
  "doc.slides": p("Slides"),
  "doc.named_things": p("Named things"),
  "doc.show_the_file": p("Show the file"),
  "doc.chat": p("Chat"),
  "doc.details": p("Details"),
  "doc.format": p("Format"),

  // ---- details pane ----
  "details.what_changed": p("What changed"),
  "details.where_from": p("Where this came from"),
  "details.worth_knowing": p("Worth knowing"),
  "details.versions": p("Versions"),
  "details.undo_mine": p("Undo mine"),
  "details.review_all": p("Review all"),
  "details.go_back": p("Go back"),
  "details.by_me": p("by me"),
  "details.by_you": p("by you"),

  // ---- repository ----
  "repo.all_files": p("All files"),
  "repo.new_folder": p("New folder"),
  "repo.show_in_finder": p("Show in Finder"),
  "repo.search": p("Search files…"),
  "repo.col.name": p("Name"),
  "repo.col.changed": p("Changed"),
  "repo.col.used_in": p("Used in"),
  "repo.col.versions": p("Versions"),
  "repo.kind.everything": p("Everything"),
  "repo.kind.documents": p("Documents"),
  "repo.kind.decks": p("Decks"),
  "repo.kind.spreadsheets": p("Spreadsheets"),
  "repo.kind.pdfs": p("PDFs"),
  "repo.real_folders": p(
    "Folders here are real folders on your Mac. Move or rename them anywhere and I'll keep up.",
  ),

  // ---- honest limits ----
  "limits.title": p("I'd lose your lawyer's markup"),
  "limits.work_on_copy": p("Work on a copy instead"),
  "limits.just_tell_me": p("Just tell me what to change"),
  "limits.i_check": p("I check every file this way before I edit it."),

  // ---- failures ----
  "fail.reconnect": p("Reconnect"),
  "fail.stopped_trying": p("I've stopped trying."),
  "fail.internal": p("Something went wrong. Your work is saved."),
  "fail.see_whats_big": p("See what's big"),

  // ---- first run ----
  "firstrun.welcome": p("Welcome to Zavora Work Studio"),
  "firstrun.privacy": p(
    "Your work stays on this computer. Paste one key to get started — you can change it later.",
  ),
  "firstrun.key_label": p("OpenAI key"),
  "firstrun.key_hint": p("Stored in your Mac keychain. Never sent anywhere except OpenAI."),
  "firstrun.start": p("Start working"),
  "firstrun.other_provider": p("Use a different service"),
  "firstrun.get_a_key": p("I don't have one yet"),

  // ---- settings ----
  "settings.title": s("Settings"),
  "settings.tab.general": s("General"),
  "settings.tab.how_i_work": s("How I should work"),
  "settings.tab.agents": s("Agents"),
  "settings.tab.accounts": s("Accounts"),
  "settings.tab.files": s("Files"),
  "settings.tab.spending": s("Spending"),
  "settings.tab.privacy": s("Privacy"),
  "settings.ai_key": s("Your AI key"),
  "settings.ai_key_hint": s("Kept in your Mac keychain"),
  "settings.working": s("working"),
  "settings.replace": s("Replace"),
  "settings.add_provider": s("Add another provider"),
  "settings.how_hard": s("How hard to think"),
  "settings.how_hard_hint": s("I pick the right level for each piece of work. This nudges it."),
  "settings.tier.cheap": s("Spend less"),
  "settings.tier.balanced": s("Balanced"),
  "settings.tier.best": s("Best quality"),
  "settings.launch": s("Start when I log in"),
  "settings.launch_hint": s("Needed for work that runs to a schedule"),
  "settings.files_live": s("Where your files live"),
  "settings.files_live_hint": s("Ordinary files you can open anywhere"),
  "settings.change": s("Change"),
  "settings.daily_limit": s("Daily limit"),
  "settings.daily_limit_hint": s(
    "Scheduled work pauses if it's reached. Your own work never stops.",
  ),
  "settings.used_today": s("used today"),
  "settings.your_data": s("Your data"),
  "settings.your_data_hint": s("Everything is on this computer"),
  "settings.what_leaves": s("What leaves this computer"),
  "settings.export": s("Export everything"),
  "settings.delete": s("Delete everything"),
  "settings.not_working": s("Something not working?"),
  "settings.technical_details": s("Technical details"),
  "settings.support_only": s("for support only"),
  "settings.how_i_work_intro": s(
    "This is everything I've learned about how you like things done. It applies to all your work. Change or delete any of it.",
  ),
  "settings.thread_wins": s(
    "If a single piece of work has its own instructions, those win over these.",
  ),
  "settings.scope.everything": s("Everything"),
  "settings.scope.documents": s("Documents"),
  "settings.scope.decks": s("Decks"),
  "settings.scope.spreadsheets": s("Spreadsheets"),
  "settings.new_note": s("Tell me something new…"),
  "settings.disconnect": s("Disconnect"),

  // ---- privacy ----
  "privacy.title": s("What leaves this computer"),
  "privacy.sub": s(
    "Everything else — your files, your tasks, what I've learned about you — stays here.",
  ),
  "privacy.your_words": s("Your words and files"),
  "privacy.to_write": s("to write and reason"),
  "privacy.outputs": s("Newsletters and posts"),
  "privacy.because": s("because you asked them to go there"),
  "privacy.accounts": s("Connected accounts"),
  "privacy.locked": s("Locked on this disk"),
  "privacy.only_part": s(
    "Only the part of a task that needs writing. Nothing is kept for training on business accounts.",
  ),


  // ---- settings: accounts ----
  "accounts.intro": s("Accounts you have connected, and what each one is allowed to do."),
  "accounts.add": s("Connect an account"),
  "accounts.reconnect": s("Reconnect"),
  "accounts.expired": s("Sign-in expired"),
  "accounts.used_by": s("Used by"),
  "accounts.none": s("Nothing is connected yet. You'll be asked when a piece of work needs an account."),

  // ---- settings: files ----
  "files.intro": s("Ordinary files in an ordinary folder. Move them, rename them, email them."),
  "files.where": s("Where your files live"),
  "files.reveal": s("Show in Finder"),
  "files.per_job": s("Folders for work that runs on its own"),
  "files.per_job_hint": s("Each piece of recurring work writes into a folder you choose."),
  "files.usage": s("Taking up"),
  "files.keep_versions": s("Keep version history"),
  "files.keep_versions_hint": s("Lets you go back to any earlier draft. Stored beside your files."),

  // ---- settings: spending ----
  "spend.intro": s("What this has cost you, and the limit you set."),
  "spend.today": s("Today"),
  "spend.this_month": s("This month"),
  "spend.limit": s("Daily limit"),
  "spend.by_work": s("By piece of work"),
  "spend.by_agent": s("By agent"),
  "spend.paused_note": s("When the limit is reached, scheduled work pauses and your own work carries on."),

  // ---- diagnostics ----
  "diag.intro": d("For support. Nothing here is needed to use Work Studio, and nothing here is sent anywhere."),
  "diag.copy": d("Copy for support"),
  "diag.recent": d("Recent activity"),
  "diag.versions": d("Versions"),
  "diag.gaps": d("What we can't tell you yet"),
  "diag.title": d("Technical details"),
  "diag.subtitle": d("For support. Nothing here is needed to use Work Studio."),
} as const satisfies Record<string, Entry>;

export type StringKey = keyof typeof CATALOGUE;

/** Look up a User-visible string. */
export function t(key: StringKey): string {
  return CATALOGUE[key].text;
}
