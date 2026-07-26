//! The vocabulary guardrail.
//!
//! Requirement 1.1 forbids a list of technical terms in any User-visible string,
//! and Requirement 1.2 requires the build to fail if one appears. This is the
//! specific defence the previous in-house attempt lacked: its product surface
//! drifted into a development environment one honest-looking label at a time.
//!
//! The rule is scoped. It is absolute on [`Scope::Primary`] surfaces. Settings
//! may name a provider (Requirement 14.7) and Diagnostics exists to hold
//! technical detail (Requirement 17.5), so both relax the corresponding terms.
//!
//! Two terms need more than a word match:
//! * "run" is prohibited only as a noun for an execution ("11 runs today",
//!   "last run"), not as a verb ("Run now"). English part-of-speech cannot be
//!   inferred from a word list, so the noun senses are matched as phrases.
//! * "AI" is permitted, since it names the thing the User pays for.

use regex::Regex;
use studio_strings::{CATALOGUE, Entry, Scope};

/// Terms forbidden outright on primary surfaces (Requirement 1.1).
pub const PROHIBITED_WORDS: &[&str] = &[
    "agent",
    "model",
    "provider",
    "LLM",
    "token",
    "prompt",
    "MCP",
    "server",
    "invocation",
    "checkpoint",
    "graph",
    "sandbox",
    "protocol",
    "API",
    "JSON",
    "schema",
    "crate",
    "session",
    "tool call",
    "tool",
];

/// Noun senses of "run" that are prohibited. The verb ("Run now") is fine.
pub const PROHIBITED_RUN_PHRASES: &[&str] = &[
    r"\bruns\s+today\b",
    r"\b\d+\s+runs?\b",
    r"\b(last|next|first|this|the)\s+run\b",
    r"\brun\s+(history|count|id)\b",
];

/// Terms Settings is permitted to use.
///
/// Requirement 14.7 confines provider and model identifiers to Settings, and
/// Requirement 23 places agent configuration there too, on the grounds that a User
/// who opens Settings has gone looking for detail.
///
/// The line is drawn at things the User can configure and reason about — an agent,
/// the model behind it, the tools it may use, the instructions it follows, what it
/// remembers. It is *not* drawn at the implementation substrate: session,
/// invocation, checkpoint, graph, sandbox, protocol, API, JSON, schema, crate, LLM
/// and token counts remain prohibited even here, because they describe how the
/// product is built rather than what it does. Those belong in the diagnostics view.
///
/// Widening this list is a product decision and should be made in the spec, not in
/// a component.
const SETTINGS_EXEMPT: &[&str] = &[
    "provider",
    "model",
    "agent",
    "tool",
    "tool call",
    "prompt",
    "MCP",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub key: &'static str,
    pub text: &'static str,
    pub term: String,
    pub scope_note: &'static str,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: \"{}\" contains the prohibited term \"{}\" ({})",
            self.key, self.text, self.term, self.scope_note
        )
    }
}

fn word_regex(term: &str) -> Regex {
    // Case-insensitive whole-word match, tolerating a simple plural. Terms
    // containing a space are matched as a phrase.
    //
    // The plural matters: an earlier version matched only `\bagent\b`, so "Slides
    // agent" was caught while "Slides agents" was not. A guardrail with that hole
    // in it is worse than none, because it produces confidence rather than safety.
    let stem = regex::escape(term);
    let pattern = if term.ends_with('s') {
        format!(r"(?i)\b{stem}\b")
    } else {
        format!(r"(?i)\b{stem}s?\b")
    };
    Regex::new(&pattern).expect("valid term regex")
}

fn applies(term: &str, scope: Scope) -> bool {
    match scope {
        Scope::Primary => true,
        Scope::Settings => !SETTINGS_EXEMPT.iter().any(|t| t.eq_ignore_ascii_case(term)),
        // Diagnostics exists to hold technical detail.
        Scope::Diagnostics => false,
    }
}

/// Scan one entry.
pub fn check_entry(entry: &Entry) -> Vec<Violation> {
    let mut out = Vec::new();

    for term in PROHIBITED_WORDS {
        if !applies(term, entry.scope) {
            continue;
        }
        if word_regex(term).is_match(entry.text) {
            out.push(Violation {
                key: entry.key,
                text: entry.text,
                term: (*term).to_string(),
                scope_note: match entry.scope {
                    Scope::Primary => "primary surface",
                    Scope::Settings => "settings",
                    Scope::Diagnostics => "diagnostics",
                },
            });
        }
    }

    if entry.scope != Scope::Diagnostics {
        for pattern in PROHIBITED_RUN_PHRASES {
            let re = Regex::new(&format!("(?i){pattern}")).expect("valid run regex");
            if let Some(m) = re.find(entry.text) {
                out.push(Violation {
                    key: entry.key,
                    text: entry.text,
                    term: m.as_str().to_string(),
                    scope_note: "\"run\" as a noun for an execution",
                });
            }
        }
    }

    out
}

/// Scan the whole catalogue. An empty result is the build-passing condition
/// (Requirement 1.2, Correctness Property 11).
pub fn check_catalogue() -> Vec<Violation> {
    CATALOGUE.iter().flat_map(check_entry).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_strings::Scope;

    /// Correctness Property 11: vocabulary containment.
    ///
    /// No string in the User-visible catalogue matches the prohibited-term list.
    /// This test is the build gate.
    #[test]
    fn property_11_catalogue_is_clean() {
        let violations = check_catalogue();
        assert!(
            violations.is_empty(),
            "vocabulary rule violated:\n{}",
            violations
                .iter()
                .map(|v| format!("  - {v}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// The lint must actually catch things. These are the exact labels the
    /// original hand-drawn mockup used — eight violations in one screen, written
    /// by the person who set the rule.
    #[test]
    fn lint_catches_the_original_mockups_violations() {
        let cases = [
            "DocX agent",
            "Slides agent",
            "Spreadsheet agent",
            "social agent",
            "digest agent",
            "Model spend today",
            "Runs today",
        ];
        for text in cases {
            let entry = Entry {
                key: "test.planted",
                text,
                scope: Scope::Primary,
            };
            let found = check_entry(&entry);
            assert!(
                !found.is_empty(),
                "lint failed to catch {text:?} — this is what it exists for"
            );
        }
    }

    #[test]
    fn lint_catches_every_prohibited_word_on_a_primary_surface() {
        for term in PROHIBITED_WORDS {
            let text: &'static str = Box::leak(format!("Open the {term} now").into_boxed_str());
            let entry = Entry {
                key: "test.planted",
                text,
                scope: Scope::Primary,
            };
            assert!(
                !check_entry(&entry).is_empty(),
                "prohibited term {term:?} was not detected"
            );
        }
    }

    /// A plural must not slip through. This was a real hole: the rule matched
    /// "Slides agent" and missed "Slides agents".
    #[test]
    fn a_plural_is_caught_too() {
        for text in [
            "Slides agents",
            "Choose your models",
            "Available tools",
            "Restart the servers",
            "1,204 tokens used",
        ] {
            let e = Entry {
                key: "t",
                text,
                scope: Scope::Primary,
            };
            assert!(
                !check_entry(&e).is_empty(),
                "{text:?} is a plural of a prohibited term and must be caught"
            );
        }
    }

    #[test]
    fn run_as_a_verb_is_allowed_but_as_a_noun_is_not() {
        let allowed = ["Run now", "Run it again", "I'll run this every weekday"];
        for text in allowed {
            let e = Entry {
                key: "t",
                text,
                scope: Scope::Primary,
            };
            assert!(
                check_entry(&e).is_empty(),
                "{text:?} uses run as a verb and must be allowed"
            );
        }
        let forbidden = [
            "Runs today",
            "11 runs",
            "last run",
            "next run",
            "run history",
        ];
        for text in forbidden {
            let e = Entry {
                key: "t",
                text,
                scope: Scope::Primary,
            };
            assert!(
                !check_entry(&e).is_empty(),
                "{text:?} uses run as a noun and must be rejected"
            );
        }
    }

    #[test]
    fn settings_may_name_a_provider_but_a_primary_surface_may_not() {
        let in_settings = Entry {
            key: "t",
            text: "Add another provider",
            scope: Scope::Settings,
        };
        assert!(
            check_entry(&in_settings).is_empty(),
            "Requirement 14.7 permits provider names in Settings"
        );

        let on_primary = Entry {
            key: "t",
            text: "Add another provider",
            scope: Scope::Primary,
        };
        assert!(
            !check_entry(&on_primary).is_empty(),
            "a primary surface must not name a provider"
        );
    }

    #[test]
    fn settings_may_configure_an_agent_but_a_primary_surface_may_not() {
        for text in [
            "What each agent can do",
            "The tools it may use",
            "Its prompt",
            "Which model it uses",
        ] {
            let in_settings = Entry {
                key: "t",
                text,
                scope: Scope::Settings,
            };
            assert!(
                check_entry(&in_settings).is_empty(),
                "Settings should be able to say {text:?} (Requirement 23)"
            );
            let on_primary = Entry {
                key: "t",
                text,
                scope: Scope::Primary,
            };
            assert!(
                !check_entry(&on_primary).is_empty(),
                "a primary surface must not say {text:?}"
            );
        }
    }

    /// The exemption covers what the User configures, not how the product is built.
    #[test]
    fn settings_may_not_name_the_implementation_substrate() {
        for text in [
            "Restart the session",
            "Replay the invocation",
            "Inspect the graph",
            "Edit the JSON schema",
            "Sandbox mode",
            "1,204 tokens used",
            "Choose an LLM",
        ] {
            let e = Entry {
                key: "t",
                text,
                scope: Scope::Settings,
            };
            assert!(
                !check_entry(&e).is_empty(),
                "{text:?} describes the substrate and belongs in diagnostics, not Settings"
            );
        }
    }

    #[test]
    fn ai_is_permitted() {
        let e = Entry {
            key: "t",
            text: "Your AI key",
            scope: Scope::Settings,
        };
        assert!(
            check_entry(&e).is_empty(),
            "\"AI\" names what the User pays for"
        );
    }

    #[test]
    fn diagnostics_may_hold_technical_detail() {
        let e = Entry {
            key: "t",
            text: "gpt-5-mini · 1,204 tokens · tool call failed",
            scope: Scope::Diagnostics,
        };
        assert!(
            check_entry(&e).is_empty(),
            "the diagnostics view exists to hold this"
        );
    }
}
