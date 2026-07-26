//! What the Core keeps, and how the interface asks for it.
//!
//! Everything here is durable. The store and its twelve tables were built and tested early
//! and then nothing wrote to them, so closing the application forgot everything that had
//! happened in it — which made the product impossible to try for more than one sitting.
//!
//! One lock around the connection. A desktop application has one User doing one thing at a
//! time, so contention is not the problem to solve; losing their work is.

use std::sync::{Arc, Mutex};

use serde::Serialize;
use studio_artefacts::home::Home;
use studio_steering::{Note, Origin, Scope, Steering};
use studio_store::Store;

/// Everything the User has told Work Studio, and everything it has noticed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteeringView {
    /// Notes that are being acted on.
    pub notes: Vec<NoteView>,
    /// Things Work Studio worked out for itself and is waiting to be told about. These
    /// influence nothing until accepted.
    pub proposed: Vec<NoteView>,
    /// Notes that apply to everything, not just this piece of work.
    pub global: Vec<NoteView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteView {
    pub id: String,
    pub note: String,
    /// Where it came from, in the User's terms. The list claims to be everything Work
    /// Studio goes on, so a note that cannot say where it came from would make that false.
    pub provenance: String,
    /// The question to put to the User, for something not yet accepted.
    pub asks: Option<String>,
    pub scope: String,
}

impl NoteView {
    fn of(note: &Note) -> Self {
        Self {
            id: note.id.clone(),
            note: note.text.clone(),
            provenance: describe_origin(note.origin),
            asks: (!note.confirmed).then(|| note.origin.confirmation_prompt(&note.text)),
            scope: note.scope.label().to_string(),
        }
    }
}

/// Where a note came from, in the User's terms.
fn describe_origin(origin: Origin) -> String {
    match origin {
        Origin::Explicit => "You told me".to_string(),
        Origin::Rejection => "You said no to a first draft".to_string(),
        Origin::DerivedFromEdit => "I noticed a change you made".to_string(),
        Origin::DerivedFromExclusion => "I noticed something you left out".to_string(),
        Origin::DerivedFromChoice => "I noticed a choice you made".to_string(),
    }
}

/// A piece of work the User has done.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadView {
    pub id: String,
    /// What the User asked for, which is how they will recognise it.
    pub purpose: String,
    /// The file it is about.
    pub file: Option<String>,
    pub changed: i64,
}

/// One turn of a conversation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnView {
    pub from: String,
    pub text: String,
}

/// The Core's durable side.
pub struct Keeper {
    store: Mutex<Store>,
    home: Home,
}

impl Keeper {
    /// Open the store under the given directory and the User's own folder.
    pub fn open(data_dir: &std::path::Path) -> Result<Arc<Self>, String> {
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let mut store = Store::open(data_dir.join("studio.db")).map_err(|e| e.to_string())?;
        store.migrate().map_err(|e| e.to_string())?;
        let home = Home::open_default().map_err(|e| e.to_string())?;
        Ok(Arc::new(Self {
            store: Mutex::new(store),
            home,
        }))
    }

    /// The same, in a chosen folder. For tests and for when the User moves it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn open_at(
        data_dir: &std::path::Path,
        home: &std::path::Path,
    ) -> Result<Arc<Self>, String> {
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let mut store = Store::open(data_dir.join("studio.db")).map_err(|e| e.to_string())?;
        store.migrate().map_err(|e| e.to_string())?;
        let home = Home::open_at(home).map_err(|e| e.to_string())?;
        Ok(Arc::new(Self {
            store: Mutex::new(store),
            home,
        }))
    }

    pub fn home(&self) -> &Home {
        &self.home
    }

    /// A piece of work the User is doing, made if it is not there yet.
    ///
    /// A thread is a piece of work, which is what the store already calls a Job — so it is
    /// stored as one rather than as a second thing that means the same. That is also what
    /// makes the notes attach to it: a note belongs to a piece of work.
    pub fn ensure_thread(&self, id: &str, purpose: &str, file: Option<&str>) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        store
            .conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, output_folder,
                                   created_at, updated_at)
                 VALUES (?1, 'one_off', ?2, 'active', ?3, ?4, unixepoch(), unixepoch())
                 ON CONFLICT(id) DO UPDATE SET
                     -- Keep the name the work already has. A note added later arrives with a
                     -- placeholder, and overwriting with it lost what the User had asked for,
                     -- so their own list showed the placeholder instead of their words.
                     purpose = CASE
                         WHEN jobs.purpose = ?5 THEN excluded.purpose
                         ELSE jobs.purpose
                     END,
                     -- Keep a file once known: a note added before a file was opened must
                     -- not erase which file the work is about.
                     output_folder = COALESCE(excluded.output_folder, jobs.output_folder),
                     updated_at = unixepoch()",
                rusqlite::params![id, purpose, local_timezone(), file, PLACEHOLDER_PURPOSE],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The pieces of work the User has done, most recent first.
    ///
    /// This is what the interface lists as "Your work". It was six invented threads.
    pub fn threads(&self) -> Result<Vec<ThreadView>, String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        let mut statement = store
            .conn()
            .prepare(
                "SELECT id, purpose, output_folder, updated_at
                 FROM jobs WHERE kind = 'one_off'
                 ORDER BY updated_at DESC LIMIT 50",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(ThreadView {
                    id: row.get(0)?,
                    purpose: row.get(1)?,
                    file: row.get(2)?,
                    changed: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Keep a turn of the conversation, so returning to a piece of work does not begin again.
    ///
    /// Only reachable in a build that can do the work, since a build with nothing to think
    /// with never produces a turn.
    #[cfg_attr(not(feature = "adk"), allow(dead_code))]
    pub fn remember_turn(&self, thread: &str, from: &str, text: &str) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        store
            .conn()
            .execute(
                "INSERT INTO thread_turns (thread_id, said_by, text, ts)
                 VALUES (?1, ?2, ?3, unixepoch())",
                rusqlite::params![thread, from, text],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The conversation so far, oldest first.
    pub fn turns(&self, thread: &str) -> Result<Vec<TurnView>, String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        let mut statement = store
            .conn()
            .prepare(
                "SELECT said_by, text FROM thread_turns
                 WHERE thread_id = ?1 ORDER BY seq ASC LIMIT 200",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([thread], |row| {
                Ok(TurnView {
                    from: row.get(0)?,
                    text: row.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// What the User has told Work Studio about this piece of work, and about everything.
    pub fn steering_view(&self, thread: Option<&str>) -> Result<SteeringView, String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        let steering = Steering::new(&store);

        let mut notes = Vec::new();
        let mut proposed = Vec::new();
        if let Some(thread) = thread {
            for note in steering
                .visible_for_job(thread)
                .map_err(|e| e.to_string())?
            {
                let view = NoteView::of(&note);
                if note.confirmed {
                    notes.push(view);
                } else {
                    proposed.push(view);
                }
            }
        }
        let global = steering
            .visible_global()
            .map_err(|e| e.to_string())?
            .iter()
            .map(NoteView::of)
            .collect();

        Ok(SteeringView {
            notes,
            proposed,
            global,
        })
    }

    /// Keep something the User told Work Studio.
    ///
    /// Anything the User types is accepted immediately: they said it, so there is nothing to
    /// confirm. Only what Work Studio worked out for itself has to be agreed to.
    pub fn add_note(&self, thread: Option<&str>, text: &str) -> Result<NoteView, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("an empty note".to_string());
        }
        // A note belongs to a piece of work, so telling Work Studio something about work it
        // has not seen yet creates that work rather than refusing the note.
        if let Some(thread) = thread {
            self.ensure_thread(thread, PLACEHOLDER_PURPOSE, None)?;
        }
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        let steering = Steering::new(&store);
        let id = new_id("note");
        let note = match thread {
            Some(thread) => steering
                .add_for_job(&id, thread, text, Origin::Explicit)
                .map_err(|e| e.to_string())?,
            None => steering
                .add_global(&id, Scope::Everything, text, Origin::Explicit)
                .map_err(|e| e.to_string())?,
        };
        Ok(NoteView::of(&note))
    }

    /// Something Work Studio noticed, offered for the User to accept.
    ///
    /// Nothing derives preferences yet — that is task 19.3 — so this is exercised by tests
    /// and by nothing else. It is kept because the acceptance rule it implements is the part
    /// worth getting right before anything depends on it.
    #[cfg_attr(not(test), allow(dead_code))]
    ///
    /// Held unconfirmed, so it influences nothing until the User agrees.
    pub fn propose_note(
        &self,
        thread: &str,
        text: &str,
        origin: Origin,
    ) -> Result<NoteView, String> {
        self.ensure_thread(thread, PLACEHOLDER_PURPOSE, None)?;
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        let steering = Steering::new(&store);
        let id = new_id("noticed");
        let note = steering
            .add_for_job(&id, thread, text, origin)
            .map_err(|e| e.to_string())?;
        Ok(NoteView::of(&note))
    }

    /// Accept, reword, stop applying, or forget a note.
    pub fn act_on_note(&self, id: &str, action: &str, text: Option<&str>) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        let steering = Steering::new(&store);
        match action {
            "accept" => steering.confirm(id).map_err(|e| e.to_string()),
            "reword" => {
                let text = text.unwrap_or("").trim();
                if text.is_empty() {
                    return Err("an empty note".to_string());
                }
                steering.reword(id, text).map_err(|e| e.to_string())?;
                // Rewording is the User saying it in their own words, which is agreement.
                steering.confirm(id).map_err(|e| e.to_string())
            }
            "stop" => steering.deactivate(id).map_err(|e| e.to_string()),
            "forget" => steering.delete(id).map_err(|e| e.to_string()),
            other => Err(format!("no such action: {other}")),
        }
    }

    /// The notes a run should be given, resolved here so nothing the User cannot see can
    /// influence it.
    pub fn notes_for_run(&self, thread: &str, kind: Option<&str>) -> Vec<String> {
        let Ok(store) = self.store.lock() else {
            return Vec::new();
        };
        let steering = Steering::new(&store);
        let artefact_kind = match kind {
            Some("spreadsheet") => Some(studio_steering::ArtefactKind::Spreadsheet),
            Some("document") => Some(studio_steering::ArtefactKind::Document),
            Some("deck") => Some(studio_steering::ArtefactKind::Deck),
            _ => None,
        };
        steering
            .resolve_for_run(thread, artefact_kind)
            .map(|notes| notes.into_iter().map(|note| note.text).collect())
            .unwrap_or_default()
    }

    /// What each specialist may reach.
    pub fn capabilities(&self) -> Result<Vec<crate::capabilities::CapabilityView>, String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        crate::capabilities::Capabilities::new(&store).list()
    }

    /// Add a connection the User has described.
    pub fn add_capability(&self, new: &crate::capabilities::NewCapability) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        let id = new
            .label
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>();
        crate::capabilities::Capabilities::new(&store).add(&id, new, false)
    }

    /// Turn one on or off, remove it, or say which specialists may use it.
    pub fn act_on_capability(
        &self,
        id: &str,
        action: &str,
        agents: &[String],
    ) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        let capabilities = crate::capabilities::Capabilities::new(&store);
        match action {
            "on" => capabilities.set_enabled(id, true),
            "off" => capabilities.set_enabled(id, false),
            "remove" => capabilities.remove(id),
            "allocate" => capabilities.allocate(id, agents),
            other => Err(format!("no such action: {other}")),
        }
    }

    /// Provision what came with Work Studio, so Settings has something true to show.
    ///
    /// Written once. A connection the User has since turned off stays off, because the
    /// insert leaves `enabled` alone on conflict.
    #[cfg_attr(not(feature = "adk"), allow(dead_code))]
    pub fn provision(&self, siblings: &std::path::Path) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "the store was left locked")?;
        let capabilities = crate::capabilities::Capabilities::new(&store);
        for (id, label, relative, agent) in [
            (
                "spreadsheets",
                "Spreadsheets",
                "mcp-servers/worksheet-mcp/target/debug/excel-mcp-server",
                "spreadsheet",
            ),
            (
                "documents",
                "Documents",
                "mcp-servers/docx-mcp/target/debug/docx-mcp-server",
                "document",
            ),
            (
                "presentations",
                "Presentations",
                "mcp-servers/mcp-slides/target/debug/slides-mcp-server",
                "presentation",
            ),
        ] {
            let command = siblings.join(relative);
            capabilities.add(
                id,
                &crate::capabilities::NewCapability {
                    label: label.to_string(),
                    command: command.to_string_lossy().into_owned(),
                    args: Vec::new(),
                    env: Default::default(),
                    agents: vec![agent.to_string()],
                },
                true,
            )?;
        }
        Ok(())
    }

    /// Record that something happened, for the diagnostics view.
    ///
    /// A rejected write is reported rather than swallowed. It was swallowed, and so a
    /// category the schema does not allow — the Activity_Log constrains them to a fixed
    /// list — failed silently for every entry the product tried to write.
    #[cfg_attr(not(feature = "adk"), allow(dead_code))]
    pub fn log(&self, category: &str, detail: &str) {
        if let Ok(store) = self.store.lock()
            && let Err(error) = store.log(category, detail, None, None)
        {
            eprintln!("[core] this was not recorded in the activity log: {error}");
        }
    }
}

/// What a piece of work is called before the User has said what it is for.
///
/// Only this may be replaced by a later, better name.
pub const PLACEHOLDER_PURPOSE: &str = "Work in progress";

/// The timezone the User is in, as the store requires. Schedules are not part of the
/// spreadsheet work, so this is recorded rather than used.
fn local_timezone() -> String {
    std::env::var("TZ").unwrap_or_else(|_| "UTC".to_string())
}

/// An identifier that reads plainly in a log.
pub fn new_id(prefix: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{now:x}")
}

/// What each specialist has been allowed to reach.
///
/// Reading it here rather than caching it means a change in Settings takes effect on the next
/// piece of work, not on the next restart.
#[cfg(feature = "adk")]
impl studio_runner::pipeline::Provides for Keeper {
    fn for_agent(&self, agent: &str) -> Vec<studio_runner::pipeline::Allocated> {
        let Ok(store) = self.store.lock() else {
            return Vec::new();
        };
        crate::capabilities::Capabilities::new(&store)
            .for_agent(agent)
            .unwrap_or_default()
            .into_iter()
            .map(|resolved| studio_runner::pipeline::Allocated {
                label: resolved.label,
                command: resolved.command,
                args: resolved.args,
                env: resolved.env,
            })
            .collect()
    }
}

/// What the specialist may remember, and how.
///
/// The same door the User's own typing goes through, so the list headed *What I've learned
/// from you* holds both and there is one place to look.
#[cfg(feature = "adk")]
impl studio_runner::memory::Remembers for Keeper {
    fn remember(&self, thread: &str, note: &str) -> Result<String, String> {
        let kept = self.add_note(Some(thread), note)?;
        self.log("action", &format!("remembered: {note}"));
        Ok(kept.provenance)
    }

    fn recall(&self, thread: &str) -> Vec<String> {
        self.notes_for_run(thread, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keeper(name: &str) -> Arc<Keeper> {
        let base = std::env::temp_dir().join(format!("zws-keep-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        Keeper::open_at(&base.join("data"), &base.join("home")).unwrap()
    }

    #[test]
    fn a_note_the_user_types_is_acted_on_at_once() {
        let keeper = keeper("typed");
        keeper.ensure_thread("t1", "Q3 model", None).unwrap();
        let note = keeper
            .add_note(Some("t1"), "Keep figures as formulas")
            .unwrap();
        assert_eq!(note.provenance, "You told me");
        assert!(
            note.asks.is_none(),
            "the User said it; there is nothing to ask"
        );

        let view = keeper.steering_view(Some("t1")).unwrap();
        assert_eq!(view.notes.len(), 1);
        assert!(view.proposed.is_empty());
        assert_eq!(
            keeper.notes_for_run("t1", Some("spreadsheet")),
            vec!["Keep figures as formulas".to_string()],
            "a note the User typed must reach the next run"
        );
    }

    /// Property 33: nothing Work Studio worked out for itself acts before it is accepted.
    #[test]
    fn something_noticed_influences_nothing_until_accepted() {
        let keeper = keeper("noticed");
        keeper.ensure_thread("t1", "Q3 model", None).unwrap();
        let proposed = keeper
            .propose_note(
                "t1",
                "Keep summaries under 150 words",
                Origin::DerivedFromEdit,
            )
            .unwrap();
        assert!(proposed.asks.is_some(), "it must ask before acting");
        assert_eq!(proposed.provenance, "I noticed a change you made");

        let view = keeper.steering_view(Some("t1")).unwrap();
        assert!(view.notes.is_empty(), "not yet acted on");
        assert_eq!(view.proposed.len(), 1, "but shown to the User");
        assert!(
            keeper.notes_for_run("t1", Some("spreadsheet")).is_empty(),
            "a run must not see it before the User agrees"
        );

        keeper.act_on_note(&proposed.id, "accept", None).unwrap();
        assert_eq!(
            keeper.notes_for_run("t1", Some("spreadsheet")),
            vec!["Keep summaries under 150 words".to_string()],
            "and must see it afterwards"
        );
    }

    #[test]
    fn rewording_is_agreement_in_the_users_own_words() {
        let keeper = keeper("reword");
        keeper.ensure_thread("t1", "Q3 model", None).unwrap();
        let proposed = keeper
            .propose_note("t1", "Keep summaries short", Origin::DerivedFromEdit)
            .unwrap();
        keeper
            .act_on_note(&proposed.id, "reword", Some("Under 120 words, always"))
            .unwrap();
        assert_eq!(
            keeper.notes_for_run("t1", None),
            vec!["Under 120 words, always".to_string()]
        );
    }

    #[test]
    fn a_note_stopped_or_forgotten_stops_acting() {
        let keeper = keeper("stopped");
        keeper.ensure_thread("t1", "Q3 model", None).unwrap();
        let a = keeper.add_note(Some("t1"), "First").unwrap();
        let b = keeper.add_note(Some("t1"), "Second").unwrap();
        keeper.act_on_note(&a.id, "stop", None).unwrap();
        assert_eq!(keeper.notes_for_run("t1", None), vec!["Second".to_string()]);
        keeper.act_on_note(&b.id, "forget", None).unwrap();
        assert!(keeper.notes_for_run("t1", None).is_empty());
    }

    /// The reason this exists: a second sitting must be possible.
    #[test]
    fn what_the_user_said_survives_the_application_closing() {
        let base = std::env::temp_dir().join(format!("zws-keep-{}-survive", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let data = base.join("data");
        let home = base.join("home");
        {
            let keeper = Keeper::open_at(&data, &home).unwrap();
            keeper.ensure_thread("t1", "Q3 model", None).unwrap();
            keeper
                .add_note(Some("t1"), "Assumptions on their own sheet")
                .unwrap();
            keeper
                .remember_turn("t1", "you", "Add a growth column")
                .unwrap();
        }
        // A different Keeper, as after a restart.
        let reopened = Keeper::open_at(&data, &home).unwrap();
        assert_eq!(
            reopened.notes_for_run("t1", None),
            vec!["Assumptions on their own sheet".to_string()],
            "closing the application must not forget what the User said"
        );
        assert_eq!(
            reopened.turns("t1").unwrap().len(),
            1,
            "nor the conversation"
        );
        assert_eq!(
            reopened.threads().unwrap().len(),
            1,
            "nor that the piece of work happened"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The list of the User's work is how they find it again, so a placeholder must never
    /// replace the name they gave it.
    #[test]
    fn a_note_added_later_does_not_rename_the_work() {
        let keeper = keeper("naming");
        keeper
            .ensure_thread(
                "t1",
                "Add a Tax column at 30% of Base",
                Some("/tmp/q3.xlsx"),
            )
            .unwrap();
        // Adding a note creates the work if absent, and must not rename it if present.
        keeper
            .add_note(Some("t1"), "Keep figures as formulas")
            .unwrap();
        let threads = keeper.threads().unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].purpose, "Add a Tax column at 30% of Base");
        assert_eq!(threads[0].file.as_deref(), Some("/tmp/q3.xlsx"));
    }

    /// The other way round: a note first, then the work is named properly.
    #[test]
    fn a_placeholder_gives_way_to_a_real_name() {
        let keeper = keeper("placeholder");
        keeper
            .add_note(Some("t2"), "Never invent a figure")
            .unwrap();
        assert_eq!(keeper.threads().unwrap()[0].purpose, PLACEHOLDER_PURPOSE);
        keeper
            .ensure_thread("t2", "Build the July board pack", Some("/tmp/x.xlsx"))
            .unwrap();
        assert_eq!(
            keeper.threads().unwrap()[0].purpose,
            "Build the July board pack"
        );
    }

    #[test]
    fn an_empty_note_is_refused() {
        let keeper = keeper("empty");
        keeper.ensure_thread("t1", "Q3 model", None).unwrap();
        assert!(keeper.add_note(Some("t1"), "   ").is_err());
    }

    #[test]
    fn a_global_note_applies_without_a_thread() {
        let keeper = keeper("global");
        keeper.add_note(None, "Never invent a figure").unwrap();
        let view = keeper.steering_view(None).unwrap();
        assert_eq!(view.global.len(), 1);
        assert_eq!(
            keeper.notes_for_run("any-thread", None),
            vec!["Never invent a figure".to_string()],
            "a note about everything applies to work it has never seen"
        );
    }
}
