//! The document and presentation paths, against the real servers.
//!
//! No model is involved, which is the point: this proves the mechanical half — gate,
//! server, file on disk, change log — works for the other two artefact kinds too, so the
//! only new variable when a model arrives is the model.
//!
//! Skipped when a server has not been built, so a checkout without the sibling
//! repositories still passes. Build them with:
//!
//! ```sh
//! cargo build --bin docx-mcp-server      # in mcp-servers/docx-mcp
//! cargo build                            # in mcp-servers/mcp-slides
//! ```

#![cfg(feature = "adk")]

use std::path::PathBuf;
use std::sync::Arc;

use studio_gate::RunMode;
use studio_jobs::{JobKind, JobState};
use studio_runner::mcp::{Server, ServerSpec};

fn binary(relative: &str) -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../")
        .join(relative);
    path.exists().then_some(path)
}

fn document_server() -> Option<PathBuf> {
    binary("mcp-servers/docx-mcp/target/debug/docx-mcp-server")
}

fn presentation_server() -> Option<PathBuf> {
    binary("mcp-servers/mcp-slides/target/debug/slides-mcp-server")
}

/// Pull a handle out of whatever shape the server answered in.
///
/// Each server nests its answer differently; the handle is what every later operation
/// refers to, so it is worth finding wherever it sits.
fn find_handle(answer: &str, key: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(answer).ok()?;
    fn search(value: &serde_json::Value, key: &str) -> Option<String> {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(found) = map.get(key) {
                    return match found {
                        serde_json::Value::String(text) => Some(text.clone()),
                        other => Some(other.to_string()),
                    };
                }
                map.values().find_map(|v| search(v, key))
            }
            serde_json::Value::Array(items) => items.iter().find_map(|v| search(v, key)),
            serde_json::Value::String(text) => {
                let inner: serde_json::Value = serde_json::from_str(text).ok()?;
                search(&inner, key)
            }
            _ => None,
        }
    }
    search(&value, key)
}

/// A change the User asked for, in a live Job, must actually be performed.
fn performs(decision: &studio_gate::Decision) -> bool {
    matches!(
        decision,
        studio_gate::Decision::Permit
            | studio_gate::Decision::PermitAndRecord
            | studio_gate::Decision::PermitAndDeliver { .. }
    )
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("zws-artefact-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The classification must describe the server the User will actually run, not the one
/// that existed when it was written. A server that has grown an operation nobody
/// classified would make work fail for a reason the User cannot act on.
#[tokio::test(flavor = "multi_thread")]
async fn the_real_document_server_exposes_only_operations_we_classified() {
    let Some(binary) = document_server() else {
        eprintln!("skipping: document server not built");
        return;
    };

    let server = Server::start(ServerSpec::document(binary.to_string_lossy()))
        .await
        .expect("the document server should start");
    let ctx = studio_runner::mcp::test_support::readonly_context();
    let exposed = server
        .operation_names(ctx)
        .await
        .expect("the server should list its operations");

    assert!(
        exposed.len() > 50,
        "expected the full document surface, got {}",
        exposed.len()
    );

    let classifier = studio_gate::catalogue_docs::document();
    let unclassified: Vec<&String> = exposed
        .iter()
        .filter(|name| classifier.get("document", name).is_none())
        .collect();
    assert!(
        unclassified.is_empty(),
        "the document server has operations nobody classified: {unclassified:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_real_presentation_server_exposes_only_operations_we_classified() {
    let Some(binary) = presentation_server() else {
        eprintln!("skipping: presentation server not built");
        return;
    };

    let server = Server::start(ServerSpec::presentation(binary.to_string_lossy()))
        .await
        .expect("the presentation server should start");
    let ctx = studio_runner::mcp::test_support::readonly_context();
    let exposed = server
        .operation_names(ctx)
        .await
        .expect("the server should list its operations");

    assert!(
        exposed.len() > 50,
        "expected the full presentation surface, got {}",
        exposed.len()
    );

    let classifier = studio_gate::catalogue_slides::presentation();
    let unclassified: Vec<&String> = exposed
        .iter()
        .filter(|name| classifier.get("presentation", name).is_none())
        .collect();
    assert!(
        unclassified.is_empty(),
        "the presentation server has operations nobody classified: {unclassified:?}"
    );
}

/// A change asked for in a live Job reaches the file, and the Core can read back what was
/// written — the same proof the spreadsheet path already carries.
#[tokio::test(flavor = "multi_thread")]
async fn a_document_change_passes_the_gate_and_reaches_the_file() {
    let Some(binary) = document_server() else {
        eprintln!("skipping: document server not built");
        return;
    };

    let dir = temp_dir("docx");
    let path = dir.join("letter.docx");
    {
        let mut doc = zavora_docx::Document::new();
        doc.add_paragraph("8. Termination").style("Heading1");
        doc.add_paragraph("Either party may terminate on notice.");
        doc.save(&path).expect("the fixture should save");
    }

    let server = Arc::new(
        Server::start(ServerSpec::document(binary.to_string_lossy()))
            .await
            .expect("the document server should start"),
    );
    let opened = server
        .call(
            "open_document",
            serde_json::json!({ "file_path": path.to_string_lossy() }),
        )
        .await
        .expect("the document should open");
    let id = find_handle(&opened, "handle")
        .unwrap_or_else(|| panic!("no document handle in the server's answer: {opened}"));

    // The gate decides before anything is called.
    let classifier = studio_gate::catalogue_docs::document();
    let class = classifier
        .get("document", "insert_paragraph")
        .expect("classified");
    assert_eq!(class.effect, studio_gate::SideEffect::LocalWrite);
    let decision = studio_gate::decide(
        &classifier,
        "document",
        "insert_paragraph",
        JobKind::OneOff,
        JobState::Active,
        RunMode::Live,
        false,
    );
    assert!(
        performs(&decision),
        "a classified local change in a live Job should be performed: {decision:?}"
    );

    server
        .call(
            "insert_paragraph",
            serde_json::json!({
                "document_handle": id,
                "index": 2,
                "text": "Notice must be given in writing."
            }),
        )
        .await
        .expect("the paragraph should be added");
    server
        .call(
            "save_document",
            serde_json::json!({ "document_handle": id, "output_path": path.to_string_lossy() }),
        )
        .await
        .expect("the document should save");

    // Read it back through the Core's own reader, not the server's, so this proves the
    // change is in the file rather than in the server's memory.
    let model = studio_docs::read(&path).expect("the Core should read the document back");
    assert!(
        model.html.contains("Notice must be given in writing."),
        "the change should be in the file: {}",
        model.html
    );
    assert!(
        model.block_count >= 3,
        "the document should have grown a paragraph, got {}",
        model.block_count
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// The same for a deck, and additionally that what the interface would click on resolves
/// to the shape the server accepts.
#[tokio::test(flavor = "multi_thread")]
async fn a_deck_change_passes_the_gate_and_reaches_the_file() {
    let Some(binary) = presentation_server() else {
        eprintln!("skipping: presentation server not built");
        return;
    };

    let dir = temp_dir("pptx");
    let path = dir.join("deck.pptx");
    {
        use zavora_slide::{Emu, Layout, Presentation};
        let mut pres = Presentation::new();
        let index = pres.add_slide(Layout::Blank);
        pres.slide_mut(index).unwrap().add_text_box(
            "Revenue by region",
            Emu(914_400),
            Emu(914_400),
            Emu(6_400_800),
            Emu(1_000_000),
        );
        pres.save(&path).expect("the fixture should save");
    }

    // What the interface would hand back after a click.
    let before = studio_decks::read(&path).expect("the Core should read the deck");
    let slide = before.active_slide().expect("a first slide");
    let target = slide
        .target_at(0)
        .expect("the drawn element must resolve to something changeable");
    let studio_decks::Target::Shape(shape_index) = target else {
        panic!("a text box should resolve to a shape, got {target:?}");
    };

    let server = Arc::new(
        Server::start(ServerSpec::presentation(binary.to_string_lossy()))
            .await
            .expect("the presentation server should start"),
    );
    let opened = server
        .call(
            "open_presentation",
            serde_json::json!({ "file_path": path.to_string_lossy() }),
        )
        .await
        .expect("the deck should open");
    let id = find_handle(&opened, "handle")
        .unwrap_or_else(|| panic!("no deck handle in the server's answer: {opened}"));

    let classifier = studio_gate::catalogue_slides::presentation();
    let decision = studio_gate::decide(
        &classifier,
        "presentation",
        "add_paragraph",
        JobKind::OneOff,
        JobState::Active,
        RunMode::Live,
        false,
    );
    assert!(
        performs(&decision),
        "a classified change should be performed"
    );

    // The shape index the interface resolved is the one the server takes.
    server
        .call(
            "add_paragraph",
            serde_json::json!({
                "handle": id,
                "slide": 0,
                "shape_idx": shape_index,
                "text": "Up 14% on last quarter"
            }),
        )
        .await
        .expect("the paragraph should be added to the shape the User clicked");
    server
        .call(
            "save_presentation",
            serde_json::json!({ "handle": id, "output_path": path.to_string_lossy() }),
        )
        .await
        .expect("the deck should save");

    let after = studio_decks::read(&path).expect("the Core should read the deck back");
    let changed = after.active_slide().expect("a first slide");
    assert!(
        changed.svg.contains("Up 14% on last quarter"),
        "the change should be in the file"
    );
    assert!(
        changed.target_at(0).is_some(),
        "the slide must still be addressable after a change"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
