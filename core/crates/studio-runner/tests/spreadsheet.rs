//! The spreadsheet path, against the real server.
//!
//! No model is involved. That is the point: this proves the mechanical half — gate,
//! server, file, change log — works end to end, so that when a model is added the only
//! new variable is the model.
//!
//! Skipped when the spreadsheet server has not been built, so a checkout without the
//! sibling repositories still passes. Build it with:
//!
//! ```sh
//! cargo build --bin excel-mcp-server   # in mcp-servers/worksheet-mcp
//! ```

#![cfg(feature = "adk")]

use std::path::PathBuf;
use std::sync::Arc;

use studio_artefacts::{Artefacts, Author};
use studio_gate::RunMode;
use studio_jobs::{JobKind, JobState};
use studio_runner::edits::{Dispatcher, EditError, ProposedEdit};
use studio_runner::mcp::{McpApplier, Server, ServerSpec};
use studio_store::Store;

fn server_binary() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../mcp-servers/worksheet-mcp/target/debug/excel-mcp-server");
    path.exists().then_some(path)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("zws-mcp-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test(flavor = "multi_thread")]
async fn the_real_server_answers_and_its_operations_are_the_ones_we_classified() {
    let Some(binary) = server_binary() else {
        eprintln!("skipping: spreadsheet server not built");
        return;
    };

    let server = Server::start(ServerSpec::spreadsheet(binary.to_string_lossy()))
        .await
        .expect("the spreadsheet server should start");

    // Ask the server what it can do, and check our classification still describes it.
    // A server that has grown an operation we have not classified is a real risk: the
    // gate would refuse it at run time, and the User would see work fail for no reason
    // they could act on.
    let ctx = studio_runner::mcp::test_support::readonly_context();
    let exposed = server
        .operation_names(ctx)
        .await
        .expect("the server should list its operations");

    assert!(
        exposed.len() > 50,
        "expected the full spreadsheet surface, got {} operations",
        exposed.len()
    );

    let classifier = studio_gate::catalogue::worksheet();
    let unclassified: Vec<&String> = exposed
        .iter()
        .filter(|name| classifier.get("worksheet", name).is_none())
        .collect();
    assert!(
        unclassified.is_empty(),
        "the server exposes operations Work Studio has not classified, so they would be \
         refused as unknown: {unclassified:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_goes_through_the_gate_into_the_file_and_into_the_history() {
    let Some(binary) = server_binary() else {
        eprintln!("skipping: spreadsheet server not built");
        return;
    };

    let dir = temp_dir("write");
    let path = dir.join("model.xlsx");

    // Start with a real spreadsheet on disk.
    {
        let mut workbook = zavora_xlsx::Workbook::new();
        let sheet = workbook.worksheet(0).unwrap();
        sheet.set_name("Summary").unwrap();
        sheet.write(0, 0, "Month").unwrap();
        workbook.save(&path).unwrap();
    }

    let server = Arc::new(
        Server::start(ServerSpec::spreadsheet(binary.to_string_lossy()))
            .await
            .expect("server starts"),
    );

    // Open it through the server, which is how the specialist would. The handle it
    // returns is what every later operation refers to.
    let opened = server
        .call(
            "open_workbook",
            serde_json::json!({ "file_path": path.to_string_lossy(), "read_only": false }),
        )
        .await
        .expect("the server should open a real spreadsheet");
    let workbook_id = extract_workbook_id(&opened)
        .unwrap_or_else(|| panic!("no workbook handle in the server's answer: {opened}"));

    let store = Store::open_in_memory().unwrap();
    let artefacts = Artefacts::new(&store);
    artefacts.register("a1", &path, "model.xlsx", None).unwrap();

    let classifier = studio_gate::catalogue::worksheet();
    let applier = McpApplier::new(Arc::clone(&server), tokio::runtime::Handle::current());

    // The specialist writes a value, then saves. Both go through the dispatcher.
    // Compare the bytes, not their count: a change can leave a compressed archive exactly
    // as long as it was, so length is a proxy that reports success and failure alike.
    let before = std::fs::read(&path).unwrap();

    applier.with_arguments(serde_json::json!({
        "workbook_id": workbook_id,
        "sheet_name": "Summary",
        "cells": [{ "cell": "A2", "value": "July" }]
    }));
    let write = Dispatcher::new(Artefacts::new(&store), &classifier, &applier).apply(
        &ProposedEdit {
            artefact_id: "a1".into(),
            server: "worksheet".into(),
            operation: "write_cells".into(),
            description: "Added July".into(),
            author: Author::Studio,
        },
        JobKind::OneOff,
        JobState::Active,
        RunMode::Manual,
    );

    // Strict on purpose. An earlier version of this test accepted either a success or a
    // server rejection, which meant it could pass while proving nothing.
    let applied = write.expect("the server should accept a write with its own schema");
    assert_eq!(
        applied.seq, 1,
        "a successful change is the first in the history"
    );
    assert!(
        !applied.picked_up_external_change,
        "nothing else touched the file, so there was nothing to pick up"
    );

    let history = Artefacts::new(&store).history("a1").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].author, Author::Studio);
    assert_eq!(history[0].description, "Added July");

    // And saving must actually change the file on disk.
    {
        applier.with_arguments(serde_json::json!({
            "workbook_id": workbook_id,
            "file_path": path.to_string_lossy(),
        }));
        Dispatcher::new(Artefacts::new(&store), &classifier, &applier)
            .apply(
                &ProposedEdit {
                    artefact_id: "a1".into(),
                    server: "worksheet".into(),
                    operation: "save_workbook".into(),
                    description: "Saved".into(),
                    author: Author::Studio,
                },
                JobKind::OneOff,
                JobState::Active,
                RunMode::Manual,
            )
            .expect("saving should work");

        let after = std::fs::read(&path).unwrap();
        assert_ne!(
            before, after,
            "saving must actually change the file on disk"
        );

        // And reading it back must show the value the specialist wrote.
        let model = studio_sheets::read(&path, studio_sheets::Window::default())
            .expect("the saved file must still be a spreadsheet we can read");
        let sheet = model.sheet("Summary").expect("the sheet survives");
        assert_eq!(
            sheet.at(1, 0).map(|cell| cell.display.as_str()),
            Some("July"),
            "what was written through the server must be what the Core reads back"
        );
    }

    std::fs::remove_dir_all(&dir).ok();
}

/// The handle the server returns, whatever shape it wraps it in.
fn extract_workbook_id(answer: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(answer).ok()?;
    fn find(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(id) = map.get("workbook_id") {
                    return match id {
                        serde_json::Value::String(text) => Some(text.clone()),
                        other => Some(other.to_string()),
                    };
                }
                map.values().find_map(find)
            }
            serde_json::Value::Array(items) => items.iter().find_map(find),
            serde_json::Value::String(text) => {
                serde_json::from_str(text).ok().as_ref().and_then(find)
            }
            _ => None,
        }
    }
    find(&value)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unclassified_operation_never_reaches_the_server() {
    let Some(binary) = server_binary() else {
        eprintln!("skipping: spreadsheet server not built");
        return;
    };

    let dir = temp_dir("refused");
    let path = dir.join("model.xlsx");
    {
        let mut workbook = zavora_xlsx::Workbook::new();
        workbook.save(&path).unwrap();
    }

    let server = Arc::new(
        Server::start(ServerSpec::spreadsheet(binary.to_string_lossy()))
            .await
            .expect("server starts"),
    );
    let store = Store::open_in_memory().unwrap();
    Artefacts::new(&store)
        .register("a1", &path, "model.xlsx", None)
        .unwrap();

    let classifier = studio_gate::catalogue::worksheet();
    let applier = McpApplier::new(Arc::clone(&server), tokio::runtime::Handle::current());

    let result = Dispatcher::new(Artefacts::new(&store), &classifier, &applier).apply(
        &ProposedEdit {
            artefact_id: "a1".into(),
            server: "worksheet".into(),
            operation: "definitely_not_a_real_operation".into(),
            description: "should never happen".into(),
            author: Author::Studio,
        },
        JobKind::OneOff,
        JobState::Active,
        RunMode::Manual,
    );

    assert!(
        matches!(result, Err(EditError::Unclassified(_))),
        "an operation nobody classified must be refused before the server is asked"
    );
    assert!(Artefacts::new(&store).history("a1").unwrap().is_empty());

    std::fs::remove_dir_all(&dir).ok();
}
