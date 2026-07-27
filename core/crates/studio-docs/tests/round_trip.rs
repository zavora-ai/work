//! What an edit costs a complicated document.
//!
//! The reading side can be judged by looking at the screen. This is the part nobody looks at: a
//! document under review carries comments, tracked changes, footnotes, more than one section with
//! its own header, equations and right-to-left text — and an edit to one paragraph must not take
//! any of it. On the spreadsheet side exactly this went wrong and stayed wrong, because every test
//! checked the thing that had been edited rather than everything that had not.
//!
//! The fixture is `word_shaped.py`, written by hand as Word writes a file, because our own writer
//! does not emit most of these constructs and a fixture that cannot contain the problem cannot
//! catch it.

use std::io::Read;

fn built(named: &str) -> Option<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("zws-doc-roundtrip-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{named}.docx"));

    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("word_shaped.py");
    let done = std::process::Command::new("python3")
        .arg(&script)
        .arg(&path)
        .output()
        .ok()?;
    if !done.status.success() {
        eprintln!("skipping: the fixture could not be built");
        return None;
    }
    Some(path)
}

/// What the file is made of, as names of parts and markers in the body.
struct Contents {
    parts: Vec<String>,
    body: String,
    rels: String,
}

fn contents_of(path: &std::path::Path) -> Contents {
    let file = std::fs::File::open(path).expect("the file should be there");
    let mut zip = zip::ZipArchive::new(file).expect("it should be a document");
    let parts: Vec<String> = zip.file_names().map(str::to_string).collect();

    let mut body = String::new();
    zip.by_name("word/document.xml")
        .expect("a document has a body")
        .read_to_string(&mut body)
        .unwrap();

    let mut rels = String::new();
    if let Ok(mut found) = zip.by_name("word/_rels/document.xml.rels") {
        found.read_to_string(&mut rels).unwrap();
    }

    Contents { parts, body, rels }
}

impl Contents {
    /// Everything this asks about, with the words a User would use for it.
    fn what_it_still_has(&self) -> Vec<(&'static str, bool)> {
        vec![
            (
                "the reviewer's comments",
                self.parts.iter().any(|name| name.contains("comments"))
                    && self.body.contains("commentRangeStart"),
            ),
            (
                "a change waiting to be accepted",
                self.body.contains("<w:ins "),
            ),
            (
                "a deletion waiting to be accepted",
                self.body.contains("<w:del "),
            ),
            (
                "the footnotes",
                self.parts.iter().any(|name| name.contains("footnotes")),
            ),
            (
                "the second section's own heading",
                self.parts.iter().any(|name| name.contains("header2")),
            ),
            ("both sections", self.body.matches("<w:sectPr").count() >= 2),
            ("the equation", self.body.contains("oMath")),
            ("the Arabic heading", self.body.contains("اتفاقية")),
            (
                "the numbering the lists use",
                self.parts.iter().any(|n| n.contains("numbering")),
            ),
            (
                "the link to the terms",
                self.rels.contains("example.com/terms"),
            ),
            ("the table", self.body.contains("<w:tbl>")),
        ]
    }
}

/// The fixture has to contain all of it, or the test below proves nothing.
#[test]
fn the_fixture_is_as_complicated_as_it_claims() {
    let Some(path) = built("claims") else { return };
    for (what, present) in contents_of(&path).what_it_still_has() {
        assert!(present, "the fixture is missing {what}");
    }
}

/// The one that matters.
#[test]
fn changing_a_paragraph_costs_the_document_nothing_else() {
    let Some(path) = built("edit") else { return };
    let before = contents_of(&path);

    // Exactly what the interface asks for: the words of one paragraph, replaced.
    let mut document = zavora_docx::Document::open(&path).expect("reopening");
    assert!(
        document.set_paragraph_text(1, "The term Services means the work in Schedule A."),
        "paragraph 1 should be changeable"
    );
    document.save(&path).expect("saving");

    let after = contents_of(&path);
    assert!(
        after.body.contains("means the work in Schedule A"),
        "the change itself is missing"
    );

    for ((what, was), (_, still)) in before
        .what_it_still_has()
        .into_iter()
        .zip(after.what_it_still_has())
    {
        if was {
            assert!(still, "changing one paragraph lost {what}");
        }
    }
}

/// A paragraph carrying a tracked change is not a plain paragraph, and replacing its words would
/// throw away a decision the reviewer has not made yet.
#[test]
fn a_paragraph_with_a_pending_change_keeps_it() {
    let Some(path) = built("tracked") else { return };
    let before = contents_of(&path);
    let insertions = before.body.matches("<w:ins ").count();
    assert!(
        insertions > 0,
        "the fixture should have a tracked insertion"
    );

    // Edit a different paragraph, then check the pending change is untouched.
    let mut document = zavora_docx::Document::open(&path).expect("reopening");
    document.set_paragraph_text(0, "Master Services Agreement (2026)");
    document.save(&path).expect("saving");

    let after = contents_of(&path);
    assert_eq!(
        after.body.matches("<w:ins ").count(),
        insertions,
        "a tracked insertion was lost by editing a different paragraph"
    );
    assert!(
        after.body.contains("<w:del "),
        "a tracked deletion was lost by editing a different paragraph"
    );
}
