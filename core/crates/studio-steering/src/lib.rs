//! Steering.
//!
//! What the User has told Work Studio about how they like things done. It is a
//! durable, ordered, editable list of their own sentences — deliberately not a
//! learned model, not an embedding, and not a hidden preference vector. Requirement
//! 8.4 forbids storing any preference the User cannot see and change, which is what
//! makes the steering list the product's main transparency surface.
//!
//! Two scopes. A per-thread note belongs to one Job. A global note lives in
//! Settings and applies across everything, optionally narrowed to one Artefact
//! kind — "use our brand colours" belongs to every deck, not to one.
//!
//! Precedence needs no special rule. Notes are assembled global-first and injected
//! in order, and later notes win, so a per-thread note naturally overrides a global
//! one (Requirement 8.9).
//!
//! A note derived from watching the User — an edit they made, a row they excluded,
//! a choice they took — starts unconfirmed and does not influence anything until
//! they say yes (Requirement 5.4).

use rusqlite::{OptionalExtension, params};
use studio_store::Store;

/// What a note applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// One piece of work.
    Job,
    /// Everything.
    Everything,
    Document,
    Deck,
    Spreadsheet,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Job => "job",
            Self::Everything => "everything",
            Self::Document => "document",
            Self::Deck => "deck",
            Self::Spreadsheet => "spreadsheet",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "job" => Some(Self::Job),
            "everything" => Some(Self::Everything),
            "document" => Some(Self::Document),
            "deck" => Some(Self::Deck),
            "spreadsheet" => Some(Self::Spreadsheet),
            _ => None,
        }
    }

    /// The User-facing label, as Settings shows it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Job => "This piece of work",
            Self::Everything => "Everything",
            Self::Document => "Documents",
            Self::Deck => "Decks",
            Self::Spreadsheet => "Spreadsheets",
        }
    }
}

/// What kind of Artefact a run is producing, for narrowing global notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtefactKind {
    Document,
    Deck,
    Spreadsheet,
}

impl ArtefactKind {
    fn matches(self, scope: Scope) -> bool {
        match scope {
            Scope::Everything => true,
            Scope::Document => self == Self::Document,
            Scope::Deck => self == Self::Deck,
            Scope::Spreadsheet => self == Self::Spreadsheet,
            Scope::Job => false,
        }
    }
}

/// Where a note came from. Everything but `Explicit` was derived from watching the
/// User, and therefore needs confirming before it counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The User typed it.
    Explicit,
    /// A comment when rejecting a first draft.
    Rejection,
    /// Inferred from an edit they made before approving.
    DerivedFromEdit,
    /// Inferred from a row they unchecked.
    DerivedFromExclusion,
    /// Inferred from an option they chose.
    DerivedFromChoice,
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Rejection => "rejection",
            Self::DerivedFromEdit => "derived_from_edit",
            Self::DerivedFromExclusion => "derived_from_exclusion",
            Self::DerivedFromChoice => "derived_from_choice",
        }
    }

    /// Derived notes must be confirmed before they influence anything.
    pub fn needs_confirmation(self) -> bool {
        !matches!(self, Self::Explicit | Self::Rejection)
    }

    /// The question Work Studio asks when proposing a derived note.
    pub fn confirmation_prompt(self, note: &str) -> String {
        match self {
            Self::DerivedFromEdit => {
                format!("I noticed your change. Should I always do this: {note}?")
            }
            Self::DerivedFromExclusion => {
                format!("Should I stop doing that from now on: {note}?")
            }
            Self::DerivedFromChoice => format!("Should I always choose that: {note}?"),
            Self::Explicit | Self::Rejection => note.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    /// None for a global note.
    pub job_id: Option<String>,
    pub scope: Scope,
    /// The User's own words.
    pub text: String,
    pub origin: Origin,
    pub confirmed: bool,
    pub active: bool,
    pub seq: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum SteeringError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("no note with id {0}")]
    NotFound(String),
    #[error("a global note cannot be scoped to a single piece of work")]
    GlobalNeedsRealScope,
    #[error("a note on one piece of work is always scoped to it")]
    JobNoteWrongScope,
}

pub type Result<T> = std::result::Result<T, SteeringError>;

pub struct Steering<'a> {
    store: &'a Store,
}

impl<'a> Steering<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    fn next_seq(&self) -> Result<i64> {
        Ok(self.store.conn().query_row(
            "SELECT coalesce(max(seq), 0) + 1 FROM steering_notes",
            [],
            |r| r.get(0),
        )?)
    }

    /// Add a note to one piece of work.
    pub fn add_for_job(&self, id: &str, job_id: &str, text: &str, origin: Origin) -> Result<Note> {
        let seq = self.next_seq()?;
        let confirmed = !origin.needs_confirmation();
        self.store.conn().execute(
            "INSERT INTO steering_notes
               (id, job_id, scope, note, origin, confirmed, active, seq, created_at)
             VALUES (?1, ?2, 'job', ?3, ?4, ?5, 1, ?6, unixepoch())",
            params![id, job_id, text, origin.as_str(), confirmed as i32, seq],
        )?;
        Ok(Note {
            id: id.into(),
            job_id: Some(job_id.into()),
            scope: Scope::Job,
            text: text.into(),
            origin,
            confirmed,
            active: true,
            seq,
        })
    }

    /// Add a note that applies across everything, held in Settings.
    pub fn add_global(&self, id: &str, scope: Scope, text: &str, origin: Origin) -> Result<Note> {
        if scope == Scope::Job {
            return Err(SteeringError::GlobalNeedsRealScope);
        }
        let seq = self.next_seq()?;
        let confirmed = !origin.needs_confirmation();
        self.store.conn().execute(
            "INSERT INTO steering_notes
               (id, job_id, scope, note, origin, confirmed, active, seq, created_at)
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, 1, ?6, unixepoch())",
            params![
                id,
                scope.as_str(),
                text,
                origin.as_str(),
                confirmed as i32,
                seq
            ],
        )?;
        Ok(Note {
            id: id.into(),
            job_id: None,
            scope,
            text: text.into(),
            origin,
            confirmed,
            active: true,
            seq,
        })
    }

    /// The User said yes to a derived note.
    pub fn confirm(&self, id: &str) -> Result<()> {
        let changed = self.store.conn().execute(
            "UPDATE steering_notes SET confirmed = 1 WHERE id = ?1",
            params![id],
        )?;
        if changed == 0 {
            return Err(SteeringError::NotFound(id.to_string()));
        }
        Ok(())
    }

    /// The User reworded a note. Rewording makes it the most recent, so it wins.
    pub fn reword(&self, id: &str, text: &str) -> Result<()> {
        let seq = self.next_seq()?;
        let changed = self.store.conn().execute(
            "UPDATE steering_notes SET note = ?1, seq = ?2 WHERE id = ?3",
            params![text, seq, id],
        )?;
        if changed == 0 {
            return Err(SteeringError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn deactivate(&self, id: &str) -> Result<()> {
        let changed = self.store.conn().execute(
            "UPDATE steering_notes SET active = 0 WHERE id = ?1",
            params![id],
        )?;
        if changed == 0 {
            return Err(SteeringError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let changed = self
            .store
            .conn()
            .execute("DELETE FROM steering_notes WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(SteeringError::NotFound(id.to_string()));
        }
        Ok(())
    }

    /// Everything the User can see for one piece of work, including unconfirmed
    /// proposals, so the list is the whole truth (Requirement 8.3).
    pub fn visible_for_job(&self, job_id: &str) -> Result<Vec<Note>> {
        self.select(
            "SELECT id, job_id, scope, note, origin, confirmed, active, seq
             FROM steering_notes WHERE job_id = ?1 ORDER BY seq",
            params![job_id],
        )
    }

    /// Everything Settings shows.
    pub fn visible_global(&self) -> Result<Vec<Note>> {
        self.select(
            "SELECT id, job_id, scope, note, origin, confirmed, active, seq
             FROM steering_notes WHERE job_id IS NULL ORDER BY seq",
            params![],
        )
    }

    /// What a run actually gets: global notes matching the Artefact kind first,
    /// then the Job's own notes. Later wins, so per-Job notes take precedence
    /// without a separate rule (Requirement 8.9).
    ///
    /// Unconfirmed and inactive notes are excluded (Requirement 8.10).
    pub fn resolve_for_run(
        &self,
        job_id: &str,
        producing: Option<ArtefactKind>,
    ) -> Result<Vec<Note>> {
        let mut out = Vec::new();
        for note in self.visible_global()? {
            if !note.active || !note.confirmed {
                continue;
            }
            let applies = match producing {
                Some(kind) => kind.matches(note.scope),
                None => note.scope == Scope::Everything,
            };
            if applies {
                out.push(note);
            }
        }
        for note in self.visible_for_job(job_id)? {
            if note.active && note.confirmed {
                out.push(note);
            }
        }
        out.sort_by_key(|n| (n.job_id.is_some(), n.seq));
        Ok(out)
    }

    /// Notes whose text suggests they contradict each other, surfaced rather than
    /// resolved invisibly (Requirement 8.6).
    pub fn conflicts(&self, notes: &[Note]) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (i, a) in notes.iter().enumerate() {
            for b in notes.iter().skip(i + 1) {
                if contradicts(&a.text, &b.text) {
                    out.push((a.id.clone(), b.id.clone()));
                }
            }
        }
        out
    }

    fn select(&self, sql: &str, p: &[&dyn rusqlite::ToSql]) -> Result<Vec<Note>> {
        let mut stmt = self.store.conn().prepare(sql)?;
        let rows = stmt.query_map(p, |row| {
            let scope: String = row.get(2)?;
            let origin: String = row.get(4)?;
            Ok(Note {
                id: row.get(0)?,
                job_id: row.get(1)?,
                scope: Scope::parse(&scope).expect("schema constrains scope"),
                text: row.get(3)?,
                origin: parse_origin(&origin),
                confirmed: row.get::<_, i32>(5)? != 0,
                active: row.get::<_, i32>(6)? != 0,
                seq: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Whether a note exists at all, for callers checking before proposing.
    pub fn exists(&self, id: &str) -> Result<bool> {
        Ok(self
            .store
            .conn()
            .query_row(
                "SELECT 1 FROM steering_notes WHERE id = ?1",
                params![id],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }
}

fn parse_origin(s: &str) -> Origin {
    match s {
        "explicit" => Origin::Explicit,
        "rejection" => Origin::Rejection,
        "derived_from_edit" => Origin::DerivedFromEdit,
        "derived_from_exclusion" => Origin::DerivedFromExclusion,
        "derived_from_choice" => Origin::DerivedFromChoice,
        other => unreachable!("schema constrains origin, got {other}"),
    }
}

/// A deliberately simple contradiction check: the same subject with opposite
/// polarity. It exists to surface a conflict to the User, not to adjudicate it.
fn contradicts(a: &str, b: &str) -> bool {
    let negated = |s: &str| {
        let l = s.to_lowercase();
        l.contains("don't") || l.contains("do not") || l.contains("never") || l.contains("stop")
    };
    if negated(a) == negated(b) {
        return false;
    }
    let subject = |s: &str| -> Vec<String> {
        s.to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 4)
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| !w.is_empty())
            .collect()
    };
    let (sa, sb) = (subject(a), subject(b));
    sa.iter().any(|w| sb.contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        let s = Store::open_in_memory().unwrap();
        s.conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES ('j1','scheduled','Daily newsletter','live','UTC',0,0)",
                [],
            )
            .unwrap();
        s.conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES ('j2','one_off','Board deck','active','UTC',0,0)",
                [],
            )
            .unwrap();
        s
    }

    #[test]
    fn a_typed_note_is_live_immediately_and_a_derived_one_is_not() {
        let s = store();
        let st = Steering::new(&s);
        let typed = st
            .add_for_job("n1", "j1", "Keep it under 400 words", Origin::Explicit)
            .unwrap();
        assert!(typed.confirmed);

        let derived = st
            .add_for_job(
                "n2",
                "j1",
                "Keep it under 400 words",
                Origin::DerivedFromEdit,
            )
            .unwrap();
        assert!(!derived.confirmed, "a derived note must wait for a yes");
    }

    /// Correctness Property 21: no unconfirmed derived preference influences a run.
    #[test]
    fn property_21_an_unconfirmed_note_does_not_reach_a_run() {
        let s = store();
        let st = Steering::new(&s);
        st.add_for_job("n1", "j1", "Lead with the EU AI Act", Origin::Explicit)
            .unwrap();
        st.add_for_job(
            "n2",
            "j1",
            "Drop the crypto prices",
            Origin::DerivedFromEdit,
        )
        .unwrap();

        let applied = st.resolve_for_run("j1", None).unwrap();
        assert_eq!(applied.len(), 1, "only the confirmed note should apply");
        assert_eq!(applied[0].id, "n1");

        // It is still visible to the User, as a proposal.
        let visible = st.visible_for_job("j1").unwrap();
        assert_eq!(visible.len(), 2, "the proposal must still be shown");

        st.confirm("n2").unwrap();
        assert_eq!(st.resolve_for_run("j1", None).unwrap().len(), 2);
    }

    /// Correctness Property 6: steering visibility.
    ///
    /// Everything influencing a run appears in a list the User can see and edit.
    #[test]
    fn property_6_everything_that_influences_a_run_is_visible() {
        let s = store();
        let st = Steering::new(&s);
        st.add_global("g1", Scope::Everything, "Write plainly", Origin::Explicit)
            .unwrap();
        st.add_global("g2", Scope::Deck, "Use our brand colours", Origin::Explicit)
            .unwrap();
        st.add_for_job("n1", "j2", "Put the ask last", Origin::Explicit)
            .unwrap();

        let applied = st.resolve_for_run("j2", Some(ArtefactKind::Deck)).unwrap();
        let visible: Vec<String> = st
            .visible_global()
            .unwrap()
            .into_iter()
            .chain(st.visible_for_job("j2").unwrap())
            .map(|n| n.id)
            .collect();
        for note in &applied {
            assert!(
                visible.contains(&note.id),
                "{} influenced the run but is not in any visible list",
                note.id
            );
        }
        assert_eq!(applied.len(), 3);
    }

    /// Correctness Property 29: steering precedence — per-thread beats global.
    #[test]
    fn property_29_a_per_job_note_wins_over_a_global_one() {
        let s = store();
        let st = Steering::new(&s);
        st.add_global(
            "g1",
            Scope::Deck,
            "Put the ask on the last slide",
            Origin::Explicit,
        )
        .unwrap();
        st.add_for_job(
            "n1",
            "j2",
            "For this one, put the ask first",
            Origin::Explicit,
        )
        .unwrap();

        let applied = st.resolve_for_run("j2", Some(ArtefactKind::Deck)).unwrap();
        assert_eq!(applied.len(), 2);
        assert_eq!(applied[0].id, "g1", "global notes are assembled first");
        assert_eq!(
            applied.last().unwrap().id,
            "n1",
            "the per-Job note must come last, so it wins"
        );

        // And it still holds when the global note was added afterwards.
        st.add_global(
            "g2",
            Scope::Deck,
            "Always put the ask last",
            Origin::Explicit,
        )
        .unwrap();
        let applied = st.resolve_for_run("j2", Some(ArtefactKind::Deck)).unwrap();
        assert_eq!(
            applied.last().unwrap().id,
            "n1",
            "a later global note must not overtake a per-Job note"
        );
    }

    /// Correctness Property 30: global steering visibility, with scope.
    #[test]
    fn property_30_global_notes_are_listed_with_their_scope() {
        let s = store();
        let st = Steering::new(&s);
        st.add_global("g1", Scope::Everything, "Write plainly", Origin::Explicit)
            .unwrap();
        st.add_global("g2", Scope::Deck, "Brand colours", Origin::Explicit)
            .unwrap();
        st.add_global(
            "g3",
            Scope::Spreadsheet,
            "Shillings, no decimals",
            Origin::Explicit,
        )
        .unwrap();

        let listed = st.visible_global().unwrap();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[1].scope.label(), "Decks");
        assert_eq!(listed[2].scope.label(), "Spreadsheets");
        assert!(listed.iter().all(|n| n.job_id.is_none()));
    }

    #[test]
    fn a_global_note_is_narrowed_by_what_the_run_is_producing() {
        let s = store();
        let st = Steering::new(&s);
        st.add_global("g1", Scope::Everything, "Write plainly", Origin::Explicit)
            .unwrap();
        st.add_global("g2", Scope::Deck, "Brand colours", Origin::Explicit)
            .unwrap();
        st.add_global("g3", Scope::Spreadsheet, "Shillings", Origin::Explicit)
            .unwrap();

        let deck: Vec<_> = st
            .resolve_for_run("j2", Some(ArtefactKind::Deck))
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(
            deck,
            vec!["g1", "g2"],
            "a deck must not get spreadsheet rules"
        );

        let sheet: Vec<_> = st
            .resolve_for_run("j2", Some(ArtefactKind::Spreadsheet))
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(sheet, vec!["g1", "g3"]);

        let no_artefact: Vec<_> = st
            .resolve_for_run("j1", None)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(no_artefact, vec!["g1"], "only the everything-scope applies");
    }

    /// Correctness Property 7: recency governs.
    #[test]
    fn property_7_rewording_makes_a_note_the_most_recent() {
        let s = store();
        let st = Steering::new(&s);
        st.add_for_job("n1", "j1", "Keep it under 400 words", Origin::Explicit)
            .unwrap();
        st.add_for_job("n2", "j1", "Keep it under 600 words", Origin::Explicit)
            .unwrap();
        assert_eq!(
            st.resolve_for_run("j1", None).unwrap().last().unwrap().id,
            "n2"
        );

        st.reword("n1", "Keep it under 300 words").unwrap();
        let applied = st.resolve_for_run("j1", None).unwrap();
        assert_eq!(
            applied.last().unwrap().id,
            "n1",
            "rewording should make it govern"
        );
        assert_eq!(applied.last().unwrap().text, "Keep it under 300 words");
    }

    #[test]
    fn a_deactivated_or_deleted_note_stops_applying() {
        let s = store();
        let st = Steering::new(&s);
        st.add_for_job("n1", "j1", "Lead with the EU AI Act", Origin::Explicit)
            .unwrap();
        st.add_for_job("n2", "j1", "Drop crypto", Origin::Explicit)
            .unwrap();

        st.deactivate("n1").unwrap();
        let applied = st.resolve_for_run("j1", None).unwrap();
        assert_eq!(applied.len(), 1);
        assert!(
            st.visible_for_job("j1")
                .unwrap()
                .iter()
                .any(|n| n.id == "n1"),
            "a deactivated note is still shown so the User can bring it back"
        );

        st.delete("n2").unwrap();
        assert!(st.resolve_for_run("j1", None).unwrap().is_empty());
        assert!(!st.exists("n2").unwrap());
        assert!(matches!(st.delete("n2"), Err(SteeringError::NotFound(_))));
    }

    #[test]
    fn a_global_note_cannot_be_scoped_to_one_piece_of_work() {
        let s = store();
        let st = Steering::new(&s);
        assert!(matches!(
            st.add_global("g1", Scope::Job, "nonsense", Origin::Explicit),
            Err(SteeringError::GlobalNeedsRealScope)
        ));
    }

    #[test]
    fn a_contradiction_is_surfaced_rather_than_resolved_quietly() {
        let s = store();
        let st = Steering::new(&s);
        st.add_for_job("n1", "j1", "Include the crypto prices", Origin::Explicit)
            .unwrap();
        st.add_for_job("n2", "j1", "Never include crypto prices", Origin::Explicit)
            .unwrap();
        let notes = st.resolve_for_run("j1", None).unwrap();
        let conflicts = st.conflicts(&notes);
        assert_eq!(
            conflicts.len(),
            1,
            "the contradiction should be reported: {conflicts:?}"
        );

        st.add_for_job("n3", "j1", "Lead with the EU AI Act", Origin::Explicit)
            .unwrap();
        let notes = st.resolve_for_run("j1", None).unwrap();
        assert_eq!(
            st.conflicts(&notes).len(),
            1,
            "an unrelated note must not be reported as a conflict"
        );
    }

    #[test]
    fn a_derived_note_is_proposed_in_the_users_terms() {
        let prompt = Origin::DerivedFromEdit.confirmation_prompt("keep it under 400 words");
        assert!(prompt.contains("keep it under 400 words"));
        assert!(prompt.starts_with("I noticed"));
        assert!(
            Origin::DerivedFromExclusion
                .confirmation_prompt("stop archiving receipts")
                .starts_with("Should I stop")
        );
        assert!(!Origin::Explicit.needs_confirmation());
        assert!(Origin::DerivedFromChoice.needs_confirmation());
    }
}
