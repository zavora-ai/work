//! The User's files, and everything done to them.
//!
//! Two ideas carry most of the weight here.
//!
//! **One edit path.** A change the User typed and a change an agent made are the same
//! kind of thing: an [`EditOperation`] with an author. That is why one change history
//! can show both, why undo works the same either way, and why the User can tell what
//! they did from what was done for them (Requirement 22.4, Correctness Property 23).
//!
//! **The file on disk is the truth.** Artefacts are ordinary files in a folder the User
//! chose (Requirement 12.1). This module keeps metadata beside them, never instead of
//! them, and notices when a file changed underneath — because someone opened it in
//! Excel, which they are entitled to do.

/// Where the User's work lives on their own disk.
pub mod home;

use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};
use studio_store::Store;

#[derive(Debug, thiserror::Error)]
pub enum ArtefactError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("that file is not one of yours: {0}")]
    Unknown(String),
    #[error("that file has been moved or deleted since I last saw it")]
    Vanished,
    #[error("nothing to undo")]
    NothingToUndo,
}

pub type Result<T> = std::result::Result<T, ArtefactError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Document,
    Deck,
    Spreadsheet,
    Pdf,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Deck => "deck",
            Self::Spreadsheet => "spreadsheet",
            Self::Pdf => "pdf",
        }
    }

    /// From a file's own extension, which is how the User thinks of it.
    pub fn of_path(path: &Path) -> Option<Self> {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("docx") => Some(Self::Document),
            Some("pptx") => Some(Self::Deck),
            Some("xlsx") | Some("xlsm") => Some(Self::Spreadsheet),
            Some("pdf") => Some(Self::Pdf),
            _ => None,
        }
    }
}

/// Who made a change. The only two answers the User cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Author {
    /// The User, in the app or in another application.
    User,
    /// Work Studio.
    Studio,
}

impl Author {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Studio => "studio",
        }
    }

    fn parse(value: &str) -> Self {
        if value == "user" {
            Self::User
        } else {
            Self::Studio
        }
    }
}

/// One change, in the vocabulary both the agent and the interface use.
///
/// `operation` is the machine name, kept for undo and for diagnostics.
/// `description` is what the User reads in the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditOperation {
    pub author: Author,
    pub operation: String,
    pub description: String,
}

impl EditOperation {
    pub fn new(
        author: Author,
        operation: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            author,
            operation: operation.into(),
            description: description.into(),
        }
    }

    /// A change Work Studio made.
    pub fn by_studio(operation: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(Author::Studio, operation, description)
    }

    /// A change the User made.
    pub fn by_user(operation: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(Author::User, operation, description)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub seq: i64,
    pub author: Author,
    pub operation: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artefact {
    pub id: String,
    pub kind: Kind,
    pub path: PathBuf,
    pub display_name: String,
    pub last_author: Author,
    /// Set when the file was last changed by another application.
    pub last_editor_app: Option<String>,
    pub derived_from: Option<String>,
}

/// What we found when checking a file against what we last recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// Unchanged since we last saw it.
    Unchanged,
    /// Changed by something other than us. Their work must be read before ours is
    /// applied (Requirement 11.6).
    ChangedElsewhere,
    /// The file is no longer where it was.
    Vanished,
}

pub struct Artefacts<'a> {
    store: &'a Store,
}

impl<'a> Artefacts<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// Record a file Work Studio now knows about.
    pub fn register(
        &self,
        id: &str,
        path: &Path,
        display_name: &str,
        derived_from: Option<&str>,
    ) -> Result<Artefact> {
        let kind = Kind::of_path(path).unwrap_or(Kind::Document);
        let (hash, mtime) = fingerprint(path)?;
        self.store.conn().execute(
            "INSERT INTO artefacts
               (id, kind, file_path, display_name, derived_from, last_author,
                content_hash, mtime, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'studio', ?6, ?7, unixepoch(), unixepoch())",
            params![
                id,
                kind.as_str(),
                path.to_string_lossy(),
                display_name,
                derived_from,
                hash,
                mtime
            ],
        )?;
        Ok(Artefact {
            id: id.to_string(),
            kind,
            path: path.to_path_buf(),
            display_name: display_name.to_string(),
            last_author: Author::Studio,
            last_editor_app: None,
            derived_from: derived_from.map(str::to_string),
        })
    }

    pub fn get(&self, id: &str) -> Result<Artefact> {
        self.store
            .conn()
            .query_row(
                "SELECT id, kind, file_path, display_name, last_author, last_editor_app, derived_from
                 FROM artefacts WHERE id = ?1",
                params![id],
                |row| {
                    let kind: String = row.get(1)?;
                    let path: String = row.get(2)?;
                    let author: String = row.get(4)?;
                    Ok(Artefact {
                        id: row.get(0)?,
                        kind: match kind.as_str() {
                            "deck" => Kind::Deck,
                            "spreadsheet" => Kind::Spreadsheet,
                            "pdf" => Kind::Pdf,
                            _ => Kind::Document,
                        },
                        path: PathBuf::from(path),
                        display_name: row.get(3)?,
                        last_author: Author::parse(&author),
                        last_editor_app: row.get(5)?,
                        derived_from: row.get(6)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| ArtefactError::Unknown(id.to_string()))
    }

    /// Has anything happened to this file that we did not do?
    pub fn freshness(&self, id: &str) -> Result<Freshness> {
        let artefact = self.get(id)?;
        if !artefact.path.exists() {
            return Ok(Freshness::Vanished);
        }
        let (hash, _) = fingerprint(&artefact.path)?;
        let recorded: String = self.store.conn().query_row(
            "SELECT content_hash FROM artefacts WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(if hash == recorded {
            Freshness::Unchanged
        } else {
            Freshness::ChangedElsewhere
        })
    }

    /// The single path every change goes through, whoever made it.
    ///
    /// Returns the recorded change. The caller has already applied the change to the
    /// file; this is what makes it visible, attributable and undoable.
    pub fn record(&self, id: &str, edit: &EditOperation) -> Result<Change> {
        let artefact = self.get(id)?;
        if !artefact.path.exists() {
            return Err(ArtefactError::Vanished);
        }

        let seq: i64 = self.store.conn().query_row(
            "SELECT coalesce(max(seq), 0) + 1 FROM artefact_changes WHERE artefact_id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        self.store.conn().execute(
            "INSERT INTO artefact_changes (artefact_id, seq, author, operation, description, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, unixepoch())",
            params![
                id,
                seq,
                edit.author.as_str(),
                edit.operation,
                edit.description
            ],
        )?;

        let (hash, mtime) = fingerprint(&artefact.path)?;
        self.store.conn().execute(
            "UPDATE artefacts
             SET content_hash = ?1, mtime = ?2, last_author = ?3, last_editor_app = NULL,
                 updated_at = unixepoch()
             WHERE id = ?4",
            params![hash, mtime, edit.author.as_str(), id],
        )?;

        Ok(Change {
            seq,
            author: edit.author,
            operation: edit.operation.clone(),
            description: edit.description.clone(),
        })
    }

    /// Note that another application changed the file, so the next edit reads it first.
    pub fn note_external_change(&self, id: &str, app: Option<&str>) -> Result<Change> {
        let artefact = self.get(id)?;
        let description = match app {
            Some(app) => format!("You changed this in {app}"),
            None => "You changed this outside Work Studio".to_string(),
        };
        let change = self.record(id, &EditOperation::by_user("external_change", description))?;
        self.store.conn().execute(
            "UPDATE artefacts SET last_editor_app = ?1 WHERE id = ?2",
            params![app, id],
        )?;
        let _ = artefact;
        Ok(change)
    }

    /// Everything done to a file, oldest first.
    pub fn history(&self, id: &str) -> Result<Vec<Change>> {
        let mut stmt = self.store.conn().prepare(
            "SELECT seq, author, operation, description
             FROM artefact_changes WHERE artefact_id = ?1 ORDER BY seq",
        )?;
        let rows = stmt.query_map(params![id], |row| {
            let author: String = row.get(1)?;
            Ok(Change {
                seq: row.get(0)?,
                author: Author::parse(&author),
                operation: row.get(2)?,
                description: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// What Work Studio changed and the User has not yet accepted or undone.
    pub fn changes_by_studio_since(&self, id: &str, seq: i64) -> Result<Vec<Change>> {
        Ok(self
            .history(id)?
            .into_iter()
            .filter(|change| change.seq > seq && change.author == Author::Studio)
            .collect())
    }

    /// Discard the history after `seq`, having reverted the file to that point.
    ///
    /// The revert itself is the engine's job; this records that it happened, so the
    /// history stays a truthful account rather than a wish.
    pub fn revert_to(&self, id: &str, seq: i64) -> Result<()> {
        let history = self.history(id)?;
        if !history.iter().any(|change| change.seq == seq) {
            return Err(ArtefactError::NothingToUndo);
        }
        self.store.conn().execute(
            "DELETE FROM artefact_changes WHERE artefact_id = ?1 AND seq > ?2",
            params![id, seq],
        )?;
        Ok(())
    }

    /// Which pieces of work have touched this file — the Repository's `Used in`.
    pub fn used_in(&self, id: &str) -> Result<Vec<String>> {
        let mut stmt = self.store.conn().prepare(
            "SELECT jobs.purpose FROM artefact_jobs
             JOIN jobs ON jobs.id = artefact_jobs.job_id
             WHERE artefact_jobs.artefact_id = ?1
             ORDER BY artefact_jobs.first_ts",
        )?;
        let rows = stmt.query_map(params![id], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn link_to_job(&self, id: &str, job_id: &str) -> Result<()> {
        self.store.conn().execute(
            "INSERT OR IGNORE INTO artefact_jobs (artefact_id, job_id, first_ts)
             VALUES (?1, ?2, unixepoch())",
            params![id, job_id],
        )?;
        Ok(())
    }
}

/// A cheap content fingerprint and the file's modification time.
///
/// Not a cryptographic hash: its only job is to notice that a file changed, and it must
/// be fast enough to run before every edit.
fn fingerprint(path: &Path) -> Result<(String, i64)> {
    let bytes = std::fs::read(path)?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in &bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    let mtime = std::fs::metadata(path)?
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Ok((format!("{hash:016x}-{}", bytes.len()), mtime))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        store: Store,
        dir: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("zws-artefacts-{}-{name}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let store = Store::open_in_memory().unwrap();
            store
                .conn()
                .execute(
                    "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                     VALUES ('j1','one_off','Q3 revenue model','active','UTC',0,0)",
                    [],
                )
                .unwrap();
            Self { store, dir }
        }

        fn file(&self, name: &str, contents: &str) -> PathBuf {
            let path = self.dir.join(name);
            std::fs::write(&path, contents).unwrap();
            path
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.dir).ok();
        }
    }

    #[test]
    fn a_files_kind_comes_from_its_own_extension() {
        assert_eq!(Kind::of_path(Path::new("a.xlsx")), Some(Kind::Spreadsheet));
        assert_eq!(Kind::of_path(Path::new("a.XLSX")), Some(Kind::Spreadsheet));
        assert_eq!(Kind::of_path(Path::new("a.docx")), Some(Kind::Document));
        assert_eq!(Kind::of_path(Path::new("a.pptx")), Some(Kind::Deck));
        assert_eq!(Kind::of_path(Path::new("a.pdf")), Some(Kind::Pdf));
        assert_eq!(Kind::of_path(Path::new("notes.txt")), None);
    }

    /// Correctness Property 23: one edit path.
    ///
    /// A change the User made and a change Work Studio made appear in one history,
    /// attributed, and are undone by the same mechanism.
    #[test]
    fn property_23_both_authors_share_one_history() {
        let fixture = Fixture::new("one-path");
        let path = fixture.file("model.xlsx", "v1");
        let artefacts = Artefacts::new(&fixture.store);
        artefacts
            .register("a1", &path, "Q3 revenue model.xlsx", None)
            .unwrap();

        std::fs::write(&path, "v2").unwrap();
        artefacts
            .record(
                "a1",
                &EditOperation::by_studio("write_formula", "Added a 12% growth column"),
            )
            .unwrap();

        std::fs::write(&path, "v3").unwrap();
        artefacts
            .record(
                "a1",
                &EditOperation::by_user("manage_cell", "You changed D8"),
            )
            .unwrap();

        let history = artefacts.history("a1").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].author, Author::Studio);
        assert_eq!(history[1].author, Author::User);
        assert_eq!(history[0].seq, 1);
        assert_eq!(history[1].seq, 2);
        assert_eq!(history[0].description, "Added a 12% growth column");
    }

    #[test]
    fn what_studio_changed_can_be_told_from_what_the_user_changed() {
        let fixture = Fixture::new("attribution");
        let path = fixture.file("model.xlsx", "v1");
        let artefacts = Artefacts::new(&fixture.store);
        artefacts.register("a1", &path, "model.xlsx", None).unwrap();

        for (index, edit) in [
            EditOperation::by_studio("write_grid", "Built the summary"),
            EditOperation::by_user("manage_cell", "You changed a heading"),
            EditOperation::by_studio("add_chart", "Added a chart"),
        ]
        .into_iter()
        .enumerate()
        {
            std::fs::write(&path, format!("v{index}")).unwrap();
            artefacts.record("a1", &edit).unwrap();
        }

        let mine = artefacts.changes_by_studio_since("a1", 0).unwrap();
        assert_eq!(mine.len(), 2, "only Work Studio's own changes");
        assert!(mine.iter().all(|change| change.author == Author::Studio));
        assert_eq!(mine[1].description, "Added a chart");
    }

    /// Requirement 11.6: a change made in another application must not be lost.
    #[test]
    fn a_change_made_elsewhere_is_noticed() {
        let fixture = Fixture::new("elsewhere");
        let path = fixture.file("model.xlsx", "v1");
        let artefacts = Artefacts::new(&fixture.store);
        artefacts.register("a1", &path, "model.xlsx", None).unwrap();

        assert_eq!(artefacts.freshness("a1").unwrap(), Freshness::Unchanged);

        std::fs::write(&path, "changed in Excel").unwrap();
        assert_eq!(
            artefacts.freshness("a1").unwrap(),
            Freshness::ChangedElsewhere,
            "an edit we did not make must be visible before we write over it"
        );

        let change = artefacts.note_external_change("a1", Some("Excel")).unwrap();
        assert_eq!(change.author, Author::User);
        assert_eq!(change.description, "You changed this in Excel");
        assert_eq!(
            artefacts.freshness("a1").unwrap(),
            Freshness::Unchanged,
            "once noted, the file is the new baseline"
        );
        assert_eq!(
            artefacts.get("a1").unwrap().last_editor_app.as_deref(),
            Some("Excel")
        );
    }

    #[test]
    fn a_file_that_has_gone_is_reported_rather_than_recreated() {
        let fixture = Fixture::new("vanished");
        let path = fixture.file("model.xlsx", "v1");
        let artefacts = Artefacts::new(&fixture.store);
        artefacts.register("a1", &path, "model.xlsx", None).unwrap();

        std::fs::remove_file(&path).unwrap();
        assert_eq!(artefacts.freshness("a1").unwrap(), Freshness::Vanished);
        assert!(
            matches!(
                artefacts.record("a1", &EditOperation::by_studio("write_cells", "x")),
                Err(ArtefactError::Vanished)
            ),
            "we must not record a change to a file that is not there"
        );
        assert!(!path.exists(), "and we must not recreate it silently");
    }

    #[test]
    fn history_can_be_wound_back_to_a_point() {
        let fixture = Fixture::new("revert");
        let path = fixture.file("model.xlsx", "v1");
        let artefacts = Artefacts::new(&fixture.store);
        artefacts.register("a1", &path, "model.xlsx", None).unwrap();

        for index in 0..4 {
            std::fs::write(&path, format!("v{index}")).unwrap();
            artefacts
                .record(
                    "a1",
                    &EditOperation::by_studio("write_cells", format!("change {index}")),
                )
                .unwrap();
        }
        assert_eq!(artefacts.history("a1").unwrap().len(), 4);

        artefacts.revert_to("a1", 2).unwrap();
        let history = artefacts.history("a1").unwrap();
        assert_eq!(history.len(), 2, "everything after the point is gone");
        assert_eq!(history.last().unwrap().seq, 2);

        assert!(matches!(
            artefacts.revert_to("a1", 99),
            Err(ArtefactError::NothingToUndo)
        ));
    }

    #[test]
    fn a_file_knows_which_work_used_it_and_what_it_came_from() {
        let fixture = Fixture::new("lineage");
        let model = fixture.file("model.xlsx", "v1");
        let deck = fixture.file("deck.pptx", "v1");
        let artefacts = Artefacts::new(&fixture.store);

        artefacts
            .register("a1", &model, "Q3 revenue model.xlsx", None)
            .unwrap();
        artefacts
            .register("a2", &deck, "Board deck — July.pptx", Some("a1"))
            .unwrap();

        artefacts.link_to_job("a1", "j1").unwrap();
        artefacts.link_to_job("a1", "j1").unwrap(); // linking twice must not duplicate

        assert_eq!(artefacts.used_in("a1").unwrap(), vec!["Q3 revenue model"]);
        assert_eq!(
            artefacts.get("a2").unwrap().derived_from.as_deref(),
            Some("a1")
        );
        assert_eq!(artefacts.get("a2").unwrap().kind, Kind::Deck);
    }

    #[test]
    fn an_unknown_file_says_so_plainly() {
        let fixture = Fixture::new("unknown");
        let artefacts = Artefacts::new(&fixture.store);
        let error = artefacts.get("nope").expect_err("must fail");
        assert!(
            error
                .to_string()
                .starts_with("that file is not one of yours")
        );
    }

    #[test]
    fn the_fingerprint_changes_when_the_file_does_and_not_otherwise() {
        let fixture = Fixture::new("fingerprint");
        let path = fixture.file("a.xlsx", "hello");
        let (first, _) = fingerprint(&path).unwrap();
        let (again, _) = fingerprint(&path).unwrap();
        assert_eq!(first, again, "reading twice must give the same answer");

        std::fs::write(&path, "hello!").unwrap();
        let (changed, _) = fingerprint(&path).unwrap();
        assert_ne!(first, changed);

        // length is part of it, so a same-length change is still caught
        std::fs::write(&path, "hellp").unwrap();
        let (same_length, _) = fingerprint(&path).unwrap();
        assert_ne!(first, same_length);
    }
}
