//! `vocab-lint` — the build gate for Requirement 1.2.
//!
//! Exits non-zero if any User-visible string contains a prohibited term.
//! Wired into CI and the pre-commit hook.
//!
//! It checks two catalogues, not one. The Core holds every string in
//! `studio-strings`, and the Shell mirrors them in `shell/src/shared/strings.ts`
//! because a React component cannot read a Rust constant. A mirror is a place
//! where a second, unchecked copy of the product's words can grow, so the rule is
//! applied to both and the two are compared: a key in one and not the other is
//! reported, and so is the same key carrying different words.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn main() -> std::process::ExitCode {
    let violations = studio_lint::check_catalogue();
    let checked = studio_strings::CATALOGUE.len();
    let mut failed = false;

    if violations.is_empty() {
        println!("vocab-lint: {checked} strings checked, all clean");
    } else {
        eprintln!(
            "vocab-lint: {} violation(s) in {checked} strings\n",
            violations.len()
        );
        for v in &violations {
            eprintln!("  {v}");
        }
        eprintln!(
            "\nThese terms are forbidden on User-visible surfaces (Requirement 1.1).\n\
             If the User genuinely needs this information, it belongs in the\n\
             diagnostics view, not on a primary surface."
        );
        failed = true;
    }

    // The Shell's mirror. Skipped only if the Shell is not in this checkout, which is
    // reported rather than passed over in silence.
    match shell_catalogue_path() {
        None => {
            eprintln!(
                "vocab-lint: the Shell catalogue was not found, so its strings were not \
                 checked. This is a gap, not a pass."
            );
            failed = true;
        }
        Some(path) => match std::fs::read_to_string(&path) {
            Err(error) => {
                eprintln!("vocab-lint: could not read {}: {error}", path.display());
                failed = true;
            }
            Ok(source) => {
                let mirrored = parse_shell_catalogue(&source);
                if mirrored.is_empty() {
                    eprintln!(
                        "vocab-lint: no strings were found in {}. The format it is parsed \
                         from has probably changed, which would let the Shell's words go \
                         unchecked.",
                        path.display()
                    );
                    failed = true;
                } else {
                    let drift = compare(&mirrored);
                    let unused = unused(&mirrored);
                    if drift.is_empty() {
                        println!(
                            "vocab-lint: {} Shell strings checked, and they match the Core's",
                            mirrored.len()
                        );
                        if !unused.is_empty() {
                            println!(
                                "vocab-lint: {} string(s) in the Core are not on screen yet: {}",
                                unused.len(),
                                unused.join(", ")
                            );
                        }
                    } else {
                        eprintln!(
                            "\nvocab-lint: the Shell and the Core disagree about {} string(s):\n",
                            drift.len()
                        );
                        for line in &drift {
                            eprintln!("  {line}");
                        }
                        eprintln!(
                            "\nBoth catalogues describe the same product, so a difference \
                             means one of them is wrong."
                        );
                        failed = true;
                    }
                }
            }
        },
    }

    if failed {
        std::process::ExitCode::FAILURE
    } else {
        std::process::ExitCode::SUCCESS
    }
}

fn shell_catalogue_path() -> Option<PathBuf> {
    // From `core/crates/studio-lint` up to the repository root.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest.join("../../../shell/src/shared/strings.ts");
    candidate.exists().then_some(candidate)
}

/// Pull `"key": p("text")` pairs out of the Shell catalogue.
///
/// Deliberately narrow: it recognises the one shape the catalogue is written in, and says
/// so loudly when it finds nothing, because a parser that quietly matches nothing would
/// turn this gate off.
fn parse_shell_catalogue(source: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for line in source.lines() {
        let line = line.trim();
        if !line.starts_with('"') {
            continue;
        }
        let Some((key_part, rest)) = line.split_once("\":") else {
            continue;
        };
        let key = key_part.trim_start_matches('"');
        // `p("...")`, `s("...")` and `d("...")` carry the scope.
        let Some(open) = rest.find("(\"") else {
            continue;
        };
        let after = &rest[open + 2..];
        let Some(close) = after.rfind("\")") else {
            continue;
        };
        let text = &after[..close];
        found.insert(key.to_string(), unescape(text));
    }
    found
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(decoded) => out.push(decoded),
                    None => out.push('\u{fffd}'),
                }
            }
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// Where the two catalogues disagree, in the User's terms rather than by index.
fn compare(mirrored: &BTreeMap<String, String>) -> Vec<String> {
    let core: BTreeMap<&str, &str> = studio_strings::CATALOGUE
        .iter()
        .map(|entry| (entry.key, entry.text))
        .collect();

    let mut problems = Vec::new();
    for (key, text) in mirrored {
        match core.get(key.as_str()) {
            None => problems.push(format!("{key}: in the Shell, not in the Core")),
            Some(core_text) if *core_text != text.as_str() => problems.push(format!(
                "{key}: the Core says {core_text:?}, the Shell says {text:?}"
            )),
            Some(_) => {}
        }
    }
    problems
}

/// Copy that exists in the Core but is not rendered by the Shell.
///
/// Not a failure. The rule has already been applied to these strings — they are in the
/// Core catalogue — so nothing is going unchecked. They are copy the interface has not
/// adopted yet, which is worth saying out loud so it does not accumulate unnoticed.
fn unused(mirrored: &BTreeMap<String, String>) -> Vec<&'static str> {
    studio_strings::CATALOGUE
        .iter()
        .map(|entry| entry.key)
        .filter(|key| !mirrored.contains_key(*key))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parser is the weak point: if it stopped matching, the Shell's words would go
    /// unchecked and the gate would still report success.
    #[test]
    fn the_parser_reads_the_shape_the_catalogue_is_written_in() {
        let source = r#"
export const CATALOGUE = {
  "nav.new_work": p("New work"),
  "settings.title": s("Settings"),
  "diag.copy": d("Copy the details"),
  "common.loading": p("Opening\u2026"),
};
"#;
        let found = parse_shell_catalogue(source);
        assert_eq!(found.len(), 4, "all four shapes must be recognised");
        assert_eq!(found.get("nav.new_work").unwrap(), "New work");
        assert_eq!(found.get("settings.title").unwrap(), "Settings");
        assert_eq!(
            found.get("common.loading").unwrap(),
            "Opening…",
            "escapes must be decoded, or a comparison would report false drift"
        );
    }

    /// The real catalogue must parse. This is what makes the gate more than a formality.
    #[test]
    fn the_real_shell_catalogue_parses_and_is_substantial() {
        let path = shell_catalogue_path().expect("the Shell catalogue must be found");
        let source = std::fs::read_to_string(path).expect("and readable");
        let found = parse_shell_catalogue(&source);
        assert!(
            found.len() > 150,
            "the Shell has far more strings than this; the parser is missing some: {}",
            found.len()
        );
    }

    /// Every string the Shell renders must also be in the Core, or the rule never sees it.
    #[test]
    fn the_shell_renders_nothing_the_rule_has_not_seen() {
        let path = shell_catalogue_path().expect("the Shell catalogue must be found");
        let source = std::fs::read_to_string(path).unwrap();
        let mirrored = parse_shell_catalogue(&source);
        let problems = compare(&mirrored);
        assert!(
            problems.is_empty(),
            "the two catalogues disagree, so some of the product's words are unchecked: {problems:#?}"
        );
    }

    #[test]
    fn a_catalogue_the_parser_cannot_read_is_not_silently_accepted() {
        // The format changed and nothing matches. The gate must notice, because an empty
        // result would otherwise look exactly like a clean one.
        let found = parse_shell_catalogue("export const CATALOGUE = new Map([['a', 'b']]);");
        assert!(found.is_empty());
        // `main` treats an empty parse as a failure; this records why that matters.
    }
}
