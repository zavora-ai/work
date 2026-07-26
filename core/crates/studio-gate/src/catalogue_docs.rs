//! The document specialist's operations.
//!
//! Same rules as the spreadsheet catalogue: authored here, never read from the server's
//! own metadata, and guarded by a test that holds the complete list the server exposes so
//! a new operation cannot default to something nobody chose.

use crate::{Classifier, OperationClass, Reversibility, SideEffect};

fn reads(verb: &'static str) -> OperationClass {
    OperationClass {
        effect: SideEffect::Read,
        verb,
        reversibility: Reversibility::Irreversible {
            reason: "reading changes nothing".into(),
        },
    }
}

fn edits(verb: &'static str) -> OperationClass {
    OperationClass {
        effect: SideEffect::LocalWrite,
        verb,
        reversibility: Reversibility::Reversible {
            how: "Undo".into(),
            window_secs: None,
        },
    }
}

const READS: &[(&str, &str)] = &[
    ("open_document", "Open a document"),
    ("close_document", "Close a document"),
    ("read_paragraph", "Read a paragraph"),
    ("read_paragraphs", "Read the text"),
    ("read_table", "Read a table"),
    ("search_text", "Search the document"),
    ("describe_document", "Look over the whole document"),
    ("document_outline", "Look at the headings"),
    ("get_metadata", "Look at the document's details"),
    ("to_html", "View as a web page"),
    ("editable_html", "View for editing"),
    ("to_markdown", "View as plain formatting"),
    ("to_plain_text", "View as plain text"),
    ("render_page", "See a page as it will print"),
    ("page_layout", "Measure the page"),
    ("image_bytes", "Look at an image in the document"),
    ("list_templates", "List the templates"),
    ("list_tracked_changes", "List the tracked changes"),
    ("audit_accessibility", "Check it can be read by everyone"),
    ("layout_frames", "Measure where things sit on the page"),
];

const EDITS: &[(&str, &str)] = &[
    // lifecycle
    ("create_document", "Start a document"),
    ("save_document", "Save"),
    ("save_pdf", "Save a PDF copy"),
    ("merge_documents", "Merge in another document"),
    ("create_novel", "Start from a long-form template"),
    ("add_chart", "Add a chart"),
    ("add_equation", "Add an equation"),
    // text
    ("insert_paragraph", "Add a paragraph"),
    ("update_paragraph_text", "Change a paragraph"),
    ("insert_run", "Add formatted text"),
    ("set_paragraph_runs", "Rewrite a paragraph's text"),
    ("replace_text", "Replace text"),
    ("replace_regex", "Replace by pattern"),
    ("delete_content", "Delete something"),
    ("insert_code_block", "Add a code block"),
    ("insert_callout", "Add a callout"),
    ("insert_scene_break", "Add a scene break"),
    ("insert_chapter_opening", "Start a chapter"),
    ("add_text_effect", "Add a text effect"),
    ("add_theme_colored_text", "Add coloured text"),
    ("set_drop_cap", "Add a drop cap"),
    // structure and formatting
    ("set_paragraph_style", "Apply a style"),
    ("set_paragraph_format", "Format a paragraph"),
    ("set_document_settings", "Change a document setting"),
    ("set_theme", "Apply a theme"),
    ("set_page_layout", "Change the page layout"),
    ("add_section_break", "Add a section break"),
    ("set_header_footer", "Set the header or footer"),
    ("set_line_numbering", "Add line numbers"),
    ("add_page_background", "Add a page background"),
    ("set_watermark", "Add a watermark"),
    ("set_metadata", "Change the document's details"),
    ("embed_font", "Embed a font"),
    // tables
    ("add_table", "Add a table"),
    ("insert_table_with_data", "Add a table of data"),
    ("add_table_row", "Add a row"),
    ("format_table", "Format a table"),
    ("format_table_cell", "Format a cell"),
    ("set_table_cell", "Change a cell"),
    ("merge_table_cells", "Merge cells"),
    ("style_table_banded", "Band the table rows"),
    // lists
    ("add_list", "Add a list"),
    ("add_custom_list", "Add a list"),
    // objects
    ("add_image", "Add an image"),
    ("add_shape", "Add a shape"),
    ("add_text_box", "Add a text box"),
    ("add_building_block", "Add a reusable block"),
    ("add_content_control", "Add a content control"),
    ("add_custom_xml", "Add embedded data"),
    // references
    ("add_hyperlink", "Add a link"),
    ("add_bookmark", "Add a bookmark"),
    ("cross_reference", "Add a cross-reference"),
    ("add_field", "Add a field"),
    ("add_toc", "Add a table of contents"),
    ("add_footnote", "Add a footnote"),
    ("add_footnote_ref", "Add a footnote reference"),
    // comments and review
    ("add_comment", "Add a comment"),
    ("add_comment_range", "Comment on a passage"),
    ("reply_to_comment", "Reply to a comment"),
    ("resolve_comment", "Resolve a comment"),
    ("add_tracked_insert", "Add tracked text"),
    ("add_tracked_delete", "Delete with tracking"),
    (
        "resolve_tracked_changes",
        "Accept or reject tracked changes",
    ),
    // forms and protection
    ("add_form_text_field", "Add a form field"),
    ("add_form_checkbox", "Add a checkbox"),
    ("add_form_dropdown", "Add a dropdown"),
    ("protect_document", "Protect the document"),
    ("unprotect_document", "Remove protection"),
];

/// The document specialist's operations.
///
/// As with spreadsheets, nothing here acts outside this computer: a document specialist
/// can change the User's file and nothing else. The test below asserts it.
pub fn document() -> Classifier {
    let mut classifier = Classifier::new();
    for (name, verb) in READS {
        classifier.insert("document", name, reads(verb));
    }
    for (name, verb) in EDITS {
        classifier.insert("document", name, edits(verb));
    }
    classifier
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every operation docx-mcp exposes. Kept literal so the build fails when the server
    /// grows one.
    const EXPOSED: &[&str] = &[
        "add_bookmark",
        "add_building_block",
        "add_chart",
        "add_comment",
        "add_comment_range",
        "add_content_control",
        "add_custom_list",
        "add_custom_xml",
        "add_equation",
        "add_field",
        "add_footnote",
        "add_footnote_ref",
        "add_form_checkbox",
        "add_form_dropdown",
        "add_form_text_field",
        "add_hyperlink",
        "add_image",
        "add_list",
        "add_page_background",
        "add_section_break",
        "add_shape",
        "add_table",
        "add_table_row",
        "add_text_box",
        "add_text_effect",
        "add_theme_colored_text",
        "add_toc",
        "add_tracked_delete",
        "add_tracked_insert",
        "audit_accessibility",
        "close_document",
        "create_document",
        "create_novel",
        "cross_reference",
        "delete_content",
        "describe_document",
        "document_outline",
        "editable_html",
        "embed_font",
        "format_table",
        "format_table_cell",
        "get_metadata",
        "image_bytes",
        "insert_callout",
        "insert_chapter_opening",
        "insert_code_block",
        "insert_paragraph",
        "insert_run",
        "insert_scene_break",
        "insert_table_with_data",
        "layout_frames",
        "list_templates",
        "list_tracked_changes",
        "merge_documents",
        "merge_table_cells",
        "open_document",
        "page_layout",
        "protect_document",
        "read_paragraph",
        "read_paragraphs",
        "read_table",
        "render_page",
        "replace_regex",
        "replace_text",
        "reply_to_comment",
        "resolve_comment",
        "resolve_tracked_changes",
        "save_document",
        "save_pdf",
        "search_text",
        "set_document_settings",
        "set_drop_cap",
        "set_header_footer",
        "set_line_numbering",
        "set_metadata",
        "set_page_layout",
        "set_paragraph_format",
        "set_paragraph_runs",
        "set_paragraph_style",
        "set_table_cell",
        "set_theme",
        "set_watermark",
        "style_table_banded",
        "to_html",
        "to_markdown",
        "to_plain_text",
        "unprotect_document",
        "update_paragraph_text",
    ];

    #[test]
    fn every_exposed_operation_is_classified() {
        let classifier = document();
        let missing: Vec<&str> = EXPOSED
            .iter()
            .copied()
            .filter(|name| classifier.get("document", name).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "these operations have no classification, so they would be refused as unknown: {missing:?}"
        );
    }

    #[test]
    fn nothing_in_the_catalogue_is_absent_from_the_server() {
        for (name, _) in READS.iter().chain(EDITS.iter()) {
            assert!(
                EXPOSED.contains(name),
                "{name} is classified but the server does not expose it — the catalogue has drifted"
            );
        }
    }

    #[test]
    fn the_document_specialist_cannot_act_outside_this_computer() {
        let classifier = document();
        for name in EXPOSED {
            assert_ne!(
                classifier.effect_of("document", name),
                SideEffect::ExternalEffect,
                "{name} is classified as acting outside this computer, which a document \
                 specialist must never do"
            );
        }
    }

    /// Viewing a document is reading it, however elaborate the view.
    #[test]
    fn every_way_of_viewing_a_document_is_a_read() {
        let classifier = document();
        for name in [
            "to_html",
            "editable_html",
            "to_markdown",
            "to_plain_text",
            "render_page",
            "document_outline",
            "layout_frames",
        ] {
            assert_eq!(
                classifier.effect_of("document", name),
                SideEffect::Read,
                "{name} only looks at the document"
            );
        }
    }

    /// Saving a PDF writes a new file, so it is a change the User can undo.
    #[test]
    fn saving_in_any_form_is_a_change() {
        let classifier = document();
        for name in ["save_document", "save_pdf", "merge_documents"] {
            assert_eq!(
                classifier.effect_of("document", name),
                SideEffect::LocalWrite
            );
        }
    }

    #[test]
    fn every_operation_reads_as_plain_language() {
        let classifier = document();
        for name in EXPOSED {
            let verb = classifier.get("document", name).expect("classified").verb;
            assert!(!verb.is_empty(), "{name} has no plain-language verb");
            assert!(
                verb.chars().next().is_some_and(char::is_uppercase),
                "{name}'s verb {verb:?} should read as a sentence opener"
            );
            assert!(
                !verb.contains('_'),
                "{name}'s verb {verb:?} still looks like an identifier"
            );
        }
    }

    /// The two catalogues must not disagree about the same idea.
    #[test]
    fn a_specialist_only_knows_its_own_operations() {
        let documents = document();
        let spreadsheets = crate::catalogue::worksheet();
        assert!(
            documents.get("worksheet", "write_formula").is_none(),
            "a document specialist must not be handed spreadsheet operations"
        );
        assert!(
            spreadsheets.get("document", "insert_paragraph").is_none(),
            "a spreadsheet specialist must not be handed document operations"
        );
    }
}
