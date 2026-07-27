//! A document from a different producer entirely.
//!
//! Every other fixture here is XML written for the purpose — by our own library, or by hand as Word
//! writes it. Both are guesses about what real files look like. macOS ships `textutil`, which
//! writes .docx with Apple's own converter, and what it produces is a fair sample of a large class
//! of real documents: no styles at all, no tables, no lists, no hyperlinks — the author's headings
//! are simply large bold text.
//!
//! Documents of that shape are common (converted files, exports from tools that keep no structure,
//! and the many people who never touch a style), and Work Studio used to show them with an empty
//! list of sections, leaving no way to move around a long one.
//!
//! Skipped where `textutil` is absent, which is anywhere but macOS. A test that quietly passes on
//! another platform is worse than one that says it did not run.

const SOURCE: &str = r#"<html><body>
<h1>Master Services Agreement</h1>
<p>The term <b>Services</b> means the work in <i>Schedule A</i>, as set out in that schedule.</p>
<h2>1. Obligations</h2>
<p>Each party shall perform its obligations with reasonable skill and care.</p>
<h2>2. Schedule A</h2>
<p>The services are described here, in the detail the parties have agreed.</p>
</body></html>"#;

fn apple_written(named: &str) -> Option<std::path::PathBuf> {
    if std::process::Command::new("which")
        .arg("textutil")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .is_none()
    {
        eprintln!("skipping: textutil is only on macOS");
        return None;
    }

    let dir = std::env::temp_dir().join(format!("zws-apple-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let source = dir.join(format!("{named}.html"));
    let out = dir.join(format!("{named}.docx"));
    std::fs::write(&source, SOURCE).ok()?;

    let done = std::process::Command::new("textutil")
        .args(["-convert", "docx", "-output"])
        .arg(&out)
        .arg(&source)
        .output()
        .ok()?;
    if !done.status.success() {
        eprintln!("skipping: textutil could not write the fixture");
        return None;
    }
    Some(out)
}

#[test]
fn a_document_from_another_producer_can_be_read_at_all() {
    let Some(path) = apple_written("readable") else {
        return;
    };
    let model = studio_docs::read(&path).expect("the Core should read it");
    assert!(
        model.html.contains("Master Services Agreement"),
        "the document's own words are missing"
    );
    assert!(model.block_count > 0, "nothing in it can be edited");
}

/// Apple's converter keeps no styles, so the size and weight are all the structure there is.
#[test]
fn direct_formatting_reaches_the_screen() {
    let Some(path) = apple_written("formatting") else {
        return;
    };
    let html = studio_docs::read(&path).unwrap().html;
    assert!(
        html.contains("font-size"),
        "a title set in 24pt is drawn at body size, so the document looks flat"
    );
    assert!(
        html.contains("<strong") || html.contains("<b>"),
        "bold is lost"
    );
}

/// The point of the exercise: a document with no styled headings still has sections.
#[test]
fn its_sections_can_still_be_navigated() {
    let Some(path) = apple_written("sections") else {
        return;
    };
    let model = studio_docs::read(&path).unwrap();
    let found: Vec<(&u8, &str)> = model
        .outline
        .iter()
        .map(|item| (&item.level, item.text.as_str()))
        .collect();

    assert!(
        !model.outline.is_empty(),
        "a document whose headings are large bold text has no sections to move around by"
    );
    assert_eq!(
        found,
        [
            (&1, "Master Services Agreement"),
            (&2, "1. Obligations"),
            (&2, "2. Schedule A")
        ],
        "the sections are not the ones a reader would name"
    );

    // And every section points at a paragraph that can actually be changed.
    let editable = studio_docs::block_indices(&model.html);
    for item in &model.outline {
        assert!(
            editable.contains(&item.index),
            "section {:?} points at block {} which cannot be edited",
            item.text,
            item.index
        );
    }
}
