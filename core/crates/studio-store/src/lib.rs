//! Local store. Everything Work Studio knows lives on this device
//! (Requirement 16.1): Jobs, runs, tray items, steering notes, Artefact metadata,
//! deliveries, the Activity_Log and the spend ledger.
//!
//! Two things deliberately do *not* live here. Credentials are in the OS keychain
//! (Requirement 13.4). Artefacts are ordinary files in the User's own folder
//! (Requirement 12.1) — only their metadata is here.
//!
//! At-rest encryption (Requirement 16.4) is applied by [`Store::open_encrypted`],
//! which is compiled only with the `sqlcipher` feature. The unencrypted path
//! exists for tests and is not permitted in a released build; task 17.4 verifies
//! that in the packaged artefact.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

pub const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../migrations/0001_init.sql")),
    (
        "0002_thread_turns",
        include_str!("../migrations/0002_thread_turns.sql"),
    ),
    (
        "0003_capabilities",
        include_str!("../migrations/0003_capabilities.sql"),
    ),
    (
        "0004_run_specialist",
        include_str!("../migrations/0004_run_specialist.sql"),
    ),
];

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("the store is append-only for {0}")]
    AppendOnly(&'static str),
}

pub type Result<T> = std::result::Result<T, StoreError>;

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (creating if needed) a store at `path` and bring it up to date.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::from_conn(conn)
    }

    /// In-memory store, for tests.
    pub fn open_in_memory() -> Result<Self> {
        Self::from_conn(Connection::open_in_memory()?)
    }

    /// Encrypted store. The key comes from the OS keychain, never from
    /// configuration or an environment variable.
    #[cfg(feature = "sqlcipher")]
    pub fn open_encrypted(path: impl AsRef<Path>, key: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "key", key)?;
        Self::from_conn(conn)
    }

    fn from_conn(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let mut store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Apply any migration not yet recorded. Idempotent.
    pub fn migrate(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
               name       TEXT PRIMARY KEY,
               applied_at INTEGER NOT NULL
             );",
        )?;
        for (name, sql) in MIGRATIONS {
            let already: Option<String> = self
                .conn
                .query_row(
                    "SELECT name FROM schema_migrations WHERE name = ?1",
                    [name],
                    |r| r.get(0),
                )
                .optional()?;
            if already.is_some() {
                continue;
            }
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (name, applied_at)
                 VALUES (?1, unixepoch())",
                [name],
            )?;
            tx.commit()?;
        }
        Ok(())
    }

    pub fn applied_migrations(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM schema_migrations ORDER BY name")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// The only way anything enters the Activity_Log.
    pub fn log(
        &self,
        category: &str,
        detail: &str,
        job_id: Option<&str>,
        run_id: Option<&str>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO activity_log (ts, job_id, run_id, category, detail)
             VALUES (unixepoch(), ?1, ?2, ?3, ?4)",
            rusqlite::params![job_id, run_id, category, detail],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn activity_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT count(*) FROM activity_log", [], |r| r.get(0))?)
    }

    /// Table names present in the schema, for structural assertions.
    pub fn tables(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open_in_memory().expect("store opens")
    }

    #[test]
    fn migration_applies_and_is_idempotent() {
        let mut s = store();
        let expected = vec![
            "0001_init",
            "0002_thread_turns",
            "0003_capabilities",
            "0004_run_specialist",
        ];
        assert_eq!(s.applied_migrations().unwrap(), expected);
        s.migrate().expect("re-running migrations is a no-op");
        assert_eq!(
            s.applied_migrations().unwrap(),
            expected,
            "a migration must not be applied twice"
        );
    }

    #[test]
    fn schema_contains_every_table_the_design_specifies() {
        let s = store();
        let tables = s.tables().unwrap();
        for expected in [
            "activity_log",
            "artefact_changes",
            "artefact_jobs",
            "artefacts",
            "connectors",
            "deliveries",
            "job_runs",
            "jobs",
            "spend_ledger",
            "capabilities",
            "capability_agents",
            "steering_notes",
            "thread_turns",
            "tray_items",
        ] {
            assert!(
                tables.iter().any(|t| t == expected),
                "missing table {expected}; have {tables:?}"
            );
        }
    }

    /// Correctness Property 16: the Activity_Log is append-only.
    ///
    /// Enforced by triggers, so no code path — ours or anyone's — can update or
    /// delete a row.
    #[test]
    fn property_16_activity_log_is_append_only() {
        let s = store();
        s.log("action", "Sent your morning digest", None, None)
            .unwrap();
        s.log("failover", "primary was rate limited", None, None)
            .unwrap();
        assert_eq!(s.activity_count().unwrap(), 2);

        let update = s.conn().execute(
            "UPDATE activity_log SET detail = 'tampered' WHERE seq = 1",
            [],
        );
        assert!(update.is_err(), "UPDATE on activity_log must be rejected");
        assert!(
            update.unwrap_err().to_string().contains("append-only"),
            "rejection should say why"
        );

        let delete = s
            .conn()
            .execute("DELETE FROM activity_log WHERE seq = 1", []);
        assert!(delete.is_err(), "DELETE on activity_log must be rejected");

        assert_eq!(
            s.activity_count().unwrap(),
            2,
            "rejected writes must leave the log intact"
        );
    }

    #[test]
    fn job_state_and_kind_are_constrained_by_the_schema() {
        let s = store();
        let insert = |kind: &str, state: &str| {
            s.conn().execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES (?1, ?2, 'x', ?3, 'Africa/Nairobi', 0, 0)",
                rusqlite::params![format!("{kind}-{state}"), kind, state],
            )
        };
        assert!(insert("scheduled", "live").is_ok());
        assert!(insert("one_off", "active").is_ok());
        assert!(
            insert("scheduled", "in_flight").is_err(),
            "an unknown state must be rejected"
        );
        assert!(
            insert("recurring", "live").is_err(),
            "an unknown kind must be rejected"
        );
    }

    #[test]
    fn one_off_jobs_cannot_carry_a_schedule() {
        let s = store();
        let r = s.conn().execute(
            "INSERT INTO jobs (id, kind, purpose, state, schedule_kind, timezone, created_at, updated_at)
             VALUES ('j', 'one_off', 'x', 'active', 'time_of_day', 'UTC', 0, 0)",
            [],
        );
        assert!(r.is_err(), "a one_off Job must not carry a schedule");
    }

    /// Requirement 8.7-8.10: a global steering note has no Job and a real scope;
    /// a per-Job note is scoped to 'job'.
    #[test]
    fn steering_scope_matches_ownership() {
        let s = store();
        s.conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES ('j1', 'scheduled', 'Daily newsletter', 'live', 'UTC', 0, 0)",
                [],
            )
            .unwrap();

        let global = s.conn().execute(
            "INSERT INTO steering_notes (id, job_id, scope, note, origin, seq, created_at)
             VALUES ('g1', NULL, 'deck', 'Use our brand colours', 'explicit', 1, 0)",
            [],
        );
        assert!(global.is_ok(), "a global note may exist without a Job");

        let per_job = s.conn().execute(
            "INSERT INTO steering_notes (id, job_id, scope, note, origin, seq, created_at)
             VALUES ('n1', 'j1', 'job', 'Keep it under 400 words', 'explicit', 2, 0)",
            [],
        );
        assert!(per_job.is_ok());

        let bad = s.conn().execute(
            "INSERT INTO steering_notes (id, job_id, scope, note, origin, seq, created_at)
             VALUES ('n2', 'j1', 'deck', 'confused scope', 'explicit', 3, 0)",
            [],
        );
        assert!(bad.is_err(), "a per-Job note must be scoped to 'job'");
    }

    /// Correctness Property 17: an irreversible delivery never carries a
    /// reversal window.
    #[test]
    fn property_17_irreversible_delivery_has_no_window() {
        let s = store();
        s.conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES ('j1', 'scheduled', 'Morning digest', 'live', 'UTC', 0, 0)",
                [],
            )
            .unwrap();
        s.conn()
            .execute(
                "INSERT INTO job_runs (id, job_id, mode, started_at) VALUES ('r1','j1','live',0)",
                [],
            )
            .unwrap();

        let ok = s.conn().execute(
            "INSERT INTO deliveries (id, run_id, connector, action, reversibility, reversal_expires_at, ts)
             VALUES ('d1','r1','x','Posted to X','reversible', 999, 0)",
            [],
        );
        assert!(ok.is_ok());

        let bad = s.conn().execute(
            "INSERT INTO deliveries (id, run_id, connector, action, reversibility, reversal_expires_at, ts)
             VALUES ('d2','r1','email','Sent your digest','irreversible', 999, 0)",
            [],
        );
        assert!(
            bad.is_err(),
            "an irreversible delivery must not claim a reversal window"
        );
    }

    #[test]
    fn tray_item_classes_and_resolutions_are_constrained() {
        let s = store();
        s.conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES ('j1', 'scheduled', 'Daily newsletter', 'draft', 'UTC', 0, 0)",
                [],
            )
            .unwrap();
        for class in ["kickoff", "escalation", "finding", "attention"] {
            assert!(
                s.conn()
                    .execute(
                        "INSERT INTO tray_items (id, job_id, class, headline, detail, created_at)
                         VALUES (?1, 'j1', ?2, 'h', 'd', 0)",
                        rusqlite::params![class, class],
                    )
                    .is_ok(),
                "{class} should be a valid tray class"
            );
        }
        assert!(
            s.conn()
                .execute(
                    "INSERT INTO tray_items (id, job_id, class, headline, detail, created_at)
                     VALUES ('x', 'j1', 'warning', 'h', 'd', 0)",
                    [],
                )
                .is_err(),
            "an unknown tray class must be rejected"
        );
    }
}
