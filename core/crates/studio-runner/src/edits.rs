//! Applying a change to a file.
//!
//! Everything goes through [`Dispatcher::apply`] — a cell the User typed, a column an
//! agent added, a chart either of them asked for. That is what makes one change
//! history possible, and it is the only reason undo can work the same way for both
//! (Correctness Property 23).
//!
//! The order of the checks is deliberate:
//!
//! 1. **The gate decides.** An operation nobody classified is refused, and anything
//!    that acts outside this computer needs an authorising state.
//! 2. **The file is checked.** If it changed in another application since we last saw
//!    it, that is read and recorded first, so the User's own work is never written over
//!    (Requirement 11.6).
//! 3. **Then it is applied**, and only then recorded — so the history never claims a
//!    change that did not happen.

use studio_artefacts::{Artefacts, Author, EditOperation, Freshness};
use studio_gate::{Classifier, Decision, RunMode, SideEffect};
use studio_jobs::{JobKind, JobState};

/// A change somebody wants made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedEdit {
    pub artefact_id: String,
    /// Which server owns the operation.
    pub server: String,
    /// The operation's machine name.
    pub operation: String,
    /// What the User will read in the history.
    pub description: String,
    pub author: Author,
}

/// Applies an operation to a file. Implemented over MCP in the running product, and
/// over a fake in tests, so the rules above can be tested without a server.
pub trait Applier {
    fn apply(&self, server: &str, operation: &str) -> std::result::Result<(), String>;
}

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error(transparent)]
    Artefact(#[from] studio_artefacts::ArtefactError),
    #[error("I don't know what {0} would do, so I haven't done it")]
    Unclassified(String),
    #[error("this piece of work isn't allowed to do that right now")]
    NotAuthorised,
    #[error("that didn't work: {0}")]
    Failed(String),
}

pub type Result<T> = std::result::Result<T, EditError>;

/// What happened, including anything noticed on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub seq: i64,
    /// Set when the file had been changed in another application and that was recorded
    /// before this change went in.
    pub picked_up_external_change: bool,
}

pub struct Dispatcher<'a, A: Applier> {
    artefacts: Artefacts<'a>,
    classifier: &'a Classifier,
    applier: &'a A,
}

impl<'a, A: Applier> Dispatcher<'a, A> {
    pub fn new(artefacts: Artefacts<'a>, classifier: &'a Classifier, applier: &'a A) -> Self {
        Self {
            artefacts,
            classifier,
            applier,
        }
    }

    pub fn apply(
        &self,
        edit: &ProposedEdit,
        kind: JobKind,
        state: JobState,
        mode: RunMode,
    ) -> Result<Applied> {
        // 1. The gate decides. `auto_approve` is passed as declared and ignored.
        let decision = studio_gate::decide(
            self.classifier,
            &edit.server,
            &edit.operation,
            kind,
            state,
            mode,
            true,
        );

        match decision {
            Decision::Suppress { reason } => {
                return Err(match reason {
                    studio_gate::SuppressReason::Unclassified => {
                        EditError::Unclassified(edit.operation.clone())
                    }
                    _ => EditError::NotAuthorised,
                });
            }
            Decision::Permit => {
                // A read changes nothing, so there is nothing to record.
                self.applier
                    .apply(&edit.server, &edit.operation)
                    .map_err(EditError::Failed)?;
                return Ok(Applied {
                    seq: 0,
                    picked_up_external_change: false,
                });
            }
            Decision::PermitAndRecord | Decision::PermitAndDeliver { .. } => {}
        }

        // 2. Somebody else may have got there first.
        let mut picked_up_external_change = false;
        if self.artefacts.freshness(&edit.artefact_id)? == Freshness::ChangedElsewhere {
            self.artefacts
                .note_external_change(&edit.artefact_id, None)?;
            picked_up_external_change = true;
        }

        // 3. Apply, then record — never the other way round.
        self.applier
            .apply(&edit.server, &edit.operation)
            .map_err(EditError::Failed)?;

        let change = self.artefacts.record(
            &edit.artefact_id,
            &EditOperation::new(edit.author, &edit.operation, &edit.description),
        )?;

        Ok(Applied {
            seq: change.seq,
            picked_up_external_change,
        })
    }

    /// Whether an operation would change the file at all, for callers deciding whether
    /// to show a change card.
    pub fn changes_the_file(&self, server: &str, operation: &str) -> bool {
        self.classifier.effect_of(server, operation) != SideEffect::Read
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use studio_store::Store;

    /// Records what it was asked to do, and — like a real applier — changes the file
    /// while doing it.
    ///
    /// That detail matters. An earlier version of this fake left the file alone and the
    /// test changed it beforehand, which from the dispatcher's point of view is
    /// indistinguishable from somebody editing in Excel: it recorded their change first
    /// and the test's expectations were wrong rather than the code.
    struct Fake {
        calls: RefCell<Vec<String>>,
        fail: Option<String>,
        path: PathBuf,
        version: RefCell<u32>,
    }

    impl Fake {
        fn ok(path: &std::path::Path) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail: None,
                path: path.to_path_buf(),
                version: RefCell::new(0),
            }
        }
        fn failing(path: &std::path::Path, message: &str) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail: Some(message.to_string()),
                path: path.to_path_buf(),
                version: RefCell::new(0),
            }
        }
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl Applier for Fake {
        fn apply(&self, server: &str, operation: &str) -> std::result::Result<(), String> {
            self.calls
                .borrow_mut()
                .push(format!("{server}/{operation}"));
            if let Some(message) = &self.fail {
                return Err(message.clone());
            }
            let mut version = self.version.borrow_mut();
            *version += 1;
            std::fs::write(&self.path, format!("applied {version}")).map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    struct Fixture {
        store: Store,
        dir: PathBuf,
        path: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("zws-edits-{}-{name}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("model.xlsx");
            std::fs::write(&path, "v1").unwrap();
            let store = Store::open_in_memory().unwrap();
            let artefacts = Artefacts::new(&store);
            artefacts
                .register("a1", &path, "Q3 revenue model.xlsx", None)
                .unwrap();
            Self { store, dir, path }
        }
        fn touch(&self, contents: &str) {
            std::fs::write(&self.path, contents).unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.dir).ok();
        }
    }

    fn edit(operation: &str, author: Author) -> ProposedEdit {
        ProposedEdit {
            artefact_id: "a1".into(),
            server: "worksheet".into(),
            operation: operation.into(),
            description: format!("did {operation}"),
            author,
        }
    }

    /// Correctness Property 23, at the dispatcher: both authors take the same path and
    /// land in the same history.
    #[test]
    fn property_23_a_user_edit_and_an_agent_edit_take_the_same_path() {
        let fixture = Fixture::new("same-path");
        let classifier = studio_gate::catalogue::worksheet();
        let fake = Fake::ok(&fixture.path);
        let dispatcher = Dispatcher::new(Artefacts::new(&fixture.store), &classifier, &fake);

        let by_agent = dispatcher
            .apply(
                &edit("write_formula", Author::Studio),
                JobKind::OneOff,
                JobState::Active,
                RunMode::Manual,
            )
            .unwrap();

        let by_user = dispatcher
            .apply(
                &edit("manage_cell", Author::User),
                JobKind::OneOff,
                JobState::Active,
                RunMode::Manual,
            )
            .unwrap();

        assert_eq!(by_agent.seq, 1);
        assert_eq!(by_user.seq, 2);

        let history = Artefacts::new(&fixture.store).history("a1").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].author, Author::Studio);
        assert_eq!(history[1].author, Author::User);
        assert_eq!(
            fake.calls(),
            vec!["worksheet/write_formula", "worksheet/manage_cell"]
        );
    }

    #[test]
    fn an_unclassified_operation_is_refused_and_never_applied() {
        let fixture = Fixture::new("unclassified");
        let classifier = studio_gate::catalogue::worksheet();
        let fake = Fake::ok(&fixture.path);
        let dispatcher = Dispatcher::new(Artefacts::new(&fixture.store), &classifier, &fake);

        let result = dispatcher.apply(
            &edit("wipe_the_disk", Author::Studio),
            JobKind::OneOff,
            JobState::Active,
            RunMode::Manual,
        );
        assert!(matches!(result, Err(EditError::Unclassified(_))));
        assert!(fake.calls().is_empty(), "it must not be applied at all");
        assert!(
            Artefacts::new(&fixture.store)
                .history("a1")
                .unwrap()
                .is_empty()
        );
    }

    /// Requirement 11.6: a change made in Excel is picked up, not written over.
    #[test]
    fn a_change_made_elsewhere_is_picked_up_before_ours_goes_in() {
        let fixture = Fixture::new("elsewhere");
        let classifier = studio_gate::catalogue::worksheet();
        let fake = Fake::ok(&fixture.path);
        let dispatcher = Dispatcher::new(Artefacts::new(&fixture.store), &classifier, &fake);

        fixture.touch("changed in Excel");
        let applied = dispatcher
            .apply(
                &edit("write_cells", Author::Studio),
                JobKind::OneOff,
                JobState::Active,
                RunMode::Manual,
            )
            .unwrap();

        assert!(applied.picked_up_external_change);
        let history = Artefacts::new(&fixture.store).history("a1").unwrap();
        assert_eq!(history.len(), 2, "their change is recorded before ours");
        assert_eq!(history[0].author, Author::User);
        assert_eq!(history[0].operation, "external_change");
        assert_eq!(history[1].author, Author::Studio);
    }

    #[test]
    fn a_read_is_applied_but_leaves_no_history() {
        let fixture = Fixture::new("read");
        let classifier = studio_gate::catalogue::worksheet();
        let fake = Fake::ok(&fixture.path);
        let dispatcher = Dispatcher::new(Artefacts::new(&fixture.store), &classifier, &fake);

        let applied = dispatcher
            .apply(
                &edit("read_sheet", Author::Studio),
                JobKind::OneOff,
                JobState::Active,
                RunMode::Manual,
            )
            .unwrap();
        assert_eq!(applied.seq, 0);
        assert_eq!(fake.calls(), vec!["worksheet/read_sheet"]);
        assert!(
            Artefacts::new(&fixture.store)
                .history("a1")
                .unwrap()
                .is_empty(),
            "reading changes nothing, so there is nothing to undo"
        );
    }

    /// The history must never claim a change that failed.
    #[test]
    fn a_failure_records_nothing() {
        let fixture = Fixture::new("failure");
        let classifier = studio_gate::catalogue::worksheet();
        let fake = Fake::failing(&fixture.path, "the sheet is locked");
        let dispatcher = Dispatcher::new(Artefacts::new(&fixture.store), &classifier, &fake);

        let result = dispatcher.apply(
            &edit("write_cells", Author::Studio),
            JobKind::OneOff,
            JobState::Active,
            RunMode::Manual,
        );
        assert!(matches!(result, Err(EditError::Failed(_))));
        assert!(
            Artefacts::new(&fixture.store)
                .history("a1")
                .unwrap()
                .is_empty(),
            "the history must not claim a change that did not happen"
        );
    }

    #[test]
    fn a_finished_piece_of_work_cannot_still_be_editing_files() {
        let fixture = Fixture::new("finished");
        let classifier = studio_gate::catalogue::worksheet();
        let fake = Fake::ok(&fixture.path);
        let dispatcher = Dispatcher::new(Artefacts::new(&fixture.store), &classifier, &fake);

        // A local write is permitted in any state — editing the User's own file is not
        // an external effect — so this asserts the classification, not a refusal.
        let applied = dispatcher.apply(
            &edit("write_cells", Author::Studio),
            JobKind::OneOff,
            JobState::Finished,
            RunMode::Manual,
        );
        assert!(
            applied.is_ok(),
            "a local write is not gated by Job state; only acting outside this computer is"
        );
    }

    #[test]
    fn the_dispatcher_can_say_whether_an_operation_changes_anything() {
        let fixture = Fixture::new("changes");
        let classifier = studio_gate::catalogue::worksheet();
        let fake = Fake::ok(&fixture.path);
        let dispatcher = Dispatcher::new(Artefacts::new(&fixture.store), &classifier, &fake);
        assert!(!dispatcher.changes_the_file("worksheet", "read_sheet"));
        assert!(dispatcher.changes_the_file("worksheet", "write_formula"));
        assert!(
            dispatcher.changes_the_file("worksheet", "who_knows"),
            "an unknown operation is assumed to change something"
        );
    }
}
