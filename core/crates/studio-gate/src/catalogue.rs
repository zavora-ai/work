//! What each operation does to the world.
//!
//! One place, authored here, never read from a server's own metadata. A component
//! that mislabels its destructive operation must not be able to widen what Work
//! Studio will do on the User's behalf (Requirement 18.8).
//!
//! The test at the bottom is the guardrail that matters: it holds the complete list of
//! operations the spreadsheet server exposes, and fails if any of them is
//! unclassified. When the server gains a tool, the build fails until somebody decides
//! what it does — rather than the tool quietly defaulting to something.

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

/// Operations that only look at the spreadsheet.
const READS: &[(&str, &str)] = &[
    ("read_cell", "Read a cell"),
    ("read_sheet", "Read a sheet"),
    ("read_cell_format", "Look at how a cell is formatted"),
    ("read_cell_comment", "Read a comment"),
    ("read_sheet_metadata", "Look at a sheet's details"),
    ("list_sheets", "List the sheets"),
    ("get_sheet_dimensions", "Measure a sheet"),
    ("search_cells", "Search the sheet"),
    ("describe_workbook", "Look over the whole workbook"),
    ("describe_formatting", "Look at the formatting"),
    ("sheet_to_csv", "Export a sheet as plain text"),
    ("open_workbook", "Open a spreadsheet"),
    ("open_workbook_encrypted", "Open a protected spreadsheet"),
    ("set_selection", "Move the selection"),
    ("close_workbook", "Close a spreadsheet"),
];

/// Operations that change the User's own file. Permitted, and recorded in the change
/// log so every one of them can be undone.
const EDITS: &[(&str, &str)] = &[
    // workbook and sheets
    ("create_workbook", "Start a new spreadsheet"),
    ("save_workbook", "Save"),
    ("save_workbook_advanced", "Save"),
    ("configure_workbook", "Change a workbook setting"),
    ("add_sheet", "Add a sheet"),
    ("delete_sheet", "Delete a sheet"),
    ("rename_sheet", "Rename a sheet"),
    ("copy_sheet", "Copy a sheet"),
    ("move_worksheet", "Reorder a sheet"),
    ("add_chart_sheet", "Add a chart sheet"),
    ("set_sheet_settings", "Change a sheet setting"),
    ("set_visibility", "Show or hide a sheet"),
    // writing values
    ("write_cells", "Change cells"),
    ("write_row", "Write a row"),
    ("write_row_range", "Write rows"),
    ("write_column", "Write a column"),
    ("write_grid", "Write a block of cells"),
    ("write_json_rows", "Write rows"),
    ("write_rich_text", "Write formatted text"),
    ("write_formula", "Write a formula"),
    ("manage_cell", "Change a cell"),
    ("clone_column_formulas", "Copy a formula down"),
    ("fill_series", "Fill a series"),
    // formatting
    ("set_cell_format", "Format cells"),
    ("batch_format", "Format a range"),
    ("copy_format", "Copy formatting"),
    ("apply_style", "Apply a style"),
    ("apply_theme", "Apply your colours"),
    ("format_as_table_header", "Format a header row"),
    ("format_as_table_range", "Format a table"),
    ("set_row_column_format", "Format rows or columns"),
    ("autofit_columns", "Fit the columns"),
    ("set_dimensions", "Set row heights or column widths"),
    ("modify_rows", "Insert or remove rows"),
    ("modify_columns", "Insert or remove columns"),
    ("merge_cells", "Merge cells"),
    ("freeze_panes", "Freeze the headings"),
    ("group", "Group rows or columns"),
    ("set_page_setup", "Set up the printed page"),
    // structures
    ("add_table", "Add a table"),
    ("add_conditional_format", "Add a formatting rule"),
    ("add_data_validation", "Restrict what can be typed"),
    ("add_sparkline", "Add a sparkline"),
    ("add_pivot_table", "Add a pivot table"),
    ("add_slicer", "Add a slicer"),
    ("add_timeline", "Add a timeline"),
    ("manage_autofilter", "Filter the rows"),
    ("manage_named_ranges", "Name a range"),
    ("manage_defined_names", "Name a range"),
    ("add_form_control", "Add a control"),
    ("add_connection", "Add an external connection"),
    // charts
    ("add_chart", "Add a chart"),
    ("add_histogram_chart", "Add a histogram"),
    ("add_waterfall_chart", "Add a waterfall chart"),
    ("add_funnel_chart", "Add a funnel chart"),
    ("add_map_chart", "Add a map chart"),
    ("add_treemap_chart", "Add a treemap"),
    ("add_sunburst_chart", "Add a sunburst chart"),
    ("add_box_whisker_chart", "Add a box plot"),
    // objects and notes
    ("add_image", "Add an image"),
    ("add_shape", "Add a shape"),
    ("add_link", "Add a link"),
    ("manage_comments", "Add or change a comment"),
    ("add_threaded_comment", "Add a comment"),
    // data operations
    ("sort_range", "Sort"),
    ("copy_range", "Copy a range"),
    ("transpose_range", "Turn rows into columns"),
    ("find_replace", "Find and replace"),
    ("remove_duplicates", "Remove duplicates"),
    ("delete_rows_where", "Delete rows"),
    ("split_column", "Split a column"),
    // protection and properties
    ("protect", "Protect the sheet"),
    ("protect_sheet_advanced", "Protect the sheet"),
    ("set_doc_properties", "Change the file's details"),
    ("set_custom_property", "Change a custom detail"),
    ("manage_custom_xml", "Change embedded data"),
    ("ignore_error", "Silence a warning"),
    ("set_sst_threshold", "Change a storage setting"),
];

/// The spreadsheet specialist's operations.
///
/// Note what is absent: nothing here acts outside this computer. A spreadsheet
/// specialist cannot send, post or delete anything beyond the User's own file, and the
/// test below asserts that, so it cannot change by accident.
pub fn worksheet() -> Classifier {
    let mut classifier = Classifier::new();
    for (name, verb) in READS {
        classifier.insert("worksheet", name, reads(verb));
    }
    for (name, verb) in EDITS {
        classifier.insert("worksheet", name, edits(verb));
    }
    classifier
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every operation the spreadsheet server exposes, as of worksheet-mcp 0.2.1.
    ///
    /// Kept as a literal list on purpose. When the server gains a tool, the test below
    /// fails and somebody has to decide what it does — which is the only reliable way
    /// to stop a new operation defaulting to something nobody chose.
    const EXPOSED: &[&str] = &[
        "add_box_whisker_chart",
        "add_chart",
        "add_chart_sheet",
        "add_conditional_format",
        "add_connection",
        "add_data_validation",
        "add_form_control",
        "add_funnel_chart",
        "add_histogram_chart",
        "add_image",
        "add_link",
        "add_map_chart",
        "add_pivot_table",
        "add_shape",
        "add_sheet",
        "add_slicer",
        "add_sparkline",
        "add_sunburst_chart",
        "add_table",
        "add_threaded_comment",
        "add_timeline",
        "add_treemap_chart",
        "add_waterfall_chart",
        "apply_style",
        "apply_theme",
        "autofit_columns",
        "batch_format",
        "clone_column_formulas",
        "close_workbook",
        "configure_workbook",
        "copy_format",
        "copy_range",
        "copy_sheet",
        "create_workbook",
        "delete_rows_where",
        "delete_sheet",
        "describe_formatting",
        "describe_workbook",
        "fill_series",
        "find_replace",
        "format_as_table_header",
        "format_as_table_range",
        "freeze_panes",
        "get_sheet_dimensions",
        "group",
        "ignore_error",
        "list_sheets",
        "manage_autofilter",
        "manage_cell",
        "manage_comments",
        "manage_custom_xml",
        "manage_defined_names",
        "manage_named_ranges",
        "merge_cells",
        "modify_columns",
        "modify_rows",
        "move_worksheet",
        "open_workbook",
        "open_workbook_encrypted",
        "protect",
        "protect_sheet_advanced",
        "read_cell",
        "read_cell_comment",
        "read_cell_format",
        "read_sheet",
        "read_sheet_metadata",
        "remove_duplicates",
        "rename_sheet",
        "save_workbook",
        "save_workbook_advanced",
        "search_cells",
        "set_cell_format",
        "set_custom_property",
        "set_dimensions",
        "set_doc_properties",
        "set_page_setup",
        "set_row_column_format",
        "set_selection",
        "set_sheet_settings",
        "set_sst_threshold",
        "set_visibility",
        "sheet_to_csv",
        "sort_range",
        "split_column",
        "transpose_range",
        "write_cells",
        "write_column",
        "write_formula",
        "write_grid",
        "write_json_rows",
        "write_rich_text",
        "write_row",
        "write_row_range",
    ];

    #[test]
    fn every_exposed_operation_is_classified() {
        let classifier = worksheet();
        let missing: Vec<&str> = EXPOSED
            .iter()
            .copied()
            .filter(|name| classifier.get("worksheet", name).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "these operations have no classification, so they would be refused as unknown: {missing:?}"
        );
        assert_eq!(
            classifier.len(),
            EXPOSED.len(),
            "the catalogue and the server should describe the same set of operations"
        );
    }

    #[test]
    fn nothing_in_the_catalogue_is_absent_from_the_server() {
        let classifier = worksheet();
        for (name, _) in READS.iter().chain(EDITS.iter()) {
            assert!(
                EXPOSED.contains(name),
                "{name} is classified but the server does not expose it — the catalogue has drifted"
            );
            let _ = classifier.get("worksheet", name).expect("classified");
        }
    }

    /// A spreadsheet specialist has no business acting outside this computer.
    #[test]
    fn the_spreadsheet_specialist_cannot_act_outside_this_computer() {
        let classifier = worksheet();
        for name in EXPOSED {
            assert_ne!(
                classifier.effect_of("worksheet", name),
                SideEffect::ExternalEffect,
                "{name} is classified as acting outside this computer, which a spreadsheet \
                 specialist must never do"
            );
        }
    }

    #[test]
    fn reading_is_never_gated_and_writing_is_always_recorded() {
        let classifier = worksheet();
        assert_eq!(
            classifier.effect_of("worksheet", "read_sheet"),
            SideEffect::Read
        );
        assert_eq!(
            classifier.effect_of("worksheet", "list_sheets"),
            SideEffect::Read
        );
        assert_eq!(
            classifier.effect_of("worksheet", "write_formula"),
            SideEffect::LocalWrite
        );
        assert_eq!(
            classifier.effect_of("worksheet", "save_workbook"),
            SideEffect::LocalWrite,
            "saving changes the User's file and must be recorded"
        );
    }

    /// Opening a file reads it; saving writes it. Getting these the wrong way round
    /// would either gate every open or fail to record any save.
    #[test]
    fn opening_reads_and_saving_writes() {
        let classifier = worksheet();
        assert_eq!(
            classifier.effect_of("worksheet", "open_workbook"),
            SideEffect::Read
        );
        assert_eq!(
            classifier.effect_of("worksheet", "save_workbook"),
            SideEffect::LocalWrite
        );
    }

    #[test]
    fn every_operation_reads_as_plain_language() {
        let classifier = worksheet();
        for name in EXPOSED {
            let class = classifier.get("worksheet", name).expect("classified");
            let verb = class.verb;
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

    #[test]
    fn a_local_write_can_always_be_undone() {
        let classifier = worksheet();
        for (name, _) in EDITS {
            let class = classifier.get("worksheet", name).expect("classified");
            assert!(
                matches!(class.reversibility, Reversibility::Reversible { .. }),
                "{name} changes the User's file but offers no way back"
            );
        }
    }
}
