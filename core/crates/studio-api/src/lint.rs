//! The renderer payload guardrail (task 1.7, Correctness Property 12).
//!
//! Requirement 3.4 lists what the renderer may not know. This module asserts it
//! structurally: any payload crossing the loopback channel is serialised and both
//! its field names and its string values are scanned for identifiers the interface
//! must never receive.
//!
//! Scanning values as well as keys matters. A well-named field carrying
//! `"gpt-5-mini"` leaks just as effectively as a field called `model`.

use serde_json::Value;

use crate::Payload;

/// Field names the renderer must never receive.
pub const FORBIDDEN_KEYS: &[&str] = &[
    "agent",
    "agentId",
    "agentName",
    "model",
    "modelId",
    "provider",
    "tier",
    "qualityTier",
    "server",
    "serverName",
    "tool",
    "toolName",
    "toolCalls",
    "tokens",
    "inputTokens",
    "outputTokens",
    "prompt",
    "session",
    "sessionId",
    "runCount",
    "runs",
    "checkpoint",
    "trace",
    "spanId",
    "cron",
    "scheduleCron",
    "mcp",
];

/// Value fragments that betray a leak regardless of the field's name.
pub const FORBIDDEN_VALUE_FRAGMENTS: &[&str] = &[
    "gpt-",
    "claude-",
    "gemini-",
    "openai",
    "anthropic",
    "mcp",
    "stdio",
    "tool_call",
    "toolcall",
    "localhost:",
    "127.0.0.1",
    "bearer ",
    "sk-",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Leak {
    Key {
        path: String,
        key: String,
    },
    Value {
        path: String,
        fragment: String,
        found: String,
    },
}

impl std::fmt::Display for Leak {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Key { path, key } => {
                write!(f, "{path}: field \"{key}\" must not reach the renderer")
            }
            Self::Value {
                path,
                fragment,
                found,
            } => write!(
                f,
                "{path}: value {found:?} contains {fragment:?}, which must not reach the renderer"
            ),
        }
    }
}

/// Scan an already-serialised payload.
pub fn scan(value: &Value, path: &str, out: &mut Vec<Leak>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let child = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                if FORBIDDEN_KEYS.iter().any(|f| f.eq_ignore_ascii_case(k)) {
                    out.push(Leak::Key {
                        path: child.clone(),
                        key: k.clone(),
                    });
                }
                scan(v, &child, out);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                scan(v, &format!("{path}[{i}]"), out);
            }
        }
        Value::String(s) => {
            let lower = s.to_ascii_lowercase();
            for frag in FORBIDDEN_VALUE_FRAGMENTS {
                if lower.contains(frag) {
                    out.push(Leak::Value {
                        path: path.to_string(),
                        fragment: (*frag).to_string(),
                        found: s.clone(),
                    });
                }
            }
        }
        _ => {}
    }
}

/// Check one payload. Diagnostic payloads are exempt by design.
pub fn check<P: Payload>(payload: &P) -> Vec<Leak> {
    if P::is_diagnostic() {
        return Vec::new();
    }
    let value = serde_json::to_value(payload).expect("payload serialises");
    let mut out = Vec::new();
    scan(&value, "", &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn sample_job() -> JobView {
        JobView {
            id: "j1".into(),
            purpose: "Daily newsletter".into(),
            badge: StateBadge::Scheduled,
            schedule_human: Some("Every weekday at 7:00 am".into()),
            next_human: Some("Tomorrow, 7:00 am".into()),
            last_outcome: Some("Sent your Monday brief — 3 sources".into()),
            status_detail: "Next tomorrow, 7:00 am".into(),
            spend_today: Some("$0.04".into()),
            steering: vec![SteeringView {
                id: "s1".into(),
                note: "Keep it under 400 words".into(),
                provenance: "You shortened Friday's draft · 2 days ago".into(),
                scope_label: None,
                awaiting_confirmation: false,
            }],
        }
    }

    /// Correctness Property 12: renderer concept containment.
    #[test]
    fn property_12_no_payload_leaks_a_technical_identifier() {
        let leaks = check(&sample_job());
        assert!(leaks.is_empty(), "JobView leaked: {leaks:?}");

        let leaks = check(&TrayItemView {
            id: "t1".into(),
            class: TrayClass::Finding,
            headline: "Your startup disk is 94% full".into(),
            detail: "Nothing is broken yet. 18 GB sits in Downloads.".into(),
            job_purpose: "Computer health".into(),
            choices: vec!["See what's big".into(), "Got it".into()],
            created_human: "10 minutes ago".into(),
        });
        assert!(leaks.is_empty(), "TrayItemView leaked: {leaks:?}");

        let leaks = check(&DeliveryView {
            id: "d1".into(),
            action: "Posted to X: product update thread".into(),
            when_human: "9:02 am".into(),
            job_purpose: "Social posting".into(),
            reversal: ReversalView::Available {
                label: "Take it down".into(),
                expires_human: Some("for 30 days".into()),
            },
        });
        assert!(leaks.is_empty(), "DeliveryView leaked: {leaks:?}");

        let leaks = check(&FileView {
            id: "f1".into(),
            name: "Board deck — July.pptx".into(),
            kind: ArtefactKind::Deck,
            changed_human: "21:04".into(),
            changed_by: "by me".into(),
            made_from: Some("Q3 revenue model.xlsx".into()),
            used_in: vec!["Board deck — July".into()],
            versions: 4,
        });
        assert!(leaks.is_empty(), "FileView leaked: {leaks:?}");

        let leaks = check(&ChangeView {
            seq: 3,
            author: "me".into(),
            description: "Slide 5 — chart added and resized".into(),
            when_human: "21:04".into(),
            can_undo: true,
        });
        assert!(leaks.is_empty(), "ChangeView leaked: {leaks:?}");
    }

    #[test]
    fn a_forbidden_field_name_is_caught() {
        let payload = serde_json::json!({
            "purpose": "Daily newsletter",
            "model": "gpt-5-mini"
        });
        let mut out = Vec::new();
        scan(&payload, "", &mut out);
        assert!(
            out.iter()
                .any(|l| matches!(l, Leak::Key { key, .. } if key == "model")),
            "the field name must be caught: {out:?}"
        );
    }

    /// A well-named field carrying a technical value leaks just as effectively.
    #[test]
    fn a_forbidden_value_is_caught_even_under_an_innocent_field_name() {
        let payload = serde_json::json!({
            "lastOutcome": "Finished using gpt-5-mini in 4.2s"
        });
        let mut out = Vec::new();
        scan(&payload, "", &mut out);
        assert!(
            out.iter()
                .any(|l| matches!(l, Leak::Value { fragment, .. } if fragment == "gpt-")),
            "the value must be caught: {out:?}"
        );
    }

    #[test]
    fn nested_and_arrayed_leaks_are_found_with_a_path() {
        let payload = serde_json::json!({
            "steering": [
                { "note": "fine" },
                { "note": "call the MCP server first" }
            ]
        });
        let mut out = Vec::new();
        scan(&payload, "", &mut out);
        assert!(
            out.iter()
                .any(|l| matches!(l, Leak::Value { path, .. } if path == "steering[1].note")),
            "the path should locate the leak: {out:?}"
        );
    }

    #[test]
    fn a_bearer_token_or_key_never_reaches_the_renderer() {
        for value in ["Bearer abc123", "sk-live-0000"] {
            let payload = serde_json::json!({ "detail": value });
            let mut out = Vec::new();
            scan(&payload, "", &mut out);
            assert!(!out.is_empty(), "{value:?} must be caught");
        }
    }

    /// Requirement 17.5: the diagnostics view exists to hold this.
    #[test]
    fn the_diagnostics_payload_is_exempt() {
        let leaks = check(&DiagnosticsPayload {
            model: "gpt-5-mini".into(),
            provider: "openai".into(),
            tool_calls: 12,
            tokens: 4096,
            last_error: Some("stdio transport closed".into()),
        });
        assert!(
            leaks.is_empty(),
            "the diagnostics payload carries technical detail by design"
        );
    }

    #[test]
    fn the_diagnostics_payload_would_otherwise_be_rejected() {
        let value = serde_json::to_value(DiagnosticsPayload {
            model: "gpt-5-mini".into(),
            provider: "openai".into(),
            tool_calls: 12,
            tokens: 4096,
            last_error: None,
        })
        .unwrap();
        let mut out = Vec::new();
        scan(&value, "", &mut out);
        assert!(
            !out.is_empty(),
            "the exemption must be doing real work, not covering an already-clean payload"
        );
    }
}
