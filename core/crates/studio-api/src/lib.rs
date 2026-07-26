//! What the renderer is allowed to know.
//!
//! Every payload crossing the loopback channel is one of the types in this crate.
//! They are the enforcement point for Requirement 3.4: a field that does not exist
//! here cannot be displayed, so the interface cannot drift into showing agent,
//! model, server or tool detail even by accident.
//!
//! The single exception is [`DiagnosticsPayload`], which exists precisely to carry
//! technical detail to the one view that is allowed to hold it (Requirement 17.5).
//! It is marked as diagnostic and the payload guardrail skips it.

use serde::{Deserialize, Serialize};

pub mod lint;

/// Marks a payload as diagnostic, exempting it from the guardrail.
pub trait Payload: Serialize {
    /// Diagnostic payloads may carry technical detail. Everything else may not.
    fn is_diagnostic() -> bool {
        false
    }
}

// ---------------------------------------------------------------- jobs

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StateBadge {
    /// Doing something right now.
    Working,
    /// Waiting for its time to come round.
    Scheduled,
    /// Waiting on the User.
    NeedsYou,
    Finished,
    Paused,
}

/// Everything the renderer may know about one piece of work.
///
/// Deliberately carries no agent, model, tier, server, tool, run count or token
/// count (Requirement 3.4). `spendToday` is a formatted currency string, never a
/// token count (Requirement 15.6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub id: String,
    /// The User's own words.
    pub purpose: String,
    pub badge: StateBadge,
    /// "Every weekday at 7:00 am" — never a cron expression.
    pub schedule_human: Option<String>,
    /// "Tomorrow, 7:00 am" — in the User's own time zone.
    pub next_human: Option<String>,
    /// One plain sentence.
    pub last_outcome: Option<String>,
    /// The concrete fact behind the badge, for hover, focus and accessible name.
    pub status_detail: String,
    /// Formatted currency, e.g. "$0.62" or "4p a day".
    pub spend_today: Option<String>,
    pub steering: Vec<SteeringView>,
}

impl Payload for JobView {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SteeringView {
    pub id: String,
    /// The User's own words.
    pub note: String,
    /// "You shortened Friday's draft · 2 days ago"
    pub provenance: String,
    /// "Everything" | "Documents" | "Decks" | "Spreadsheets" — absent for
    /// per-thread notes.
    pub scope_label: Option<String>,
    /// A derived note is not applied until the User confirms it.
    pub awaiting_confirmation: bool,
}

impl Payload for SteeringView {}

// ---------------------------------------------------------------- trays

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TrayClass {
    /// A first draft to check.
    Kickoff,
    /// Needs the User's call.
    Escalation,
    /// Worth knowing. Nothing is broken.
    Finding,
    /// Needs fixing.
    Attention,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrayItemView {
    pub id: String,
    pub class: TrayClass,
    pub headline: String,
    pub detail: String,
    /// Which piece of work raised it, by its User-facing purpose.
    pub job_purpose: String,
    pub choices: Vec<String>,
    pub created_human: String,
}

impl Payload for TrayItemView {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReversalView {
    /// "Take it down" / "Undo"
    Available {
        label: String,
        expires_human: Option<String>,
    },
    /// "Can't be unsent"
    Unavailable { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryView {
    pub id: String,
    /// "Posted to X: product update thread"
    pub action: String,
    pub when_human: String,
    pub job_purpose: String,
    pub reversal: ReversalView,
}

impl Payload for DeliveryView {}

// ---------------------------------------------------------------- artefacts

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ArtefactKind {
    Document,
    Deck,
    Spreadsheet,
    Pdf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    pub id: String,
    /// "Board deck — July.pptx"
    pub name: String,
    pub kind: ArtefactKind,
    /// "21:04" / "2 days ago"
    pub changed_human: String,
    /// "by you, in Word" / "by me"
    pub changed_by: String,
    /// "Made from Q3 revenue model.xlsx"
    pub made_from: Option<String>,
    /// The pieces of work that touched this file, by purpose.
    pub used_in: Vec<String>,
    pub versions: u32,
}

impl Payload for FileView {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeView {
    pub seq: u32,
    /// "you" or "me" — never an agent identifier.
    pub author: String,
    /// "Slide 5 — chart added and resized"
    pub description: String,
    pub when_human: String,
    pub can_undo: bool,
}

impl Payload for ChangeView {}

// ---------------------------------------------------------------- diagnostics

/// The one payload permitted to carry technical detail, for the single
/// diagnostics view reachable from Settings (Requirement 17.5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsPayload {
    pub model: String,
    pub provider: String,
    pub tool_calls: u32,
    pub tokens: u64,
    pub last_error: Option<String>,
}

impl Payload for DiagnosticsPayload {
    fn is_diagnostic() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_view_round_trips() {
        let v = JobView {
            id: "j1".into(),
            purpose: "Daily newsletter".into(),
            badge: StateBadge::Scheduled,
            schedule_human: Some("Every weekday at 7:00 am".into()),
            next_human: Some("Tomorrow, 7:00 am".into()),
            last_outcome: Some("Sent your Monday brief".into()),
            status_detail: "Next tomorrow, 7:00 am".into(),
            spend_today: Some("$0.04".into()),
            steering: vec![],
        };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(serde_json::from_str::<JobView>(&json).unwrap(), v);
    }

    #[test]
    fn job_view_has_no_technical_field() {
        let json = serde_json::to_value(JobView {
            id: "j1".into(),
            purpose: "x".into(),
            badge: StateBadge::Working,
            schedule_human: None,
            next_human: None,
            last_outcome: None,
            status_detail: "Checking now".into(),
            spend_today: None,
            steering: vec![],
        })
        .unwrap();
        let keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
        for forbidden in [
            "model", "provider", "agent", "tier", "tools", "tokens", "runs",
        ] {
            assert!(
                !keys.iter().any(|k| k == forbidden),
                "JobView must not expose {forbidden}; has {keys:?}"
            );
        }
    }
}
