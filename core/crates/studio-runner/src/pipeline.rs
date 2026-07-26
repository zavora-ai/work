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
    #[error("Work Studio could not finish that")]
    Failed { detail: String },
}

impl RunError {
    /// The cause, for the diagnostics view only.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::ModelUnusable { detail }
            | Self::ServerUnavailable { detail }
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
}

/// Everything a run needs that outlives one request.
pub struct Engine {
    policy: Policy,
    /// Where each specialist's capability server lives.
    servers: ServerBinaries,
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
            presentation: at("mcp-servers/mcp_slides/target/debug/slides-mcp-server"),
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

impl Engine {
    pub fn new(policy: Policy, servers: ServerBinaries) -> Self {
        Self { policy, servers }
    }

    /// Which model this work will use.
    ///
    /// Editing an Artefact is the User waiting on an answer, so it takes the balanced
    /// tier rather than the cheapest.
    pub fn model_reference(&self) -> Option<&ModelRef> {
        self.policy.chain_for(QualityTier::Balanced).first()
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
        let binary = self
            .servers
            .for_kind(kind)
            .ok_or_else(|| RunError::ServerUnavailable {
                detail: format!("no server provisioned for {kind:?}"),
            })?;

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
        let agent = kind
            .agent(model, vec![toolset], &request.steering)
            .map_err(|detail| RunError::Failed { detail })?;

        let session_service: Arc<dyn adk_session::SessionService> =
            Arc::new(adk_session::InMemorySessionService::new());

        // A thread is a continuing conversation, so the session is created under the
        // thread's own name and reused. Without this the runtime has nothing to append to
        // and the first message fails.
        let _ = session_service
            .create(adk_session::CreateRequest {
                app_name: "work-studio".to_string(),
                user_id: "the-user".to_string(),
                session_id: Some(request.thread.clone()),
                state: Default::default(),
            })
            .await;
        let runner = adk_runner::Runner::builder()
            .app_name("work-studio")
            .agent(agent)
            .session_service(session_service.clone())
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
        let opening = format!(
            "The file open is {}.\n\n{}",
            request.artefact.display(),
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
