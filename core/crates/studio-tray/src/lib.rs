//! The tray.
//!
//! One durable queue, two views. Everything the User must see arrives here, and
//! nothing leaves it except by their decision (Requirement 6.4): items do not
//! expire, are not auto-approved, and are never silently discarded.
//!
//! Four classes, and the distinction between them is not cosmetic:
//!
//! * `Kickoff` — a first draft to check. Neutral. This is reviewing work.
//! * `Escalation` — the Job could not proceed confidently and needs a decision.
//! * `Finding` — the Job worked perfectly and found something worth knowing.
//!   Presenting this as a fault would tell the User that Work Studio is broken
//!   when it is doing exactly its job.
//! * `Attention` — something is actually wrong and needs fixing.
//!
//! Durability is the property that matters most here. An approval queue that loses
//! items on restart destroys trust permanently, so items are committed
//! transactionally and survive an abrupt end to the process.

use rusqlite::{OptionalExtension, params};
use studio_store::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayClass {
    Kickoff,
    Escalation,
    Finding,
    Attention,
}

impl TrayClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kickoff => "kickoff",
            Self::Escalation => "escalation",
            Self::Finding => "finding",
            Self::Attention => "attention",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "kickoff" => Some(Self::Kickoff),
            "escalation" => Some(Self::Escalation),
            "finding" => Some(Self::Finding),
            "attention" => Some(Self::Attention),
            _ => None,
        }
    }

    /// Whether resolving an item of this class may alter the raising Job's state.
    ///
    /// A Finding never does: the Job worked (Requirement 6.10).
    pub fn may_change_job_state(self) -> bool {
        !matches!(self, Self::Finding)
    }
}

/// How the User dealt with an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Approved,
    ApprovedWithEdits,
    ApprovedWithExclusions,
    /// Perform this batch, but stay in draft (Requirement 5.10).
    ApprovedOnce,
    Rejected,
    Chosen,
    Dismissed,
}

impl Resolution {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::ApprovedWithEdits => "approved_with_edits",
            Self::ApprovedWithExclusions => "approved_with_exclusions",
            Self::ApprovedOnce => "approved_once",
            Self::Rejected => "rejected",
            Self::Chosen => "chosen",
            Self::Dismissed => "dismissed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewItem {
    pub id: String,
    pub job_id: String,
    pub class: TrayClass,
    /// Plain language. "Your daily newsletter is ready"
    pub headline: String,
    pub detail: String,
    /// Inline actions for an escalation (Requirement 6.8).
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub job_id: String,
    pub class: TrayClass,
    pub headline: String,
    pub detail: String,
    pub choices: Vec<String>,
    pub resolved: bool,
    pub resolution: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    #[error(transparent)]
    Store(#[from] studio_store::StoreError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("no tray item with id {0}")]
    NotFound(String),
    #[error("tray item {0} was already dealt with")]
    AlreadyResolved(String),
}

pub type Result<T> = std::result::Result<T, TrayError>;

pub struct Tray<'a> {
    store: &'a Store,
}

impl<'a> Tray<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// Put something in front of the User.
    ///
    /// For an `Attention` item the caller passes the account or cause as
    /// `dedupe_on`. A second fault on the same cause updates the existing item
    /// rather than adding another, so three Jobs failing on one expired account
    /// produce one item, not three (Requirement 13.8).
    pub fn add(&self, item: &NewItem, dedupe_on: Option<&str>) -> Result<String> {
        if let Some(cause) = dedupe_on {
            let existing: Option<String> = self
                .store
                .conn()
                .query_row(
                    "SELECT id FROM tray_items
                     WHERE class = ?1 AND resolved_at IS NULL AND cause = ?2
                     LIMIT 1",
                    params![item.class.as_str(), cause],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(id) = existing {
                self.store.conn().execute(
                    "UPDATE tray_items SET headline = ?1, detail = ?2 WHERE id = ?3",
                    params![item.headline, item.detail, id],
                )?;
                return Ok(id);
            }
        }

        let choices = if item.choices.is_empty() {
            None
        } else {
            Some(item.choices.join("\u{1f}"))
        };
        self.store.conn().execute(
            "INSERT INTO tray_items
               (id, job_id, class, headline, detail, cause, choices, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())",
            params![
                item.id,
                item.job_id,
                item.class.as_str(),
                item.headline,
                item.detail,
                dedupe_on,
                choices
            ],
        )?;
        Ok(item.id.clone())
    }

    /// Everything still waiting on the User, newest first.
    pub fn unresolved(&self) -> Result<Vec<Item>> {
        self.query(
            "SELECT id, job_id, class, headline, detail, choices, resolved_at, resolution
                    FROM tray_items WHERE resolved_at IS NULL ORDER BY created_at DESC, id DESC",
        )
    }

    pub fn unresolved_of_class(&self, class: TrayClass) -> Result<Vec<Item>> {
        Ok(self
            .unresolved()?
            .into_iter()
            .filter(|i| i.class == class)
            .collect())
    }

    pub fn get(&self, id: &str) -> Result<Item> {
        let row = self.store.conn().query_row(
            "SELECT id, job_id, class, headline, detail, choices, resolved_at, resolution
             FROM tray_items WHERE id = ?1",
            params![id],
            Self::row_to_item,
        );
        match row {
            Ok(item) => Ok(item),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(TrayError::NotFound(id.to_string())),
            Err(e) => Err(e.into()),
        }
    }

    /// The User decided. Recorded once and only once.
    pub fn resolve(&self, id: &str, resolution: Resolution) -> Result<()> {
        let existing: Option<Option<i64>> = self
            .store
            .conn()
            .query_row(
                "SELECT resolved_at FROM tray_items WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?;
        match existing {
            None => return Err(TrayError::NotFound(id.to_string())),
            Some(Some(_)) => return Err(TrayError::AlreadyResolved(id.to_string())),
            Some(None) => {}
        }
        self.store.conn().execute(
            "UPDATE tray_items SET resolved_at = unixepoch(), resolution = ?1 WHERE id = ?2",
            params![resolution.as_str(), id],
        )?;
        Ok(())
    }

    pub fn count_unresolved(&self) -> Result<i64> {
        Ok(self.store.conn().query_row(
            "SELECT count(*) FROM tray_items WHERE resolved_at IS NULL",
            [],
            |r| r.get(0),
        )?)
    }

    fn query(&self, sql: &str) -> Result<Vec<Item>> {
        let mut stmt = self.store.conn().prepare(sql)?;
        let rows = stmt.query_map([], Self::row_to_item)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<Item> {
        let class: String = row.get(2)?;
        let choices: Option<String> = row.get(5)?;
        let resolved_at: Option<i64> = row.get(6)?;
        Ok(Item {
            id: row.get(0)?,
            job_id: row.get(1)?,
            class: TrayClass::parse(&class).expect("schema constrains class"),
            headline: row.get(3)?,
            detail: row.get(4)?,
            choices: choices
                .map(|c| c.split('\u{1f}').map(str::to_string).collect())
                .unwrap_or_default(),
            resolved: resolved_at.is_some(),
            resolution: row.get(7)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_job() -> Store {
        let s = Store::open_in_memory().unwrap();
        seed_job(&s, "j1", "Daily newsletter");
        seed_job(&s, "j2", "Inbox triage");
        seed_job(&s, "j3", "Morning digest");
        s
    }

    fn seed_job(store: &Store, id: &str, purpose: &str) {
        store
            .conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES (?1, 'scheduled', ?2, 'live', 'Africa/Nairobi', 0, 0)",
                params![id, purpose],
            )
            .unwrap();
    }

    fn item(id: &str, job: &str, class: TrayClass) -> NewItem {
        NewItem {
            id: id.into(),
            job_id: job.into(),
            class,
            headline: format!("headline {id}"),
            detail: format!("detail {id}"),
            choices: vec![],
        }
    }

    #[test]
    fn all_four_classes_are_accepted_and_round_trip() {
        let s = store_with_job();
        let tray = Tray::new(&s);
        for (i, class) in [
            TrayClass::Kickoff,
            TrayClass::Escalation,
            TrayClass::Finding,
            TrayClass::Attention,
        ]
        .into_iter()
        .enumerate()
        {
            tray.add(&item(&format!("t{i}"), "j1", class), None)
                .unwrap();
        }
        assert_eq!(tray.count_unresolved().unwrap(), 4);
        for class in [
            TrayClass::Kickoff,
            TrayClass::Escalation,
            TrayClass::Finding,
            TrayClass::Attention,
        ] {
            assert_eq!(
                tray.unresolved_of_class(class).unwrap().len(),
                1,
                "expected exactly one {class:?}"
            );
        }
    }

    /// Correctness Property 18: a Finding is never a fault.
    #[test]
    fn property_18_a_finding_never_changes_the_job_state() {
        assert!(!TrayClass::Finding.may_change_job_state());
        for class in [
            TrayClass::Kickoff,
            TrayClass::Escalation,
            TrayClass::Attention,
        ] {
            assert!(class.may_change_job_state(), "{class:?} may change state");
        }

        let s = store_with_job();
        let tray = Tray::new(&s);
        let before: String = s
            .conn()
            .query_row("SELECT state FROM jobs WHERE id = 'j1'", [], |r| r.get(0))
            .unwrap();
        tray.add(
            &NewItem {
                id: "f1".into(),
                job_id: "j1".into(),
                class: TrayClass::Finding,
                headline: "Your startup disk is 94% full".into(),
                detail: "Nothing is broken yet.".into(),
                choices: vec!["See what's big".into(), "Got it".into()],
            },
            None,
        )
        .unwrap();
        tray.resolve("f1", Resolution::Dismissed).unwrap();
        let after: String = s
            .conn()
            .query_row("SELECT state FROM jobs WHERE id = 'j1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after, "dismissing a Finding must not touch the Job");
    }

    /// Requirement 6.8: an escalation's choices are inline actions.
    #[test]
    fn an_escalation_carries_its_choices() {
        let s = store_with_job();
        let tray = Tray::new(&s);
        tray.add(
            &NewItem {
                id: "e1".into(),
                job_id: "j1".into(),
                class: TrayClass::Escalation,
                headline: "Two of these look like the same invoice".into(),
                detail: "A £480 charge twice, three days apart.".into(),
                choices: vec!["File both".into(), "File one".into()],
            },
            None,
        )
        .unwrap();
        let items = tray.unresolved().unwrap();
        assert_eq!(items[0].choices, vec!["File both", "File one"]);
    }

    /// Requirement 6.4: nothing expires, is auto-approved, or is silently dropped.
    #[test]
    fn an_item_is_resolved_once_and_only_by_a_decision() {
        let s = store_with_job();
        let tray = Tray::new(&s);
        tray.add(&item("t1", "j1", TrayClass::Kickoff), None)
            .unwrap();

        tray.resolve("t1", Resolution::Approved).unwrap();
        assert_eq!(tray.count_unresolved().unwrap(), 0);

        let again = tray.resolve("t1", Resolution::Rejected);
        assert!(
            matches!(again, Err(TrayError::AlreadyResolved(_))),
            "a second decision on the same item must be refused"
        );
        assert!(matches!(
            tray.resolve("nope", Resolution::Approved),
            Err(TrayError::NotFound(_))
        ));

        let resolved = tray.get("t1").unwrap();
        assert!(resolved.resolved);
        assert_eq!(resolved.resolution.as_deref(), Some("approved"));
    }

    #[test]
    fn every_resolution_is_accepted_by_the_schema() {
        let s = store_with_job();
        let tray = Tray::new(&s);
        for (i, r) in [
            Resolution::Approved,
            Resolution::ApprovedWithEdits,
            Resolution::ApprovedWithExclusions,
            Resolution::ApprovedOnce,
            Resolution::Rejected,
            Resolution::Chosen,
            Resolution::Dismissed,
        ]
        .into_iter()
        .enumerate()
        {
            let id = format!("r{i}");
            tray.add(&item(&id, "j1", TrayClass::Kickoff), None)
                .unwrap();
            tray.resolve(&id, r)
                .unwrap_or_else(|e| panic!("{r:?}: {e}"));
        }
    }

    /// Correctness Property 22: consolidated connector faults.
    ///
    /// One account expiring produces one item however many Jobs depend on it.
    #[test]
    fn property_22_one_account_fault_produces_one_item() {
        let s = store_with_job();
        let tray = Tray::new(&s);
        for (i, job) in ["j1", "j2", "j3"].into_iter().enumerate() {
            tray.add(
                &NewItem {
                    id: format!("a{i}"),
                    job_id: job.into(),
                    class: TrayClass::Attention,
                    headline: "Gmail needs reconnecting".into(),
                    detail: "Your sign-in for Gmail expired on Thursday.".into(),
                    choices: vec![],
                },
                Some("Gmail"),
            )
            .unwrap();
        }
        assert_eq!(
            tray.unresolved_of_class(TrayClass::Attention)
                .unwrap()
                .len(),
            1,
            "three Jobs failing on one account must raise one item"
        );

        // A different account is a different item.
        tray.add(
            &NewItem {
                id: "a9".into(),
                job_id: "j1".into(),
                class: TrayClass::Attention,
                headline: "X needs reconnecting".into(),
                detail: "Your sign-in for X expired.".into(),
                choices: vec![],
            },
            Some("X"),
        )
        .unwrap();
        assert_eq!(
            tray.unresolved_of_class(TrayClass::Attention)
                .unwrap()
                .len(),
            2
        );
    }

    /// Correctness Property 3: tray durability.
    ///
    /// Unresolved items survive the process ending, and an uncommitted write
    /// leaves nothing behind.
    #[test]
    fn property_3_the_tray_survives_the_process_ending() {
        let dir = std::env::temp_dir().join(format!("zws-tray-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("studio.db");
        let _ = std::fs::remove_file(&path);

        // First process: add three, resolve one.
        {
            let s = Store::open(&path).unwrap();
            seed_job(&s, "j1", "Daily newsletter");
            let tray = Tray::new(&s);
            tray.add(&item("t1", "j1", TrayClass::Kickoff), None)
                .unwrap();
            tray.add(&item("t2", "j1", TrayClass::Finding), None)
                .unwrap();
            tray.add(&item("t3", "j1", TrayClass::Attention), None)
                .unwrap();
            tray.resolve("t2", Resolution::Dismissed).unwrap();
            assert_eq!(tray.count_unresolved().unwrap(), 2);
        } // dropped, as if the process ended

        // Second process: the same two are still waiting.
        {
            let s = Store::open(&path).unwrap();
            let tray = Tray::new(&s);
            let ids: Vec<_> = tray
                .unresolved()
                .unwrap()
                .into_iter()
                .map(|i| i.id)
                .collect();
            assert_eq!(ids.len(), 2, "unresolved items must survive: {ids:?}");
            assert!(ids.contains(&"t1".to_string()));
            assert!(ids.contains(&"t3".to_string()));
            assert!(
                tray.get("t2").unwrap().resolved,
                "a decision must also survive"
            );
        }

        // An abandoned transaction leaves nothing.
        {
            let mut s = Store::open(&path).unwrap();
            let tx = s.conn_mut().transaction().unwrap();
            tx.execute(
                "INSERT INTO tray_items (id, job_id, class, headline, detail, created_at)
                 VALUES ('t4','j1','kickoff','h','d',0)",
                [],
            )
            .unwrap();
            drop(tx); // rolls back
        }
        {
            let s = Store::open(&path).unwrap();
            let tray = Tray::new(&s);
            assert_eq!(
                tray.count_unresolved().unwrap(),
                2,
                "a write that was never committed must not appear"
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
