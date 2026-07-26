//! Job identity and lifecycle.
//!
//! A Job is the only container of work Zavora Work Studio exposes to the User
//! (Requirement 3.1). A Job is either `scheduled` — recurring proactive work — or
//! `one_off` — work the User starts directly. Both kinds share history, steering,
//! change log, reversal and spend attribution; they differ only in lifecycle
//! (Requirement 3.2).
//!
//! The transition set is closed. Any transition not enumerated in
//! [`ALLOWED_SCHEDULED`] or [`ALLOWED_ONE_OFF`] is rejected without mutating
//! state, and no `scheduled`-only state is reachable by a `one_off` Job or
//! vice versa (Requirement 3.7, Correctness Property 4).

use std::fmt;

/// Whether a Job recurs on a schedule or was started directly by the User.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobKind {
    /// Recurring proactive work created from a template.
    Scheduled,
    /// Work the User started from New work. No schedule.
    OneOff,
}

impl JobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::OneOff => "one_off",
        }
    }

    /// The state a Job of this kind begins life in.
    pub fn initial_state(self) -> JobState {
        match self {
            Self::Scheduled => JobState::Draft,
            Self::OneOff => JobState::Active,
        }
    }
}

/// Job lifecycle state. Some states belong to one kind only; `NeedsAttention`
/// and `Retired` are shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobState {
    // scheduled only
    Draft,
    AwaitingKickoff,
    Live,
    Paused,
    // one_off only
    Active,
    Finished,
    // both
    NeedsAttention,
    Retired,
}

impl JobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::AwaitingKickoff => "awaiting_kickoff",
            Self::Live => "live",
            Self::Paused => "paused",
            Self::Active => "active",
            Self::Finished => "finished",
            Self::NeedsAttention => "needs_attention",
            Self::Retired => "retired",
        }
    }

    pub const ALL: [JobState; 8] = [
        Self::Draft,
        Self::AwaitingKickoff,
        Self::Live,
        Self::Paused,
        Self::Active,
        Self::Finished,
        Self::NeedsAttention,
        Self::Retired,
    ];

    /// Whether this state is reachable at all for the given kind.
    pub fn belongs_to(self, kind: JobKind) -> bool {
        match self {
            Self::NeedsAttention | Self::Retired => true,
            Self::Draft | Self::AwaitingKickoff | Self::Live | Self::Paused => {
                kind == JobKind::Scheduled
            }
            Self::Active | Self::Finished => kind == JobKind::OneOff,
        }
    }
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

use JobState as S;

/// Enumerated transitions for `scheduled` Jobs (Requirement 3.7).
pub const ALLOWED_SCHEDULED: &[(JobState, JobState)] = &[
    (S::Draft, S::AwaitingKickoff),
    (S::AwaitingKickoff, S::Live),
    (S::AwaitingKickoff, S::Draft),
    (S::Live, S::Paused),
    (S::Live, S::NeedsAttention),
    (S::Paused, S::Live),
    (S::Paused, S::AwaitingKickoff),
    (S::NeedsAttention, S::Live),
    (S::NeedsAttention, S::Paused),
    // Read-only Jobs skip Kickoff entirely (Requirement 5.7).
    (S::Draft, S::Live),
    // From any state to retired.
    (S::Draft, S::Retired),
    (S::AwaitingKickoff, S::Retired),
    (S::Live, S::Retired),
    (S::Paused, S::Retired),
    (S::NeedsAttention, S::Retired),
];

/// Enumerated transitions for `one_off` Jobs (Requirement 3.7).
pub const ALLOWED_ONE_OFF: &[(JobState, JobState)] = &[
    (S::Active, S::Finished),
    (S::Finished, S::Active),
    (S::Active, S::NeedsAttention),
    (S::NeedsAttention, S::Active),
    (S::Active, S::Retired),
    (S::Finished, S::Retired),
    (S::NeedsAttention, S::Retired),
];

pub fn allowed_for(kind: JobKind) -> &'static [(JobState, JobState)] {
    match kind {
        JobKind::Scheduled => ALLOWED_SCHEDULED,
        JobKind::OneOff => ALLOWED_ONE_OFF,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransitionError {
    #[error("a {kind} piece of work cannot go from {from} to {to}")]
    NotAllowed {
        kind: &'static str,
        from: JobState,
        to: JobState,
    },
    #[error("{state} does not apply to {kind} work")]
    WrongKind { kind: &'static str, state: JobState },
}

/// The only way a Job's state changes.
///
/// Returns the new state on success. On failure the caller's state is untouched —
/// this function is pure, so rejection cannot mutate anything.
pub fn transition(
    kind: JobKind,
    from: JobState,
    to: JobState,
) -> Result<JobState, TransitionError> {
    if !from.belongs_to(kind) {
        return Err(TransitionError::WrongKind {
            kind: kind.as_str(),
            state: from,
        });
    }
    if !to.belongs_to(kind) {
        return Err(TransitionError::WrongKind {
            kind: kind.as_str(),
            state: to,
        });
    }
    if allowed_for(kind).contains(&(from, to)) {
        Ok(to)
    } else {
        Err(TransitionError::NotAllowed {
            kind: kind.as_str(),
            from,
            to,
        })
    }
}

/// A Job as the engine holds it. Deliberately free of any agent, model, server
/// or tool field — those live in the execution layer, never on the Job
/// (Requirement 3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: String,
    pub kind: JobKind,
    /// The User's own words, shown verbatim.
    pub purpose: String,
    pub state: JobState,
    /// True when the Job can produce no external effect and no Artefact, and is
    /// therefore exempt from Kickoff review (Requirement 5.7).
    pub read_only: bool,
}

impl Job {
    pub fn new(id: impl Into<String>, kind: JobKind, purpose: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            purpose: purpose.into(),
            state: kind.initial_state(),
            read_only: false,
        }
    }

    pub fn read_only(mut self, yes: bool) -> Self {
        self.read_only = yes;
        self
    }

    /// Apply a transition, mutating only on success.
    pub fn move_to(&mut self, to: JobState) -> Result<(), TransitionError> {
        let next = transition(self.kind, self.state, to)?;
        self.state = next;
        Ok(())
    }

    /// Activation path. A read-only Job goes straight to work because asking the
    /// User to approve "nothing is wrong" teaches them to dismiss reviews
    /// (Requirement 5.7).
    pub fn activate(&mut self) -> Result<(), TransitionError> {
        match self.kind {
            JobKind::Scheduled if self.read_only => self.move_to(JobState::Live),
            JobKind::Scheduled => self.move_to(JobState::AwaitingKickoff),
            JobKind::OneOff => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Correctness Property 4: closed transition set per kind.
    ///
    /// Exhaustive over every (kind, from, to) triple: a transition is accepted
    /// if and only if it is enumerated, and rejection never mutates state.
    #[test]
    fn property_4_transition_set_is_closed_per_kind() {
        for kind in [JobKind::Scheduled, JobKind::OneOff] {
            for from in JobState::ALL {
                for to in JobState::ALL {
                    let enumerated = allowed_for(kind).contains(&(from, to))
                        && from.belongs_to(kind)
                        && to.belongs_to(kind);
                    let accepted = transition(kind, from, to).is_ok();
                    assert_eq!(
                        enumerated, accepted,
                        "{kind:?}: {from} -> {to} enumerated={enumerated} accepted={accepted}"
                    );
                }
            }
        }
    }

    #[test]
    fn property_4_rejection_does_not_mutate() {
        let mut job = Job::new("j1", JobKind::Scheduled, "Daily newsletter");
        let before = job.state;
        assert!(job.move_to(JobState::Finished).is_err());
        assert_eq!(job.state, before, "a rejected transition must not mutate");
    }

    #[test]
    fn scheduled_states_are_unreachable_for_one_off_and_vice_versa() {
        for state in [
            JobState::Draft,
            JobState::AwaitingKickoff,
            JobState::Live,
            JobState::Paused,
        ] {
            assert!(
                !state.belongs_to(JobKind::OneOff),
                "{state} leaked to one_off"
            );
        }
        for state in [JobState::Active, JobState::Finished] {
            assert!(
                !state.belongs_to(JobKind::Scheduled),
                "{state} leaked to scheduled"
            );
        }
    }

    #[test]
    fn every_enumerated_transition_is_kind_consistent() {
        for kind in [JobKind::Scheduled, JobKind::OneOff] {
            for (from, to) in allowed_for(kind) {
                assert!(
                    from.belongs_to(kind) && to.belongs_to(kind),
                    "{kind:?} enumerates {from} -> {to} but a state does not belong to it"
                );
            }
        }
    }

    #[test]
    fn retired_is_reachable_from_every_live_state() {
        for kind in [JobKind::Scheduled, JobKind::OneOff] {
            for from in JobState::ALL {
                if from == JobState::Retired || !from.belongs_to(kind) {
                    continue;
                }
                assert!(
                    transition(kind, from, JobState::Retired).is_ok(),
                    "{kind:?}: cannot retire from {from}"
                );
            }
        }
    }

    /// Requirement 5.7: a read-only Job is never gated by a Kickoff review.
    #[test]
    fn read_only_scheduled_job_skips_kickoff() {
        let mut monitor = Job::new("j2", JobKind::Scheduled, "Computer health").read_only(true);
        monitor.activate().expect("read-only job should activate");
        assert_eq!(monitor.state, JobState::Live);

        let mut newsletter = Job::new("j3", JobKind::Scheduled, "Daily newsletter");
        newsletter.activate().expect("job should activate");
        assert_eq!(newsletter.state, JobState::AwaitingKickoff);
    }

    #[test]
    fn one_off_job_starts_active_and_can_finish_and_resume() {
        let mut j = Job::new("j4", JobKind::OneOff, "Board deck from last quarter");
        assert_eq!(j.state, JobState::Active);
        j.move_to(JobState::Finished).unwrap();
        j.move_to(JobState::Active).unwrap();
        assert_eq!(j.state, JobState::Active);
    }
}
