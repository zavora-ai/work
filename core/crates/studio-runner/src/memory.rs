//! Remembering, as something the specialist can actually do.
//!
//! The specialists were told to say they would remember things, and had no way to. Told "my
//! name is James Maina" one would reply "I'll remember that and keep it where you can see it"
//! and write nothing anywhere — the panel headed *What I've learned from you* stayed empty
//! under a promise the product had just made. An agent that says it will remember and cannot
//! is worse than one that says it cannot.
//!
//! ## Where a memory goes
//!
//! Into the same list the User already sees and edits, not into a store of its own. The panel
//! claims to be everything Work Studio goes on, and that claim only survives if there is one
//! place to look. `mcp-session-memory` is the right home for an agent's own cross-session
//! memory and is a better server for the work done on it, but a second store would mean the
//! User's notes and the agent's memories could disagree about what Work Studio believes, with
//! no way to tell from the interface which was acting.
//!
//! So these tools write through the same door the User's own typing does: one list, one set of
//! rules about what acts and what waits to be agreed to.

use std::sync::Arc;

use serde_json::{Value, json};

/// Somewhere to keep what the User has said, and to read it back.
///
/// A trait so the runner does not depend on the store. The Core implements it over the
/// durable one; a test implements it over a vector.
pub trait Remembers: Send + Sync {
    /// Keep something the User said in this piece of work.
    ///
    /// Returns what the interface will show as its provenance, so the specialist can say
    /// truthfully where it will appear.
    fn remember(&self, thread: &str, note: &str) -> Result<String, String>;

    /// What is already known about this piece of work.
    fn recall(&self, thread: &str) -> Vec<String>;
}

/// The two tools, as a toolset the specialist can be given.
pub struct MemoryTools {
    thread: String,
    store: Arc<dyn Remembers>,
}

impl MemoryTools {
    pub fn new(thread: impl Into<String>, store: Arc<dyn Remembers>) -> Self {
        Self {
            thread: thread.into(),
            store,
        }
    }
}

#[async_trait::async_trait]
impl adk_core::Toolset for MemoryTools {
    fn name(&self) -> &str {
        "memory"
    }

    async fn tools(
        &self,
        _ctx: Arc<dyn adk_core::ReadonlyContext>,
    ) -> adk_core::Result<Vec<Arc<dyn adk_core::Tool>>> {
        Ok(vec![
            Arc::new(RememberTool {
                thread: self.thread.clone(),
                store: Arc::clone(&self.store),
            }) as Arc<dyn adk_core::Tool>,
            Arc::new(RecallTool {
                thread: self.thread.clone(),
                store: Arc::clone(&self.store),
            }) as Arc<dyn adk_core::Tool>,
        ])
    }
}

struct RememberTool {
    thread: String,
    store: Arc<dyn Remembers>,
}

#[async_trait::async_trait]
impl adk_core::Tool for RememberTool {
    fn name(&self) -> &str {
        "remember"
    }

    fn description(&self) -> &str {
        "Keep something this person has told you about how they want their work done, or about \
         themselves — their name, a preference, something to avoid. Use it whenever they tell \
         you something worth remembering. It goes into the list they can see and edit, not \
         into their file. Do not use it for anything you worked out for yourself rather than \
         being told, and never for a password or key."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "note": {
                    "type": "string",
                    "description": "What to remember, in this person's own words where possible, \
                                    as one self-contained sentence."
                }
            },
            "required": ["note"]
        }))
    }

    async fn execute(
        &self,
        _ctx: Arc<dyn adk_core::ToolContext>,
        args: Value,
    ) -> adk_core::Result<Value> {
        let note = args
            .get("note")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if note.is_empty() {
            return Ok(json!({ "kept": false, "why": "there was nothing to remember" }));
        }
        // A credential is never a preference. Refused here rather than trusted to the
        // instruction, because the instruction is advice and this is a rule.
        if looks_like_a_secret(note) {
            return Ok(json!({
                "kept": false,
                "why": "I do not keep passwords or keys. Tell me the preference instead."
            }));
        }
        match self.store.remember(&self.thread, note) {
            Ok(provenance) => Ok(json!({
                "kept": true,
                "note": note,
                "shown_as": provenance,
                "where": "the list of what I go on, which they can edit or delete"
            })),
            Err(why) => Ok(json!({ "kept": false, "why": why })),
        }
    }
}

struct RecallTool {
    thread: String,
    store: Arc<dyn Remembers>,
}

#[async_trait::async_trait]
impl adk_core::Tool for RecallTool {
    fn name(&self) -> &str {
        "recall"
    }

    fn description(&self) -> &str {
        "What this person has already told you about how they want their work done. Use it when \
         they ask what you know about them, or when you are about to make a choice they may have \
         a preference about."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(json!({ "type": "object", "properties": {} }))
    }

    async fn execute(
        &self,
        _ctx: Arc<dyn adk_core::ToolContext>,
        _args: Value,
    ) -> adk_core::Result<Value> {
        let known = self.store.recall(&self.thread);
        Ok(json!({
            "count": known.len(),
            "known": known,
        }))
    }
}

/// Whether a note looks like a credential rather than a preference.
///
/// Deliberately blunt. A false positive costs one refusal the User can work around; a false
/// negative writes a key into a list that is shown on screen.
pub fn looks_like_a_secret(text: &str) -> bool {
    let lowered = text.to_lowercase();
    const NAMES: &[&str] = &[
        "password",
        "passphrase",
        "api key",
        "api-key",
        "apikey",
        "secret key",
        "access token",
        "bearer ",
        "private key",
        "credit card",
        "cvv",
    ];
    if NAMES.iter().any(|name| lowered.contains(name)) {
        return true;
    }
    // Shapes that are credentials whatever they are called.
    if text.contains("sk-") || text.contains("ghp_") || text.contains("-----BEGIN") {
        return true;
    }
    // A long unbroken run of credential-ish characters is not a sentence.
    text.split_whitespace().any(|word| {
        word.len() >= 32
            && word
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use adk_core::Tool as _;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Remembered {
        notes: Mutex<Vec<String>>,
    }

    impl Remembers for Remembered {
        fn remember(&self, _thread: &str, note: &str) -> Result<String, String> {
            self.notes.lock().unwrap().push(note.to_string());
            Ok("You told me".to_string())
        }
        fn recall(&self, _thread: &str) -> Vec<String> {
            self.notes.lock().unwrap().clone()
        }
    }

    /// The specialist must describe remembering in the User's terms, and must not name the
    /// file as the place it goes.
    #[test]
    fn the_tools_are_described_in_the_users_words() {
        let store: Arc<dyn Remembers> = Arc::new(Remembered::default());
        let remember = RememberTool {
            thread: "t1".into(),
            store: Arc::clone(&store),
        };
        let recall = RecallTool {
            thread: "t1".into(),
            store,
        };
        for tool in [remember.description(), recall.description()] {
            let lowered = tool.to_lowercase();
            for banned in [
                "mcp",
                "json",
                "database",
                "store",
                "sql",
                "vector",
                "embedding",
            ] {
                assert!(!lowered.contains(banned), "says {banned}: {tool}");
            }
        }
        assert!(
            remember.description().contains("not \ninto their file")
                || remember.description().contains("not into their file"),
            "it must be clear a memory does not go into the User's document"
        );
        assert!(!remember.is_read_only(), "remembering changes something");
        assert!(recall.is_read_only(), "recalling changes nothing");
    }

    #[test]
    fn a_credential_is_never_kept() {
        for secret in [
            "my password is hunter2",
            "the API key is abcdefghijklmnop",
            "use sk-proj-abc123 for this",
            "ghp_aaaabbbbccccddddeeeeffffgggghhhh",
            "-----BEGIN PRIVATE KEY-----",
            "remember AKIAIOSFODNN7EXAMPLEZZZZZZZZZZZZZZZZ",
        ] {
            assert!(looks_like_a_secret(secret), "should have refused: {secret}");
        }
        for fine in [
            "Keep figures as formulas",
            "My name is James Maina",
            "Never include crypto prices",
            "Put assumptions on their own sheet",
        ] {
            assert!(!looks_like_a_secret(fine), "should have kept: {fine}");
        }
    }

    #[tokio::test]
    async fn remembering_reaches_the_store_and_says_where_it_went() {
        let store = Arc::new(Remembered::default());
        let tool = RememberTool {
            thread: "t1".into(),
            store: Arc::clone(&store) as Arc<dyn Remembers>,
        };
        let answer = tool
            .execute(context(), json!({ "note": "Keep figures as formulas" }))
            .await
            .unwrap();
        assert_eq!(answer["kept"], json!(true));
        assert_eq!(answer["shown_as"], json!("You told me"));
        assert_eq!(
            store.notes.lock().unwrap().as_slice(),
            &["Keep figures as formulas".to_string()],
            "the note must actually be kept, not merely acknowledged"
        );
    }

    #[tokio::test]
    async fn nothing_is_kept_when_there_is_nothing_to_keep() {
        let store = Arc::new(Remembered::default());
        let tool = RememberTool {
            thread: "t1".into(),
            store: Arc::clone(&store) as Arc<dyn Remembers>,
        };
        let answer = tool
            .execute(context(), json!({ "note": "   " }))
            .await
            .unwrap();
        assert_eq!(answer["kept"], json!(false));
        assert!(store.notes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_secret_is_refused_by_the_tool_not_only_by_the_instruction() {
        let store = Arc::new(Remembered::default());
        let tool = RememberTool {
            thread: "t1".into(),
            store: Arc::clone(&store) as Arc<dyn Remembers>,
        };
        let answer = tool
            .execute(context(), json!({ "note": "my password is hunter2" }))
            .await
            .unwrap();
        assert_eq!(answer["kept"], json!(false));
        assert!(
            store.notes.lock().unwrap().is_empty(),
            "a credential must never reach the list, whatever the model was told"
        );
    }

    #[tokio::test]
    async fn recall_returns_what_was_kept() {
        let store = Arc::new(Remembered::default());
        store.remember("t1", "Keep figures as formulas").unwrap();
        store
            .remember("t1", "Assumptions on their own sheet")
            .unwrap();
        let tool = RecallTool {
            thread: "t1".into(),
            store: Arc::clone(&store) as Arc<dyn Remembers>,
        };
        let answer = tool.execute(context(), json!({})).await.unwrap();
        assert_eq!(answer["count"], json!(2));
        assert!(answer["known"].to_string().contains("formulas"));
    }

    fn context() -> Arc<dyn adk_core::ToolContext> {
        Arc::new(adk_tool::SimpleToolContext::new("test"))
    }
}
