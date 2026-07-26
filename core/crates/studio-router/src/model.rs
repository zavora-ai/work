//! Turning a choice into something callable.
//!
//! `ModelRef` names a provider and a model; this makes one that can actually answer.
//! Kept apart from the policy on purpose: the policy is about what the User prefers and is
//! testable without a network or a credential, and this is the one place that needs both.
//!
//! ## Where the credential comes from
//!
//! For now, the environment. The product's promise is the OS keychain (Requirement 14.7,
//! and the first-run screen says so in as many words), so this is a development
//! arrangement and is reported as one: [`Credentials::source`] says which it was, and the
//! diagnostics view shows it. It is deliberately awkward to mistake one for the other.

use std::sync::Arc;

use crate::ModelRef;

/// Where a credential came from. Shown in diagnostics, never on a primary surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    /// The OS keychain — what the product promises.
    Keychain,
    /// The environment. Development only.
    Environment,
}

impl CredentialSource {
    /// For the diagnostics view.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Keychain => "from this Mac's keychain",
            Self::Environment => "from the environment (development only)",
        }
    }

    /// Whether this is the arrangement the product promises the User.
    pub fn is_as_promised(self) -> bool {
        matches!(self, Self::Keychain)
    }
}

#[derive(Debug, Clone)]
pub struct Credentials {
    key: String,
    source: CredentialSource,
}

impl Credentials {
    pub fn source(&self) -> CredentialSource {
        self.source
    }

    /// The key for a provider, from the environment.
    ///
    /// Returns `None` rather than an empty credential, because an empty key produces a
    /// failure from the provider that reads like a fault in the product.
    pub fn from_environment(provider: &str) -> Option<Self> {
        let variable = match provider {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" | "google" => "GEMINI_API_KEY",
            "groq" => "GROQ_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            "openrouter" => "OPENROUTER_API_KEY",
            _ => return None,
        };
        let key = std::env::var(variable).ok()?;
        let key = key.trim().to_string();
        if key.is_empty() {
            return None;
        }
        Some(Self {
            key,
            source: CredentialSource::Environment,
        })
    }
}

/// Why work could not be done. Both variants are in the User's terms, because both are
/// things they can act on.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("Work Studio has not been set up to think yet")]
    NoCredential { provider: String },
    #[error("Work Studio could not reach the service it thinks with")]
    NotUsable { detail: String },
    #[error("Work Studio does not know how to use {provider}")]
    UnknownProvider { provider: String },
}

impl ModelError {
    /// The cause, for the diagnostics view only.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::NotUsable { detail } => Some(detail),
            _ => None,
        }
    }
}

/// Build a model from a choice, taking the credential from the environment.
pub fn model_for(reference: &ModelRef) -> Result<Arc<dyn adk_core::Llm>, ModelError> {
    let credentials = Credentials::from_environment(&reference.provider).ok_or_else(|| {
        ModelError::NoCredential {
            provider: reference.provider.clone(),
        }
    })?;
    model_with(reference, &credentials)
}

/// Build a model from a choice and a credential you already hold.
pub fn model_with(
    reference: &ModelRef,
    credentials: &Credentials,
) -> Result<Arc<dyn adk_core::Llm>, ModelError> {
    match reference.provider.as_str() {
        "openai" => {
            let config = adk_model::openai::OpenAIConfig::new(&credentials.key, &reference.model);
            adk_model::openai::OpenAIClient::new(config)
                .map(|client| Arc::new(client) as Arc<dyn adk_core::Llm>)
                .map_err(|error| ModelError::NotUsable {
                    detail: error.to_string(),
                })
        }
        "anthropic" => {
            let config =
                adk_model::anthropic::AnthropicConfig::new(&credentials.key, &reference.model);
            adk_model::anthropic::AnthropicClient::new(config)
                .map(|client| Arc::new(client) as Arc<dyn adk_core::Llm>)
                .map_err(|error| ModelError::NotUsable {
                    detail: error.to_string(),
                })
        }
        other => Err(ModelError::UnknownProvider {
            provider: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A missing credential must read as something the User can act on, not as a fault.
    #[test]
    fn without_a_credential_the_reason_is_plain() {
        let error = ModelError::NoCredential {
            provider: "openai".into(),
        };
        let message = error.to_string();
        assert!(!message.to_lowercase().contains("api"), "leaks: {message}");
        assert!(!message.to_lowercase().contains("key"), "leaks: {message}");
        assert!(!message.contains("openai"), "names a provider: {message}");
    }

    /// A provider nobody wired must be refused rather than silently treated as OpenAI,
    /// which would send the User's work to the wrong place.
    #[test]
    fn an_unknown_provider_is_refused() {
        let credentials = Credentials {
            key: "irrelevant".into(),
            source: CredentialSource::Environment,
        };
        let result = model_with(&ModelRef::new("some-other-service", "m"), &credentials);
        assert!(matches!(result, Err(ModelError::UnknownProvider { .. })));
    }

    #[test]
    fn an_empty_credential_is_not_a_credential() {
        // Safety: single-threaded test, and the variable is removed straight after.
        unsafe { std::env::set_var("OPENAI_API_KEY", "   ") };
        assert!(Credentials::from_environment("openai").is_none());
        unsafe { std::env::remove_var("OPENAI_API_KEY") };
    }

    #[test]
    fn a_development_credential_never_claims_to_be_the_promised_one() {
        assert!(!CredentialSource::Environment.is_as_promised());
        assert!(CredentialSource::Keychain.is_as_promised());
        assert!(
            CredentialSource::Environment
                .describe()
                .contains("development")
        );
    }

    /// The default policy must resolve to something this factory can actually build,
    /// or the product would name a model it cannot use.
    #[test]
    fn every_model_the_default_policy_names_can_be_built() {
        let policy = crate::Policy::openai_default();
        let credentials = Credentials {
            key: "test-key-not-used-for-a-request".into(),
            source: CredentialSource::Environment,
        };
        for tier in [
            crate::QualityTier::Fast,
            crate::QualityTier::Balanced,
            crate::QualityTier::Best,
        ] {
            for reference in policy.chain_for(tier) {
                model_with(reference, &credentials)
                    .unwrap_or_else(|e| panic!("{} cannot be built: {e}", reference.qualified()));
            }
        }
    }
}
