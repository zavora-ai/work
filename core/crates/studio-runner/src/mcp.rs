//! Talking to a capability server, and composing the specialist that uses it.
//!
//! Two things live here, both behind the `adk` feature because both need the
//! Capability_Layer present:
//!
//! * [`McpApplier`] — carries out one operation against a real server, and is what the
//!   [`crate::edits::Dispatcher`] calls once the gate has allowed it. Nothing in it
//!   decides anything; the rules stay above it.
//! * [`spreadsheet_agent`] — the specialist itself.
//!
//! **One agent, not a pipeline.** `excel-agent-app` composes planner → writer → styler
//! and falls back to a single agent for non-Gemini providers. OpenAI is our default, so
//! that fallback is our normal path anyway, and three model passes triple both latency
//! and cost for the same spreadsheet — against Requirement 19.3 and against the spend
//! figure the User can see. The Quality_Tier router is the lever when a harder task
//! needs more thinking.

use std::sync::Arc;

use adk_tool::McpToolset;
use adk_tool::mcp::rmcp::{
    RoleClient, ServiceExt, service::RunningService, transport::TokioChildProcess,
};
use tokio::process::Command;

use crate::edits::Applier;

/// How to start a capability server.
#[derive(Debug, Clone)]
pub struct ServerSpec {
    /// The name Work Studio classifies its operations under.
    pub name: String,
    /// The executable.
    pub command: String,
    pub args: Vec<String>,
}

impl ServerSpec {
    /// The spreadsheet server. Its binary is provisioned beside the app.
    pub fn spreadsheet(command: impl Into<String>) -> Self {
        Self {
            name: "worksheet".to_string(),
            command: command.into(),
            args: Vec::new(),
        }
    }

    /// The document server.
    pub fn document(command: impl Into<String>) -> Self {
        Self {
            name: "document".to_string(),
            command: command.into(),
            args: Vec::new(),
        }
    }

    /// The presentation server.
    pub fn presentation(command: impl Into<String>) -> Self {
        Self {
            name: "presentation".to_string(),
            command: command.into(),
            args: Vec::new(),
        }
    }
}

/// A running capability server and the toolset over it.
pub struct Server {
    pub spec: ServerSpec,
    pub toolset: McpToolset<()>,
}

/// The server's toolset, shareable.
///
/// `Server` owns its toolset and the pipeline still needs the server afterwards — to ask
/// what operations it exposes — so the agent is given a handle that borrows through the
/// same `Arc` rather than a copy that could drift from it.
pub struct SharedToolset(Arc<Server>);

#[async_trait::async_trait]
impl adk_core::Toolset for SharedToolset {
    fn name(&self) -> &str {
        &self.0.spec.name
    }

    async fn tools(
        &self,
        ctx: Arc<dyn adk_core::ReadonlyContext>,
    ) -> adk_core::Result<Vec<Arc<dyn adk_core::Tool>>> {
        adk_core::Toolset::tools(&self.0.toolset, ctx).await
    }
}

impl Server {
    /// A toolset an agent can hold, backed by this same server.
    pub fn toolset_for_agent(self: Arc<Self>) -> Arc<dyn adk_core::Toolset> {
        Arc::new(SharedToolset(self))
    }

    /// Start the server and connect over stdio.
    pub async fn start(spec: ServerSpec) -> Result<Self, String> {
        let mut command = Command::new(&spec.command);
        command.args(&spec.args);
        let child = TokioChildProcess::new(command).map_err(|e| e.to_string())?;
        let client: RunningService<RoleClient, ()> =
            ().serve(child).await.map_err(|e| e.to_string())?;
        Ok(Self {
            spec,
            toolset: McpToolset::new(client),
        })
    }

    /// What the server says it can do, for checking that Work Studio's classification
    /// still describes the server it is talking to.
    pub async fn operation_names(
        &self,
        ctx: Arc<dyn adk_core::ReadonlyContext>,
    ) -> Result<Vec<String>, String> {
        let tools = adk_core::Toolset::tools(&self.toolset, ctx)
            .await
            .map_err(|e| e.to_string())?;
        Ok(tools.iter().map(|tool| tool.name().to_string()).collect())
    }

    /// Carry out one operation.
    pub async fn call(
        &self,
        operation: &str,
        arguments: serde_json::Value,
    ) -> Result<String, String> {
        let map = match arguments {
            serde_json::Value::Object(map) => map,
            other => {
                let mut map = serde_json::Map::new();
                map.insert("input".to_string(), other);
                map
            }
        };
        let answer = self
            .toolset
            .call_tool_value(operation, map)
            .await
            .map(|value| value.to_string())
            .map_err(|e| e.to_string())?;

        // A delivered reply is not a successful operation. These servers answer a refusal with a
        // status in the body rather than by failing the call, so a caller that checks only for
        // transport failure treats every refusal as a success — which is how "you added a chart"
        // came to be said about a chart that was never added. Checked here, once, because eleven
        // call sites checking it themselves is ten chances to forget.
        if let Some(refused) = refusal_in(&answer) {
            return Err(refused);
        }
        Ok(answer)
    }

    /// Call an operation and hand back whatever it says, refusal included.
    ///
    /// For the few places where "no" is information rather than a failure — asking whether
    /// something exists, for instance.
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn call_allowing_refusal(
        &self,
        operation: &str,
        arguments: serde_json::Value,
    ) -> Result<String, String> {
        let map = match arguments {
            serde_json::Value::Object(map) => map,
            other => {
                let mut map = serde_json::Map::new();
                map.insert("input".to_string(), other);
                map
            }
        };
        self.toolset
            .call_tool_value(operation, map)
            .await
            .map(|value| value.to_string())
            .map_err(|e| e.to_string())
    }
}

/// Applies operations against a running server.
///
/// Deliberately thin: the gate has already decided by the time this is called, and the
/// change log is written after it returns. This type only does the doing.
pub struct McpApplier {
    server: Arc<Server>,
    /// The arguments for the next call, set by the caller that knows them.
    arguments: std::sync::Mutex<serde_json::Value>,
    runtime: tokio::runtime::Handle,
}

impl McpApplier {
    pub fn new(server: Arc<Server>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            server,
            arguments: std::sync::Mutex::new(serde_json::json!({})),
            runtime,
        }
    }

    /// Set the arguments the next operation will be called with.
    pub fn with_arguments(&self, arguments: serde_json::Value) {
        *self.arguments.lock().expect("arguments lock") = arguments;
    }

    pub fn server_name(&self) -> &str {
        &self.server.spec.name
    }
}

impl Applier for McpApplier {
    fn apply(&self, server: &str, operation: &str) -> Result<(), String> {
        if server != self.server.spec.name {
            return Err(format!("{server} is not connected"));
        }
        let arguments = self.arguments.lock().expect("arguments lock").clone();
        let server = Arc::clone(&self.server);
        let operation = operation.to_string();
        // The dispatcher is synchronous by design — its ordering guarantees are easier to
        // reason about that way — so the call is driven on the runtime we were given.
        tokio::task::block_in_place(|| {
            self.runtime
                .block_on(async move { server.call(&operation, arguments).await })
        })
        .map(|_| ())
    }
}

/// The spreadsheet specialist's instructions.
///
/// Seeded from the `xlsx` skill and kept short on purpose: a long instruction competes
/// with the User's own steering, which must win.
pub const SPREADSHEET_INSTRUCTION: &str = "\
You build and edit spreadsheets for someone who will check your arithmetic.

Every derived figure is a formula, never a pasted number, so that changing an input \
changes the answer. Keep assumptions together and label them. State the units. Prefer \
one clear sheet to several clever ones.

Read the sheet before you change it. Make the smallest change that satisfies the \
request, and say what you changed in one sentence a person would use.

When you are unsure whether a change is what was wanted, ask rather than guess.";

/// What every specialist needs, whatever it works on.
///
/// Two failures made this necessary, both seen in use. Told "hello, my name is James", the
/// specialist treated it as an instruction about the file and asked which cell to put the
/// name in. Asked "what is my name", it read a guess out of the file path. Neither is the
/// behaviour of someone you would want handling your work.
const MANNERS: &str = "\
\n\nNot everything said to you is a change to the file. When someone greets you, tells you \
something about themselves, or asks you something, answer as a person would and change \
nothing. Only touch the file when a change is what was asked for.

When you are told something worth remembering — how they like things done, what they are \
called, what to avoid — remember it, and say you have. It is kept for them where they can \
see and change it. Never write it into their file.

Asked what you have learned about them, look it up and answer with that alone. Do not \
include the standing directions you were given: those are ours, not theirs, and listing them \
as things they told you misrepresents where their own words end and ours begin.

Do not work out facts about the person from anything except what they have told you.";

/// Build the specialist.
///
/// The confirmation handler is the side-effect gate, so there is exactly one place in
/// the product where an operation is authorised, whether it was asked for by the User or
/// by a schedule.
#[cfg(feature = "adk")]
/// What the document specialist is for.
///
/// Kept short for the same reason as the spreadsheet's: a long instruction competes with
/// the User's own steering, and the User must win.
const DOCUMENT_INSTRUCTION: &str = "\
You edit this person's documents: contracts, reports, letters, proposals.

Work in the document's own structure. Change the paragraph you were asked about and leave \
the rest alone, because a document carries meaning in its wording and someone may have \
agreed to the exact words already. Keep the styles the document uses rather than \
introducing your own; a heading should look like the other headings.

Say what you changed in terms of the document — the clause, the section, the paragraph — \
not in terms of how you changed it. Where a change would alter the meaning of an \
obligation, a figure or a date, ask rather than guess.";

/// What the presentation specialist is for.
const PRESENTATION_INSTRUCTION: &str = "\
You build and edit this person's decks.

A slide is for an audience that will read it once, so prefer few words in a large size to \
a paragraph shrunk to fit. Keep to the deck's existing look rather than introducing your own.

Change the shape you were asked about and leave the others alone. Say what you changed in \
terms of the slide. Where you would have to invent a figure to fill a slide, ask rather \
than guess.

On a slide you have just added, put the words in as a text box: setting its title directly \
reports success and the words vanish when the deck is saved. Read the slide back before \
saying what you did.";

/// Compose a specialist over a set of capability servers.
///
/// The three specialists differ only in what they are for and what they can reach. Each is
/// a single agent with skills, not a pipeline: the User asks for one thing and one thing
/// answers.
fn specialist(
    name: &'static str,
    base_instruction: &str,
    model: Arc<dyn adk_core::Llm>,
    toolsets: Vec<Arc<dyn adk_core::Toolset>>,
    steering: &[String],
) -> Result<Arc<dyn adk_core::Agent>, String> {
    let mut instruction = String::from(base_instruction);
    instruction.push_str(MANNERS);
    if !steering.is_empty() {
        // The User's own words go last, so they win over ours.
        instruction.push_str("\n\nWhat this person has told you:\n");
        for note in steering {
            instruction.push_str("- ");
            instruction.push_str(note);
            instruction.push('\n');
        }
    }

    let mut builder = adk_agent::LlmAgentBuilder::new(name)
        .model(model)
        .instruction(instruction);
    for toolset in toolsets {
        builder = builder.toolset(toolset);
    }
    builder
        .build()
        .map(|agent| Arc::new(agent) as Arc<dyn adk_core::Agent>)
        .map_err(|e| e.to_string())
}

/// Which specialist a piece of work needs.
///
/// One place decides, so a specialist cannot be paired with the wrong server or the wrong
/// classification. Getting that pairing wrong would mean the gate looked up an operation
/// in a catalogue that had never heard of it and refused work for no reason the User
/// could act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtefactKind {
    Spreadsheet,
    Document,
    Presentation,
}

impl ArtefactKind {
    /// The kind of a file, from its name.
    pub fn of(path: &std::path::Path) -> Option<Self> {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("xlsx" | "xlsm" | "xls") => Some(Self::Spreadsheet),
            Some("docx" | "doc") => Some(Self::Document),
            Some("pptx" | "ppt") => Some(Self::Presentation),
            _ => None,
        }
    }

    /// What this specialist is called, where the User is choosing what it may reach.
    ///
    /// Deliberately not `server_name`. That one is the scope the gate and the catalogue are
    /// keyed by, and for spreadsheets it is "worksheet" — the capability server's own word.
    /// Using it to look up what the User allocated found nothing, silently, because they had
    /// allocated to "spreadsheet". Two names for one thing needs two accessors, not one that
    /// means whichever the caller assumed.
    pub fn specialist_name(self) -> &'static str {
        match self {
            Self::Spreadsheet => "spreadsheet",
            Self::Document => "document",
            Self::Presentation => "presentation",
        }
    }

    /// The name the gate and the catalogue both know this specialist by.
    pub fn server_name(self) -> &'static str {
        match self {
            Self::Spreadsheet => "worksheet",
            Self::Document => "document",
            Self::Presentation => "presentation",
        }
    }

    /// How this specialist's operations are classified.
    pub fn classifier(self) -> studio_gate::Classifier {
        match self {
            Self::Spreadsheet => studio_gate::catalogue::worksheet(),
            Self::Document => studio_gate::catalogue_docs::document(),
            Self::Presentation => studio_gate::catalogue_slides::presentation(),
        }
    }

    /// This specialist's capability server, given its binary.
    pub fn server_spec(self, command: impl Into<String>) -> ServerSpec {
        match self {
            Self::Spreadsheet => ServerSpec::spreadsheet(command),
            Self::Document => ServerSpec::document(command),
            Self::Presentation => ServerSpec::presentation(command),
        }
    }

    /// Compose the specialist for this kind of work.
    pub fn agent(
        self,
        model: Arc<dyn adk_core::Llm>,
        toolsets: Vec<Arc<dyn adk_core::Toolset>>,
        steering: &[String],
    ) -> Result<Arc<dyn adk_core::Agent>, String> {
        match self {
            Self::Spreadsheet => spreadsheet_agent(model, toolsets, steering),
            Self::Document => document_agent(model, toolsets, steering),
            Self::Presentation => presentation_agent(model, toolsets, steering),
        }
    }
}

pub fn spreadsheet_agent(
    model: Arc<dyn adk_core::Llm>,
    toolsets: Vec<Arc<dyn adk_core::Toolset>>,
    steering: &[String],
) -> Result<Arc<dyn adk_core::Agent>, String> {
    specialist(
        "spreadsheet",
        SPREADSHEET_INSTRUCTION,
        model,
        toolsets,
        steering,
    )
}

pub fn document_agent(
    model: Arc<dyn adk_core::Llm>,
    toolsets: Vec<Arc<dyn adk_core::Toolset>>,
    steering: &[String],
) -> Result<Arc<dyn adk_core::Agent>, String> {
    specialist("document", DOCUMENT_INSTRUCTION, model, toolsets, steering)
}

pub fn presentation_agent(
    model: Arc<dyn adk_core::Llm>,
    toolsets: Vec<Arc<dyn adk_core::Toolset>>,
    steering: &[String],
) -> Result<Arc<dyn adk_core::Agent>, String> {
    specialist(
        "presentation",
        PRESENTATION_INSTRUCTION,
        model,
        toolsets,
        steering,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_instruction_asks_for_formulas_and_leaves_room_for_steering() {
        assert!(SPREADSHEET_INSTRUCTION.contains("never a pasted number"));
        assert!(SPREADSHEET_INSTRUCTION.contains("ask rather than guess"));
        assert!(
            SPREADSHEET_INSTRUCTION.len() < 900,
            "a long instruction competes with the User's own steering, which must win"
        );
    }

    #[test]
    fn the_spreadsheet_server_is_named_as_the_catalogue_names_it() {
        let spec = ServerSpec::spreadsheet("/usr/local/bin/excel-mcp-server");
        assert_eq!(
            spec.name, "worksheet",
            "the name must match the classification catalogue or every operation reads as unknown"
        );
    }
    /// Every specialist must leave room for the User's own words, and none may be so long
    /// that it drowns them out.
    #[test]
    fn no_specialist_instruction_competes_with_the_user() {
        for (name, instruction) in [
            ("spreadsheet", SPREADSHEET_INSTRUCTION),
            ("document", DOCUMENT_INSTRUCTION),
            ("presentation", PRESENTATION_INSTRUCTION),
        ] {
            assert!(
                instruction.len() < 900,
                "{name}'s instruction is {} characters, which competes with steering",
                instruction.len()
            );
            // The shared part is added to every one of them, so it counts against the same
            // budget.
            assert!(
                instruction.len() + MANNERS.len() < 1600,
                "{name} plus the shared guidance is {} characters",
                instruction.len() + MANNERS.len()
            );
            assert!(
                instruction.contains("ask rather than guess"),
                "{name} must ask rather than invent"
            );
        }
    }

    /// Being told something personal is not a request to edit a file, and a path is not a
    /// source of facts about a person. Both were real failures.
    #[test]
    fn the_shared_guidance_covers_the_two_failures_that_prompted_it() {
        let lowered = MANNERS.to_lowercase();
        assert!(lowered.contains("not everything said to you is a change"));
        assert!(lowered.contains("never write it into their file"));
        assert!(
            lowered.contains("except what they have told you"),
            "it must forbid working the person out from anything else"
        );
        assert!(
            lowered.contains("do not \ninclude the standing directions")
                || lowered.contains("do not include the standing directions"),
            "it must not pass our own directions off as the User's preferences"
        );
        for banned in ["tool", "mcp", "json", "api", "path", "session"] {
            assert!(!lowered.contains(banned), "the guidance says {banned}");
        }
    }

    /// The instructions must speak about the User's work, not about the machinery.
    #[test]
    fn no_specialist_instruction_uses_our_vocabulary() {
        for (name, instruction) in [
            ("spreadsheet", SPREADSHEET_INSTRUCTION),
            ("document", DOCUMENT_INSTRUCTION),
            ("presentation", PRESENTATION_INSTRUCTION),
        ] {
            let lowered = instruction.to_lowercase();
            for banned in ["tool", "mcp", "json", "api", "invocation", "session"] {
                assert!(
                    !lowered.contains(banned),
                    "{name}'s instruction says {banned}, which is our word and not the User's"
                );
            }
        }
    }

    /// Each specialist is for one kind of work, and says so.
    #[test]
    fn each_specialist_is_about_its_own_artefact() {
        assert!(DOCUMENT_INSTRUCTION.contains("document"));
        assert!(PRESENTATION_INSTRUCTION.contains("deck"));
        assert!(
            PRESENTATION_INSTRUCTION.contains("shape"),
            "the presentation specialist edits shapes, which is what a click resolves to"
        );
    }

    #[test]
    fn the_three_servers_are_named_apart() {
        let names = [
            ServerSpec::spreadsheet("x").name,
            ServerSpec::document("x").name,
            ServerSpec::presentation("x").name,
        ];
        let unique: std::collections::BTreeSet<_> = names.iter().collect();
        assert_eq!(unique.len(), 3, "each server must be its own gate scope");
        // The names must match the catalogues, or the gate would find no classification.
        assert!(
            studio_gate::catalogue::worksheet()
                .get("worksheet", "write_cells")
                .is_some()
        );
        assert!(
            studio_gate::catalogue_docs::document()
                .get("document", "save_document")
                .is_some()
        );
        assert!(
            studio_gate::catalogue_slides::presentation()
                .get("presentation", "add_slide")
                .is_some()
        );
    }

    /// A file must reach the specialist that understands it, and nothing else.
    #[test]
    fn a_file_reaches_the_specialist_that_understands_it() {
        use std::path::Path;
        for (name, expected) in [
            ("model.xlsx", Some(ArtefactKind::Spreadsheet)),
            ("Q3.XLSX", Some(ArtefactKind::Spreadsheet)),
            ("agreement.docx", Some(ArtefactKind::Document)),
            ("board.pptx", Some(ArtefactKind::Presentation)),
            ("notes.txt", None),
            ("no-extension", None),
        ] {
            assert_eq!(ArtefactKind::of(Path::new(name)), expected, "for {name}");
        }
    }

    /// The pairing that matters: each specialist's server name must be the one its own
    /// catalogue is keyed by, or the gate would look an operation up in the wrong place
    /// and refuse it.
    #[test]
    fn every_specialist_is_paired_with_its_own_classification() {
        for (kind, operation) in [
            (ArtefactKind::Spreadsheet, "write_cells"),
            (ArtefactKind::Document, "insert_paragraph"),
            (ArtefactKind::Presentation, "add_slide"),
        ] {
            let classifier = kind.classifier();
            assert!(
                classifier.get(kind.server_name(), operation).is_some(),
                "{:?} cannot classify its own {operation}",
                kind
            );
            assert_eq!(
                kind.server_spec("x").name,
                kind.server_name(),
                "the server and the classification must agree on the name"
            );
        }
    }

    /// A specialist must not be able to classify another's work.
    #[test]
    fn one_specialist_cannot_authorise_anothers_operations() {
        let document = ArtefactKind::Document;
        assert!(
            document
                .classifier()
                .get(document.server_name(), "write_cells")
                .is_none(),
            "the document specialist must not be able to write spreadsheet cells"
        );
        let presentation = ArtefactKind::Presentation;
        assert!(
            presentation
                .classifier()
                .get(presentation.server_name(), "insert_paragraph")
                .is_none()
        );
    }

    /// Two names for one specialist, and a lookup that used the wrong one found nothing
    /// silently. These are the values the rest of the product is keyed by, so they are pinned.
    #[test]
    fn the_two_names_for_a_specialist_are_both_pinned() {
        for (kind, specialist, scope) in [
            (ArtefactKind::Spreadsheet, "spreadsheet", "worksheet"),
            (ArtefactKind::Document, "document", "document"),
            (ArtefactKind::Presentation, "presentation", "presentation"),
        ] {
            assert_eq!(kind.specialist_name(), specialist);
            assert_eq!(kind.server_name(), scope);
        }
        // The one place they differ is the one that caused the bug.
        assert_ne!(
            ArtefactKind::Spreadsheet.specialist_name(),
            ArtefactKind::Spreadsheet.server_name(),
            "if these ever become the same, the two accessors can be merged"
        );
    }
}

/// A minimal context, for asking a server what it can do outside a run.
///
/// Listing operations needs a context only because the toolset trait takes one; nothing
/// about the answer depends on it.
pub mod test_support {
    use std::sync::Arc;

    #[derive(Debug)]
    struct Bare {
        content: adk_core::Content,
    }

    impl adk_core::ReadonlyContext for Bare {
        fn invocation_id(&self) -> &str {
            "listing"
        }
        fn agent_name(&self) -> &str {
            "listing"
        }
        fn user_id(&self) -> &str {
            "listing"
        }
        fn app_name(&self) -> &str {
            "work-studio"
        }
        fn session_id(&self) -> &str {
            "listing"
        }
        fn branch(&self) -> &str {
            ""
        }
        fn user_content(&self) -> &adk_core::Content {
            &self.content
        }
    }

    pub fn readonly_context() -> Arc<dyn adk_core::ReadonlyContext> {
        Arc::new(Bare {
            content: adk_core::Content::new("user"),
        })
    }
}

/// The message in a reply that says "error", if it says so.
///
/// The status can be nested: the reply is a JSON value whose text is itself JSON, sometimes inside
/// a content array. So it is looked for wherever it is rather than assumed to be at the top —
/// assuming that is why the first version of this check passed every refusal through.
fn refusal_in(answer: &str) -> Option<String> {
    fn within(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::String(text) => serde_json::from_str::<serde_json::Value>(text)
                .ok()
                .and_then(|inner| within(&inner)),
            serde_json::Value::Object(map) => {
                if map.get("status").and_then(|status| status.as_str()) == Some("error") {
                    return Some(
                        map.get("message")
                            .and_then(|message| message.as_str())
                            .unwrap_or("the connection refused it")
                            .to_string(),
                    );
                }
                map.values().find_map(within)
            }
            serde_json::Value::Array(items) => items.iter().find_map(within),
            _ => None,
        }
    }

    serde_json::from_str::<serde_json::Value>(answer)
        .ok()
        .and_then(|body| within(&body))
}

#[cfg(test)]
mod refusal_tests {
    use super::refusal_in;

    #[test]
    fn a_plain_success_is_not_a_refusal() {
        let reply = serde_json::json!({ "status": "success", "message": "Saved" }).to_string();
        assert_eq!(refusal_in(&serde_json::json!(reply).to_string()), None);
    }

    /// The shape these servers actually answer in: JSON inside a JSON string.
    #[test]
    fn a_refusal_nested_in_a_string_is_found() {
        let inner = serde_json::json!({
            "status": "error",
            "message": "Sheet 'Nope' not found"
        })
        .to_string();
        let wrapped = serde_json::json!(inner).to_string();
        assert_eq!(
            refusal_in(&wrapped).as_deref(),
            Some("Sheet 'Nope' not found")
        );
    }

    #[test]
    fn a_refusal_inside_a_content_array_is_found() {
        let inner = serde_json::json!({ "status": "error", "message": "no" }).to_string();
        let wrapped =
            serde_json::json!({ "content": [{ "type": "text", "text": inner }] }).to_string();
        assert_eq!(refusal_in(&wrapped).as_deref(), Some("no"));
    }

    #[test]
    fn something_that_is_not_json_is_left_alone() {
        assert_eq!(refusal_in("saved"), None);
    }
}
