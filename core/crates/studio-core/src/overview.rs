//! What the Dashboard says.
//!
//! Every figure here was invented — `5`, `3`, `11`, `$0.62` — and indistinguishable on screen
//! from one that had been measured. These are counted from the store instead, and a figure that
//! cannot be counted is reported as unavailable rather than as zero.
//!
//! That distinction matters more than it sounds. Zero is a claim: it says nothing happened.
//! Unavailable says we do not know. Showing the first when the second is true is how a User
//! stops trusting the rest of the screen.

use serde::Serialize;
use studio_store::Store;

/// A figure, or the honest absence of one.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Figure {
    /// How it reads. An em dash when there is nothing to say.
    pub value: String,
    /// False when Work Studio cannot answer, so the interface can say so rather than imply a
    /// count of zero.
    pub known: bool,
}

impl Figure {
    fn of(count: i64) -> Self {
        Self {
            value: count.to_string(),
            known: true,
        }
    }

    fn money(micros: i64) -> Self {
        Self {
            value: studio_router::format_spend(micros),
            known: true,
        }
    }

    /// A figure that was measured, already in words.
    pub fn plain(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            known: true,
        }
    }

    /// Not measured. The interface shows the dash and may explain why.
    pub fn unavailable() -> Self {
        Self {
            value: "—".to_string(),
            known: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    /// Pieces of work that have had something happen to them today.
    pub working: Figure,
    /// Things waiting for the User to decide.
    pub waiting: Figure,
    /// Things finished today.
    pub done: Figure,
    /// What today has cost, as the provider counted it.
    pub cost: Figure,
    /// Why a figure is missing, when one is. In the User's words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Count what can be counted.
pub fn overview(store: &Store) -> Result<Overview, String> {
    let conn = store.conn();

    let count = |sql: &str| -> Result<i64, String> {
        conn.query_row(sql, [], |row| row.get::<_, i64>(0))
            .map_err(|e| e.to_string())
    };

    // Work in hand: a piece of work touched today.
    let working = count(
        "SELECT count(*) FROM jobs
         WHERE updated_at >= unixepoch('now', 'start of day')",
    )?;

    // Waiting on the User: unresolved tray items.
    let waiting = count("SELECT count(*) FROM tray_items WHERE resolved_at IS NULL")?;

    // Finished today: recorded changes to the User's files.
    let done = count(
        "SELECT count(*) FROM artefact_changes
         WHERE ts >= unixepoch('now', 'start of day')",
    )?;

    // What it cost. Nothing metered means nothing spent *that we counted*, which is not the
    // same as nothing spent — so it is unavailable rather than zero.
    let metered = count("SELECT count(*) FROM spend_ledger")?;
    let cost = if metered == 0 {
        Figure::unavailable()
    } else {
        Figure::money(count(
            "SELECT coalesce(sum(micros), 0) FROM spend_ledger
             WHERE ts >= unixepoch('now', 'start of day')",
        )?)
    };

    let note = if cost.known {
        None
    } else {
        Some("Nothing has been metered yet today.".to_string())
    };

    Ok(Overview {
        working: Figure::of(working),
        waiting: Figure::of(waiting),
        done: Figure::of(done),
        cost,
        note,
    })
}

/// What has happened, for the diagnostics view.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub seq: i64,
    pub when: i64,
    pub category: String,
    pub detail: String,
}

/// The most recent entries, newest first.
pub fn activity(store: &Store, limit: usize) -> Result<Vec<Entry>, String> {
    let conn = store.conn();
    let mut statement = conn
        .prepare(
            "SELECT seq, ts, category, detail FROM activity_log
             ORDER BY seq DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([limit as i64], |row| {
            Ok(Entry {
                seq: row.get(0)?,
                when: row.get(1)?,
                category: row.get(2)?,
                detail: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Record what a piece of work cost.
///
/// The surface is `documents`, which is what the store calls work on the User's own files —
/// its allowed values are fixed by the schema, and a value outside them is rejected rather
/// than stored.
pub fn record_spend(store: &Store, job_id: &str, micros: i64) -> Result<(), String> {
    if micros <= 0 {
        return Ok(());
    }
    store
        .conn()
        .execute(
            "INSERT INTO spend_ledger (id, ts, job_id, surface, tier, micros)
             VALUES (?1, unixepoch(), ?2, 'documents', 'balanced', ?3)",
            rusqlite::params![crate::keeper::new_id("spend"), job_id, micros],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        let mut store = Store::open_in_memory().unwrap();
        store.migrate().unwrap();
        store
    }

    /// The reason this exists: a figure nobody measured must not read as a measured zero.
    #[test]
    fn a_figure_we_cannot_answer_says_so() {
        let store = store();
        let view = overview(&store).unwrap();
        assert_eq!(view.cost.value, "—");
        assert!(!view.cost.known, "cost must be marked unknown, not zero");
        assert!(view.note.is_some(), "and the reason should be sayable");

        // Counts we genuinely can answer are answered, and zero here is a real zero.
        assert_eq!(view.working.value, "0");
        assert!(view.working.known);
    }

    #[test]
    fn cost_becomes_real_once_something_is_metered() {
        let store = store();
        store
            .conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES ('j1', 'one_off', 'x', 'active', 'UTC', unixepoch(), unixepoch())",
                [],
            )
            .unwrap();
        record_spend(&store, "j1", 62_000).unwrap();

        let view = overview(&store).unwrap();
        assert!(view.cost.known);
        assert!(
            view.cost.value.contains('0'),
            "reads as money: {}",
            view.cost.value
        );
        assert!(view.note.is_none());
    }

    #[test]
    fn nothing_is_recorded_for_nothing_spent() {
        let store = store();
        record_spend(&store, "j1", 0).unwrap();
        assert!(
            !overview(&store).unwrap().cost.known,
            "a zero charge is not a measurement"
        );
    }

    #[test]
    fn work_touched_today_is_counted() {
        let store = store();
        store
            .conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES ('j1', 'one_off', 'Add a column', 'active', 'UTC', unixepoch(), unixepoch())",
                [],
            )
            .unwrap();
        assert_eq!(overview(&store).unwrap().working.value, "1");
    }

    #[test]
    fn the_activity_log_reads_newest_first() {
        let store = store();
        store.log("action", "first", None, None).unwrap();
        store.log("action", "second", None, None).unwrap();
        let entries = activity(&store, 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].detail, "second");
    }

    #[test]
    fn the_activity_log_can_be_limited() {
        let store = store();
        for n in 0..30 {
            store
                .log("action", &format!("entry {n}"), None, None)
                .unwrap();
        }
        assert_eq!(activity(&store, 5).unwrap().len(), 5);
    }
}
