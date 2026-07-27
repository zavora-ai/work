//! What a document written by Word looks like on screen.
//!
//! Every other fixture here is written by `zavora-docx`, so the suite proves the library agrees
//! with itself. Real documents come from Word, which writes constructs our own writer never emits:
//! lists driven by `numbering.xml` rather than by literal numbers, hyperlinks as relationships,
//! fields for page numbers, footnotes, styles referenced by an id whose readable name differs, and
//! runs split wherever formatting changes mid-sentence.
//!
//! The fixture is the XML in `word_shaped.py`, built by hand for that reason. Each assertion says
//! what a User would notice, because "no `<ul>` in the output" only matters as "the bullet in your
//! contract is drawn as number 3".

/// `named` because these tests run at the same time, and two of them writing one fixture while a
/// third reads it fails on a half-written file rather than on anything real.
fn word_shaped_document(named: &str) -> Option<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("zws-word-shaped-{}", std::process::id()));
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
        eprintln!(
            "skipping: the fixture could not be built: {}",
            String::from_utf8_lossy(&done.stderr)
        );
        return None;
    }
    Some(path)
}

fn html(named: &str) -> Option<(String, studio_docs::DocModel)> {
    let path = word_shaped_document(named)?;
    let model = studio_docs::read(&path).expect("the Core should read a Word document");
    Some((model.html.clone(), model))
}

#[test]
fn a_word_document_arrives_with_its_structure_intact() {
    let Some((html, model)) = html("structure") else {
        return;
    };

    // Headings, from a style id whose readable name is "heading 1".
    assert!(html.contains("<h1"), "the title is not a heading on screen");
    assert!(html.contains("<h2"), "the sections are not headings");
    assert_eq!(
        model
            .outline
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        [
            "Master Services Agreement",
            "1. Obligations",
            "2. Schedule A"
        ],
        "the outline is not what a person would call the sections"
    );

    // Formatting inside a sentence, which Word writes as several runs.
    assert!(
        html.contains("<strong>Services</strong>"),
        "a defined term that is bold reads as ordinary text"
    );
    assert!(
        html.contains("<em>Schedule A</em>"),
        "italics are lost mid-sentence"
    );

    // A hyperlink, which Word can only write as a relationship.
    assert!(
        html.contains("href=\"https://example.com/terms\""),
        "a link in the document is not a link on screen"
    );

    // A table, including the merged cell most real tables have somewhere.
    assert!(html.contains("<table"), "the fee table is missing");
    assert!(
        html.contains("colspan"),
        "a merged cell is drawn as separate cells, so the row is the wrong shape"
    );
    assert!(html.contains("35,000"), "what is in the table is missing");

    // A page break, distinguishable from a rule the author drew themselves.
    assert!(
        html.contains("page-break"),
        "a page break is invisible, or indistinguishable from a horizontal rule"
    );

    // The furniture, kept out of the body so it is not drawn twice.
    assert!(
        model.header_html.contains("commercial in confidence"),
        "the confidentiality marking is not shown"
    );
    assert!(
        model.footer_html.contains("Page"),
        "the page number is not shown: {:?}",
        model.footer_html
    );
}

/// The one that was wrong: a bulleted item after a numbered one.
#[test]
fn a_bullet_is_a_bullet_and_not_the_next_number() {
    let Some((html, _)) = html("lists") else {
        return;
    };

    assert!(
        html.contains("<ol>"),
        "the numbered obligations are not a numbered list"
    );
    assert!(
        html.contains("<ul>"),
        "the bulleted item was folded into the numbered list above it, so a bullet in the \
         document is drawn as the next number on screen"
    );

    // And in that order: the bullet comes after the numbered list, not inside it.
    let ordered = html.find("<ol>").expect("an ordered list");
    let closed = html.find("</ol>").expect("which closes");
    let bulleted = html.find("<ul>").expect("a bulleted list");
    assert!(
        ordered < closed && closed < bulleted,
        "the lists overlap: ol at {ordered}, /ol at {closed}, ul at {bulleted}"
    );
}

/// Everything the interface offers to edit must be something the Core can address.
#[test]
fn every_block_in_a_word_document_is_addressable() {
    let Some((html, _)) = html("blocks") else {
        return;
    };
    let indices = studio_docs::block_indices(&html);
    assert!(!indices.is_empty(), "nothing can be edited");

    let mut sorted = indices.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted, indices,
        "block indices repeat or run backwards, so editing one block would change another: \
         {indices:?}"
    );
}

/// A document under review: comments, pending changes, more than one section, an equation, and a
/// language that reads the other way.
#[test]
fn a_document_under_review_arrives_readable() {
    let Some((html, model)) = html("review") else {
        return;
    };

    assert!(
        html.contains("Payment is due on receipt"),
        "a phrase someone has commented on is missing from the view"
    );
    assert!(
        html.contains("twelve"),
        "text a reviewer has inserted is not shown, so the document reads as it was rather than          as it is being changed to"
    );
    assert!(html.contains("اتفاقية"), "right-to-left text is dropped");
    assert!(
        html.contains("class=\"formula\""),
        "an equation is drawn as an empty line, and the User cannot see the formula in their own          document"
    );
    assert!(
        html.contains("mc"),
        "the formula is marked but says nothing"
    );

    // Both sections' headings are in the outline, so the document's shape is not truncated at the
    // section break.
    let headings: Vec<&str> = model.outline.iter().map(|i| i.text.as_str()).collect();
    assert!(
        headings.contains(&"2. Schedule A"),
        "the second section's heading is missing from the outline: {headings:?}"
    );
}
