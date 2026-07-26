//! The charts a workbook holds, with their numbers already looked up.
//!
//! A chart in a file is a set of references — "Sheet1!$B$6:$B$9" — and nothing else. The renderer
//! could be handed those and go and fetch them, but then it would be reading the file a second
//! time and deciding for itself what a range means, which is the thing this crate exists to
//! prevent: the number in the chart has to be the number in the cell.
//!
//! So the ranges are resolved here, against the same grid the interface is about to draw, and the
//! renderer receives points it can plot without knowing what a range is.

use serde::{Deserialize, Serialize};

use crate::{Cell, Sheet};

/// A chart, ready to draw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Drawing {
    /// `bar`, `column`, `line`, `pie`, `area`, `scatter` — or `other` for a kind the interface
    /// has no way to draw, which it then says rather than drawing something else.
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub across_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up_name: Option<String>,
    /// Where its corner sits, so it can be drawn over the right part of the sheet.
    pub at_row: u32,
    pub at_col: u16,
    pub width: u32,
    pub height: u32,
    pub series: Vec<Series>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The labels along the bottom, where the chart has them.
    pub labels: Vec<String>,
    /// The numbers. Absent values are held as nulls rather than zeros, because a gap in a series
    /// is not a value of nothing — drawing it as zero would invent a data point.
    pub values: Vec<Option<f64>>,
}

/// How the kind is said, in words the interface can act on.
fn kind_of(chart_type: zavora_xlsx::ChartType) -> &'static str {
    use zavora_xlsx::ChartType as T;
    match chart_type {
        T::Bar | T::Bar3D => "bar",
        T::Column | T::Column3D => "column",
        T::Line | T::Line3D => "line",
        T::Pie | T::Pie3D | T::Doughnut => "pie",
        T::Area | T::Area3D => "area",
        T::Scatter | T::Bubble => "scatter",
        // Named honestly. A radar drawn as a bar chart is a lie about the User's own file.
        _ => "other",
    }
}

/// Read the charts on a sheet, resolving each series against the sheets already read.
pub fn drawings_on(worksheet: &zavora_xlsx::Worksheet, sheets: &[Sheet]) -> Vec<Drawing> {
    worksheet
        .charts()
        .iter()
        .map(|chart| {
            let (across, up) = chart.axis_names();
            let (at_row, at_col) = chart.anchor();
            let (width, height) = chart.size();
            Drawing {
                kind: kind_of(chart.kind()).to_string(),
                title: chart.heading().map(str::to_string),
                across_name: across.map(str::to_string),
                up_name: up.map(str::to_string),
                at_row,
                at_col,
                width,
                height,
                series: chart
                    .series_list()
                    .iter()
                    .map(|series| Series {
                        // A series name is often a reference to the heading cell rather than
                        // the heading itself. Passed through unresolved it would put
                        // "Summary!B5:B5" in the legend.
                        name: series.series_name().map(|name| name_of(name, sheets)),
                        labels: series
                            .categories_range()
                            .map(|range| {
                                cells_in(range, sheets)
                                    .into_iter()
                                    .map(|cell| cell.map(|c| c.display.clone()).unwrap_or_default())
                                    .collect()
                            })
                            .unwrap_or_default(),
                        values: cells_in(series.values_range(), sheets)
                            .into_iter()
                            .map(|cell| cell.and_then(number_in))
                            .collect(),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// What a series is called: the text if it is text, or the cell's contents if it names one.
fn name_of(given: &str, sheets: &[Sheet]) -> String {
    if !given.contains('!') && !given.contains('$') {
        return given.to_string();
    }
    match cells_in(given, sheets).first().copied().flatten() {
        Some(cell) if !cell.display.is_empty() => cell.display.clone(),
        // A reference that resolves to nothing is better shown as the reference than as an empty
        // legend entry, which looks like a series with no name rather than one we could not read.
        _ => given.to_string(),
    }
}

/// The number a cell holds, if it holds one.
///
/// Read from the display the Core already produced, so a chart and the cell beside it cannot
/// disagree. Separators and currency symbols are stripped, because they are how it reads rather
/// than what it is.
fn number_in(cell: &Cell) -> Option<f64> {
    if !cell.numeric {
        return None;
    }
    let cleaned: String = cell
        .display
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    cleaned.parse::<f64>().ok()
}

/// The cells a reference names, in order. Empty when the reference cannot be read.
///
/// A chart reference looks like `Sheet1!$B$6:$B$9`, sometimes with the sheet name quoted, and
/// sometimes with no sheet at all when it means the sheet the chart is on.
fn cells_in<'a>(reference: &str, sheets: &'a [Sheet]) -> Vec<Option<&'a Cell>> {
    let (sheet_name, range) = match reference.rsplit_once('!') {
        Some((name, range)) => (Some(name.trim_matches(['\'', '"'])), range),
        None => (None, reference),
    };

    let Some(sheet) = (match sheet_name {
        Some(name) => sheets.iter().find(|candidate| candidate.name == name),
        None => sheets.first(),
    }) else {
        return Vec::new();
    };

    let (start, end) = match range.split_once(':') {
        Some((start, end)) => (start, end),
        None => (range, range),
    };
    let Some((first_row, first_col)) = position_of(start) else {
        return Vec::new();
    };
    let Some((last_row, last_col)) = position_of(end) else {
        return Vec::new();
    };

    let mut found = Vec::new();
    for row in first_row.min(last_row)..=first_row.max(last_row) {
        for col in first_col.min(last_col)..=first_col.max(last_col) {
            found.push(sheet.at(row, col));
        }
    }
    found
}

/// "$B$6" or "B6" as a row and column, counting from zero.
fn position_of(reference: &str) -> Option<(u32, u16)> {
    let plain: String = reference.chars().filter(|c| *c != '$').collect();
    let letters: String = plain
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let digits: String = plain
        .chars()
        .skip(letters.len())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if letters.is_empty() || digits.is_empty() {
        return None;
    }
    let mut col: u32 = 0;
    for letter in letters.to_ascii_uppercase().chars() {
        col = col * 26 + (letter as u32 - 'A' as u32 + 1);
    }
    Some((
        digits.parse::<u32>().ok()?.checked_sub(1)?,
        u16::try_from(col.checked_sub(1)?).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_becomes_a_position() {
        assert_eq!(position_of("A1"), Some((0, 0)));
        assert_eq!(position_of("$B$6"), Some((5, 1)));
        assert_eq!(position_of("AA10"), Some((9, 26)));
        assert_eq!(position_of(""), None);
        assert_eq!(position_of("B"), None);
        assert_eq!(position_of("$1"), None);
    }

    fn a_sheet() -> Sheet {
        Sheet {
            name: "Summary".to_string(),
            first_row: 4,
            first_col: 0,
            rows: vec![
                vec![
                    Cell {
                        display: "July".to_string(),
                        ..Cell::default()
                    },
                    Cell {
                        display: "1,240.00".to_string(),
                        numeric: true,
                        ..Cell::default()
                    },
                ],
                vec![
                    Cell {
                        display: "August".to_string(),
                        ..Cell::default()
                    },
                    Cell {
                        display: "1310".to_string(),
                        numeric: true,
                        ..Cell::default()
                    },
                ],
            ],
            merges: Vec::new(),
            column_widths: Vec::new(),
            charts: Vec::new(),
        }
    }

    #[test]
    fn a_range_resolves_against_the_sheet_it_names() {
        let sheets = vec![a_sheet()];
        let found = cells_in("Summary!$B$5:$B$6", &sheets);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].map(|c| c.display.as_str()), Some("1,240.00"));
        assert_eq!(found[1].map(|c| c.display.as_str()), Some("1310"));
    }

    #[test]
    fn a_quoted_sheet_name_is_read_too() {
        let sheets = vec![a_sheet()];
        assert_eq!(cells_in("'Summary'!$A$5", &sheets).len(), 1);
    }

    /// A sheet the file does not have must give nothing, not the first sheet's numbers.
    #[test]
    fn a_range_naming_a_sheet_that_is_not_there_gives_nothing() {
        let sheets = vec![a_sheet()];
        assert!(cells_in("Elsewhere!$B$5:$B$6", &sheets).is_empty());
    }

    #[test]
    fn a_number_is_read_through_the_way_it_is_shown() {
        let sheets = vec![a_sheet()];
        let found = cells_in("Summary!$B$5", &sheets);
        assert_eq!(found[0].and_then(number_in), Some(1240.0));
    }

    /// Text is not a number, and a gap is not a zero.
    #[test]
    fn what_is_not_a_number_is_held_as_absent() {
        let sheets = vec![a_sheet()];
        let text = cells_in("Summary!$A$5", &sheets);
        assert_eq!(text[0].and_then(number_in), None);

        let beyond = cells_in("Summary!$Z$99", &sheets);
        assert_eq!(beyond[0].and_then(number_in), None);
    }

    #[test]
    fn a_kind_the_interface_cannot_draw_is_named_as_such() {
        assert_eq!(kind_of(zavora_xlsx::ChartType::Column), "column");
        assert_eq!(kind_of(zavora_xlsx::ChartType::Pie), "pie");
        assert_eq!(kind_of(zavora_xlsx::ChartType::Radar), "other");
    }

    #[test]
    fn a_series_named_by_a_reference_reads_as_the_heading() {
        let sheets = vec![a_sheet()];
        // B5 holds "1,240.00" in the fixture; the point is that the reference is followed.
        assert_eq!(name_of("Summary!$B$5:$B$5", &sheets), "1,240.00");
    }

    #[test]
    fn a_series_named_in_words_keeps_those_words() {
        let sheets = vec![a_sheet()];
        assert_eq!(name_of("Units", &sheets), "Units");
    }
}
