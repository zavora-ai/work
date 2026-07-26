//! The confirmation adapter.
//!
//! ADK-Rust asks a handler to approve or deny each tool call it is configured to
//! confirm. Work Studio answers with the side-effect gate, so there is exactly one
//! place in the product where an external action is authorised, whether the request
//! came from a proactive execution or from the User's own document work.
//!
//! The adapter is deliberately thin, and the decision logic below it does not
//! depend on ADK at all. That keeps the invariants in [`studio_gate`] testable
//! without the Capability_Layer present, and it means a different runtime could be
//! substituted without touching the rules.
//!
//! One detail the ADK contract forces: a confirmation request carries a tool name
//! and its arguments, but no server name. Work Studio therefore resolves a tool
//! name to a classification in [`Resolver`], which accepts both `server/tool` and a
//! bare tool name, and falls back to treating the operation as externally visible
//! when it recognises neither.

use std::sync::Mutex;

use studio_gate::{
    Classifier, Decision, IntendedActionManifest, RunMode, SideEffect, record_intent,
};
use studio_jobs::{JobKind, JobState};

/// Approve or deny, in Work Studio's own terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    /// Refused, and the intent recorded for the User to review.
    Refuse,
}

/// Resolves a tool name to the server that owns it.
///
/// Names arrive either fully qualified (`email/send_email`) or bare
/// (`send_email`), depending on how the toolset was mounted.
#[derive(Debug, Clone, Default)]
pub struct Resolver {
    bare: std::collections::HashMap<String, String>,
}

impl Resolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare that `tool` belongs to `server`.
    pub fn declare(&mut self, server: &str, tool: &str) -> &mut Self {
        self.bare.insert(tool.to_string(), server.to_string());
        self
    }

    /// Split a possibly-qualified name into `(server, tool)`.
    pub fn split<'n>(&'n self, name: &'n str) -> Option<(&'n str, &'n str)> {
        if let Some((server, tool)) = name.split_once('/') {
            return Some((server, tool));
        }
        self.bare.get(name).map(|s| (s.as_str(), name))
    }
}

/// What the gate needs to know about the run asking permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunContext {
    pub kind: JobKind,
    pub state: JobState,
    pub mode: RunMode,
}

/// The gate, wearing the shape a runtime expects.
///
/// Holds the manifest of everything it refused, which becomes the first-draft
/// review for a Job whose output is a set of actions.
#[derive(Debug)]
pub struct GateHandler {
    classifier: Classifier,
    resolver: Resolver,
    context: RunContext,
    refused: Mutex<IntendedActionManifest>,
    permitted: Mutex<Vec<String>>,
}

impl GateHandler {
    pub fn new(classifier: Classifier, resolver: Resolver, context: RunContext) -> Self {
        Self {
            classifier,
            resolver,
            context,
            refused: Mutex::new(IntendedActionManifest::default()),
            permitted: Mutex::new(Vec::new()),
        }
    }

    /// Decide one tool call. This is the whole of the adapter's judgement.
    pub fn judge(&self, tool_name: &str, description: &str, affected: u32) -> Verdict {
        let (server, tool) = match self.resolver.split(tool_name) {
            Some(pair) => pair,
            // A name we cannot even attribute is treated as externally visible.
            None => ("unknown", tool_name),
        };

        let decision = studio_gate::decide(
            &self.classifier,
            server,
            tool,
            self.context.kind,
            self.context.state,
            self.context.mode,
            // Faithfully parsed from configuration, and never consulted.
            true,
        );

        match decision {
            Decision::Suppress { .. } => {
                let class = self.classifier.get(server, tool);
                if let Some(class) = class {
                    let mut refused = self.refused.lock().expect("manifest lock");
                    record_intent(&mut refused, class, description, affected);
                } else {
                    // Unclassified: still shown to the User, described plainly.
                    let mut refused = self.refused.lock().expect("manifest lock");
                    refused.push(studio_gate::IntendedAction {
                        verb: "Do something",
                        description: description.to_string(),
                        affected,
                        reversibility: studio_gate::Reversibility::Irreversible {
                            reason: "I don't know how to take this back".into(),
                        },
                        excluded: false,
                    });
                }
                Verdict::Refuse
            }
            permitted => {
                if !matches!(permitted, Decision::Permit) {
                    self.permitted
                        .lock()
                        .expect("permitted lock")
                        .push(tool_name.to_string());
                }
                Verdict::Allow
            }
        }
    }

    /// What this run wanted to do and was not allowed to.
    pub fn manifest(&self) -> IntendedActionManifest {
        self.refused.lock().expect("manifest lock").clone()
    }

    /// Operations that changed something, for the change log and deliveries.
    pub fn recorded(&self) -> Vec<String> {
        self.permitted.lock().expect("permitted lock").clone()
    }

    /// Whether this run performed anything visible outside the computer.
    pub fn performed_external_effect(&self) -> bool {
        self.recorded()
            .iter()
            .any(|name| match self.resolver.split(name) {
                Some((server, tool)) => {
                    self.classifier.effect_of(server, tool) == SideEffect::ExternalEffect
                }
                None => true,
            })
    }
}

/// Implemented against ADK-Rust when the Capability_Layer is present.
///
/// Enabled by the `adk` feature. The decision itself is [`GateHandler::judge`],
/// which is why this block contains no rules.
#[cfg(feature = "adk")]
mod adk_impl {
    use adk_core::context::{
        ToolConfirmationDecision, ToolConfirmationHandler, ToolConfirmationRequest,
    };

    use super::{GateHandler, Verdict};

    #[async_trait::async_trait]
    impl ToolConfirmationHandler for GateHandler {
        async fn decide(
            &self,
            request: &ToolConfirmationRequest,
        ) -> adk_core::Result<ToolConfirmationDecision> {
            // The arguments are the only description available at this point, so
            // they are summarised rather than shown raw: the User reads plain
            // language, never a payload.
            let description = summarise(&request.args);
            let verdict = self.judge(&request.tool_name, &description, 1);
            Ok(match verdict {
                Verdict::Allow => ToolConfirmationDecision::Approve,
                Verdict::Refuse => ToolConfirmationDecision::Deny,
            })
        }
    }

    /// A one-line, User-readable summary of a tool's arguments.
    fn summarise(args: &serde_json::Value) -> String {
        match args {
            serde_json::Value::Object(map) => {
                let mut parts: Vec<String> = map
                    .iter()
                    .filter_map(|(k, v)| match v {
                        serde_json::Value::String(s) if !s.is_empty() => {
                            Some(format!("{k}: {}", truncate(s, 60)))
                        }
                        serde_json::Value::Number(n) => Some(format!("{k}: {n}")),
                        _ => None,
                    })
                    .collect();
                parts.sort();
                if parts.is_empty() {
                    "an action with no details".to_string()
                } else {
                    parts.join(", ")
                }
            }
            other => truncate(&other.to_string(), 80),
        }
    }

    fn truncate(s: &str, max: usize) -> String {
        if s.chars().count() <= max {
            s.to_string()
        } else {
            let kept: String = s.chars().take(max).collect();
            format!("{kept}…")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_gate::{OperationClass, Reversibility};

    fn classifier() -> Classifier {
        let mut c = Classifier::new();
        c.insert(
            "email",
            "list_inbox",
            OperationClass {
                effect: SideEffect::Read,
                verb: "Read",
                reversibility: Reversibility::Irreversible {
                    reason: "reading changes nothing".into(),
                },
            },
        );
        c.insert(
            "email",
            "send_email",
            OperationClass {
                effect: SideEffect::ExternalEffect,
                verb: "Send",
                reversibility: Reversibility::Irreversible {
                    reason: "a sent message can't be unsent".into(),
                },
            },
        );
        c.insert(
            "worksheet",
            "set_cell",
            OperationClass {
                effect: SideEffect::LocalWrite,
                verb: "Change",
                reversibility: Reversibility::Reversible {
                    how: "Undo".into(),
                    window_secs: None,
                },
            },
        );
        c
    }

    fn resolver() -> Resolver {
        let mut r = Resolver::new();
        r.declare("email", "list_inbox");
        r.declare("email", "send_email");
        r.declare("worksheet", "set_cell");
        r
    }

    fn handler(state: JobState, mode: RunMode) -> GateHandler {
        GateHandler::new(
            classifier(),
            resolver(),
            RunContext {
                kind: JobKind::Scheduled,
                state,
                mode,
            },
        )
    }

    #[test]
    fn a_bare_or_qualified_tool_name_both_resolve() {
        let r = resolver();
        assert_eq!(r.split("send_email"), Some(("email", "send_email")));
        assert_eq!(r.split("email/send_email"), Some(("email", "send_email")));
        assert_eq!(r.split("who_knows"), None);
    }

    #[test]
    fn a_dry_run_refuses_every_external_call_and_records_it() {
        let h = handler(JobState::Draft, RunMode::KickoffDryRun);
        assert_eq!(
            h.judge("list_inbox", "read 42 messages", 42),
            Verdict::Allow
        );
        assert_eq!(h.judge("send_email", "4 replies", 4), Verdict::Refuse);
        assert!(
            !h.performed_external_effect(),
            "nothing external may happen"
        );

        let manifest = h.manifest();
        assert_eq!(
            manifest.rows.len(),
            1,
            "the refusal must be held for review"
        );
        assert_eq!(manifest.rows[0].verb, "Send");
        assert_eq!(manifest.rows[0].affected, 4);
    }

    #[test]
    fn a_live_job_may_act_and_the_action_is_recorded() {
        let h = handler(JobState::Live, RunMode::Live);
        assert_eq!(
            h.judge("send_email", "the morning digest", 1),
            Verdict::Allow
        );
        assert!(h.manifest().rows.is_empty());
        assert!(h.performed_external_effect());
        assert_eq!(h.recorded(), vec!["send_email"]);
    }

    #[test]
    fn a_local_write_is_recorded_but_is_not_an_external_effect() {
        let h = handler(JobState::Live, RunMode::Live);
        assert_eq!(h.judge("set_cell", "D7 = C7*1.12", 1), Verdict::Allow);
        assert_eq!(h.recorded(), vec!["set_cell"]);
        assert!(
            !h.performed_external_effect(),
            "editing the User's own file is not an external effect"
        );
    }

    #[test]
    fn a_read_is_not_recorded_at_all() {
        let h = handler(JobState::Live, RunMode::Live);
        h.judge("list_inbox", "read the inbox", 42);
        assert!(h.recorded().is_empty(), "reads leave no trace to undo");
    }

    /// An unrecognised tool is refused and still described to the User.
    #[test]
    fn an_unknown_tool_is_refused_and_shown_honestly() {
        let h = handler(JobState::Live, RunMode::Live);
        assert_eq!(
            h.judge("wipe_everything", "delete 400 messages", 400),
            Verdict::Refuse
        );
        let manifest = h.manifest();
        assert_eq!(manifest.rows.len(), 1);
        assert!(
            matches!(
                manifest.rows[0].reversibility,
                studio_gate::Reversibility::Irreversible { .. }
            ),
            "if we cannot classify it we must not promise reversal"
        );
    }

    /// The adapter passes `auto_approve = true` on every call, so this proves
    /// Correctness Property 2 holds through the adapter as well as under it.
    #[test]
    fn the_adapter_cannot_be_talked_into_permission_by_configuration() {
        for state in [JobState::Draft, JobState::AwaitingKickoff, JobState::Paused] {
            let h = handler(state, RunMode::Live);
            assert_eq!(
                h.judge("send_email", "a reply", 1),
                Verdict::Refuse,
                "{state} must not be able to send"
            );
        }
    }

    #[test]
    fn the_manifest_accumulates_across_a_whole_run() {
        let h = handler(JobState::Draft, RunMode::KickoffDryRun);
        h.judge("send_email", "4 replies", 4);
        h.judge("wipe_everything", "archive 18 newsletters", 18);
        h.judge("list_inbox", "read 42 messages", 42);
        let manifest = h.manifest();
        assert_eq!(manifest.rows.len(), 2, "only refusals are held");
        assert_eq!(manifest.retained().count(), 2);
    }
}
