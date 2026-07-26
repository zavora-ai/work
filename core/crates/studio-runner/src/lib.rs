//! Executing a piece of work.
//!
//! Two things live here today: the per-Job lease that makes run exclusivity a
//! database guarantee, and the adapter that answers a runtime's confirmation
//! requests with the side-effect gate.
//!
//! The ADK-Rust implementation of that adapter is behind the `adk` feature, so the
//! rules stay testable without the Capability_Layer checked out beside us. Nothing
//! in the decision path depends on the feature.

pub mod confirm;
pub mod edits;
pub mod lease;

/// Talking to a capability server. Needs the Capability_Layer beside us.
#[cfg(feature = "adk")]
pub mod mcp;

pub use confirm::{GateHandler, Resolver, RunContext, Verdict};
pub use edits::{Applied, Applier, Dispatcher, EditError, ProposedEdit};
pub use lease::{Lease, LeaseError, Leases, STALE_AFTER_SECS};
