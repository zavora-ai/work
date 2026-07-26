//! Run exclusivity.
//!
//! Requirement 9.4: two executions of the same Job never overlap. A newsletter
//! that takes longer than its interval must not start a second copy of itself, and
//! a User pressing run now while a scheduled execution is in flight must not get
//! two.
//!
//! A lease is a row. Acquiring it is an insert that the primary key rejects if one
//! is already held, so exclusivity is enforced by the database rather than by a
//! check-then-act in our code. A lease left behind by a process that died is
//! reclaimed after [`STALE_AFTER_SECS`], because a crash must not lock a Job out
//! of working forever.

use rusqlite::{OptionalExtension, params};
use studio_store::Store;

/// How long a lease may go without a heartbeat before another run may reclaim it.
pub const STALE_AFTER_SECS: i64 = 300;

#[derive(Debug, thiserror::Error)]
pub enum LeaseError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("this piece of work is already running")]
    AlreadyHeld,
    #[error("no lease held for {0}")]
    NotHeld(String),
}

pub type Result<T> = std::result::Result<T, LeaseError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    pub job_id: String,
    pub run_id: String,
    pub acquired_at: i64,
}

pub struct Leases<'a> {
    store: &'a Store,
}

impl<'a> Leases<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// Take the lease for a Job, or fail because someone else has it.
    pub fn acquire(&self, job_id: &str, run_id: &str) -> Result<Lease> {
        self.reclaim_stale()?;
        let inserted = self.store.conn().execute(
            "INSERT OR IGNORE INTO job_leases (job_id, run_id, acquired_at, heartbeat_at)
             VALUES (?1, ?2, unixepoch(), unixepoch())",
            params![job_id, run_id],
        )?;
        if inserted == 0 {
            return Err(LeaseError::AlreadyHeld);
        }
        let acquired_at = self.store.conn().query_row(
            "SELECT acquired_at FROM job_leases WHERE job_id = ?1",
            params![job_id],
            |r| r.get(0),
        )?;
        Ok(Lease {
            job_id: job_id.to_string(),
            run_id: run_id.to_string(),
            acquired_at,
        })
    }

    /// Say the run is still alive, so its lease is not reclaimed.
    pub fn heartbeat(&self, job_id: &str, run_id: &str) -> Result<()> {
        let updated = self.store.conn().execute(
            "UPDATE job_leases SET heartbeat_at = unixepoch()
             WHERE job_id = ?1 AND run_id = ?2",
            params![job_id, run_id],
        )?;
        if updated == 0 {
            return Err(LeaseError::NotHeld(job_id.to_string()));
        }
        Ok(())
    }

    pub fn release(&self, job_id: &str, run_id: &str) -> Result<()> {
        let deleted = self.store.conn().execute(
            "DELETE FROM job_leases WHERE job_id = ?1 AND run_id = ?2",
            params![job_id, run_id],
        )?;
        if deleted == 0 {
            return Err(LeaseError::NotHeld(job_id.to_string()));
        }
        Ok(())
    }

    pub fn held_by(&self, job_id: &str) -> Result<Option<String>> {
        Ok(self
            .store
            .conn()
            .query_row(
                "SELECT run_id FROM job_leases WHERE job_id = ?1",
                params![job_id],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Drop leases whose run stopped reporting. A crash must not lock a Job out.
    pub fn reclaim_stale(&self) -> Result<usize> {
        Ok(self.store.conn().execute(
            "DELETE FROM job_leases WHERE heartbeat_at < unixepoch() - ?1",
            params![STALE_AFTER_SECS],
        )?)
    }

    /// Force a stale lease for tests and for recovery tooling.
    pub fn age_heartbeat(&self, job_id: &str, seconds: i64) -> Result<()> {
        self.store.conn().execute(
            "UPDATE job_leases SET heartbeat_at = heartbeat_at - ?1 WHERE job_id = ?2",
            params![seconds, job_id],
        )?;
        Ok(())
    }
}

/// Two runs of one Job overlap if either starts before the other finishes.
///
/// Used by the property test, and by diagnostics to prove the invariant held over
/// real history.
pub fn overlapping_runs(store: &Store, job_id: &str) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = store.conn().prepare(
        "SELECT a.id, b.id
         FROM job_runs a JOIN job_runs b
           ON a.job_id = b.job_id AND a.id < b.id
         WHERE a.job_id = ?1
           AND b.started_at < coalesce(a.finished_at, 9223372036854775807)
           AND a.started_at < coalesce(b.finished_at, 9223372036854775807)",
    )?;
    let rows = stmt.query_map(params![job_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
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
        s
    }

    #[test]
    fn a_second_run_of_the_same_job_cannot_start() {
        let s = store();
        let l = Leases::new(&s);
        l.acquire("j1", "r1").unwrap();
        assert!(matches!(
            l.acquire("j1", "r2"),
            Err(LeaseError::AlreadyHeld)
        ));
        assert_eq!(l.held_by("j1").unwrap().as_deref(), Some("r1"));

        l.release("j1", "r1").unwrap();
        assert!(
            l.acquire("j1", "r2").is_ok(),
            "the next run may start once released"
        );
    }

    #[test]
    fn releasing_a_lease_you_do_not_hold_is_refused() {
        let s = store();
        let l = Leases::new(&s);
        l.acquire("j1", "r1").unwrap();
        assert!(matches!(l.release("j1", "r2"), Err(LeaseError::NotHeld(_))));
        assert!(matches!(
            l.heartbeat("j1", "r2"),
            Err(LeaseError::NotHeld(_))
        ));
        l.heartbeat("j1", "r1").unwrap();
    }

    #[test]
    fn a_lease_left_by_a_dead_process_is_reclaimed() {
        let s = store();
        let l = Leases::new(&s);
        l.acquire("j1", "r1").unwrap();
        // As if the process holding it died and stopped reporting.
        l.age_heartbeat("j1", STALE_AFTER_SECS + 60).unwrap();
        assert!(
            l.acquire("j1", "r2").is_ok(),
            "a crash must not lock a Job out of working forever"
        );
        assert_eq!(l.held_by("j1").unwrap().as_deref(), Some("r2"));
    }

    /// Correctness Property 5: run exclusivity.
    ///
    /// Under repeated contention, no two runs of one Job have overlapping
    /// start and finish intervals.
    #[test]
    fn property_5_no_two_runs_of_a_job_overlap() {
        let s = store();
        let l = Leases::new(&s);

        let mut started = 0;
        let mut refused = 0;
        for attempt in 0..40 {
            let run_id = format!("r{attempt}");
            match l.acquire("j1", &run_id) {
                Ok(_) => {
                    started += 1;
                    s.conn()
                        .execute(
                            "INSERT INTO job_runs (id, job_id, mode, started_at)
                             VALUES (?1, 'j1', 'live', unixepoch())",
                            params![run_id],
                        )
                        .unwrap();
                    // Half the attempts finish; half stay in flight, holding the lease.
                    if attempt % 2 == 0 {
                        s.conn()
                            .execute(
                                "UPDATE job_runs SET finished_at = unixepoch() WHERE id = ?1",
                                params![run_id],
                            )
                            .unwrap();
                        l.release("j1", &run_id).unwrap();
                    }
                }
                Err(LeaseError::AlreadyHeld) => refused += 1,
                Err(e) => panic!("unexpected: {e}"),
            }
        }

        assert!(
            started > 0 && refused > 0,
            "the test should exercise both paths"
        );
        let overlaps = overlapping_runs(&s, "j1").unwrap();
        assert!(
            overlaps.is_empty(),
            "runs of one Job overlapped: {overlaps:?}"
        );
    }

    #[test]
    fn different_jobs_run_at_the_same_time() {
        let s = store();
        s.conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES ('j2','scheduled','Inbox triage','live','UTC',0,0)",
                [],
            )
            .unwrap();
        let l = Leases::new(&s);
        l.acquire("j1", "r1").unwrap();
        assert!(
            l.acquire("j2", "r2").is_ok(),
            "exclusivity is per Job, not global"
        );
    }
}
