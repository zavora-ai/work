//! Turning a stored value into what the User reads.
//!
//! Formatting happens here, in the Core, and never in the renderer — so the number
//! on screen is the number the file holds, formatted once.

use zavora_xlsx::{ExcelDateTime, Format};

use crate::CellStyle;

/// A number as a person would write it.
///
/// Integers lose their decimal point; anything else keeps enough precision to be
/// recognisable without turning into floating-point noise.
pub fn number(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_string();
    }
    // Round to significant digits, not decimal places. A product of two floats
    // often lands a few billionths off a round number, and "5555200.000000001" in a
    // revenue model reads as a defect even though the arithmetic is right. Rounding
    // by decimal place cannot fix that, because how much noise there is depends on
    // how large the number is.
    const SIGNIFICANT: i32 = 12;
    let rounded = if value == 0.0 {
        0.0
    } else {
        let magnitude = value.abs().log10().floor() as i32;
        let places = (SIGNIFICANT - 1 - magnitude).clamp(0, 15);
        let scale = 10f64.powi(places);
        (value * scale).round() / scale
    };

    if rounded.fract() == 0.0 && rounded.abs() < 1e15 {
        return format!("{}", rounded as i64);
    }
    let rendered = format!("{rounded:.15}");
    rendered
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// A date as a person would read it, not as a serial number.
pub fn date_time(value: &ExcelDateTime) -> String {
    value.to_iso_string()
}

/// Only the styling that changes how a cell reads.
pub fn style_of(format: Format) -> CellStyle {
    CellStyle {
        bold: format.is_bold(),
        italic: format.is_italic(),
        colour: format.get_font_color().map(rgb),
        background: format.get_bg_color().map(rgb),
        align: align_of(format.get_h_align()),
    }
}

fn rgb([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Excel's alignment codes, reduced to the three the interface honours.
fn align_of(code: u8) -> Option<String> {
    match code {
        1 => Some("left".to_string()),
        2 => Some("center".to_string()),
        3 => Some("right".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_integer_loses_its_decimal_point() {
        assert_eq!(number(1240.0), "1240");
        assert_eq!(number(0.0), "0");
        assert_eq!(number(-42.0), "-42");
    }

    #[test]
    fn a_fraction_keeps_enough_precision_without_floating_point_noise() {
        assert_eq!(number(1.5), "1.5");
        assert_eq!(number(0.1), "0.1");
        assert_eq!(number(11.4), "11.4");
        assert_eq!(
            number(5_555_200.000000001),
            "5555200",
            "noise below the tenth decimal is not information"
        );
    }

    #[test]
    fn an_impossible_number_reads_as_a_dash_rather_than_nan() {
        assert_eq!(number(f64::NAN), "—");
        assert_eq!(number(f64::INFINITY), "—");
    }

    #[test]
    fn a_colour_becomes_a_hex_string_the_renderer_can_use() {
        assert_eq!(rgb([255, 255, 255]), "#ffffff");
        assert_eq!(rgb([0, 0, 0]), "#000000");
        assert_eq!(rgb([61, 103, 51]), "#3d6733");
    }

    #[test]
    fn only_the_three_alignments_the_interface_honours_are_carried() {
        assert_eq!(align_of(1).as_deref(), Some("left"));
        assert_eq!(align_of(2).as_deref(), Some("center"));
        assert_eq!(align_of(3).as_deref(), Some("right"));
        assert_eq!(align_of(0), None, "the default carries nothing");
        assert_eq!(align_of(99), None, "an unknown code is not guessed at");
    }
}
