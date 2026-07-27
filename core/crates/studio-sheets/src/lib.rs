//! Reading a spreadsheet for the interface.
//!
//! ## Why the Core does this and not the renderer
//!
//! The obvious shortcut is to hand the `.xlsx` bytes to the renderer and parse them
//! in the browser with SheetJS, which is what `excel-agent-app` does and it works.
//! Work Studio does not, for one reason that outweighs the extra work: **a
//! spreadsheet must have exactly one calculator.**
//!
//! `zavora-xlsx` owns a formula engine and is what writes the file. If the renderer
//! also evaluated formulas, the number on screen and the number in the saved file
//! could disagree — and disagree silently, in a financial model, where the whole
//! value of the artefact is that the arithmetic is right. Two parsers is a
//! divergence class of bug; two calculators is a wrong answer.
//!
//! So the Core reads the file, evaluates it, and emits a [`GridModel`]: values
//! already formatted for display, the formula behind each one, and the styling that
//! affects how a cell reads. The renderer draws that model and never parses a file.
//!
//! The same argument was made for documents and decks, where it settled on
//! `data-node-id` from the engines. This is the spreadsheet form of it.

use serde::{Deserialize, Serialize};
use zavora_xlsx::{CellValue, Workbook};

pub mod charts;
pub mod format;
pub mod numbers;

/// What went wrong, in two registers.
///
/// `Display` is what the User reads and carries no technical detail (Requirement
/// 17.2); [`SheetError::detail`] carries the underlying cause for the diagnostics view
/// (Requirement 17.5). An earlier version put the cause in the message, and a bad file
/// produced "ZIP error: invalid Zip archive: Could not find EOCD" on screen.
#[derive(Debug, thiserror::Error)]
pub enum SheetError {
    #[error("that file could not be opened — it may not be a spreadsheet")]
    Open { detail: String },
    #[error("that file has no sheets in it")]
    Empty,
    #[error("there is no sheet called {0}")]
    NoSuchSheet(String),
}

impl SheetError {
    /// The underlying cause, for support. Never shown on a primary surface.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Open { detail } => Some(detail),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, SheetError>;

/// One cell, as the interface needs it.
///
/// `display` is what the User reads; `formula` is what produced it. Both are needed:
/// the grid shows the first and the formula bar shows the second.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cell {
    /// Already formatted. The renderer never formats a number itself, so that what
    /// is on screen matches what is in the file.
    pub display: String,
    /// Present only when the cell holds one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    /// True when the value is a number, so the grid can align it right.
    pub numeric: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<CellStyle>,
    /// Set when the cell holds an error, so the interface can say so plainly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Only the styling that changes how a cell reads. Deliberately not the whole
/// format model: the interface does not need to reproduce Excel, it needs the User
/// to recognise their own sheet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CellStyle {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    /// `#rrggbb`, when set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// `left` | `center` | `right`, when the file says so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
}

impl CellStyle {
    fn is_plain(&self) -> bool {
        !self.bold
            && !self.italic
            && self.colour.is_none()
            && self.background.is_none()
            && self.align.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merge {
    pub first_row: u32,
    pub first_col: u16,
    pub last_row: u32,
    pub last_col: u16,
}

/// One sheet as a rectangle of cells, plus what the interface needs around it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub name: String,
    /// Row and column the rectangle starts at, zero-based, so the grid can show
    /// real row numbers rather than renumbering from one.
    pub first_row: u32,
    pub first_col: u16,
    pub rows: Vec<Vec<Cell>>,
    pub merges: Vec<Merge>,
    /// Column widths, where the file sets them.
    pub column_widths: Vec<Option<f64>>,
    /// The charts drawn on this sheet, with their numbers already resolved. Empty for a sheet
    /// with none, so the interface draws nothing rather than a chart of nothing.
    #[serde(default)]
    pub charts: Vec<crate::charts::Drawing>,
    /// The first row and column that scroll, where the file freezes its headings. Zero means
    /// nothing is frozen.
    #[serde(default)]
    pub frozen_row: u32,
    #[serde(default)]
    pub frozen_col: u16,
}

impl Sheet {
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.rows.first().map(Vec::len).unwrap_or(0)
    }

    /// A cell by its absolute position, or None when outside the used range.
    pub fn at(&self, row: u32, col: u16) -> Option<&Cell> {
        let r = row.checked_sub(self.first_row)? as usize;
        let c = col.checked_sub(self.first_col)? as usize;
        self.rows.get(r)?.get(c)
    }
}

/// A whole workbook, ready to render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridModel {
    pub file_name: String,
    pub sheets: Vec<Sheet>,
    /// Which sheet the interface should show first.
    pub active: usize,
}

impl GridModel {
    pub fn sheet(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|sheet| sheet.name == name)
    }

    pub fn active_sheet(&self) -> Option<&Sheet> {
        self.sheets.get(self.active)
    }
}

/// How many rows and columns to read at most.
///
/// A sheet with a million rows must not become a million rows of JSON crossing the
/// loopback channel. The interface pages; this is the page size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub max_rows: usize,
    pub max_cols: usize,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            max_rows: 500,
            max_cols: 64,
        }
    }
}

/// Read a spreadsheet into a model the interface can draw.
pub fn read(path: &std::path::Path, window: Window) -> Result<GridModel> {
    let workbook = Workbook::open_readonly(path).map_err(|e| SheetError::Open {
        detail: e.to_string(),
    })?;

    let names: Vec<String> = workbook
        .sheet_names()
        .into_iter()
        .map(str::to_string)
        .collect();
    if names.is_empty() {
        return Err(SheetError::Empty);
    }

    let mut sheets = Vec::with_capacity(names.len());
    for index in 0..names.len() {
        let worksheet = workbook
            .worksheet_ref(index)
            .map_err(|e| SheetError::Open {
                detail: e.to_string(),
            })?;
        sheets.push(read_sheet(worksheet, window));
    }

    // Charts second, because a series can point at a range on another sheet and resolving it
    // needs every sheet already read. Doing it in the first pass would give a chart of a sheet
    // that had not been looked at yet — which is to say, a chart of nothing.
    let mut drawings = Vec::with_capacity(names.len());
    for index in 0..names.len() {
        let worksheet = workbook
            .worksheet_ref(index)
            .map_err(|e| SheetError::Open {
                detail: e.to_string(),
            })?;
        drawings.push(charts::drawings_on(worksheet, &sheets));
    }
    for (sheet, found) in sheets.iter_mut().zip(drawings) {
        sheet.charts = found;
    }

    // Open on the first sheet that has something in it. A workbook often carries an
    // empty leading sheet, and landing the User on it makes their own file look lost.
    let active = sheets
        .iter()
        .position(|sheet| sheet.row_count() > 0)
        .unwrap_or(0);

    Ok(GridModel {
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        sheets,
        active,
    })
}

fn read_sheet(worksheet: &zavora_xlsx::Worksheet, window: Window) -> Sheet {
    let name = worksheet.name().to_string();
    let frozen = worksheet.frozen_at();
    let Some((first_row, first_col, last_row, last_col)) = worksheet.used_range() else {
        return Sheet {
            name,
            first_row: 0,
            first_col: 0,
            rows: Vec::new(),
            merges: Vec::new(),
            column_widths: Vec::new(),
            // Filled in a second pass, once every sheet has been read.
            charts: Vec::new(),
            frozen_row: 0,
            frozen_col: 0,
        };
    };

    let last_row = last_row.min(first_row + window.max_rows.saturating_sub(1) as u32);
    let last_col = last_col.min(first_col + window.max_cols.saturating_sub(1) as u16);

    let mut rows = Vec::new();
    for row in first_row..=last_row {
        let mut cells = Vec::new();
        for col in first_col..=last_col {
            cells.push(read_one(worksheet, row, col));
        }
        rows.push(cells);
    }

    let merges = worksheet
        .merge_ranges()
        .iter()
        .map(|&(fr, fc, lr, lc)| Merge {
            first_row: fr,
            first_col: fc,
            last_row: lr,
            last_col: lc,
        })
        .collect();

    let column_widths = (first_col..=last_col)
        .map(|col| worksheet.column_width(col))
        .collect();

    Sheet {
        name,
        first_row,
        first_col,
        rows,
        merges,
        column_widths,
        charts: Vec::new(),
        frozen_row: frozen.0,
        frozen_col: frozen.1,
    }
}

fn read_one(worksheet: &zavora_xlsx::Worksheet, row: u32, col: u16) -> Cell {
    let value = worksheet.read_cell(row, col);
    let cell_format = worksheet.cell_format(row, col);
    // The code the file says this number should read by. Held so the display below can honour
    // it: a total the file formats as "1,240.00" was appearing as "1240".
    let code = cell_format
        .as_ref()
        .map(|format| format.get_num_format().to_string())
        .unwrap_or_default();
    let style = cell_format
        .map(format::style_of)
        .filter(|style| !style.is_plain());

    let mut cell = Cell {
        style,
        ..Cell::default()
    };

    match value {
        CellValue::Empty => {}
        CellValue::String(text) => cell.display = text,
        CellValue::RichText(rich) => cell.display = rich.plain_text(),
        CellValue::Number(number) => {
            cell.display =
                numbers::by_code(number, &code).unwrap_or_else(|| format::number(number));
            cell.numeric = true;
        }
        CellValue::Bool(flag) => cell.display = if flag { "TRUE" } else { "FALSE" }.to_string(),
        CellValue::DateTime(when) => cell.display = format::date_time(&when),
        CellValue::Error(text) => {
            cell.display = text.clone();
            cell.error = Some(text);
        }
        CellValue::Formula {
            formula,
            cached_value,
        } => {
            cell.formula = Some(normalise_formula(&formula));
            // The cached result is what Excel last computed. Showing it is right;
            // recomputing it here would be a second calculator, which is the thing
            // this module exists to avoid.
            match *cached_value {
                CellValue::Number(number) => {
                    // A formula's result is formatted the same way a typed number is. A total
                    // reading "4960000" beside a typed "4,960,000" is the same file disagreeing
                    // with itself.
                    cell.display =
                        numbers::by_code(number, &code).unwrap_or_else(|| format::number(number));
                    cell.numeric = true;
                }
                CellValue::String(text) => cell.display = text,
                CellValue::Bool(flag) => {
                    cell.display = if flag { "TRUE" } else { "FALSE" }.to_string()
                }
                CellValue::DateTime(when) => cell.display = format::date_time(&when),
                CellValue::Error(text) => {
                    cell.display = text.clone();
                    cell.error = Some(text);
                }
                // Never evaluated. Saying so beats showing a blank cell as though
                // the sheet were empty there.
                _ => cell.display = "…".to_string(),
            }
        }
    }

    cell
}

/// Formulas are stored without the leading `=`; the formula bar shows it.
fn normalise_formula(formula: &str) -> String {
    if formula.starts_with('=') {
        formula.to_string()
    } else {
        format!("={formula}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zavora_xlsx::{Format, Workbook};

    fn fixture(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("zws-sheets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    /// A small model with a header row, numbers, a formula and some styling — the
    /// shape of the fixture in the mockups.
    fn write_model(path: &std::path::Path) {
        let mut workbook = Workbook::new();
        let bold = Format::new().bold();
        let sheet = workbook.worksheet(0).unwrap();
        sheet.set_name("Summary").unwrap();

        for (col, heading) in ["Month", "Units", "Base", "+12%"].iter().enumerate() {
            sheet
                .write_with_format(4, col as u16, *heading, &bold)
                .unwrap();
        }
        let months = [
            ("July", 1240.0, 4_960_000.0),
            ("August", 1310.0, 5_240_000.0),
        ];
        for (index, (month, units, base)) in months.iter().enumerate() {
            let row = 5 + index as u32;
            sheet.write(row, 0, *month).unwrap();
            sheet.write(row, 1, *units).unwrap();
            sheet.write(row, 2, *base).unwrap();
            // With a cached result, so the model can show what the file holds
            // without becoming a second calculator.
            sheet
                .write_formula_with_result(row, 3, &format!("C{}*1.12", row + 1), base * 1.12)
                .unwrap();
        }
        workbook.save(path).unwrap();
    }

    #[test]
    fn a_sheet_is_read_into_a_rectangle_at_its_real_position() {
        let path = fixture("model.xlsx");
        write_model(&path);

        let model = read(&path, Window::default()).expect("reads");
        assert_eq!(model.file_name, "model.xlsx");
        let sheet = model.active_sheet().expect("one sheet");
        assert_eq!(sheet.name, "Summary");
        assert_eq!(
            sheet.first_row, 4,
            "the rectangle should start where the data does, not at row 0"
        );
        assert_eq!(sheet.first_col, 0);
        assert_eq!(sheet.row_count(), 3, "header plus two months");
        assert_eq!(sheet.col_count(), 4);
    }

    #[test]
    fn a_cell_carries_what_the_user_reads_and_what_produced_it() {
        let path = fixture("model2.xlsx");
        write_model(&path);
        let model = read(&path, Window::default()).unwrap();
        let sheet = model.active_sheet().unwrap();

        let heading = sheet.at(4, 0).expect("A5");
        assert_eq!(heading.display, "Month");
        assert!(heading.formula.is_none());
        assert!(!heading.numeric);

        let units = sheet.at(5, 1).expect("B6");
        assert_eq!(units.display, "1240");
        assert!(
            units.numeric,
            "a number must be marked so the grid can align it"
        );

        let derived = sheet.at(5, 3).expect("D6");
        assert_eq!(
            derived.formula.as_deref(),
            Some("=C6*1.12"),
            "the formula bar needs the formula with its leading ="
        );
    }

    #[test]
    fn styling_is_carried_only_where_it_changes_how_a_cell_reads() {
        let path = fixture("model3.xlsx");
        write_model(&path);
        let model = read(&path, Window::default()).unwrap();
        let sheet = model.active_sheet().unwrap();

        let heading = sheet.at(4, 0).expect("A5");
        assert!(
            heading.style.as_ref().is_some_and(|style| style.bold),
            "a bold header must survive into the model: {:?}",
            heading.style
        );

        let plain = sheet.at(5, 0).expect("A6");
        assert!(
            plain.style.is_none(),
            "an unstyled cell should carry no style at all, so the payload stays small"
        );
    }

    /// A million-row sheet must not become a million rows of JSON.
    #[test]
    fn the_window_bounds_what_is_read() {
        let path = fixture("big.xlsx");
        {
            let mut workbook = Workbook::new();
            let sheet = workbook.worksheet(0).unwrap();
            for row in 0..300u32 {
                sheet.write(row, 0, row as f64).unwrap();
            }
            workbook.save(&path).unwrap();
        }

        let model = read(
            &path,
            Window {
                max_rows: 50,
                max_cols: 8,
            },
        )
        .unwrap();
        let sheet = model.active_sheet().unwrap();
        assert_eq!(sheet.row_count(), 50, "the window must bound the read");
    }

    #[test]
    fn every_sheet_is_read_and_the_first_is_active() {
        let path = fixture("multi.xlsx");
        {
            let mut workbook = Workbook::new();
            for (index, name) in ["Summary", "Detail", "Assumptions"].iter().enumerate() {
                let sheet = if index == 0 {
                    workbook.worksheet(0).unwrap()
                } else {
                    workbook.add_worksheet()
                };
                sheet.set_name(name).unwrap();
                sheet.write(0, 0, *name).unwrap();
            }
            workbook.save(&path).unwrap();
        }
        let model = read(&path, Window::default()).unwrap();
        assert_eq!(
            model
                .sheets
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Summary", "Detail", "Assumptions"]
        );
        assert_eq!(model.active, 0);
        assert!(model.sheet("Detail").is_some());
        assert!(model.sheet("Nowhere").is_none());
    }

    #[test]
    fn an_empty_sheet_reads_as_empty_rather_than_failing() {
        let path = fixture("empty.xlsx");
        {
            let mut workbook = Workbook::new();
            workbook.save(&path).unwrap();
        }
        let model = read(&path, Window::default()).unwrap();
        let sheet = model.active_sheet().unwrap();
        assert_eq!(sheet.row_count(), 0);
        assert_eq!(sheet.col_count(), 0);
    }

    #[test]
    fn a_file_that_is_not_a_spreadsheet_says_so_in_the_users_terms() {
        let path = fixture("not-a-sheet.xlsx");
        std::fs::write(&path, b"this is not a spreadsheet").unwrap();
        let error = read(&path, Window::default()).expect_err("must fail");
        let message = error.to_string();
        assert!(
            message.starts_with("that file could not be opened"),
            "the message must read as plain language: {message}"
        );
        assert!(
            !message.to_lowercase().contains("zip") && !message.contains("EOCD"),
            "the User must never be shown the underlying cause: {message}"
        );
        assert!(
            error.detail().is_some_and(|d| !d.is_empty()),
            "but support must still be able to see it"
        );
    }

    #[test]
    fn the_model_survives_a_round_trip_through_json() {
        let path = fixture("json.xlsx");
        write_model(&path);
        let model = read(&path, Window::default()).unwrap();
        let json = serde_json::to_string(&model).unwrap();
        let back: GridModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model, back);
        assert!(
            json.contains("\"display\""),
            "the renderer reads camelCase fields"
        );
    }
}
