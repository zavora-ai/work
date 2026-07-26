//! Doing the work.
//!
//! Everything needed to answer a request already existed and nothing joined it up: the
//! router named a model, the gate decided about operations, the catalogues described the
//! servers, the change log recorded edits. This is the sequence that turns "make column D
//! a 12% case" into a changed file.
//!
//! The shape follows `excel-agent-app`, which has been doing this against ADK-Rust for a
//! while: build a model, build one agent with a toolset, hand it to a `Runner`, and read
//! the event stream. What is added here is the gate — installed as the runtime's
//! confirmation handler, so there is still exactly one place an external action is
//! authorised, and it is in the path rather than beside it.

use std::sync::Arc;

use studio_gate::RunMode;
use studio_jobs::{JobKind, JobState};
use studio_router::model::{ModelError, model_for};
use studio_router::{ModelRef, Policy, QualityTier};

use crate::confirm::{GateHandler, Resolver, RunContext};
use crate::mcp::{ArtefactKind, Server};

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("Work Studio has not been set up to think yet")]
    NoModel,
    #[error("Work Studio could not reach the service it thinks with")]
    ModelUnusable { detail: String },
    #[error("Work Studio cannot open that kind of file")]
    UnknownKind,
    #[error("Work Studio could not start what it needs to edit this file")]
    ServerUnavailable { detail: String },
    /// Distinct from the above on purpose. "Could not start" says the app is broken; this says
    /// the User turned it off, which is true, actionable, and not a fault.
    #[error("You have switched this off for now")]
    NotAllowed { detail: String },
    #[error("Work Studio could not finish that")]
    Failed { detail: String },
}

impl RunError {
    /// The cause, for the diagnostics view only.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::ModelUnusable { detail }
            | Self::ServerUnavailable { detail }
            | Self::NotAllowed { detail }
            | Self::Failed { detail } => Some(detail),
            _ => None,
        }
    }
}

impl From<ModelError> for RunError {
    fn from(error: ModelError) -> Self {
        match error {
            ModelError::NoCredential { .. } => Self::NoModel,
            other => Self::ModelUnusable {
                detail: other.to_string(),
            },
        }
    }
}

/// What the User asked for, and about which file.
#[derive(Debug, Clone)]
pub struct Request {
    /// The User's own words.
    pub asked: String,
    /// The file they are looking at.
    pub artefact: std::path::PathBuf,
    /// What they have told Work Studio, per thread and globally, already resolved.
    pub steering: Vec<String>,
    /// Which thread this belongs to, so the conversation continues rather than restarting.
    pub thread: String,
    /// What was already said in this piece of work, oldest first.
    #[allow(clippy::struct_field_names)]
    pub history: Vec<Said>,
}

/// One thing that was said.
#[derive(Debug, Clone)]
pub struct Said {
    pub from_user: bool,
    pub text: String,
}

/// What Work Studio is doing, while it does it.
///
/// The operation names the specialist uses are ours, not the User's, so each is turned
/// into something worth reading before it leaves here. Anything unrecognised is reported as
/// plain work rather than by name, because a name the vocabulary rule has never seen is
/// exactly how technical language reaches a primary surface.
pub fn progress_for(operation: &str) -> Option<&'static str> {
    Some(match operation {
        "open_workbook" | "open_document" | "open_presentation" => "Opening your file",
        "read_sheet"
        | "read_paragraphs"
        | "read_slide"
        | "get_sheet_dimensions"
        | "describe_presentation"
        | "describe_document"
        | "inspect_slide" => "Reading it",
        "write_cells" | "set_cell_text" | "insert_paragraph" | "add_paragraph" | "add_run"
        | "edit_run" | "add_text_box" => "Making the change",
        "apply_style" | "format_text" | "set_run_format" | "set_shape_fill" => "Tidying it up",
        "add_chart" | "set_chart_data" => "Building the chart",
        "save_workbook" | "save_document" | "save_presentation" => "Saving",
        _ => return None,
    })
}

/// The structural things the User can do to a spreadsheet by hand.
///
/// A closed set, on purpose. Every one of these is a real operation on the other side of the
/// connection, and nothing else can be reached this way — so widening what the interface can do
/// to the User's file means adding a variant here, which shows up in a diff.
#[derive(Debug, Clone)]
pub enum SheetAction {
    /// Make room. `at` is a row number as the row header shows it, counting from one.
    InsertRows { at: u32, count: u32 },
    /// Take rows away, from `at` downwards.
    DeleteRows { at: u32, count: u32 },
    /// Make room, at a column letter.
    InsertColumns { at: String, count: u16 },
    /// Take columns away, from `at` rightwards.
    DeleteColumns { at: String, count: u16 },
    /// Sort a range by one of its columns. `by` is a column letter within the range.
    Sort {
        range: String,
        by: String,
        ascending: bool,
        has_header: bool,
    },
    /// Hold the rows above and columns left of this cell in place while the rest scrolls.
    Freeze { at: String },
    /// Let the columns go back to scrolling.
    Unfreeze,
    /// Make one cell of several.
    Merge { range: String },
    /// Widen the columns to fit what is in them.
    FitColumns,
}

impl SheetAction {
    /// The operation this asks for, in the capability server's own words.
    pub fn operation(&self) -> &'static str {
        match self {
            Self::InsertRows { .. } | Self::DeleteRows { .. } => "modify_rows",
            Self::InsertColumns { .. } | Self::DeleteColumns { .. } => "modify_columns",
            Self::Sort { .. } => "sort_range",
            Self::Freeze { .. } | Self::Unfreeze => "freeze_panes",
            Self::Merge { .. } => "merge_cells",
            Self::FitColumns => "autofit_columns",
        }
    }

    /// What to send with it. The sheet is added here so no caller can forget it.
    fn arguments(&self, sheet: &str) -> serde_json::Value {
        match self {
            Self::InsertRows { at, count } => serde_json::json!({
                "sheet_name": sheet,
                "action": "insert",
                // One-based on the wire, because the User is reading a row header.
                "at_row": at,
                "count": count,
            }),
            Self::DeleteRows { at, count } => serde_json::json!({
                "sheet_name": sheet, "action": "delete", "at_row": at, "count": count,
            }),
            Self::InsertColumns { at, count } => serde_json::json!({
                "sheet_name": sheet, "action": "insert", "at_column": at, "count": count,
            }),
            Self::DeleteColumns { at, count } => serde_json::json!({
                "sheet_name": sheet, "action": "delete", "at_column": at, "count": count,
            }),
            Self::Sort {
                range,
                by,
                ascending,
                has_header,
            } => serde_json::json!({
                "sheet_name": sheet,
                "range": range,
                // "direction", not "ascending". The key accepts unknown fields silently, so an
                // "ascending" field was taken, ignored, and the sort reported success having
                // sorted the other way — the shape of a control that appears not to work.
                "sort_keys": [{
                    "column": by,
                    "direction": if *ascending { "ascending" } else { "descending" },
                }],
                "has_header": has_header,
            }),
            Self::Freeze { at } => serde_json::json!({ "sheet_name": sheet, "cell": at }),
            // Freezing at the top-left corner is how a spreadsheet says "nothing is frozen".
            Self::Unfreeze => serde_json::json!({ "sheet_name": sheet, "cell": "A1" }),
            Self::Merge { range } => serde_json::json!({ "sheet_name": sheet, "range": range }),
            Self::FitColumns => serde_json::json!({ "sheet_name": sheet }),
        }
    }

    /// What happened, in the User's words, for the history.
    pub fn in_words(&self) -> String {
        match self {
            Self::InsertRows { at, count } => format!("you inserted {count} row(s) at row {at}"),
            Self::DeleteRows { at, count } => format!("you deleted {count} row(s) from row {at}"),
            Self::InsertColumns { at, count } => {
                format!("you inserted {count} column(s) at column {at}")
            }
            Self::DeleteColumns { at, count } => {
                format!("you deleted {count} column(s) from column {at}")
            }
            Self::Sort { range, by, .. } => format!("you sorted {range} by column {by}"),
            Self::Freeze { at } => format!("you froze the headings at {at}"),
            Self::Unfreeze => "you unfroze the headings".to_string(),
            Self::Merge { range } => format!("you merged {range}"),
            Self::FitColumns => "you fitted the columns".to_string(),
        }
    }
}

/// What a run cost, as the provider counted it.
///
/// Recorded rather than estimated: the figure on the Dashboard is real money, and a plausible
/// guess there is worse than no figure at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub answer_tokens: i64,
}

impl Usage {
    /// Millionths of a currency unit, at the given per-million-token rates.
    ///
    /// Rates belong to the caller, not here: they change without warning, and a number baked
    /// into the engine would quietly become wrong.
    pub fn micros(self, prompt_per_million: i64, answer_per_million: i64) -> i64 {
        (self.prompt_tokens * prompt_per_million + self.answer_tokens * answer_per_million)
            / 1_000_000
    }

    pub fn is_empty(self) -> bool {
        self.prompt_tokens == 0 && self.answer_tokens == 0
    }
}

/// What happened, in terms the interface can show.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// What Work Studio said, in its own words.
    pub said: String,
    /// The operations that were actually performed, in order.
    pub performed: Vec<String>,
    /// Operations the gate refused, and why. Never silent.
    pub refused: Vec<String>,
    /// Whether the file on disk was saved.
    pub saved: bool,
    /// What it cost, as counted by the provider. Empty when nothing reported usage.
    pub usage: Usage,
}

/// Everything a run needs that outlives one request.
pub struct Engine {
    policy: Policy,
    /// Where each specialist's capability server lives.
    servers: ServerBinaries,
    /// One conversation store for the Engine's whole life.
    ///
    /// This was created fresh for every request, which meant each thing the User said began
    /// a new conversation: they could say their name and be asked for it again in the next
    /// breath. A conversation is a conversation.
    sessions: Arc<dyn adk_session::SessionService>,
    /// Threads whose earlier turns have already been given back to the model, so a restart
    /// is caught up exactly once rather than on every message.
    caught_up: std::sync::Mutex<std::collections::BTreeSet<String>>,
    /// Where what the User has told Work Studio is kept. Absent means the specialist has no
    /// way to remember, and it is told so rather than left to promise otherwise.
    remembers: Option<Arc<dyn crate::memory::Remembers>>,
    /// What the User has allowed each specialist to reach. Absent falls back to the
    /// connections provisioned beside the app.
    provides: Option<Arc<dyn Provides>>,
}

/// One connection a specialist may use, as the Core resolved it.
#[derive(Debug, Clone, PartialEq)]
pub struct Allocated {
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::BTreeMap<String, String>,
}

/// What a specialist has been allocated.
///
/// A trait so the runner does not depend on the store. Without one, the specialists fall back
/// to the connections provisioned beside the app — which is what happens before the User has
/// been near Settings.
pub trait Provides: Send + Sync {
    fn for_agent(&self, agent: &str) -> Vec<Allocated>;
}

/// The binaries the specialists need. Provisioned beside the app in a release.
#[derive(Debug, Clone, Default)]
pub struct ServerBinaries {
    pub spreadsheet: Option<std::path::PathBuf>,
    pub document: Option<std::path::PathBuf>,
    pub presentation: Option<std::path::PathBuf>,
}

impl ServerBinaries {
    /// The binaries as found beside the sibling checkouts, for development.
    pub fn from_siblings(root: &std::path::Path) -> Self {
        let at = |relative: &str| {
            let path = root.join(relative);
            path.exists().then_some(path)
        };
        Self {
            spreadsheet: at("mcp-servers/worksheet-mcp/target/debug/excel-mcp-server"),
            document: at("mcp-servers/docx-mcp/target/debug/docx-mcp-server"),
            presentation: at("mcp-servers/mcp-slides/target/debug/slides-mcp-server"),
        }
    }

    fn for_kind(&self, kind: ArtefactKind) -> Option<&std::path::Path> {
        match kind {
            ArtefactKind::Spreadsheet => self.spreadsheet.as_deref(),
            ArtefactKind::Document => self.document.as_deref(),
            ArtefactKind::Presentation => self.presentation.as_deref(),
        }
    }
}

/// The file's own name.
///
/// The full path is what the tools need, not what the conversation needs. Handing over the
/// path invited exactly the wrong kind of inference: asked for their name, the specialist
/// read it out of `/Users/...` and offered it back as a guess about the person.
fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("this file"))
}

/// Where the content actually is, in terms the specialist can act on.
///
/// Cheap to compute and prevents a whole class of confident mistake: a specialist that
/// assumes A1 will write against empty cells and produce a column of zeros.
fn describe_artefact(path: &std::path::Path) -> String {
    match studio_sheets::read(path, studio_sheets::Window::default()) {
        Ok(model) => {
            let mut lines = vec![String::from("What is in it:")];
            for sheet in &model.sheets {
                let first_row = sheet.first_row + 1;
                let last_row = sheet.first_row + sheet.rows.len() as u32;
                let headers: Vec<String> = sheet
                    .rows
                    .first()
                    .map(|row| row.iter().map(|cell| cell.display.clone()).collect())
                    .unwrap_or_default();
                lines.push(format!(
                    "- sheet \"{}\": rows {first_row} to {last_row}, first column {}. \
                     The first row of content reads: {}",
                    sheet.name,
                    column_name(sheet.first_col as u32),
                    if headers.is_empty() {
                        "nothing".to_string()
                    } else {
                        headers.join(", ")
                    }
                ));
            }
            lines.push(String::from(
                "Write into the rows named above; do not assume the table starts at row 1.",
            ));
            lines.join("\n")
        }
        // Not being able to describe it is not a reason to refuse the work.
        Err(_) => String::new(),
    }
}

/// A column's letter, as the User would say it.
fn column_name(index: u32) -> String {
    let mut name = String::new();
    let mut n = index;
    loop {
        name.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    name
}

/// What the User typed, as the kind of thing they meant.
///
/// A spreadsheet is not a text editor: typing 1999 into a cell means the number, and a
/// number stored as text breaks every formula that refers to it. Sending the text verbatim
/// did exactly that.
fn typed_value(text: &str) -> serde_json::Value {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return serde_json::Value::String(String::new());
    }
    // A formula stays a string; the engine recognises the leading sign.
    if trimmed.starts_with('=') {
        return serde_json::Value::String(trimmed.to_string());
    }
    if let Ok(number) = trimmed.replace(',', "").parse::<f64>()
        && let Some(value) = serde_json::Number::from_f64(number)
    {
        return serde_json::Value::Number(value);
    }
    if trimmed.eq_ignore_ascii_case("true") {
        return serde_json::Value::Bool(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return serde_json::Value::Bool(false);
    }
    serde_json::Value::String(trimmed.to_string())
}

/// Pull a handle out of whatever shape a server answered in.
///
/// Each server nests its answer differently and the handle is what every later operation
/// refers to, so it is worth finding wherever it sits.
fn find_handle(answer: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(answer).ok()?;
    fn search(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::Object(map) => {
                for key in ["workbook_id", "handle"] {
                    if let Some(found) = map.get(key) {
                        return match found {
                            serde_json::Value::String(text) => Some(text.clone()),
                            other => Some(other.to_string()),
                        };
                    }
                }
                map.values().find_map(search)
            }
            serde_json::Value::Array(items) => items.iter().find_map(search),
            serde_json::Value::String(text) => {
                let inner: serde_json::Value = serde_json::from_str(text).ok()?;
                search(&inner)
            }
            _ => None,
        }
    }
    search(&value)
}

impl Engine {
    pub fn new(policy: Policy, servers: ServerBinaries) -> Self {
        Self {
            policy,
            servers,
            sessions: Arc::new(adk_session::InMemorySessionService::new()),
            caught_up: std::sync::Mutex::new(std::collections::BTreeSet::new()),
            remembers: None,
            provides: None,
        }
    }

    /// The same, honouring what the User has allowed in Settings.
    pub fn providing(mut self, provides: Arc<dyn Provides>) -> Self {
        self.provides = Some(provides);
        self
    }

    /// The same, able to remember.
    pub fn remembering(mut self, remembers: Arc<dyn crate::memory::Remembers>) -> Self {
        self.remembers = Some(remembers);
        self
    }

    /// Which model this work will use.
    ///
    /// Editing an Artefact is the User waiting on an answer, so it takes the balanced
    /// tier rather than the cheapest.
    pub fn model_reference(&self) -> Option<&ModelRef> {
        self.policy.chain_for(QualityTier::Balanced).first()
    }

    /// A change the User made by hand.
    ///
    /// No model is involved — they already know what they want — but the same gate decides and
    /// the same capability writes, so a change made by hand and a change made for them are the
    /// same kind of thing on the way to the file. `where_at` and `what` name the target in the
    /// terms of the kind: a cell reference for a spreadsheet, a block index for a document, a
    /// shape index for a slide.
    /// One short answer from the model, with no tools and no file.
    ///
    /// For the one decision that has to be made before there is a file to work on: what the User
    /// is asking to have made. Deliberately narrow — no capability server is started, so this
    /// cannot change anything.
    pub async fn answer_briefly(&self, question: &str) -> Result<String, RunError> {
        use adk_core::{Content, LlmRequest, Part};

        let reference = self.model_reference().ok_or(RunError::NoModel)?;
        let model = model_for(reference)?;

        let request = LlmRequest {
            model: reference.model.clone(),
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text {
                    text: question.to_string(),
                }],
            }],
            config: None,
            tools: std::collections::HashMap::new(),
            previous_response_id: None,
        };

        // Not streamed: this is one word, and the caller is holding the User's request while it
        // waits.
        let mut stream = model
            .generate_content(request, false)
            .await
            .map_err(|error| RunError::ModelUnusable {
                detail: error.to_string(),
            })?;

        let mut said = String::new();
        while let Some(next) = futures::StreamExt::next(&mut stream).await {
            let response = next.map_err(|error| RunError::ModelUnusable {
                detail: error.to_string(),
            })?;
            if let Some(content) = response.content.as_ref() {
                for part in &content.parts {
                    if let Part::Text { text } = part {
                        said.push_str(text);
                    }
                }
            }
        }

        Ok(said)
    }

    /// Make an empty Artefact of this kind at this path.
    ///
    /// Creation goes through the gate and the same capability server as every other change,
    /// rather than a file being written beside the one path the product authorises. It is the
    /// first half of starting work from a sentence: the specialist is then asked to fill it in
    /// through the ordinary run, so there is no second, quieter way to change a file.
    ///
    /// Two steps, not one. Every server creates in memory and saves separately, and none of them
    /// takes a path when creating — `create_workbook` accepts no arguments at all. Asking for a
    /// file at a path in one call reported success and left nothing on disk.
    pub async fn start_new(&self, path: &std::path::Path) -> Result<(), RunError> {
        let kind = ArtefactKind::of(path).ok_or(RunError::UnknownKind)?;
        let binary = self.command_for(kind)?;

        let server = Server::start(kind.server_spec(binary.to_string_lossy()))
            .await
            .map_err(|detail| RunError::ServerUnavailable { detail })?;

        // Each server spells this differently, which is exactly the kind of difference that gets
        // discovered at run time when it is written down in one place and assumed in another.
        let (create, save, handle_key, path_key) = match kind {
            ArtefactKind::Spreadsheet => (
                "create_workbook",
                "save_workbook",
                "workbook_id",
                "file_path",
            ),
            ArtefactKind::Document => (
                "create_document",
                "save_document",
                "document_handle",
                "output_path",
            ),
            ArtefactKind::Presentation => (
                "create_presentation",
                "save_presentation",
                "handle",
                "output_path",
            ),
        };

        let classifier = kind.classifier();
        for operation in [create, save] {
            let decision = studio_gate::decide(
                &classifier,
                kind.server_name(),
                operation,
                JobKind::OneOff,
                JobState::Active,
                RunMode::Live,
                false,
            );
            if matches!(decision, studio_gate::Decision::Suppress { .. }) {
                return Err(RunError::NotAllowed {
                    detail: format!("the gate refused {operation}"),
                });
            }
        }

        // `create_workbook` takes no arguments and refuses any it is given; the other two accept
        // an optional format, and "blank" is what an empty one is called.
        let making = match kind {
            ArtefactKind::Spreadsheet => serde_json::json!({}),
            _ => serde_json::json!({ "format": "blank" }),
        };
        let made = server
            .call(create, making)
            .await
            .map_err(|detail| RunError::Failed { detail })?;
        let handle = find_handle(&made).ok_or_else(|| RunError::Failed {
            detail: format!("no handle in the answer to {create}: {made}"),
        })?;

        server
            .call(
                save,
                serde_json::json!({
                    handle_key: handle,
                    path_key: path.to_string_lossy(),
                }),
            )
            .await
            .map_err(|detail| RunError::Failed { detail })?;

        // A server reporting success is not evidence the file is there. This check is why the
        // first attempt was caught: create_document answered "success" and wrote nothing.
        if !path.exists() {
            return Err(RunError::Failed {
                detail: format!("{save} reported success and wrote no file"),
            });
        }
        Ok(())
    }

    /// Change how a range looks, by hand.
    ///
    /// Formatting is a change to the file like any other, so it goes through the gate and the
    /// change log rather than round the side. Only what the User actually chose is sent: an
    /// absent field leaves the file's own formatting alone, so making something bold does not
    /// quietly reset its colour.
    pub async fn format_by_hand(
        &self,
        path: &str,
        sheet: &str,
        range: &str,
        how: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), RunError> {
        let file = std::path::Path::new(path);
        let kind = ArtefactKind::of(file).ok_or(RunError::UnknownKind)?;
        if kind != ArtefactKind::Spreadsheet {
            return Err(RunError::UnknownKind);
        }
        let binary = self.command_for(kind)?;
        let server = Server::start(kind.server_spec(binary.to_string_lossy()))
            .await
            .map_err(|detail| RunError::ServerUnavailable { detail })?;

        let classifier = kind.classifier();
        for operation in ["set_cell_format", "save_workbook"] {
            let decision = studio_gate::decide(
                &classifier,
                kind.server_name(),
                operation,
                JobKind::OneOff,
                JobState::Active,
                RunMode::Live,
                false,
            );
            if matches!(decision, studio_gate::Decision::Suppress { .. }) {
                return Err(RunError::NotAllowed {
                    detail: format!("the gate refused {operation}"),
                });
            }
        }

        let opened = server
            .call(
                "open_workbook",
                serde_json::json!({ "file_path": path, "read_only": false }),
            )
            .await
            .map_err(|detail| RunError::Failed { detail })?;
        let handle = find_handle(&opened).ok_or_else(|| RunError::Failed {
            detail: format!("no handle in the answer to open_workbook: {opened}"),
        })?;

        let mut arguments = how.clone();
        arguments.insert("workbook_id".to_string(), serde_json::json!(handle));
        arguments.insert("sheet_name".to_string(), serde_json::json!(sheet));
        arguments.insert("range".to_string(), serde_json::json!(range));

        server
            .call("set_cell_format", serde_json::Value::Object(arguments))
            .await
            .map_err(|detail| RunError::Failed { detail })?;

        server
            .call(
                "save_workbook",
                serde_json::json!({ "workbook_id": handle, "file_path": path }),
            )
            .await
            .map_err(|detail| RunError::Failed { detail })?;
        Ok(())
    }

    /// A structural change to a spreadsheet, by hand.
    ///
    /// Inserting a row, sorting a range, freezing the headings: the things a person does to a
    /// spreadsheet that are not typing in a cell. Named actions rather than a way to send any
    /// operation through, because a passthrough would put the whole capability surface behind one
    /// door and the gate's classification would be the only thing between the User and 93
    /// operations they never asked for.
    ///
    /// Each still goes through the gate and is saved the same way, so there remains one path
    /// into the file.
    pub async fn act_on_sheet(
        &self,
        path: &str,
        sheet: &str,
        action: SheetAction,
    ) -> Result<(), RunError> {
        let file = std::path::Path::new(path);
        let kind = ArtefactKind::of(file).ok_or(RunError::UnknownKind)?;
        if kind != ArtefactKind::Spreadsheet {
            return Err(RunError::UnknownKind);
        }
        let binary = self.command_for(kind)?;
        let server = Server::start(kind.server_spec(binary.to_string_lossy()))
            .await
            .map_err(|detail| RunError::ServerUnavailable { detail })?;

        let operation = action.operation();
        let classifier = kind.classifier();
        for gated in [operation, "save_workbook"] {
            let decision = studio_gate::decide(
                &classifier,
                kind.server_name(),
                gated,
                JobKind::OneOff,
                JobState::Active,
                RunMode::Live,
                false,
            );
            if matches!(decision, studio_gate::Decision::Suppress { .. }) {
                return Err(RunError::NotAllowed {
                    detail: format!("the gate refused {gated}"),
                });
            }
        }

        let opened = server
            .call(
                "open_workbook",
                serde_json::json!({ "file_path": path, "read_only": false }),
            )
            .await
            .map_err(|detail| RunError::Failed { detail })?;
        let handle = find_handle(&opened).ok_or_else(|| RunError::Failed {
            detail: format!("no handle in the answer to open_workbook: {opened}"),
        })?;

        let mut arguments = action.arguments(sheet);
        if let Some(object) = arguments.as_object_mut() {
            object.insert("workbook_id".to_string(), serde_json::json!(handle));
        }

        server
            .call(operation, arguments)
            .await
            .map_err(|detail| RunError::Failed { detail })?;

        server
            .call(
                "save_workbook",
                serde_json::json!({ "workbook_id": handle, "file_path": path }),
            )
            .await
            .map_err(|detail| RunError::Failed { detail })?;
        Ok(())
    }

    /// Change one thing by hand.
    pub async fn edit_by_hand(
        &self,
        path: &str,
        where_at: &str,
        what: &str,
        value: &str,
    ) -> Result<(), RunError> {
        self.edit_many_by_hand(path, where_at, &[(what.to_string(), value.to_string())])
            .await
    }

    /// Change several cells by hand in one pass.
    ///
    /// Pasting a block is one change to the User, and doing it a cell at a time would open and
    /// save the file once per cell — slow, and a dozen entries in the history for one action.
    /// Only spreadsheets can take more than one at a time; the others fall back to the first,
    /// because a document paragraph and a slide shape are addressed one at a time by nature.
    pub async fn edit_many_by_hand(
        &self,
        path: &str,
        where_at: &str,
        changes: &[(String, String)],
    ) -> Result<(), RunError> {
        let Some((what, value)) = changes.first() else {
            return Ok(());
        };
        let (what, value) = (what.as_str(), value.as_str());
        let file = std::path::Path::new(path);
        let kind = ArtefactKind::of(file).ok_or(RunError::UnknownKind)?;
        let binary = self.command_for(kind)?;

        let server = Arc::new(
            Server::start(kind.server_spec(binary.to_string_lossy()))
                .await
                .map_err(|detail| RunError::ServerUnavailable { detail })?,
        );

        // The three operations this needs, per kind: open, change, save.
        let (open, change, save, handle_key) = match kind {
            ArtefactKind::Spreadsheet => (
                "open_workbook",
                "write_cells",
                "save_workbook",
                "workbook_id",
            ),
            ArtefactKind::Document => (
                "open_document",
                "update_paragraph_text",
                "save_document",
                "document_handle",
            ),
            ArtefactKind::Presentation => (
                "open_presentation",
                "edit_run",
                "save_presentation",
                "handle",
            ),
        };

        // The gate decides before anything is written, for the User's own edit as much as for
        // an agent's.
        let classifier = kind.classifier();
        for operation in [change, save] {
            let decision = studio_gate::decide(
                &classifier,
                kind.server_name(),
                operation,
                JobKind::OneOff,
                JobState::Active,
                RunMode::Live,
                false,
            );
            if matches!(decision, studio_gate::Decision::Suppress { .. }) {
                return Err(RunError::Failed {
                    detail: format!("the gate refused {operation}: {decision:?}"),
                });
            }
        }

        // Only the spreadsheet server takes `read_only`. The presentation server rejects an
        // argument it does not know and the document server ignores it, so sending one shape
        // to all three worked for two of them and failed for the third.
        let opening = match kind {
            ArtefactKind::Spreadsheet => {
                serde_json::json!({ "file_path": path, "read_only": false })
            }
            _ => serde_json::json!({ "file_path": path }),
        };
        let opened = server
            .call(open, opening)
            .await
            .map_err(|detail| RunError::Failed { detail })?;
        let handle = find_handle(&opened).ok_or_else(|| RunError::Failed {
            detail: format!("no handle in the answer to {open}: {opened}"),
        })?;

        let arguments = match kind {
            ArtefactKind::Spreadsheet => serde_json::json!({
                handle_key: handle,
                "sheet_name": where_at,
                "cells": changes
                    .iter()
                    .map(|(cell, value)| serde_json::json!({
                        "cell": cell,
                        "value": typed_value(value),
                    }))
                    .collect::<Vec<_>>(),
            }),
            ArtefactKind::Document => serde_json::json!({
                handle_key: handle,
                // The block identifier the Core marked in the editable view.
                "index": where_at.parse::<u32>().unwrap_or(0),
                "text": value,
            }),
            ArtefactKind::Presentation => serde_json::json!({
                handle_key: handle,
                "slide": where_at.parse::<u32>().unwrap_or(0),
                "shape_idx": what.parse::<u32>().unwrap_or(0),
                "para_idx": 0,
                "run_idx": 0,
                "text": value,
            }),
        };

        server
            .call(change, arguments)
            .await
            .map_err(|detail| RunError::Failed { detail })?;

        let saving = match kind {
            ArtefactKind::Spreadsheet => {
                serde_json::json!({ handle_key: handle, "file_path": path })
            }
            _ => serde_json::json!({ handle_key: handle, "output_path": path }),
        };
        server
            .call(save, saving)
            .await
            .map_err(|detail| RunError::Failed { detail })?;
        Ok(())
    }

    /// The command a specialist should use for this kind: what the User allowed, else what was
    /// provisioned beside the app.
    fn command_for(&self, kind: ArtefactKind) -> Result<std::path::PathBuf, RunError> {
        let allocated = self
            .provides
            .as_ref()
            .map(|provides| provides.for_agent(kind.specialist_name()))
            .unwrap_or_default();
        match allocated.first() {
            Some(first) => Ok(std::path::PathBuf::from(&first.command)),
            None if self.provides.is_some() => Err(RunError::NotAllowed {
                detail: format!(
                    "the {} specialist has not been allowed anything it can use",
                    kind.specialist_name()
                ),
            }),
            None => self
                .servers
                .for_kind(kind)
                .map(|p| p.to_path_buf())
                .ok_or_else(|| RunError::ServerUnavailable {
                    detail: format!("no connection provisioned for {kind:?}"),
                }),
        }
    }

    /// Do the work the User asked for.
    pub async fn run(&self, request: &Request) -> Result<Outcome, RunError> {
        self.run_reporting(request, |_| {}).await
    }

    /// Do the work, saying what is happening as it happens.
    ///
    /// The User is waiting, and a spreadsheet edit takes the better part of a minute, so
    /// silence for that long reads as a hang.
    pub async fn run_reporting(
        &self,
        request: &Request,
        mut report: impl FnMut(&str),
    ) -> Result<Outcome, RunError> {
        let kind = ArtefactKind::of(&request.artefact).ok_or(RunError::UnknownKind)?;

        // What the User has allowed this specialist, if they have said. A connection they
        // turned off is simply not here, so turning one off in Settings takes it away rather
        // than being recorded and ignored.
        let allocated = self
            .provides
            .as_ref()
            .map(|provides| provides.for_agent(kind.specialist_name()))
            .unwrap_or_default();

        let binary: std::path::PathBuf = match allocated.first() {
            Some(first) => std::path::PathBuf::from(&first.command),
            None if self.provides.is_some() => {
                return Err(RunError::NotAllowed {
                    detail: format!(
                        "the {} specialist has not been allowed anything it can use",
                        kind.specialist_name()
                    ),
                });
            }
            None => self
                .servers
                .for_kind(kind)
                .ok_or_else(|| RunError::ServerUnavailable {
                    detail: format!("no connection provisioned for {kind:?}"),
                })?
                .to_path_buf(),
        };

        let reference = self.model_reference().ok_or(RunError::NoModel)?;
        let model = model_for(reference)?;
        report("Getting ready");

        let server = Arc::new(
            Server::start(kind.server_spec(binary.to_string_lossy()))
                .await
                .map_err(|detail| RunError::ServerUnavailable { detail })?,
        );

        // The gate, as the runtime's confirmation handler. Every operation the specialist
        // reaches for passes through the same classification the tests exercise.
        let mut resolver = Resolver::new();
        let readonly = crate::mcp::test_support::readonly_context();
        let names = server
            .operation_names(readonly)
            .await
            .map_err(|detail| RunError::ServerUnavailable { detail })?;
        for name in &names {
            resolver.declare(kind.server_name(), name);
        }

        let gate = Arc::new(GateHandler::new(
            kind.classifier(),
            resolver,
            RunContext {
                kind: JobKind::OneOff,
                state: JobState::Active,
                mode: RunMode::Live,
            },
        ));

        // The toolset the specialist is given. `Server` owns it, so it is shared rather
        // than moved: the pipeline still needs the server to answer for its operations.
        let toolset: Arc<dyn adk_core::Toolset> = server.clone().toolset_for_agent();
        let mut toolsets = vec![toolset];

        // Remembering is a thing the specialist does, not a thing it claims. Without this it
        // would say "I'll remember that" and write nothing anywhere.
        if let Some(remembers) = self.remembers.as_ref() {
            toolsets.push(Arc::new(crate::memory::MemoryTools::new(
                request.thread.clone(),
                Arc::clone(remembers),
            )) as Arc<dyn adk_core::Toolset>);
        }

        let agent = kind
            .agent(model, toolsets, &request.steering)
            .map_err(|detail| RunError::Failed { detail })?;

        // A thread is a continuing conversation, so it is created once and then reused.
        //
        // Creating it is not harmless: `create` stores a session with no events, so calling
        // it on every request wiped the conversation each time. That is why someone could
        // give their name and be asked for it again in the next breath.
        let session_service = Arc::clone(&self.sessions);
        let existing = session_service
            .get(adk_session::GetRequest {
                app_name: "work-studio".to_string(),
                user_id: "the-user".to_string(),
                session_id: request.thread.clone(),
                num_recent_events: None,
                after: None,
            })
            .await;
        if existing.is_err() {
            let _ = session_service
                .create(adk_session::CreateRequest {
                    app_name: "work-studio".to_string(),
                    user_id: "the-user".to_string(),
                    session_id: Some(request.thread.clone()),
                    state: Default::default(),
                })
                .await;
        }
        let runner = adk_runner::Runner::builder()
            .app_name("work-studio")
            .agent(agent)
            .session_service(session_service)
            .run_config(
                adk_core::RunConfig::builder()
                    .tool_confirmation_handler(
                        gate.clone() as Arc<dyn adk_core::ToolConfirmationHandler>
                    )
                    .build(),
            )
            .build()
            .map_err(|error| RunError::Failed {
                detail: error.to_string(),
            })?;

        // The file the User is looking at, said plainly, so the specialist does not have to
        // be told twice.
        // What is actually in the file, so the specialist does not have to guess. It guessed
        // wrong in a way worth preventing: it assumed the table started at row 1 when it
        // started at row 5, and wrote a column of formulas against empty cells.
        // What was said before, given back once after a restart. Within one run of the
        // application the conversation store already holds it.
        let earlier = {
            let mut caught_up = self
                .caught_up
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if caught_up.insert(request.thread.clone()) && !request.history.is_empty() {
                let mut said = String::from("Earlier in this conversation:\n");
                for turn in &request.history {
                    let who = if turn.from_user {
                        "They said"
                    } else {
                        "You said"
                    };
                    said.push_str(&format!("- {who}: {}\n", turn.text));
                }
                said
            } else {
                String::new()
            }
        };

        let opening = format!(
            "{}The file open is called {}. Open it at exactly this path: {}\n\
             That path is only how to reach the file. It says nothing about the person you \
             are working with, so do not read anything from it.\n{}\n\n{}",
            earlier,
            file_name(&request.artefact),
            request.artefact.display(),
            describe_artefact(&request.artefact),
            request.asked
        );

        let mut outcome = Outcome::default();
        let stream = runner
            .run_str(
                "the-user",
                &request.thread,
                adk_core::Content::new("user").with_text(&opening),
            )
            .await
            .map_err(|error| RunError::Failed {
                detail: error.to_string(),
            })?;

        use futures::StreamExt;
        let mut stream = stream;
        while let Some(next) = stream.next().await {
            let event = match next {
                Ok(event) => event,
                Err(error) => {
                    return Err(RunError::Failed {
                        detail: error.to_string(),
                    });
                }
            };
            // Usage arrives on the response, not on a part, and only on some of them.
            if let Some(usage) = event.llm_response.usage_metadata.as_ref() {
                outcome.usage.prompt_tokens += usage.prompt_token_count as i64;
                outcome.usage.answer_tokens += usage.candidates_token_count as i64;
            }

            let Some(content) = event.llm_response.content.as_ref() else {
                continue;
            };
            for part in &content.parts {
                match part {
                    // Text arrives a piece at a time, so the pieces are joined as they
                    // came. Trimming or separating them here turned one sentence into a
                    // column of words.
                    adk_core::Part::Text { text } => outcome.said.push_str(text),
                    adk_core::Part::FunctionCall { name, .. } => {
                        if name.contains("save") {
                            outcome.saved = true;
                        }
                        if let Some(said) = progress_for(name) {
                            report(said);
                        } else {
                            report("Working on it");
                        }
                        outcome.performed.push(name.clone());
                    }
                    _ => {}
                }
            }
        }

        outcome.said = outcome.said.trim().to_string();

        // What the gate refused is part of the answer, not a detail: the User is owed the
        // difference between what was asked and what was done.
        outcome.refused = gate
            .manifest()
            .rows
            .iter()
            .map(|row| row.description.clone())
            .collect();
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Progress is a primary surface, so it must never carry an operation name or any of
    /// the vocabulary the product forbids.
    #[test]
    fn progress_is_said_in_the_users_words() {
        for operation in [
            "open_workbook",
            "read_sheet",
            "write_cells",
            "apply_style",
            "save_workbook",
            "insert_paragraph",
            "add_slide",
            "set_chart_data",
        ] {
            let said = progress_for(operation).unwrap_or("Working on it");
            assert!(!said.contains('_'), "{operation} leaks its name: {said}");
            let lowered = said.to_lowercase();
            for banned in ["tool", "workbook", "cell", "mcp", "json", "api", "server"] {
                assert!(
                    !lowered.contains(banned),
                    "{operation} says {banned}: {said}"
                );
            }
            assert!(
                said.chars().next().unwrap().is_uppercase(),
                "{operation} should read as a sentence: {said}"
            );
        }
    }

    /// An operation nobody has described must still produce something readable, because a
    /// server can grow one at any time.
    #[test]
    fn an_undescribed_operation_still_reads_plainly() {
        assert!(progress_for("some_new_operation").is_none());
    }

    /// A run must not claim to have saved unless something saved.
    #[test]
    fn an_outcome_starts_having_saved_nothing() {
        let outcome = Outcome::default();
        assert!(!outcome.saved);
        assert!(outcome.performed.is_empty());
        assert!(outcome.refused.is_empty());
    }

    /// A spreadsheet is not a text editor: a typed number must stay a number, or every
    /// formula that refers to the cell breaks. It was being sent as text.
    #[test]
    fn what_the_user_types_keeps_its_kind() {
        assert!(typed_value("1999").is_number());
        assert!(
            typed_value(" 4,960,000 ").is_number(),
            "thousands separators are still a number"
        );
        assert!(typed_value("-2.5").is_number());
        assert_eq!(typed_value("=C6*0.3"), serde_json::json!("=C6*0.3"));
        assert_eq!(typed_value("July"), serde_json::json!("July"));
        assert_eq!(typed_value("true"), serde_json::json!(true));
        assert_eq!(typed_value(""), serde_json::json!(""));
        // A thing that only looks numeric is text, because it is.
        assert!(typed_value("1999 units").is_string());
    }

    #[test]
    fn a_column_is_named_as_the_user_would_say_it() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(3), "D");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
    }

    #[test]
    fn a_files_kind_decides_which_server_is_needed() {
        let servers = ServerBinaries {
            spreadsheet: Some("/somewhere/excel".into()),
            document: None,
            presentation: None,
        };
        assert!(servers.for_kind(ArtefactKind::Spreadsheet).is_some());
        assert!(
            servers.for_kind(ArtefactKind::Document).is_none(),
            "a server that is not provisioned must not be borrowed from another kind"
        );
    }
}
