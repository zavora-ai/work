//! Choosing how much intelligence to spend, and keeping working when a provider
//! has an outage.
//!
//! The User never picks a model (Requirement 14.1). Work Studio assigns a
//! [`QualityTier`] to each unit of work, and each tier resolves to an ordered chain
//! whose head is the primary and whose tail is failover. When the primary fails the
//! next option serves the work and the User sees a normal result, not an error
//! (Requirement 14.5) — the failover is recorded in the Activity_Log instead.
//!
//! Every call is metered, whoever made it: proactive execution, the User's own
//! document work, or internal classification (Requirement 15.3). Spend is held in
//! micros and only ever shown as currency (Requirement 15.6), rounded so a
//! fraction of a cent never reaches a primary surface (Requirement 15.7).

/// Building a model that can actually answer. Needs the sibling ADK-Rust checkout.
#[cfg(feature = "adk")]
pub mod model;

use std::collections::BTreeMap;

use rusqlite::params;
use studio_store::Store;

/// How much judgement a unit of work needs. Assigned by the product, never by the
/// User.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityTier {
    /// High volume, low judgement: intent routing, triage classification,
    /// monitor threshold checks.
    Fast,
    /// Default drafting quality: newsletters, social copy, document and deck content.
    Balanced,
    /// Reasoning-heavy and low volume: weekly roll-ups, multi-source synthesis.
    Best,
}

impl QualityTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Best => "best",
        }
    }
}

/// Which surface asked. Used for attribution, never shown to the User.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Proactive,
    Documents,
    Internal,
}

impl Surface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proactive => "proactive",
            Self::Documents => "documents",
            Self::Internal => "internal",
        }
    }
}

/// A concrete choice. Confined to Settings and diagnostics (Requirement 14.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    pub provider: String,
    pub model: String,
}

impl ModelRef {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.to_string(),
        }
    }

    /// The form Settings and the diagnostics view use.
    pub fn qualified(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }
}

/// Where the User sits between cost and quality. One global preference, no
/// per-Job choice (Requirement 14.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Preference {
    SpendLess,
    #[default]
    Balanced,
    BestQuality,
}

/// The tier-to-chain policy.
#[derive(Debug, Clone)]
pub struct Policy {
    chains: BTreeMap<QualityTier, Vec<ModelRef>>,
    preference: Preference,
}

impl Policy {
    /// First-run defaults. Every tier resolves to OpenAI (Requirement 14.2).
    pub fn openai_default() -> Self {
        let mut chains = BTreeMap::new();
        chains.insert(
            QualityTier::Fast,
            vec![ModelRef::new("openai", "gpt-5-mini")],
        );
        chains.insert(
            QualityTier::Balanced,
            vec![ModelRef::new("openai", "gpt-5")],
        );
        chains.insert(QualityTier::Best, vec![ModelRef::new("openai", "gpt-5")]);
        Self {
            chains,
            preference: Preference::default(),
        }
    }

    pub fn with_chain(mut self, tier: QualityTier, chain: Vec<ModelRef>) -> Self {
        self.chains.insert(tier, chain);
        self
    }

    pub fn with_preference(mut self, preference: Preference) -> Self {
        self.preference = preference;
        self
    }

    /// The chain for a unit of work, after the User's cost-versus-quality nudge.
    ///
    /// The nudge shifts which tier is consulted; it never asks the User to name a
    /// model.
    pub fn chain_for(&self, tier: QualityTier) -> &[ModelRef] {
        let effective = match (self.preference, tier) {
            (Preference::SpendLess, QualityTier::Best) => QualityTier::Balanced,
            (Preference::SpendLess, QualityTier::Balanced) => QualityTier::Fast,
            (Preference::BestQuality, QualityTier::Fast) => QualityTier::Balanced,
            (Preference::BestQuality, QualityTier::Balanced) => QualityTier::Best,
            (_, t) => t,
        };
        self.chains
            .get(&effective)
            .or_else(|| self.chains.get(&tier))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// How a unit of work was served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T> {
    /// The head of the chain served it.
    PrimarySuccess { value: T, used: ModelRef },
    /// The head failed and a later option served it. The User sees a normal result.
    FallbackUsed {
        value: T,
        used: ModelRef,
        primary_error: String,
    },
    /// Nothing in the chain could serve it. This one the User must hear about.
    AllFailed { errors: Vec<(ModelRef, String)> },
}

impl<T> Outcome<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::PrimarySuccess { value, .. } | Self::FallbackUsed { value, .. } => Some(value),
            Self::AllFailed { .. } => None,
        }
    }

    /// True when the work completed, however it completed.
    pub fn completed(&self) -> bool {
        !matches!(self, Self::AllFailed { .. })
    }

    /// True when it completed on something other than the primary.
    pub fn degraded(&self) -> bool {
        matches!(self, Self::FallbackUsed { .. })
    }
}

/// What the caller must tell us to bill a unit of work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Charge {
    pub micros: i64,
}

/// One attempt at serving work with a specific model.
pub type Attempt<T> = dyn Fn(&ModelRef) -> std::result::Result<(T, Charge), String>;

#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Store(#[from] studio_store::StoreError),
    #[error("no model is configured for {0} work")]
    NoChain(&'static str),
    #[error("today's limit has been reached")]
    LimitReached,
}

pub type Result<T> = std::result::Result<T, RouterError>;

pub struct Router<'a> {
    store: &'a Store,
    policy: Policy,
    /// None means no limit.
    daily_limit_micros: Option<i64>,
}

impl<'a> Router<'a> {
    pub fn new(store: &'a Store, policy: Policy) -> Self {
        Self {
            store,
            policy,
            daily_limit_micros: None,
        }
    }

    pub fn with_daily_limit(mut self, micros: Option<i64>) -> Self {
        self.daily_limit_micros = micros;
        self
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Serve a unit of work, failing over down the chain, metering whatever ran.
    ///
    /// `surface` and `job_id` are for attribution only. Proactive work is refused
    /// once the daily limit is reached; the User's own work never is
    /// (Requirement 15.5).
    pub fn serve<T>(
        &self,
        tier: QualityTier,
        surface: Surface,
        job_id: Option<&str>,
        attempt: &Attempt<T>,
    ) -> Result<Outcome<T>> {
        if surface == Surface::Proactive && self.limit_reached()? {
            return Err(RouterError::LimitReached);
        }

        let chain = self.policy.chain_for(tier);
        if chain.is_empty() {
            return Err(RouterError::NoChain(tier.as_str()));
        }

        let mut errors: Vec<(ModelRef, String)> = Vec::new();
        for candidate in chain {
            match attempt(candidate) {
                Ok((value, charge)) => {
                    self.meter(tier, surface, job_id, charge)?;
                    return Ok(if errors.is_empty() {
                        Outcome::PrimarySuccess {
                            value,
                            used: candidate.clone(),
                        }
                    } else {
                        let primary_error = errors[0].1.clone();
                        // Recorded, not reported: the User sees a normal result.
                        self.store.log(
                            "failover",
                            &format!(
                                "{} was unavailable, {} served the work instead",
                                errors[0].0.qualified(),
                                candidate.qualified()
                            ),
                            job_id,
                            None,
                        )?;
                        Outcome::FallbackUsed {
                            value,
                            used: candidate.clone(),
                            primary_error,
                        }
                    });
                }
                Err(e) => errors.push((candidate.clone(), e)),
            }
        }

        self.store.log(
            "retry",
            "nothing configured could do this piece of work",
            job_id,
            None,
        )?;
        Ok(Outcome::AllFailed { errors })
    }

    fn meter(
        &self,
        tier: QualityTier,
        surface: Surface,
        job_id: Option<&str>,
        charge: Charge,
    ) -> Result<()> {
        self.store.conn().execute(
            "INSERT INTO spend_ledger (id, ts, job_id, surface, tier, micros)
             VALUES (hex(randomblob(16)), unixepoch(), ?1, ?2, ?3, ?4)",
            params![job_id, surface.as_str(), tier.as_str(), charge.micros],
        )?;
        Ok(())
    }

    /// Everything spent today, across every surface.
    pub fn spent_today_micros(&self) -> Result<i64> {
        Ok(self.store.conn().query_row(
            "SELECT coalesce(sum(micros), 0) FROM spend_ledger
             WHERE ts >= unixepoch('now','start of day')",
            [],
            |r| r.get(0),
        )?)
    }

    pub fn spent_today_for_job_micros(&self, job_id: &str) -> Result<i64> {
        Ok(self.store.conn().query_row(
            "SELECT coalesce(sum(micros), 0) FROM spend_ledger
             WHERE job_id = ?1 AND ts >= unixepoch('now','start of day')",
            params![job_id],
            |r| r.get(0),
        )?)
    }

    pub fn limit_reached(&self) -> Result<bool> {
        match self.daily_limit_micros {
            None => Ok(false),
            Some(limit) => Ok(self.spent_today_micros()? >= limit),
        }
    }
}

/// Currency for a primary surface. Never tokens, never more than two decimals,
/// and never a fraction of a cent (Requirements 15.6, 15.7).
pub fn format_spend(micros: i64) -> String {
    let cents = (micros as f64 / 10_000.0).round() as i64;
    if cents == 0 && micros > 0 {
        // Real, but smaller than the smallest unit worth showing.
        "under $0.01".to_string()
    } else {
        format!("${}.{:02}", cents / 100, (cents % 100).abs())
    }
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

    fn chain_policy() -> Policy {
        Policy::openai_default().with_chain(
            QualityTier::Balanced,
            vec![
                ModelRef::new("openai", "gpt-5"),
                ModelRef::new("anthropic", "claude-sonnet"),
                ModelRef::new("ollama", "local-8b"),
            ],
        )
    }

    /// Succeeds only for the named provider; everything else is "down".
    fn only(
        provider: &'static str,
    ) -> impl Fn(&ModelRef) -> std::result::Result<(&'static str, Charge), String> {
        move |m: &ModelRef| {
            if m.provider == provider {
                Ok(("drafted", Charge { micros: 1_200 }))
            } else {
                Err(format!("{} is rate limited", m.qualified()))
            }
        }
    }

    #[test]
    fn first_run_defaults_every_tier_to_openai() {
        let p = Policy::openai_default();
        for tier in [QualityTier::Fast, QualityTier::Balanced, QualityTier::Best] {
            let chain = p.chain_for(tier);
            assert!(!chain.is_empty(), "{tier:?} must resolve");
            assert_eq!(chain[0].provider, "openai");
        }
    }

    #[test]
    fn the_users_one_preference_shifts_tiers_without_naming_a_model() {
        let p = chain_policy();
        let balanced = p.chain_for(QualityTier::Balanced)[0].clone();

        let cheap = p.clone().with_preference(Preference::SpendLess);
        assert_eq!(
            cheap.chain_for(QualityTier::Balanced)[0],
            Policy::openai_default().chain_for(QualityTier::Fast)[0],
            "spending less should drop a tier"
        );

        let best = p.clone().with_preference(Preference::BestQuality);
        assert_ne!(
            best.chain_for(QualityTier::Fast)[0],
            Policy::openai_default().chain_for(QualityTier::Fast)[0],
            "best quality should lift a tier"
        );
        assert_eq!(p.chain_for(QualityTier::Balanced)[0], balanced);
    }

    #[test]
    fn the_primary_serves_the_work_when_it_is_up() {
        let s = store();
        let r = Router::new(&s, chain_policy());
        let outcome = r
            .serve(
                QualityTier::Balanced,
                Surface::Proactive,
                Some("j1"),
                &only("openai"),
            )
            .unwrap();
        assert!(matches!(outcome, Outcome::PrimarySuccess { .. }));
        assert!(!outcome.degraded());
        assert_eq!(outcome.value(), Some(&"drafted"));
    }

    /// Correctness Property 14: failover transparency.
    ///
    /// Work completed via failover produces the same User-visible outcome class as
    /// work completed on the primary, and the failover is recorded.
    #[test]
    fn property_14_failover_completes_the_work_and_is_recorded_not_reported() {
        let s = store();
        let r = Router::new(&s, chain_policy());
        let before = s.activity_count().unwrap();

        let outcome = r
            .serve(
                QualityTier::Balanced,
                Surface::Proactive,
                Some("j1"),
                &only("anthropic"),
            )
            .unwrap();

        assert!(outcome.completed(), "the work must still get done");
        assert_eq!(
            outcome.value(),
            Some(&"drafted"),
            "the result must be the same result"
        );
        assert!(outcome.degraded());
        match &outcome {
            Outcome::FallbackUsed {
                used,
                primary_error,
                ..
            } => {
                assert_eq!(used.provider, "anthropic");
                assert!(primary_error.contains("rate limited"));
            }
            other => panic!("expected a failover, got {other:?}"),
        }

        assert_eq!(
            s.activity_count().unwrap(),
            before + 1,
            "the failover must be recorded"
        );
        let detail: String = s
            .conn()
            .query_row(
                "SELECT detail FROM activity_log WHERE category = 'failover' ORDER BY seq DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            detail.contains("openai/gpt-5"),
            "the record should say what failed"
        );
    }

    #[test]
    fn failing_over_twice_still_completes() {
        let s = store();
        let r = Router::new(&s, chain_policy());
        let outcome = r
            .serve(
                QualityTier::Balanced,
                Surface::Documents,
                None,
                &only("ollama"),
            )
            .unwrap();
        assert!(outcome.degraded() && outcome.completed());
    }

    #[test]
    fn when_nothing_can_serve_the_work_the_failure_is_explicit() {
        let s = store();
        let r = Router::new(&s, chain_policy());
        let outcome = r
            .serve(
                QualityTier::Balanced,
                Surface::Proactive,
                Some("j1"),
                &only("nobody"),
            )
            .unwrap();
        assert!(!outcome.completed());
        match outcome {
            Outcome::AllFailed { errors } => assert_eq!(errors.len(), 3),
            other => panic!("expected total failure, got {other:?}"),
        }
    }

    /// Correctness Property 15: spend completeness.
    ///
    /// The ledger equals total usage across every surface.
    #[test]
    fn property_15_every_surface_is_metered() {
        let s = store();
        let r = Router::new(&s, chain_policy());
        let charge = 1_200;

        r.serve(
            QualityTier::Balanced,
            Surface::Proactive,
            Some("j1"),
            &only("openai"),
        )
        .unwrap();
        r.serve(
            QualityTier::Balanced,
            Surface::Documents,
            None,
            &only("openai"),
        )
        .unwrap();
        r.serve(QualityTier::Fast, Surface::Internal, None, &only("openai"))
            .unwrap();

        assert_eq!(
            r.spent_today_micros().unwrap(),
            charge * 3,
            "proactive, documents and internal work must all be metered"
        );
        assert_eq!(
            r.spent_today_for_job_micros("j1").unwrap(),
            charge,
            "per-Job attribution"
        );

        let by_surface: i64 = s
            .conn()
            .query_row(
                "SELECT count(DISTINCT surface) FROM spend_ledger",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(by_surface, 3);
    }

    #[test]
    fn a_failed_attempt_is_not_billed_but_the_one_that_worked_is() {
        let s = store();
        let r = Router::new(&s, chain_policy());
        r.serve(
            QualityTier::Balanced,
            Surface::Proactive,
            Some("j1"),
            &only("ollama"),
        )
        .unwrap();
        assert_eq!(
            r.spent_today_micros().unwrap(),
            1_200,
            "only the attempt that produced something is charged"
        );
    }

    /// Requirement 15.5: the limit pauses proactive work and never the User's own.
    #[test]
    fn the_daily_limit_stops_proactive_work_but_not_the_users_own() {
        let s = store();
        let r = Router::new(&s, chain_policy()).with_daily_limit(Some(2_000));

        r.serve(
            QualityTier::Balanced,
            Surface::Proactive,
            Some("j1"),
            &only("openai"),
        )
        .unwrap();
        assert!(!r.limit_reached().unwrap());
        r.serve(
            QualityTier::Balanced,
            Surface::Proactive,
            Some("j1"),
            &only("openai"),
        )
        .unwrap();
        assert!(r.limit_reached().unwrap());

        assert!(
            matches!(
                r.serve(
                    QualityTier::Balanced,
                    Surface::Proactive,
                    Some("j1"),
                    &only("openai")
                ),
                Err(RouterError::LimitReached)
            ),
            "proactive work must pause"
        );
        assert!(
            r.serve(
                QualityTier::Balanced,
                Surface::Documents,
                None,
                &only("openai")
            )
            .is_ok(),
            "the User's own work must never stop"
        );
    }

    #[test]
    fn a_tier_with_nothing_configured_says_so_plainly() {
        let s = store();
        let empty = Policy::openai_default().with_chain(QualityTier::Best, vec![]);
        let r = Router::new(&s, empty);
        assert!(matches!(
            r.serve(QualityTier::Best, Surface::Documents, None, &only("openai")),
            Err(RouterError::NoChain("best"))
        ));
    }

    /// Requirement 15.7: never render a fraction of a cent.
    #[test]
    fn spend_is_shown_as_currency_and_never_as_a_fraction_of_a_cent() {
        assert_eq!(format_spend(620_000), "$0.62");
        assert_eq!(format_spend(0), "$0.00");
        assertated_two_dp(format_spend(1_234_567));
        assert_eq!(format_spend(40), "under $0.01");
        assert_eq!(format_spend(12_345_600), "$12.35");

        fn assertated_two_dp(s: String) {
            let decimals = s.split('.').nth(1).unwrap_or("");
            assert_eq!(decimals.len(), 2, "expected two decimals in {s}");
        }
    }

    #[test]
    fn a_model_reference_is_only_ever_qualified_for_settings_and_diagnostics() {
        let m = ModelRef::new("openai", "gpt-5-mini");
        assert_eq!(m.qualified(), "openai/gpt-5-mini");
    }
}
