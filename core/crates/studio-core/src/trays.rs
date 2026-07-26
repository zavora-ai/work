//! What is waiting on the User, and what has gone out.
//!
//! `studio-tray` was written, tested and complete — four item classes, resolve-once semantics,
//! dedupe — and nothing ever called `add`. So the In Tray on screen was five invented rows
//! while the real queue sat empty. This connects the two, and gives the tray its first real
//! source: a specialist that wanted to do something it is not allowed to do.
//!
//! That is the honest definition of an escalation. Previously a refused operation was a line in
//! a log the User never reads; now it is an item with the User's own words on it, and the work
//! it belongs to.

use serde::{Deserialize, Serialize};
use studio_store::Store;
use studio_tray::{NewItem, Resolution, Tray, TrayClass};

/// One thing waiting on the User, in terms the interface can draw.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Waiting {
    pub id: String,
    /// Which piece of work it belongs to.
    pub about: String,
    /// `kickoff`, `escalation`, `finding` or `attention` — the interface draws each differently.
    pub sort: String,
    pub headline: String,
    pub detail: String,
    /// The actions offered inline. Empty means the only answers are accept and dismiss.
    pub choices: Vec<String>,
}

/// Everything unresolved, oldest first, because the oldest has waited longest.
pub fn waiting(store: &Store) -> Result<Vec<Waiting>, String> {
    let tray = Tray::new(store);
    let items = tray.unresolved().map_err(|e| e.to_string())?;
    Ok(items
        .into_iter()
        .map(|item| Waiting {
            id: item.id,
            about: item.job_id,
            sort: item.class.as_str().to_string(),
            headline: item.headline,
            detail: item.detail,
            choices: item.choices,
        })
        .collect())
}

/// What the User did about it.
#[derive(Debug, Deserialize)]
pub struct Decision {
    pub id: String,
    /// `accept`, `accept once`, `decline`, `dismiss`, or the text of a choice.
    pub answer: String,
}

/// Resolve an item. Resolving twice is refused by the tray itself, not by a check here.
pub fn decide(store: &Store, decision: &Decision) -> Result<(), String> {
    let resolution = match decision.answer.as_str() {
        "accept" => Resolution::Approved,
        "accept once" => Resolution::ApprovedOnce,
        "accept with changes" => Resolution::ApprovedWithEdits,
        "decline" => Resolution::Rejected,
        "dismiss" => Resolution::Dismissed,
        // Anything else is one of the offered choices, which is a decision in its own right.
        _ => Resolution::Chosen,
    };
    Tray::new(store)
        .resolve(&decision.id, resolution)
        .map_err(|e| e.to_string())
}

/// Put something in the tray, making sure the work it belongs to exists first.
///
/// A tray item references a Job, so an item about a conversation needs that conversation to be
/// a Job on the record. It already is — threads are stored as Jobs — but a thread that has not
/// been written yet would make this fail silently, which is exactly the class of bug that made
/// every `activity_log` write fail under a `let _`.
pub fn add(
    store: &Store,
    job_id: &str,
    class: TrayClass,
    headline: &str,
    detail: &str,
    choices: Vec<String>,
    dedupe_on: Option<&str>,
) -> Result<String, String> {
    let item = NewItem {
        id: crate::keeper::new_id("tray"),
        job_id: job_id.to_string(),
        class,
        headline: headline.to_string(),
        detail: detail.to_string(),
        choices,
    };
    Tray::new(store)
        .add(&item, dedupe_on)
        .map_err(|e| e.to_string())
}

/// What went out, for the Out Tray.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivered {
    pub id: String,
    pub what: String,
    pub where_to: String,
    pub when: i64,
    /// True while the User can still take it back.
    pub reversible: bool,
    /// When the chance to take it back runs out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reversible_until: Option<i64>,
    pub reversed: bool,
}

/// Everything delivered, newest first.
pub fn delivered(store: &Store) -> Result<Vec<Delivered>, String> {
    let conn = store.conn();
    let mut statement = conn
        .prepare(
            "SELECT d.id, coalesce(r.summary, d.action), coalesce(d.target, d.connector),
                    d.ts, d.reversibility, d.reversal_expires_at, d.reversed_at
             FROM deliveries d
             JOIN job_runs r ON r.id = d.run_id
             ORDER BY d.ts DESC LIMIT 50",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let reversed_at: Option<i64> = row.get(6)?;
            let reversibility: String = row.get(4)?;
            Ok(Delivered {
                id: row.get(0)?,
                what: row.get(1)?,
                where_to: row.get(2)?,
                when: row.get(3)?,
                // "partial" counts as reversible to the User: something can still be undone,
                // and the note says what. Claiming otherwise would understate their options.
                reversible: reversibility != "irreversible",
                reversible_until: row.get(5)?,
                reversed: reversed_at.is_some(),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Record that something went out.
///
/// A delivery belongs to a run, not to a Job — because "what went out" is a fact about one
/// execution, and the Out Tray needs to say which. So a run is opened and closed around it.
pub fn record_delivery(
    store: &Store,
    job_id: &str,
    summary: &str,
    destination: &str,
    reversible_for: Option<i64>,
) -> Result<String, String> {
    let conn = store.conn();
    let run = crate::keeper::new_id("run");
    conn.execute(
        "INSERT INTO job_runs (id, job_id, mode, started_at, finished_at, outcome, summary)
         VALUES (?1, ?2, 'manual', unixepoch(), unixepoch(), 'completed', ?3)",
        rusqlite::params![run, job_id, summary],
    )
    .map_err(|e| e.to_string())?;

    let id = crate::keeper::new_id("del");
    // An irreversible delivery must not claim a window it does not have. The schema enforces
    // that (Correctness Property 17), so this cannot quietly promise an undo — the insert
    // would be rejected rather than stored.
    let reversibility = if reversible_for.is_some() {
        "reversible"
    } else {
        "irreversible"
    };
    conn.execute(
        "INSERT INTO deliveries
           (id, run_id, connector, action, target, reversibility, reversal_expires_at, ts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())",
        rusqlite::params![
            id,
            run,
            destination,
            summary,
            destination,
            reversibility,
            reversible_for.map(|seconds| seconds_now() + seconds)
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

pub fn seconds_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_work() -> Store {
        let mut store = Store::open_in_memory().unwrap();
        store.migrate().unwrap();
        store
            .conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES ('j1', 'one_off', 'Q3 model', 'active', 'UTC', unixepoch(), unixepoch())",
                [],
            )
            .unwrap();
        store
    }

    #[test]
    fn an_empty_tray_is_empty_rather_than_furnished() {
        let store = store_with_work();
        assert!(waiting(&store).unwrap().is_empty());
        assert!(delivered(&store).unwrap().is_empty());
    }

    #[test]
    fn something_added_is_waiting_in_the_users_words() {
        let store = store_with_work();
        add(
            &store,
            "j1",
            TrayClass::Escalation,
            "I could not send that",
            "Sending email is switched off.",
            vec!["Turn it on".to_string(), "Leave it off".to_string()],
            None,
        )
        .unwrap();

        let items = waiting(&store).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].headline, "I could not send that");
        assert_eq!(items[0].sort, "escalation");
        assert_eq!(items[0].choices.len(), 2);
    }

    #[test]
    fn a_decision_takes_it_out_of_the_tray() {
        let store = store_with_work();
        let id = add(
            &store,
            "j1",
            TrayClass::Finding,
            "Two rows look wrong",
            "Rows 12 and 13 have no date.",
            vec![],
            None,
        )
        .unwrap();

        decide(
            &store,
            &Decision {
                id: id.clone(),
                answer: "dismiss".to_string(),
            },
        )
        .unwrap();
        assert!(waiting(&store).unwrap().is_empty());

        // Resolve-once: the second answer is refused by the tray, not by this module.
        let again = decide(
            &store,
            &Decision {
                id,
                answer: "accept".to_string(),
            },
        );
        assert!(again.is_err(), "an item must not be resolvable twice");
    }

    #[test]
    fn one_item_per_cause_however_many_times_it_happens() {
        let store = store_with_work();
        for _ in 0..4 {
            add(
                &store,
                "j1",
                TrayClass::Escalation,
                "I could not reach your email",
                "The account needs reconnecting.",
                vec![],
                Some("account:email"),
            )
            .unwrap();
        }
        assert_eq!(
            waiting(&store).unwrap().len(),
            1,
            "four failures of one cause is one thing to decide, not four"
        );
    }

    #[test]
    fn what_went_out_is_recorded_and_says_whether_it_can_be_taken_back() {
        let store = store_with_work();
        record_delivery(&store, "j1", "Weekly summary", "Out Tray", Some(600)).unwrap();
        record_delivery(&store, "j1", "Sent invoice", "billing@example.com", None).unwrap();

        let items = delivered(&store).unwrap();
        assert_eq!(items.len(), 2);
        let irreversible = items.iter().find(|d| d.what == "Sent invoice").unwrap();
        assert!(!irreversible.reversible);
        assert!(
            irreversible.reversible_until.is_none(),
            "an irreversible delivery must not claim a window"
        );
        let reversible = items.iter().find(|d| d.what == "Weekly summary").unwrap();
        assert!(reversible.reversible && reversible.reversible_until.is_some());
    }
}
