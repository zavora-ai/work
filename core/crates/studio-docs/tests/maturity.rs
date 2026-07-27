//! What of a real document reaches the User.
//!
//! A document is not paragraphs. It is headings, lists, tables, images, links, footnotes, page
//! breaks, columns, comments and tracked changes — and the interface shows whatever survives the
//! journey from the file to the editable view. This measures that journey construct by construct,
//! so a gap is a named gap rather than a vague sense that complicated documents look wrong.
//!
//! Each case says what a User would notice if it failed, because "no `<table>` in the output" is
//! only worth reporting as "the table in your contract is not on screen".

use zavora_docx::{Document, Length};

const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

fn html_of(path: &std::path::Path) -> String {
    studio_docs::read(path)
        .expect("the Core should read it")
        .html
}

fn a_document_with_everything(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let mut doc = Document::new();

    doc.add_paragraph("Master Services Agreement")
        .style("Heading1");
    doc.add_paragraph("1. Definitions").style("Heading2");

    {
        let mut p = doc.add_paragraph("The term ");
        p.add_run("Services").bold(true);
        p.add_run(" means the work in Schedule A, as ");
        p.add_run("varied").italic(true);
        p.add_run(" in writing.");
    }

    doc.add_paragraph("First obligation").style("ListParagraph");
    doc.add_paragraph("Second obligation")
        .style("ListParagraph");

    {
        let mut table = doc.add_table(3, 3);
        for (row, cells) in [
            ["Item", "Amount", "Due"],
            ["Retainer", "5,000", "On signing"],
            ["Monthly", "2,500", "Each month"],
        ]
        .iter()
        .enumerate()
        {
            for (col, text) in cells.iter().enumerate() {
                table.cell(row, col).unwrap().set_text(text);
            }
        }
    }

    doc.add_picture(PNG, "logo.png", Length::inches(1.0), Length::inches(1.0));
    doc.set_header("Zavora — commercial in confidence");
    doc.set_footer("Schedule A follows");

    doc.save(path).expect("saving should work");
}

fn fixture(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("zws-doc-maturity-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn a_heading_arrives_as_a_heading() {
    let path = fixture("headings.docx");
    a_document_with_everything(&path);
    let html = html_of(&path);
    assert!(
        html.contains("<h1"),
        "the document's title is not a heading on screen"
    );
    assert!(
        html.contains("<h2"),
        "its sections are not headings on screen"
    );
}

#[test]
fn formatting_inside_a_paragraph_arrives() {
    let path = fixture("runs.docx");
    a_document_with_everything(&path);
    let html = html_of(&path);
    assert!(
        html.contains("<strong") || html.contains("<b>"),
        "a defined term that is bold in the contract reads as ordinary text"
    );
    assert!(
        html.contains("<em") || html.contains("<i>"),
        "italics are lost"
    );
}

#[test]
fn a_table_arrives_as_a_table() {
    let path = fixture("table.docx");
    a_document_with_everything(&path);
    let html = html_of(&path);
    assert!(html.contains("<table"), "the fee table is not on screen");
    assert!(
        html.matches("<tr").count() >= 3,
        "its rows are not all there: {}",
        html.matches("<tr").count()
    );
    assert!(html.contains("Retainer"), "and neither is what is in them");
}

#[test]
fn a_picture_arrives() {
    let path = fixture("picture.docx");
    a_document_with_everything(&path);
    let html = html_of(&path);
    assert!(
        html.contains("<img"),
        "the logo in the letterhead is not on screen"
    );
}

#[test]
fn the_header_and_footer_arrive_separately_from_the_body() {
    let path = fixture("furniture.docx");
    a_document_with_everything(&path);
    let model = studio_docs::read(&path).unwrap();
    assert!(
        model.header_html.contains("commercial in confidence"),
        "the confidentiality marking is not shown: {:?}",
        model.header_html
    );
    assert!(
        model.footer_html.contains("Schedule A"),
        "the footer is not shown: {:?}",
        model.footer_html
    );
    assert!(
        !model.html.contains("commercial in confidence"),
        "the header is also in the body, so it would be drawn twice"
    );
}

#[test]
fn the_outline_is_what_a_person_would_call_the_sections() {
    let path = fixture("outline.docx");
    a_document_with_everything(&path);
    let model = studio_docs::read(&path).unwrap();
    let headings: Vec<&str> = model
        .outline
        .iter()
        .map(|item| item.text.as_str())
        .collect();
    assert_eq!(headings, ["Master Services Agreement", "1. Definitions"]);
}

/// Every block the interface offers to edit has to be one the Core can actually change.
#[test]
fn every_editable_block_is_addressable() {
    let path = fixture("blocks.docx");
    a_document_with_everything(&path);
    let model = studio_docs::read(&path).unwrap();
    let indices = studio_docs::block_indices(&model.html);
    assert!(
        !indices.is_empty(),
        "nothing in the document can be edited at all"
    );
    // Ascending and without repeats: two blocks sharing an index means editing one changes the
    // other.
    let mut sorted = indices.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted, indices,
        "block indices repeat or are out of order: {indices:?}"
    );
}
