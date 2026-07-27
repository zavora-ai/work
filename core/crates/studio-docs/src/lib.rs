//! Reading a document for the interface.
//!
//! The same argument as spreadsheets: the Core reads the file and the renderer draws
//! what it is given. Here the reason is not arithmetic but fidelity — `zavora-docx` is
//! what writes the file, so it must also be what describes it, or the two will disagree
//! about what the document contains.
//!
//! ## The identifier that makes editing possible
//!
//! A rendered document is useless for editing unless each block can be traced back to
//! the model. `zavora-docx-html` already does this: with `editable: true` it emits
//! `data-p="{body-index}"` on every block, and that index is exactly the one
//! `update_paragraph_text` and `insert_paragraph` accept.
//!
//! This matters because the plan recorded it as unbuilt. Task 13.6 called for
//! contributing `data-node-id` upstream to `zavora-docx-html` and named it a
//! prerequisite for the document client. It is already there under a different name, so
//! that work is not needed and the document specialist was never blocked on it.

use serde::{Deserialize, Serialize};
mod sections;

use zavora_docx::Document;

#[derive(Debug, thiserror::Error)]
pub enum DocError {
    #[error("that file could not be opened — it may not be a document")]
    Open { detail: String },
}

impl DocError {
    /// The underlying cause, for support. Never shown on a primary surface.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Open { detail } => Some(detail),
        }
    }
}

pub type Result<T> = std::result::Result<T, DocError>;

/// One entry in the document's own navigator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineItem {
    /// The block index — the same number the interface sends back to change it.
    pub index: usize,
    /// Heading depth, 1 being the top.
    pub level: u8,
    pub text: String,
}

/// The page, in CSS pixels, so a paginated view can be laid out.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub margin_top: f64,
    pub margin_right: f64,
    pub margin_bottom: f64,
    pub margin_left: f64,
}

/// A document, ready to render and edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocModel {
    pub file_name: String,
    /// An HTML fragment where every block carries `data-p`.
    pub html: String,
    /// Rendered header and footer, for a paginated view.
    pub header_html: String,
    pub footer_html: String,
    pub page: Page,
    pub outline: Vec<OutlineItem>,
    /// How many top-level blocks there are, so the interface can tell whether an index
    /// it holds is still in range after a change.
    pub block_count: usize,
}

/// Read a document into a model the interface can draw and edit.
pub fn read(path: &std::path::Path) -> Result<DocModel> {
    let document = Document::open(path).map_err(|e| DocError::Open {
        detail: e.to_string(),
    })?;

    let html = document.to_editable_html();
    let layout = document.page_layout();

    Ok(DocModel {
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        // A document that says what its headings are is believed. One that never used a heading
        // style still has sections a reader can see, and showing an empty list for it left the
        // User no way to move around a long document.
        outline: match outline_of(&html) {
            found if !found.is_empty() => found,
            _ => sections::inferred_sections(&html),
        },
        block_count: block_indices(&html).len(),
        header_html: layout.header_html,
        footer_html: layout.footer_html,
        page: Page {
            width: layout.page_width,
            height: layout.page_height,
            margin_top: layout.margin_top,
            margin_right: layout.margin_right,
            margin_bottom: layout.margin_bottom,
            margin_left: layout.margin_left,
        },
        html,
    })
}

/// Every block index present in a rendered fragment, in order.
///
/// Reading them back out is how the interface and the Core agree on what can be edited,
/// and how the test below proves an index survives a change elsewhere in the document.
pub fn block_indices(html: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find("data-p=\"") {
        rest = &rest[at + 8..];
        if let Some(end) = rest.find('"') {
            if let Ok(index) = rest[..end].parse::<usize>() {
                found.push(index);
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    found
}

/// The document's headings, for the navigator in the left panel.
fn outline_of(html: &str) -> Vec<OutlineItem> {
    let mut items = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find("<h") {
        rest = &rest[at + 2..];
        let Some(level) = rest.chars().next().and_then(|c| c.to_digit(10)) else {
            continue;
        };
        let Some(tag_end) = rest.find('>') else { break };
        let attributes = &rest[..tag_end];
        let index = attributes
            .find("data-p=\"")
            .and_then(|start| {
                let tail = &attributes[start + 8..];
                tail.find('"')
                    .and_then(|end| tail[..end].parse::<usize>().ok())
            })
            .unwrap_or(0);
        let body = &rest[tag_end + 1..];
        let Some(close) = body.find("</h") else { break };
        let text = strip_tags(&body[..close]);
        if !text.is_empty() {
            items.push(OutlineItem {
                index,
                level: level as u8,
                text,
            });
        }
        rest = &body[close..];
    }
    items
}

/// Text without markup. The outline shows words, not tags.
pub(crate) fn strip_tags(fragment: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for character in fragment.chars() {
        match character {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(character),
            _ => {}
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("zws-docs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn write_agreement(path: &std::path::Path) {
        let mut document = Document::new();
        // A heading is a paragraph carrying a heading style, which is how Word models it.
        document.add_paragraph("8. Termination").style("Heading1");
        document.add_paragraph("8.1 Either party may terminate for material breach.");
        document.add_paragraph("8.2 Termination shall not affect accrued rights.");
        document
            .add_paragraph("9. Confidentiality")
            .style("Heading1");
        document.add_paragraph("9.1 Each party shall keep confidential all information.");
        document.save(path).unwrap();
    }

    #[test]
    fn every_block_carries_the_index_that_can_change_it() {
        let path = fixture("agreement.docx");
        write_agreement(&path);
        let model = read(&path).expect("reads");

        assert_eq!(model.file_name, "agreement.docx");
        let indices = block_indices(&model.html);
        assert_eq!(
            indices,
            vec![0, 1, 2, 3, 4],
            "each top-level block must be addressable, in order"
        );
        assert_eq!(model.block_count, 5);
    }

    /// The point of the identifier: an index still refers to the same block after
    /// something else in the document changes.
    #[test]
    fn an_index_still_means_the_same_block_after_an_edit_elsewhere() {
        let path = fixture("stable.docx");
        write_agreement(&path);

        let before = read(&path).unwrap();
        let confidentiality = before
            .outline
            .iter()
            .find(|item| item.text.starts_with("9."))
            .expect("the second heading is in the outline")
            .index;

        // Change a paragraph before it, as an agent would.
        // The same two steps `update_paragraph_text` takes: remove the block and put a
        // new one at the same index, so the indices after it do not move.
        let mut document = Document::open(&path).unwrap();
        document.remove_content(1);
        document.insert_paragraph(1, "8.1 Either party may terminate on sixty days' notice.");
        document.save(&path).unwrap();

        let after = read(&path).unwrap();
        assert_eq!(
            after.block_count, before.block_count,
            "changing text must not change how many blocks there are"
        );
        assert_eq!(
            after
                .outline
                .iter()
                .find(|item| item.text.starts_with("9."))
                .map(|item| item.index),
            Some(confidentiality),
            "an index the interface is holding must still mean the same block"
        );
        assert!(
            after.html.contains("sixty days"),
            "and the change must be visible in what the interface draws"
        );
    }

    #[test]
    fn the_outline_is_the_documents_own_headings() {
        let path = fixture("outline.docx");
        write_agreement(&path);
        let model = read(&path).unwrap();

        let headings: Vec<&str> = model
            .outline
            .iter()
            .map(|item| item.text.as_str())
            .collect();
        assert_eq!(headings, vec!["8. Termination", "9. Confidentiality"]);
        assert!(model.outline.iter().all(|item| item.level == 1));
        assert!(
            model
                .outline
                .iter()
                .all(|item| item.index < model.block_count),
            "every outline entry must point at a block that exists"
        );
    }

    #[test]
    fn the_page_has_real_geometry_so_it_can_be_laid_out() {
        let path = fixture("page.docx");
        write_agreement(&path);
        let model = read(&path).unwrap();
        assert!(model.page.width > 100.0, "a page must have a width");
        assert!(model.page.height > model.page.width, "portrait by default");
        assert!(model.page.margin_left > 0.0);
    }

    #[test]
    fn a_file_that_is_not_a_document_says_so_without_technical_detail() {
        let path = fixture("not-a-doc.docx");
        std::fs::write(&path, b"this is not a document").unwrap();
        let error = read(&path).expect_err("must fail");
        let message = error.to_string();
        assert!(message.starts_with("that file could not be opened"));
        assert!(
            !message.to_lowercase().contains("zip") && !message.contains("EOCD"),
            "the User must never be shown the cause: {message}"
        );
        assert!(
            error.detail().is_some(),
            "but support must be able to see it"
        );
    }

    #[test]
    fn the_model_survives_a_round_trip_through_json() {
        let path = fixture("json.docx");
        write_agreement(&path);
        let model = read(&path).unwrap();
        let json = serde_json::to_string(&model).unwrap();
        let back: DocModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model, back);
        assert!(
            json.contains("\"blockCount\""),
            "the renderer reads camelCase"
        );
    }

    #[test]
    fn markup_is_stripped_from_outline_text() {
        assert_eq!(
            strip_tags("<span>8. <b>Termination</b></span>"),
            "8. Termination"
        );
        assert_eq!(strip_tags("plain"), "plain");
        assert_eq!(strip_tags("  spaced  "), "spaced");
    }

    #[test]
    fn indices_are_read_back_out_of_a_fragment() {
        assert_eq!(
            block_indices(r#"<p data-p="0">a</p><p data-p="3">b</p>"#),
            vec![0, 3]
        );
        assert_eq!(block_indices("<p>no ids</p>"), Vec::<usize>::new());
        assert_eq!(
            block_indices(r#"<p data-p="not-a-number">x</p>"#),
            Vec::<usize>::new()
        );
    }
}
