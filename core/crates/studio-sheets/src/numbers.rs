//! Numbers as the file says they should read.
//!
//! A spreadsheet cell holds a number and a format code that says how to show it. The Core was
//! showing every number the same plain way, so a total the file formats as "1,240.00" appeared
//! as "1240" and a rate stored as 0.075 with a percent format appeared as "0.075". The
//! arithmetic was right and the screen still disagreed with what the User would see in any other
//! spreadsheet — and formatting a range as money did nothing visible, which reads as a broken
//! control.
//!
//! This covers the codes people actually use: thousands separators, fixed decimal places,
//! percentages, and a leading currency symbol. Anything more elaborate falls through to the
//! plain formatter rather than being half-rendered, because a number shown wrongly is worse
//! than one shown plainly.

/// Show `value` as the format `code` asks. `None` when the code is one we do not handle.
pub fn by_code(value: f64, code: &str) -> Option<String> {
    if code.is_empty() || code.eq_ignore_ascii_case("general") {
        return None;
    }

    // Only the positive section matters for display here. A code like "#,##0.00;[Red]-#,##0.00"
    // differs from the plain reading only in colour, which the style carries separately.
    let section = code.split(';').next().unwrap_or(code);

    // Anything with conditions, text substitution, fractions, dates or scientific notation is
    // left alone rather than guessed at.
    if section.contains('[')
        && !section.contains("[$")
        && !section.to_ascii_lowercase().contains("[red]")
    {
        return None;
    }
    if section.contains('/')
        || section.contains('E')
        || section.contains('e')
        || section.contains('@')
        || section.contains('y')
        || section.contains('d')
        || section.contains('h')
        || section.contains('s')
        || section.contains("m")
    {
        return None;
    }

    let percent = section.contains('%');
    let scaled = if percent { value * 100.0 } else { value };

    // The number part, with any currency or trailing text stripped off the ends.
    let currency = leading_symbol(section);
    let numeric_part: String = section
        .chars()
        .filter(|c| *c == '#' || *c == '0' || *c == '.' || *c == ',')
        .collect();
    if numeric_part.is_empty() {
        return None;
    }

    let places = numeric_part
        .split_once('.')
        .map(|(_, after)| after.chars().filter(|c| *c == '0' || *c == '#').count())
        .unwrap_or(0);
    let grouped = numeric_part.contains(',');

    let mut out = format!("{:.*}", places, scaled.abs());
    if grouped {
        out = group_thousands(&out);
    }
    if scaled < 0.0 {
        // Parentheses when the code asks for them, a minus sign otherwise — the two conventions
        // a spreadsheet uses, and the code says which.
        let negative_section = code.split(';').nth(1).unwrap_or("");
        if negative_section.contains('(') {
            out = format!("({out})");
        } else {
            out = format!("-{out}");
        }
    }
    if let Some(symbol) = currency {
        out = format!("{symbol}{out}");
    }
    if percent {
        out.push('%');
    }
    Some(out)
}

/// A currency symbol at the front of a format code, if it has one.
fn leading_symbol(section: &str) -> Option<String> {
    if let Some(rest) = section.strip_prefix("[$") {
        // "[$£-809]#,##0.00" — the symbol is everything up to the locale part.
        let symbol: String = rest
            .chars()
            .take_while(|c| *c != '-' && *c != ']')
            .collect();
        if !symbol.is_empty() {
            return Some(symbol);
        }
    }
    let first = section.chars().next()?;
    if matches!(first, '$' | '£' | '€' | '¥') {
        return Some(first.to_string());
    }
    None
}

/// Commas every three digits, left of the decimal point only.
fn group_thousands(text: &str) -> String {
    let (whole, rest) = match text.split_once('.') {
        Some((whole, after)) => (whole, Some(after)),
        None => (text, None),
    };
    let mut grouped = String::new();
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    match rest {
        Some(after) => format!("{grouped}.{after}"),
        None => grouped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_reads_as_money() {
        assert_eq!(by_code(1240.0, "#,##0.00").as_deref(), Some("1,240.00"));
        assert_eq!(by_code(4960000.0, "#,##0").as_deref(), Some("4,960,000"));
        assert_eq!(by_code(1240.5, "$#,##0.00").as_deref(), Some("$1,240.50"));
    }

    #[test]
    fn a_rate_stored_as_a_fraction_reads_as_a_percentage() {
        assert_eq!(by_code(0.075, "0.0%").as_deref(), Some("7.5%"));
        assert_eq!(by_code(0.3, "0%").as_deref(), Some("30%"));
    }

    #[test]
    fn a_negative_follows_the_convention_the_code_asks_for() {
        assert_eq!(by_code(-1240.0, "#,##0.00").as_deref(), Some("-1,240.00"));
        assert_eq!(
            by_code(-1240.0, "#,##0.00;(#,##0.00)").as_deref(),
            Some("(1,240.00)")
        );
    }

    /// The plain formatter is better than a wrong one, so anything elaborate is declined.
    #[test]
    fn a_code_we_do_not_handle_is_declined_rather_than_guessed() {
        assert_eq!(by_code(1.0, "General"), None);
        assert_eq!(by_code(1.0, ""), None);
        assert_eq!(
            by_code(45000.0, "yyyy-mm-dd"),
            None,
            "dates are their own thing"
        );
        assert_eq!(by_code(1.5, "# ?/?"), None, "fractions");
        assert_eq!(by_code(1200.0, "0.00E+00"), None, "scientific");
        assert_eq!(by_code(1.0, "[Blue][>100]0.0"), None, "conditions");
    }

    #[test]
    fn grouping_lands_in_the_right_places() {
        assert_eq!(group_thousands("1"), "1");
        assert_eq!(group_thousands("100"), "100");
        assert_eq!(group_thousands("1000"), "1,000");
        assert_eq!(group_thousands("1234567"), "1,234,567");
        assert_eq!(group_thousands("1234.56"), "1,234.56");
    }
}
