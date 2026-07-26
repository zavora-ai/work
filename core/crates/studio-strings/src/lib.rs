//! Every string the User can read lives here, and nowhere else.
//!
//! This exists so that the vocabulary rule in Requirement 1.1 can be enforced
//! mechanically rather than by review. Interface code must reference a catalogue
//! entry; inline literals in components are rejected by a lint (task 1.6).
//!
//! Each entry carries a [`Scope`]. The prohibition is absolute on
//! [`Scope::Primary`] surfaces. Settings may name a provider because Requirement
//! 14.7 confines provider and model identifiers there, and Diagnostics exists
//! precisely to hold technical detail for support (Requirement 17.5).

/// Where a string can appear. Determines how strictly the vocabulary rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Dashboard, trays, threads, the Documents workspace and the Repository.
    /// The vocabulary rule is absolute here.
    Primary,
    /// Settings. May name a provider or a model, per Requirement 14.7.
    Settings,
    /// The single technical-details view, reachable only from Settings.
    Diagnostics,
}

/// One User-visible string.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub key: &'static str,
    pub text: &'static str,
    pub scope: Scope,
}

const fn p(key: &'static str, text: &'static str) -> Entry {
    Entry {
        key,
        text,
        scope: Scope::Primary,
    }
}
const fn s(key: &'static str, text: &'static str) -> Entry {
    Entry {
        key,
        text,
        scope: Scope::Settings,
    }
}
const fn d(key: &'static str, text: &'static str) -> Entry {
    Entry {
        key,
        text,
        scope: Scope::Diagnostics,
    }
}

/// The catalogue. Copy is taken from the mockups in
/// `.kiro/specs/zavora-work-studio/mockups/`, which are the normative reference
/// for register.
pub const CATALOGUE: &[Entry] = &[
    // ---- navigation ----
    p("nav.new_work", "New work"),
    p("nav.dashboard", "Dashboard"),
    p("nav.your_work", "Your work"),
    p("nav.documents", "Documents"),
    p("nav.settings", "Settings"),
    // ---- dashboard ----
    p("dash.metric.working", "Working for you"),
    p("dash.metric.waiting", "Waiting on you"),
    p("dash.metric.done", "Done today"),
    p("dash.metric.cost", "Cost today"),
    p("dash.waiting_heading", "Waiting on you"),
    p("dash.done_heading", "Done for you"),
    p("dash.running_heading", "Running on their own"),
    // ---- thread status ----
    p("status.working", "Working now"),
    p("status.scheduled", "Waiting for its time"),
    p("status.needs_you", "Needs you"),
    p("status.done", "Finished"),
    p("status.paused", "Paused"),
    // ---- tray classes ----
    p("tray.kickoff.label", "First draft to check"),
    p("tray.escalation.label", "Needs your call"),
    p("tray.finding.label", "Worth knowing"),
    p("tray.attention.label", "Needs fixing"),
    p(
        "tray.nothing_expires",
        "Nothing here expires and nothing gets decided for you.",
    ),
    // ---- kickoff review ----
    p("kickoff.output.title", "This is what I'd have emailed you"),
    p(
        "kickoff.output.sub",
        "Nothing was sent. Change anything you like — I'll remember it.",
    ),
    p(
        "kickoff.manifest.title",
        "Here's what I'd have done to your inbox",
    ),
    p(
        "kickoff.manifest.sub",
        "Your inbox is untouched. Uncheck anything you'd rather I left alone.",
    ),
    p("kickoff.approve", "Looks good"),
    p("kickoff.edit_first", "Edit it first"),
    p("kickoff.once", "Just this once"),
    p("kickoff.reject", "Not quite — tell it why"),
    // ---- out tray ----
    p("out.undo", "Undo"),
    p("out.take_down", "Take it down"),
    p("out.steer", "Steer"),
    p("out.cannot_unsend", "Can't be unsent"),
    p("out.steer_saved", "Saved. This applies from your next one."),
    // ---- steering ----
    p("steer.heading", "What I've learned from you"),
    p(
        "steer.everything_i_go_on",
        "This is everything I go on. Change or delete any of it.",
    ),
    p("steer.new_placeholder", "Tell it something new…"),
    // ---- documents ----
    p("new.open_a_file", "Open a file"),
    p(
        "steer.nothing_yet",
        "Nothing yet. Tell it how you like things done, or correct it and it will notice.",
    ),
    p("steer.yes_do_that", "Yes, do that"),
    p("steer.reword", "Say it differently"),
    p("steer.no_thanks", "No, forget it"),
    p("steer.keep_this", "Keep this"),
    p("steer.forget", "Forget"),
    p("details.edit", "Edit"),
    p("common.never_mind", "Never mind"),
    p(
        "nav.no_work_yet",
        "Nothing yet. Open a file or describe what you need.",
    ),
    p("repo.back_to_top", "Back to all files"),
    p("repo.new_folder_name", "What should the folder be called?"),
    p("repo.nothing_yet", "Nothing here yet"),
    p("repo.nothing_of_that_kind", "Nothing of that kind here"),
    p(
        "repo.where_files_live",
        "Files you and I make together live here, as ordinary files you can open anywhere.",
    ),
    p("repo.col.size", "Size"),
    p("kickoff.not_done", "What I left alone"),
    p("doc.blocks", "paragraphs"),
    p("doc.in_this_document", "In this document"),
    p("doc.no_headings", "No headings yet"),
    p("doc.selected_block", "This paragraph is selected"),
    p("deck.slides", "slides"),
    p("deck.selected_shape", "This is selected"),
    p("common.loading", "Opening…"),
    p("doc.new.title", "What do you need?"),
    p(
        "doc.new.placeholder",
        "Describe it — \"a board deck from last quarter's numbers\"",
    ),
    p("doc.new.drop", "Or drop a file here to keep working on it."),
    p(
        "doc.new.recurring",
        "Or hand over something that runs on its own",
    ),
    p("doc.new.resume", "Pick up where you left off"),
    p(
        "doc.click_to_change",
        "Click anything on the slide to change it yourself.",
    ),
    p("doc.type_to_edit", "Type anywhere to edit."),
    p(
        "doc.recalc_here",
        "Formulas recalculate here, not in Excel.",
    ),
    p("doc.changes_by_me", "changes by me"),
    p("doc.i_added_this", "I added this"),
    p("doc.ask_change", "Ask for a change…"),
    // ---- details pane ----
    p("details.what_changed", "What changed"),
    p("details.where_from", "Where this came from"),
    p("details.worth_knowing", "Worth knowing"),
    p("details.versions", "Versions"),
    p("details.undo_mine", "Undo mine"),
    p("details.review_all", "Review all"),
    p("details.go_back", "Go back"),
    // ---- repository ----
    p("repo.all_files", "All files"),
    p("repo.new_folder", "New folder"),
    p("repo.show_in_finder", "Show in Finder"),
    p("repo.col.name", "Name"),
    p("repo.col.changed", "Changed"),
    p("repo.col.used_in", "Used in"),
    p("repo.col.versions", "Versions"),
    p(
        "repo.real_folders",
        "Folders here are real folders on your Mac. Move or rename them anywhere and I'll keep up.",
    ),
    // ---- fidelity refusal ----
    p("fidelity.title", "I'd lose your lawyer's markup"),
    p("fidelity.work_on_copy", "Work on a copy instead"),
    p("fidelity.just_tell_me", "Just tell me what to change"),
    p(
        "fidelity.i_check_every_file",
        "I check every file this way before I edit it.",
    ),
    // ---- failures, in the User's terms ----
    p("fail.reconnect_account", "Gmail needs reconnecting"),
    p("fail.stopped_trying", "I've stopped trying."),
    p("fail.internal", "Something went wrong. Your work is saved."),
    // ---- first run ----
    p("firstrun.welcome", "Welcome to Zavora Work Studio"),
    p("firstrun.privacy", "Your work stays on this computer."),
    p("firstrun.start", "Start working"),
    // ---- settings: provider names are permitted here (Requirement 14.7) ----
    s("settings.title", "Settings"),
    s("settings.ai_key", "Your AI key"),
    s("settings.ai_key_hint", "Kept in your Mac keychain"),
    s("settings.replace_key", "Replace"),
    s("settings.add_provider", "Add another provider"),
    s("settings.how_hard", "How hard to think"),
    s("settings.tier.cheap", "Spend less"),
    s("settings.tier.balanced", "Balanced"),
    s("settings.tier.best", "Best quality"),
    s("settings.launch_at_login", "Start when I log in"),
    s("settings.files_live", "Where your files live"),
    s("settings.daily_limit", "Daily limit"),
    s("settings.your_data", "Your data"),
    s("settings.what_leaves", "What leaves this computer"),
    s("settings.export_all", "Export everything"),
    s("settings.delete_all", "Delete everything"),
    s("settings.how_i_work", "How I should work"),
    s(
        "settings.how_i_work_intro",
        "This is everything I've learned about how you like things done. It applies to all your work.",
    ),
    s(
        "settings.thread_wins",
        "If a single piece of work has its own instructions, those win over these.",
    ),
    s("settings.scope.everything", "Everything"),
    s("settings.scope.documents", "Documents"),
    s("settings.scope.decks", "Decks"),
    s("settings.scope.spreadsheets", "Spreadsheets"),
    s("settings.technical_details", "Technical details"),
    s("settings.for_support_only", "for support only"),
    // ---- diagnostics: the only place technical detail is allowed ----
    d("diag.title", "Technical details"),
    d(
        "diag.subtitle",
        "For support. Nothing here is needed to use Work Studio.",
    ),
    // ---- mirrored from the Shell ----
    //
    // These were only ever written in the Shell, which meant the vocabulary rule
    // never saw them. They are held here so that it does.
    p("nav.skip", "Skip to what's on this screen"),
    p("nav.collapse", "Collapse this panel"),
    p("nav.expand", "Show this panel"),
    p("dash.nothing_waiting", "Nothing needs you right now."),
    p("tray.read_it", "Read it"),
    p("tray.got_it", "Got it"),
    p("tray.heading", "Waiting on you"),
    p(
        "kickoff.approve_daily",
        "Looks good — send it daily at 7:00 am",
    ),
    p("kickoff.approve_recurring", "Do this every 2 hours"),
    p(
        "kickoff.can_undo",
        "Everything here can be undone from Done for you for 30 days.",
    ),
    p("kickoff.open_draft", "Open draft"),
    p("kickoff.read_one", "Want to read one of the drafts?"),
    p("kickoff.manifest.count", "42 messages this morning"),
    p("kickoff.manifest.reversible", "reversible"),
    p("kickoff.manifest.you_send", "you send them"),
    p("out.heading", "Done for you"),
    p("out.undo_all", "Undo all"),
    p("out.see_list", "See list"),
    p("out.open_file", "Open file"),
    p("out.steer_placeholder", "Tell it what to change next time…"),
    p("thread.what_its_done", "What it's done"),
    p("thread.learned", "What I've learned from you"),
    p(
        "thread.everything_i_go_on",
        "This is everything I go on. Change or delete any of it.",
    ),
    p("thread.new_note", "Tell it something new…"),
    p("thread.pause", "Pause"),
    p("thread.resume", "Resume"),
    p("thread.run_now", "Run now"),
    p("thread.edit", "Edit"),
    p("new.title", "What do you need?"),
    p("new.drop", "Or drop a file here to keep working on it."),
    p(
        "new.recurring",
        "Or hand over something that runs on its own",
    ),
    p("new.resume", "Pick up where you left off"),
    p(
        "new.files_live",
        "Your files live in Documents › Work Studio on your Mac.",
    ),
    p("new.see_all", "See all 10"),
    p("new.try_it", "Try it"),
    p("new.ready", "Ready"),
    p("library.title", "What should I take off your hands?"),
    p(
        "library.sub",
        "Pick one. I'll do it once and show you before anything leaves this computer.",
    ),
    p("library.more", "More to choose from"),
    p("consent.title", "Connect Gmail for Inbox triage"),
    p("consent.intro", "So it can do this job, it will:"),
    p("consent.read", "Read the messages in your inbox"),
    p(
        "consent.label",
        "Add labels and archive what you don't need",
    ),
    p(
        "consent.draft",
        "Write draft replies — and leave them as drafts",
    ),
    p(
        "consent.never_send",
        "It will never send a message without you. Only Inbox triage uses this.",
    ),
    p("consent.connect", "Connect Gmail"),
    p("consent.not_now", "Not now"),
    p("doc.sheets", "Sheets"),
    p("doc.slides", "Slides"),
    p("doc.named_things", "Named things"),
    p("doc.show_the_file", "Show the file"),
    p("doc.chat", "Chat"),
    p("doc.details", "Details"),
    p("doc.format", "Format"),
    p("details.by_me", "by me"),
    p("details.by_you", "by you"),
    p("repo.search", "Search files…"),
    p("repo.kind.everything", "Everything"),
    p("repo.kind.documents", "Documents"),
    p("repo.kind.decks", "Decks"),
    p("repo.kind.spreadsheets", "Spreadsheets"),
    p("repo.kind.pdfs", "PDFs"),
    p("limits.title", "I'd lose your lawyer's markup"),
    p("limits.work_on_copy", "Work on a copy instead"),
    p("limits.just_tell_me", "Just tell me what to change"),
    p(
        "limits.i_check",
        "I check every file this way before I edit it.",
    ),
    p("fail.reconnect", "Reconnect"),
    p("fail.see_whats_big", "See what's big"),
    p("firstrun.key_label", "OpenAI key"),
    p(
        "firstrun.key_hint",
        "Stored in your Mac keychain. Never sent anywhere except OpenAI.",
    ),
    p("firstrun.other_provider", "Use a different service"),
    p("firstrun.get_a_key", "I don't have one yet"),
    s("settings.tab.general", "General"),
    s("settings.tab.how_i_work", "How I should work"),
    s("settings.tab.agents", "Agents"),
    s("settings.tab.accounts", "Accounts"),
    s("settings.tab.files", "Files"),
    s("settings.tab.spending", "Spending"),
    s("settings.tab.privacy", "Privacy"),
    s("settings.working", "working"),
    s("settings.replace", "Replace"),
    s(
        "settings.how_hard_hint",
        "I pick the right level for each piece of work. This nudges it.",
    ),
    s("settings.launch", "Start when I log in"),
    s(
        "settings.launch_hint",
        "Needed for work that runs to a schedule",
    ),
    s(
        "settings.files_live_hint",
        "Ordinary files you can open anywhere",
    ),
    s("settings.change", "Change"),
    s("settings.used_today", "used today"),
    s("settings.your_data_hint", "Everything is on this computer"),
    s("settings.export", "Export everything"),
    s("settings.delete", "Delete everything"),
    s("settings.not_working", "Something not working?"),
    s("settings.support_only", "for support only"),
    s("settings.new_note", "Tell me something new…"),
    s("settings.disconnect", "Disconnect"),
    s("privacy.title", "What leaves this computer"),
    s("privacy.your_words", "Your words and files"),
    s("privacy.to_write", "to write and reason"),
    s("privacy.outputs", "Newsletters and posts"),
    s("privacy.because", "because you asked them to go there"),
    s("privacy.accounts", "Connected accounts"),
    s("privacy.locked", "Locked on this disk"),
    s(
        "accounts.intro",
        "Accounts you have connected, and what each one is allowed to do.",
    ),
    s("accounts.add", "Connect an account"),
    s("accounts.reconnect", "Reconnect"),
    s("accounts.expired", "Sign-in expired"),
    s("accounts.used_by", "Used by"),
    s(
        "accounts.none",
        "Nothing is connected yet. You'll be asked when a piece of work needs an account.",
    ),
    s(
        "files.intro",
        "Ordinary files in an ordinary folder. Move them, rename them, email them.",
    ),
    s("files.where", "Where your files live"),
    s("files.reveal", "Show in Finder"),
    s("files.per_job", "Folders for work that runs on its own"),
    s(
        "files.per_job_hint",
        "Each piece of recurring work writes into a folder you choose.",
    ),
    s("files.usage", "Taking up"),
    s("files.keep_versions", "Keep version history"),
    s(
        "files.keep_versions_hint",
        "Lets you go back to any earlier draft. Stored beside your files.",
    ),
    s(
        "spend.intro",
        "What this has cost you, and the limit you set.",
    ),
    s("spend.today", "Today"),
    s("spend.this_month", "This month"),
    s("spend.limit", "Daily limit"),
    s("spend.by_work", "By piece of work"),
    s("spend.by_agent", "By agent"),
    s(
        "spend.paused_note",
        "When the limit is reached, scheduled work pauses and your own work carries on.",
    ),
    d(
        "diag.intro",
        "For support. Nothing here is needed to use Work Studio, and nothing here is sent anywhere.",
    ),
    d("diag.copy", "Copy for support"),
    d("diag.recent", "Recent activity"),
    d("diag.versions", "Versions"),
    d("diag.gaps", "What we can't tell you yet"),
];

pub fn get(key: &str) -> Option<&'static Entry> {
    CATALOGUE.iter().find(|e| e.key == key)
}

pub fn text(key: &str) -> &'static str {
    get(key).map(|e| e.text).unwrap_or_else(|| {
        panic!("missing catalogue key: {key}");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn keys_are_unique() {
        let mut seen = HashSet::new();
        for e in CATALOGUE {
            assert!(seen.insert(e.key), "duplicate catalogue key: {}", e.key);
        }
    }

    #[test]
    fn no_entry_is_empty() {
        for e in CATALOGUE {
            assert!(!e.text.trim().is_empty(), "empty text for {}", e.key);
        }
    }

    #[test]
    fn lookup_works() {
        assert_eq!(text("nav.new_work"), "New work");
    }
}
