//! How each specialist is doing, measured.
//!
//! The Agents screen showed "Accepted as-is 72%", "Typical wait 4.1s" and "A day $0.08" for
//! three specialists. None of it had been measured; the numbers were written into a fixture and
//! looked exactly like evidence. A User deciding whether to trust a specialist with their work
//! was reading a decoration.
//!
//! What can be counted is counted here. What cannot is reported as unavailable, with the reason
//! sayable, so the screen tells the truth about its own ignorance.

use crate::overview::Figure;
use serde::Serialize;
use studio_store::Store;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Standing {
    /// The specialist's own key: `spreadsheet`, `document`, `presentation`.
    pub id: String,
    /// How many pieces of work it has finished.
    pub finished: Figure,
    /// How long the User typically waits, in the User's words ("about 40 seconds").
    pub typical_wait: Figure,
    /// How often what it produced was kept as it was.
    pub kept_as_is: Figure,
    /// What it has been told, and remembers.
    pub learned: Figure,
}

/// Measure each specialist. Absent evidence is reported, never filled in.
pub fn standings(store: &Store) -> Result<Vec<Standing>, String> {
    ["spreadsheet", "document", "presentation"]
        .into_iter()
        .map(|id| standing(store, id))
        .collect()
}

fn standing(store: &Store, id: &str) -> Result<Standing, String> {
    let conn = store.conn();

    let finished: i64 = conn
        .query_row(
            "SELECT count(*) FROM job_runs WHERE specialist = ?1 AND outcome = 'completed'",
            [id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // The median, not the mean: one 90-second run among nine quick ones should not describe
    // the experience of the other nine.
    let waits: Vec<i64> = {
        let mut statement = conn
            .prepare(
                "SELECT finished_at - started_at FROM job_runs
                 WHERE specialist = ?1 AND finished_at IS NOT NULL
                 ORDER BY finished_at - started_at",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([id], |row| row.get::<_, i64>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    let typical_wait = if waits.is_empty() {
        Figure::unavailable()
    } else {
        Figure::plain(in_words(waits[waits.len() / 2]))
    };

    // Kept as-is: changes this specialist made that the User did not then change themselves.
    // Needs both authors in the change log, which is why nothing could be said before it had a
    // writer.
    let by_studio: i64 = conn
        .query_row(
            "SELECT count(*) FROM artefact_changes c
             JOIN artefacts a ON a.id = c.artefact_id
             WHERE c.author = 'studio' AND a.kind = ?1",
            [kind_of(id)],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let then_changed: i64 = conn
        .query_row(
            "SELECT count(*) FROM artefact_changes c
             JOIN artefacts a ON a.id = c.artefact_id
             WHERE c.author = 'user' AND a.kind = ?1",
            [kind_of(id)],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let kept_as_is = if by_studio == 0 {
        Figure::unavailable()
    } else {
        let kept = (by_studio - then_changed.min(by_studio)) * 100 / by_studio;
        Figure::plain(format!("{kept}%"))
    };

    let learned: i64 = conn
        .query_row(
            // What applies to this specialist: notes about its kind of Artefact, plus the ones
            // the User meant for everything. Only active, confirmed notes count — nothing
            // derived acts before the User has accepted it (Property 33).
            "SELECT count(*) FROM steering_notes
             WHERE active = 1 AND confirmed = 1 AND scope IN (?1, 'everything')",
            [scope_of(id)],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(Standing {
        id: id.to_string(),
        finished: Figure::plain(finished.to_string()),
        typical_wait,
        kept_as_is,
        learned: Figure::plain(learned.to_string()),
    })
}

/// The steering scope for a specialist. The store says "deck" where the specialist is called
/// "presentation", and a mismatch here would silently count nothing — which is how the gate
/// scope and the allocation key drifted apart once already.
fn scope_of(specialist: &str) -> &'static str {
    match specialist {
        "spreadsheet" => "spreadsheet",
        "presentation" => "deck",
        _ => "document",
    }
}

/// The Artefact kind a specialist works on, as the store spells it.
///
/// "presentation" is the specialist; "deck" is the kind. Writing the specialist's word here
/// would match no rows and report an honest-looking zero, which is the same failure as the gate
/// scope that said "worksheet" against an allocation key that said "spreadsheet".
fn kind_of(specialist: &str) -> &'static str {
    match specialist {
        "spreadsheet" => "spreadsheet",
        "presentation" => "deck",
        _ => "document",
    }
}

/// A duration in the User's words. Nobody thinks in milliseconds about their own waiting.
fn in_words(seconds: i64) -> String {
    match seconds {
        s if s < 2 => "under a second".to_string(),
        s if s < 60 => format!("about {s} seconds"),
        s if s < 120 => "about a minute".to_string(),
        s => format!("about {} minutes", s / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        let mut store = Store::open_in_memory().unwrap();
        store.migrate().unwrap();
        store
    }

    fn a_run(store: &Store, specialist: &str, waited: i64) {
        let id = format!("run-{specialist}-{waited}-{}", crate::keeper::new_id("x"));
        store
            .conn()
            .execute(
                "INSERT INTO jobs (id, kind, purpose, state, timezone, created_at, updated_at)
                 VALUES (?1, 'one_off', 'x', 'active', 'UTC', unixepoch(), unixepoch())
                 ON CONFLICT DO NOTHING",
                [specialist],
            )
            .unwrap();
        store
            .conn()
            .execute(
                "INSERT INTO job_runs
                   (id, job_id, mode, started_at, finished_at, outcome, specialist)
                 VALUES (?1, ?2, 'manual', unixepoch() - ?3, unixepoch(), 'completed', ?4)",
                rusqlite::params![id, specialist, waited, specialist],
            )
            .unwrap();
    }

    /// The whole point: a specialist that has done nothing says so, rather than showing 72%.
    #[test]
    fn a_specialist_with_no_history_reports_no_figures() {
        let store = store();
        let all = standings(&store).unwrap();
        assert_eq!(all.len(), 3);
        for one in &all {
            assert_eq!(one.finished.value, "0", "a count of runs is genuinely zero");
            assert!(!one.typical_wait.known, "{}: wait must be unknown", one.id);
            assert!(
                !one.kept_as_is.known,
                "{}: acceptance must be unknown",
                one.id
            );
        }
    }

    #[test]
    fn the_typical_wait_is_the_middle_one_not_the_average() {
        let store = store();
        for waited in [3, 4, 5, 6, 90] {
            a_run(&store, "spreadsheet", waited);
        }
        let sheet = standings(&store)
            .unwrap()
            .into_iter()
            .find(|s| s.id == "spreadsheet")
            .unwrap();
        assert_eq!(sheet.finished.value, "5");
        // The mean would be 21 seconds, which describes nobody's experience here.
        assert_eq!(sheet.typical_wait.value, "about 5 seconds");
        assert!(sheet.typical_wait.known);
    }

    #[test]
    fn one_specialists_work_is_not_counted_for_another() {
        let store = store();
        a_run(&store, "document", 7);
        let all = standings(&store).unwrap();
        let doc = all.iter().find(|s| s.id == "document").unwrap();
        let deck = all.iter().find(|s| s.id == "presentation").unwrap();
        assert_eq!(doc.finished.value, "1");
        assert_eq!(deck.finished.value, "0");
        assert!(!deck.typical_wait.known);
    }

    #[test]
    fn a_wait_is_said_in_words_a_person_uses() {
        assert_eq!(in_words(0), "under a second");
        assert_eq!(in_words(45), "about 45 seconds");
        assert_eq!(in_words(75), "about a minute");
        assert_eq!(in_words(300), "about 5 minutes");
    }

    /// A name the schema will not accept matches nothing and reports zero, which reads as
    /// evidence that the specialist did no work. So the names are checked against the database
    /// itself rather than against this file.
    #[test]
    fn the_kind_and_scope_names_are_ones_the_store_accepts() {
        let store = store();
        for specialist in ["spreadsheet", "document", "presentation"] {
            let kind = kind_of(specialist);
            store
                .conn()
                .execute(
                    "INSERT INTO artefacts
                       (id, kind, file_path, display_name, last_author, content_hash, mtime,
                        created_at, updated_at)
                     VALUES (?1, ?2, ?3, 'x', 'studio', 'h', 0, unixepoch(), unixepoch())",
                    rusqlite::params![
                        format!("art-{specialist}"),
                        kind,
                        format!("/tmp/{specialist}")
                    ],
                )
                .unwrap_or_else(|e| panic!("{specialist}: the store rejects kind {kind}: {e}"));

            let scope = scope_of(specialist);
            store
                .conn()
                .execute(
                    "INSERT INTO steering_notes (id, scope, note, origin, seq, created_at)
                     VALUES (?1, ?2, 'x', 'explicit', 1, unixepoch())",
                    rusqlite::params![format!("note-{specialist}"), scope],
                )
                .unwrap_or_else(|e| panic!("{specialist}: the store rejects scope {scope}: {e}"));
        }
    }
}
