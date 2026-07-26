//! The presentation specialist's operations.
//!
//! Same rules as the other two catalogues: authored here, never read from the server's own
//! metadata, and guarded by a test holding the complete list the server exposes, so a new
//! operation fails the build rather than defaulting to something nobody chose.

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
    ("check_contrast", "Check the colours can be read"),
    ("close_presentation", "Close a deck"),
    ("describe_presentation", "Look over the whole deck"),
    ("diff_slide_render", "Compare two versions of a slide"),
    ("extract_outline", "Look at the outline"),
    ("get_doc_properties", "Look at the deck's details"),
    ("inspect_slide", "Look closely at a slide"),
    ("lint_design", "Check the design"),
    ("list_font_pairings", "List the type pairings"),
    ("list_palettes", "List the colour sets"),
    ("list_templates", "List the templates"),
    ("open_presentation", "Open a deck"),
    ("read_slide", "Read a slide"),
    ("render_slide", "See a slide as it will look"),
    ("to_markdown", "View as plain formatting"),
];

const EDITS: &[(&str, &str)] = &[
    ("add_autoshape", "Add a shape"),
    ("add_bullets", "Add bullets"),
    ("add_chart", "Add a chart"),
    ("add_connector", "Add a connector"),
    ("add_freeform", "Add a drawn shape"),
    ("add_image", "Add a picture"),
    ("add_line_break", "Add a line break"),
    ("add_paragraph", "Add a paragraph"),
    ("add_run", "Add text"),
    ("add_shape", "Add a shape"),
    ("add_slide", "Add a slide"),
    ("add_table", "Add a table"),
    ("add_text_box", "Add a text box"),
    ("apply_layout_pattern", "Apply an arrangement"),
    ("apply_theme", "Apply a look"),
    ("create_presentation", "Start a deck"),
    ("delete_paragraph", "Delete a paragraph"),
    ("delete_run", "Delete text"),
    ("delete_shape", "Delete a shape"),
    ("delete_slide", "Delete a slide"),
    ("duplicate_slide", "Duplicate a slide"),
    ("edit_run", "Change text"),
    ("format_text", "Change how text looks"),
    ("merge_cells", "Merge cells"),
    ("move_paragraph", "Move a paragraph"),
    ("move_slide", "Move a slide"),
    ("reorder_shape", "Move a shape in front or behind"),
    ("save_pdf", "Save a PDF copy"),
    ("save_presentation", "Save"),
    ("set_autofit", "Change how text fits"),
    ("set_background", "Change the background"),
    ("set_cell_style", "Change how a cell looks"),
    ("set_cell_text", "Change a cell's text"),
    ("set_chart_data", "Change a chart's figures"),
    ("set_click_action", "Set what happens on click"),
    ("set_doc_properties", "Change the deck's details"),
    ("set_footer", "Change the footer"),
    ("set_hyperlink", "Add a link"),
    ("set_image_crop", "Crop a picture"),
    ("set_image_rotation", "Turn a picture"),
    ("set_notes", "Change the speaker notes"),
    ("set_paragraph_format", "Change a paragraph's layout"),
    ("set_run_format", "Change how text looks"),
    ("set_shape_fill", "Change a shape's colour"),
    ("set_shape_geometry", "Move or resize a shape"),
    ("set_shape_line", "Change a shape's outline"),
    ("set_slide_layout", "Change a slide's arrangement"),
    ("set_slide_size", "Change the slide size"),
    ("set_table_cell", "Change a cell"),
    ("set_table_sizing", "Resize a table"),
    ("set_title", "Change the title"),
    ("split_cell", "Split a cell"),
    ("table_add_column", "Add a column"),
    ("table_add_row", "Add a row"),
    ("table_remove_column", "Delete a column"),
    ("table_remove_row", "Delete a row"),
];

pub fn presentation() -> Classifier {
    let mut classifier = Classifier::new();
    for (name, verb) in READS {
        classifier.insert("presentation", name, reads(verb));
    }
    for (name, verb) in EDITS {
        classifier.insert("presentation", name, edits(verb));
    }
    classifier
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every operation the presentation server exposes. Kept literal so the build fails
    /// when the server grows one.
    const EXPOSED: &[&str] = &[
        "add_autoshape",
        "add_bullets",
        "add_chart",
        "add_connector",
        "add_freeform",
        "add_image",
        "add_line_break",
        "add_paragraph",
        "add_run",
        "add_shape",
        "add_slide",
        "add_table",
        "add_text_box",
        "apply_layout_pattern",
        "apply_theme",
        "check_contrast",
        "close_presentation",
        "create_presentation",
        "delete_paragraph",
        "delete_run",
        "delete_shape",
        "delete_slide",
        "describe_presentation",
        "diff_slide_render",
        "duplicate_slide",
        "edit_run",
        "extract_outline",
        "format_text",
        "get_doc_properties",
        "inspect_slide",
        "lint_design",
        "list_font_pairings",
        "list_palettes",
        "list_templates",
        "merge_cells",
        "move_paragraph",
        "move_slide",
        "open_presentation",
        "read_slide",
        "render_slide",
        "reorder_shape",
        "save_pdf",
        "save_presentation",
        "set_autofit",
        "set_background",
        "set_cell_style",
        "set_cell_text",
        "set_chart_data",
        "set_click_action",
        "set_doc_properties",
        "set_footer",
        "set_hyperlink",
        "set_image_crop",
        "set_image_rotation",
        "set_notes",
        "set_paragraph_format",
        "set_run_format",
        "set_shape_fill",
        "set_shape_geometry",
        "set_shape_line",
        "set_slide_layout",
        "set_slide_size",
        "set_table_cell",
        "set_table_sizing",
        "set_title",
        "split_cell",
        "table_add_column",
        "table_add_row",
        "table_remove_column",
        "table_remove_row",
        "to_markdown",
    ];

    #[test]
    fn every_operation_the_server_exposes_is_classified() {
        let classifier = presentation();
        let missing: Vec<_> = EXPOSED
            .iter()
            .filter(|name| classifier.get("presentation", name).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "unclassified presentation operations would be refused at run time: {missing:?}"
        );
    }

    #[test]
    fn the_catalogue_invents_nothing() {
        let invented: Vec<_> = READS
            .iter()
            .chain(EDITS.iter())
            .map(|(name, _)| *name)
            .filter(|name| !EXPOSED.contains(name))
            .collect();
        assert!(
            invented.is_empty(),
            "these are not operations the server has: {invented:?}"
        );
    }

    #[test]
    fn a_presentation_specialist_reaches_nothing_outside_this_computer() {
        let classifier = presentation();
        for name in EXPOSED {
            let class = classifier.get("presentation", name).expect("classified");
            assert!(
                matches!(class.effect, SideEffect::Read | SideEffect::LocalWrite),
                "{name} would reach outside this computer"
            );
        }
    }

    /// Seeing a deck must never change it. A view that saved would make preview unsafe.
    #[test]
    fn looking_at_a_deck_never_changes_it() {
        let classifier = presentation();
        for name in [
            "read_slide",
            "render_slide",
            "inspect_slide",
            "extract_outline",
            "describe_presentation",
            "to_markdown",
            "check_contrast",
            "lint_design",
        ] {
            let class = classifier.get("presentation", name).expect("classified");
            assert_eq!(class.effect, SideEffect::Read, "{name} should only look");
        }
    }

    /// Saving in any form is a change, including to another format.
    #[test]
    fn saving_is_always_a_change() {
        let classifier = presentation();
        for name in ["save_presentation", "save_pdf"] {
            let class = classifier.get("presentation", name).expect("classified");
            assert_eq!(class.effect, SideEffect::LocalWrite, "{name} writes a file");
        }
    }

    /// The three specialists are separate: one cannot borrow another's operations.
    #[test]
    fn a_specialist_only_knows_its_own_operations() {
        let classifier = presentation();
        assert!(classifier.get("presentation", "write_cells").is_none());
        assert!(classifier.get("presentation", "insert_paragraph").is_none());
        assert!(classifier.get("worksheet", "add_slide").is_none());
    }

    /// Every operation is described in the User's words, not the server's.
    #[test]
    fn every_operation_is_described_plainly() {
        for (name, verb) in READS.iter().chain(EDITS.iter()) {
            assert!(!verb.is_empty(), "{name} has no description");
            let first = verb.chars().next().unwrap();
            assert!(
                first.is_uppercase(),
                "{name}'s description should read as a sentence: {verb}"
            );
            assert!(
                !verb.contains('_'),
                "{name}'s description reads like an operation name: {verb}"
            );
        }
    }
}
