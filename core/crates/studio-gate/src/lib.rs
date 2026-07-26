//! The side-effect gate.
//!
//! Every operation a Job wants to perform passes through here. This is the single
//! enforcement point for three requirements that the product's whole trust model
//! rests on:
//!
//! * **5.2** — while a Job awaits its first review, nothing externally visible
//!   happens on the User's behalf.
//! * **18.3** — no externally visible action occurs without either a Kickoff
//!   approval or a `live` Job authorisation.
//! * **18.4** — an `autoApprove` flag declared in Capability_Layer configuration
//!   is never treated as authorisation.
//!
//! Two design choices are deliberate and load-bearing.
//!
//! The classification of every operation is **authored here**, not read from
//! server-declared metadata. A server that mislabels its own destructive
//! operation as harmless must not be able to widen what Work Studio will do.
//!
//! Suppressed operations are not discarded. They are collected into an
//! [`IntendedActionManifest`], which is what the User reviews for a Job whose
//! output is a set of actions rather than a document (Requirement 5.8).

use std::collections::HashMap;

use studio_jobs::{JobKind, JobState};

pub mod catalogue;
pub mod catalogue_docs;
pub mod catalogue_slides;

/// What an operation does to the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideEffect {
    /// Reads only. Never gated.
    Read,
    /// Writes to a local Artefact. Permitted, and recorded in the change log.
    LocalWrite,
    /// Visible outside this computer: sends, posts, deletes, files, invites.
    ExternalEffect,
}

/// Whether an action can be taken back, and for how long.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reversibility {
    Reversible {
        /// "Take it down", "Undo"
        how: String,
        /// None means no expiry.
        window_secs: Option<u64>,
    },
    Partial {
        limits: String,
        window_secs: Option<u64>,
    },
    Irreversible {
        reason: String,
    },
}

impl Reversibility {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Reversible { .. } => "reversible",
            Self::Partial { .. } => "partial",
            Self::Irreversible { .. } => "irreversible",
        }
    }

    /// Correctness Property 17: an irreversible action never claims a window.
    pub fn window_secs(&self) -> Option<u64> {
        match self {
            Self::Reversible { window_secs, .. } | Self::Partial { window_secs, .. } => {
                *window_secs
            }
            Self::Irreversible { .. } => None,
        }
    }
}

/// How an operation is classified, and how it reads to the User.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationClass {
    pub effect: SideEffect,
    /// "Archive", "Label", "Draft", "Post to X"
    pub verb: &'static str,
    pub reversibility: Reversibility,
}

/// Which run this operation belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// The one-time first pass. Nothing externally visible may happen.
    KickoffDryRun,
    /// A scheduled execution of a `live` Job.
    Live,
    /// The User pressed run now.
    Manual,
}

/// What the gate decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Go ahead. Nothing to record beyond the usual.
    Permit,
    /// Go ahead, and record it in the Artefact change log.
    PermitAndRecord,
    /// Go ahead, and record a Delivery the User can see and possibly reverse.
    PermitAndDeliver { reversibility: Reversibility },
    /// Do not perform it. Collect it for the User to review instead.
    Suppress { reason: SuppressReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressReason {
    /// This is the first pass and the User has not approved anything yet.
    AwaitingFirstReview,
    /// The Job is not in a state that authorises acting on the User's behalf.
    NotAuthorised,
    /// Nobody has classified this operation, so Work Studio can neither describe
    /// what it would do nor offer to undo it.
    ///
    /// Suppressed in every state, including a `live` Job. Design principle 4 says
    /// autonomy requires reversibility, and an action we cannot explain or reverse
    /// has neither property. The User is asked instead.
    Unclassified,
}

impl Decision {
    pub fn is_permitted(&self) -> bool {
        !matches!(self, Self::Suppress { .. })
    }
}

/// One thing the Job would have done, held for review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntendedAction {
    pub verb: &'static str,
    /// Plain language, including the count. "18 newsletters you've never opened"
    pub description: String,
    pub affected: u32,
    pub reversibility: Reversibility,
    /// Excluded by the User during review (Requirement 5.9).
    pub excluded: bool,
}

/// The reviewable form of a Job whose output is actions rather than a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntendedActionManifest {
    pub rows: Vec<IntendedAction>,
}

impl IntendedActionManifest {
    pub fn push(&mut self, action: IntendedAction) {
        self.rows.push(action);
    }

    /// What would actually be performed if the User approved as-is.
    pub fn retained(&self) -> impl Iterator<Item = &IntendedAction> {
        self.rows.iter().filter(|r| !r.excluded)
    }

    pub fn exclude(&mut self, index: usize) -> bool {
        match self.rows.get_mut(index) {
            Some(row) => {
                row.excluded = true;
                true
            }
            None => false,
        }
    }

    /// A single sentence covering the whole manifest, for the review footer.
    pub fn reversal_summary(&self) -> String {
        let any_irreversible = self
            .rows
            .iter()
            .any(|r| matches!(r.reversibility, Reversibility::Irreversible { .. }));
        if any_irreversible {
            "Some of this can't be undone — it's marked below.".to_string()
        } else {
            "Everything here can be undone from Done for you.".to_string()
        }
    }
}

/// The authored classification table. Keyed by `(server, operation)`.
///
/// Nothing is classified by asking the server. A server that mislabels its own
/// destructive operation cannot widen what Work Studio will do.
#[derive(Debug, Clone, Default)]
pub struct Classifier {
    table: HashMap<(String, String), OperationClass>,
}

impl Classifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, server: &str, operation: &str, class: OperationClass) -> &mut Self {
        self.table
            .insert((server.to_string(), operation.to_string()), class);
        self
    }

    pub fn get(&self, server: &str, operation: &str) -> Option<&OperationClass> {
        self.table.get(&(server.to_string(), operation.to_string()))
    }

    /// An unclassified operation is treated as externally visible.
    ///
    /// This is the safe default: a new operation nobody has classified yet is
    /// gated rather than waved through.
    pub fn effect_of(&self, server: &str, operation: &str) -> SideEffect {
        self.get(server, operation)
            .map(|c| c.effect)
            .unwrap_or(SideEffect::ExternalEffect)
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }
}

/// Whether this Job, in this state, is authorised to act outside the computer.
fn authorised_to_act(kind: JobKind, state: JobState) -> bool {
    match (kind, state) {
        // A scheduled Job acts once the User has approved its first output.
        (JobKind::Scheduled, JobState::Live) => true,
        // A one-off Job is the User directing work in the moment.
        (JobKind::OneOff, JobState::Active) => true,
        _ => false,
    }
}

/// The gate.
///
/// `declared_auto_approve` is accepted so that Capability_Layer configuration can
/// be parsed faithfully, and then ignored — see Correctness Property 2.
pub fn decide(
    classifier: &Classifier,
    server: &str,
    operation: &str,
    kind: JobKind,
    state: JobState,
    mode: RunMode,
    declared_auto_approve: bool,
) -> Decision {
    let _ = declared_auto_approve; // never consulted. Requirement 18.4.

    let class = classifier.get(server, operation);
    let effect = class
        .map(|c| c.effect)
        .unwrap_or(SideEffect::ExternalEffect);

    match effect {
        SideEffect::Read => Decision::Permit,
        SideEffect::LocalWrite => Decision::PermitAndRecord,
        SideEffect::ExternalEffect => {
            if mode == RunMode::KickoffDryRun {
                return Decision::Suppress {
                    reason: SuppressReason::AwaitingFirstReview,
                };
            }
            // An operation nobody classified is never performed, however
            // authorised the Job is: we could neither tell the User what it did
            // nor offer to undo it. Design principle 4 — autonomy requires
            // reversibility — and an unknown action has neither property.
            let Some(class) = class else {
                return Decision::Suppress {
                    reason: SuppressReason::Unclassified,
                };
            };
            if !authorised_to_act(kind, state) {
                return Decision::Suppress {
                    reason: SuppressReason::NotAuthorised,
                };
            }
            Decision::PermitAndDeliver {
                reversibility: class.reversibility.clone(),
            }
        }
    }
}

/// A convenience for building the manifest as suppressed operations arrive.
pub fn record_intent(
    manifest: &mut IntendedActionManifest,
    class: &OperationClass,
    description: impl Into<String>,
    affected: u32,
) {
    manifest.push(IntendedAction {
        verb: class.verb,
        description: description.into(),
        affected,
        reversibility: class.reversibility.clone(),
        excluded: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reversible(how: &str, window: Option<u64>) -> Reversibility {
        Reversibility::Reversible {
            how: how.to_string(),
            window_secs: window,
        }
    }

    /// The classification an inbox-triage Job would carry.
    fn triage_classifier() -> Classifier {
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
            "move_to_folder",
            OperationClass {
                effect: SideEffect::ExternalEffect,
                verb: "Archive",
                reversibility: reversible("Move it back", Some(30 * 24 * 3600)),
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
                reversibility: reversible("Undo", None),
            },
        );
        c
    }

    /// Correctness Property 1: no unauthorised external effect.
    ///
    /// For any Job in a dry run, the set of performed external operations is empty.
    #[test]
    fn property_1_a_dry_run_performs_no_external_effect() {
        let c = triage_classifier();
        for (kind, state) in [
            (JobKind::Scheduled, JobState::Draft),
            (JobKind::Scheduled, JobState::AwaitingKickoff),
            (JobKind::Scheduled, JobState::Live),
            (JobKind::OneOff, JobState::Active),
        ] {
            for op in ["move_to_folder", "send_email"] {
                let d = decide(&c, "email", op, kind, state, RunMode::KickoffDryRun, false);
                assert!(
                    !d.is_permitted(),
                    "{kind:?}/{state} dry run permitted {op}: {d:?}"
                );
            }
        }
    }

    /// Requirement 18.3: an external effect requires an authorising state.
    #[test]
    fn an_external_effect_needs_an_authorising_state() {
        let c = triage_classifier();
        let denied = [
            (JobKind::Scheduled, JobState::Draft),
            (JobKind::Scheduled, JobState::AwaitingKickoff),
            (JobKind::Scheduled, JobState::Paused),
            (JobKind::Scheduled, JobState::NeedsAttention),
            (JobKind::OneOff, JobState::Finished),
            (JobKind::OneOff, JobState::NeedsAttention),
        ];
        for (kind, state) in denied {
            let d = decide(
                &c,
                "email",
                "move_to_folder",
                kind,
                state,
                RunMode::Live,
                false,
            );
            assert_eq!(
                d,
                Decision::Suppress {
                    reason: SuppressReason::NotAuthorised
                },
                "{kind:?}/{state} must not act outside this computer"
            );
        }

        for (kind, state) in [
            (JobKind::Scheduled, JobState::Live),
            (JobKind::OneOff, JobState::Active),
        ] {
            let d = decide(
                &c,
                "email",
                "move_to_folder",
                kind,
                state,
                RunMode::Live,
                false,
            );
            assert!(d.is_permitted(), "{kind:?}/{state} should be authorised");
        }
    }

    /// Correctness Property 2: auto-approval is never authorisation.
    ///
    /// The decision is identical with the flag set and unset, in every state, for
    /// every classification.
    #[test]
    fn property_2_auto_approve_never_changes_a_decision() {
        let c = triage_classifier();
        let ops = [
            "list_inbox",
            "move_to_folder",
            "send_email",
            "set_cell",
            "unknown_op",
        ];
        let modes = [RunMode::KickoffDryRun, RunMode::Live, RunMode::Manual];
        let states = [
            (JobKind::Scheduled, JobState::Draft),
            (JobKind::Scheduled, JobState::AwaitingKickoff),
            (JobKind::Scheduled, JobState::Live),
            (JobKind::Scheduled, JobState::Paused),
            (JobKind::Scheduled, JobState::NeedsAttention),
            (JobKind::OneOff, JobState::Active),
            (JobKind::OneOff, JobState::Finished),
        ];
        for op in ops {
            for mode in modes {
                for (kind, state) in states {
                    let server = if op == "set_cell" {
                        "worksheet"
                    } else {
                        "email"
                    };
                    let without = decide(&c, server, op, kind, state, mode, false);
                    let with = decide(&c, server, op, kind, state, mode, true);
                    assert_eq!(
                        without, with,
                        "autoApprove changed the decision for {op} in {kind:?}/{state}/{mode:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn reads_are_never_gated_and_local_writes_are_recorded() {
        let c = triage_classifier();
        assert_eq!(
            decide(
                &c,
                "email",
                "list_inbox",
                JobKind::Scheduled,
                JobState::Draft,
                RunMode::KickoffDryRun,
                false
            ),
            Decision::Permit
        );
        assert_eq!(
            decide(
                &c,
                "worksheet",
                "set_cell",
                JobKind::OneOff,
                JobState::Active,
                RunMode::Manual,
                false
            ),
            Decision::PermitAndRecord
        );
    }

    /// An operation nobody has classified is never performed, in any state.
    ///
    /// Stronger than merely treating it as externally visible: even a `live` Job is
    /// refused, because Work Studio could neither describe the action to the User
    /// nor offer to undo it.
    #[test]
    fn an_unclassified_operation_is_never_performed() {
        let c = triage_classifier();
        assert_eq!(
            c.effect_of("email", "delete_everything"),
            SideEffect::ExternalEffect
        );
        for (kind, state) in [
            (JobKind::Scheduled, JobState::AwaitingKickoff),
            (JobKind::Scheduled, JobState::Live),
            (JobKind::OneOff, JobState::Active),
        ] {
            for mode in [RunMode::KickoffDryRun, RunMode::Live, RunMode::Manual] {
                let d = decide(&c, "email", "delete_everything", kind, state, mode, true);
                assert!(
                    !d.is_permitted(),
                    "{kind:?}/{state}/{mode:?} performed an unclassified operation: {d:?}"
                );
            }
        }
    }

    /// Correctness Property 17: an irreversible action never claims a window.
    #[test]
    fn property_17_irreversible_actions_carry_no_window() {
        let irreversible = Reversibility::Irreversible {
            reason: "a sent message can't be unsent".into(),
        };
        assert_eq!(irreversible.window_secs(), None);
        assert_eq!(irreversible.label(), "irreversible");

        let c = triage_classifier();
        let d = decide(
            &c,
            "email",
            "send_email",
            JobKind::Scheduled,
            JobState::Live,
            RunMode::Live,
            false,
        );
        match d {
            Decision::PermitAndDeliver { reversibility } => {
                assert_eq!(reversibility.window_secs(), None);
            }
            other => panic!("expected a delivery, got {other:?}"),
        }
    }

    /// Correctness Property 20: manifest fidelity.
    ///
    /// What is performed on approval equals the rows the User did not exclude,
    /// and nothing outside the manifest is performed.
    #[test]
    fn property_20_manifest_holds_exactly_what_was_suppressed() {
        let c = triage_classifier();
        let mut manifest = IntendedActionManifest::default();

        let attempts: [(&str, &str, u32); 3] = [
            ("move_to_folder", "18 newsletters you've never opened", 18),
            ("move_to_folder", "9 messages labelled Needs reply", 9),
            ("send_email", "4 replies", 4),
        ];

        for (op, description, affected) in attempts {
            let d = decide(
                &c,
                "email",
                op,
                JobKind::Scheduled,
                JobState::Draft,
                RunMode::KickoffDryRun,
                false,
            );
            assert!(!d.is_permitted(), "{op} should have been suppressed");
            record_intent(
                &mut manifest,
                c.get("email", op).unwrap(),
                description,
                affected,
            );
        }

        assert_eq!(
            manifest.rows.len(),
            3,
            "every suppressed action must be held"
        );
        assert_eq!(manifest.retained().count(), 3);

        // The User unchecks the sends.
        assert!(manifest.exclude(2));
        let retained: Vec<_> = manifest.retained().map(|r| r.description.clone()).collect();
        assert_eq!(retained.len(), 2);
        assert!(
            !retained.iter().any(|d| d.contains("replies")),
            "an excluded row must not be performed"
        );
        assert!(
            !manifest.exclude(99),
            "an out-of-range exclusion is refused"
        );
    }

    #[test]
    fn the_manifest_is_honest_about_what_cannot_be_undone() {
        let c = triage_classifier();
        let mut only_reversible = IntendedActionManifest::default();
        record_intent(
            &mut only_reversible,
            c.get("email", "move_to_folder").unwrap(),
            "18 newsletters",
            18,
        );
        assert!(only_reversible.reversal_summary().contains("can be undone"));

        let mut has_irreversible = only_reversible.clone();
        record_intent(
            &mut has_irreversible,
            c.get("email", "send_email").unwrap(),
            "4 replies",
            4,
        );
        assert!(
            has_irreversible
                .reversal_summary()
                .contains("can't be undone"),
            "the summary must not promise reversal it cannot honour"
        );
    }
}
