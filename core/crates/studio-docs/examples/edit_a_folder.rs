//! Edit a real document and see what it cost.
//!
//! Reading a file proves the parser copes. Saving one proves rather more: everything the parser did
//! not model has to come back out of the file unchanged, and a real document from Word carries a
//! great deal that nothing here models — theme parts, fonts, settings, custom XML, revision
//! history, embedded objects.
//!
//! Run against copies. The source documents belong to the User.

use std::collections::BTreeMap;
use std::io::Read;

/// Every part of the package and how large each is.
fn parts_of(path: &std::path::Path) -> BTreeMap<String, u64> {
    let file = std::fs::File::open(path).expect("the copy should be readable");
    let mut zip = zip::ZipArchive::new(file).expect("it should be a document");
    let mut found = BTreeMap::new();
    for index in 0..zip.len() {
        let entry = zip.by_index(index).expect("an entry");
        found.insert(entry.name().to_string(), entry.size());
    }
    found
}

fn body_of(path: &std::path::Path) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut out = String::new();
    zip.by_name("word/document.xml")
        .expect("a body")
        .read_to_string(&mut out)
        .unwrap();
    out
}

fn main() {
    let folder = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/real-small".to_string());
    let working = std::path::Path::new("/tmp/real-edit-copies");
    std::fs::create_dir_all(working).expect("somewhere to work");

    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&folder)
        .expect("the folder should be readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "docx"))
        .collect();
    paths.sort_by_key(|path| path.metadata().map(|m| m.len()).unwrap_or(0));

    println!(
        "{:>7}  {:>7}  {:>6} {:>6}  {:<34} document",
        "size", "took", "parts", "kept", "what changed"
    );

    let mut clean = 0usize;
    let mut lost = 0usize;

    for source in &paths {
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let copy = working.join(&name);
        if std::fs::copy(source, &copy).is_err() {
            println!(
                "{:>7}  {:>7}  {:>6} {:>6}  {:<34} {}",
                "", "", "", "", "COULD NOT COPY", name
            );
            continue;
        }

        let before = parts_of(&copy);
        let began = std::time::Instant::now();

        // What the interface asks for: the words of one block, replaced.
        let edited = std::panic::catch_unwind(|| {
            let mut document = zavora_docx::Document::open(&copy)?;
            // The first block that is a paragraph. A document beginning with a table would
            // otherwise be reported as a refusal when it is nothing of the kind.
            let mut changed = false;
            for index in 0..12 {
                if document.set_paragraph_text(index, "Changed by Work Studio.") {
                    changed = true;
                    break;
                }
            }
            if !changed {
                return Ok(false);
            }
            document.save(&copy)?;
            Ok::<bool, zavora_docx::Error>(true)
        });
        let took = began.elapsed().as_millis();

        match edited {
            Ok(Ok(true)) => {
                let after = parts_of(&copy);
                let missing: Vec<&String> = before
                    .keys()
                    .filter(|name| !after.contains_key(*name))
                    .collect();
                let landed = body_of(&copy).contains("Changed by Work Studio");

                let mut notes = Vec::new();
                if !missing.is_empty() {
                    notes.push(format!(
                        "LOST {}: {:?}",
                        missing.len(),
                        &missing[..missing.len().min(3)]
                    ));
                }
                if !landed {
                    notes.push("the change did not land".to_string());
                }
                if notes.is_empty() {
                    clean += 1;
                } else {
                    lost += 1;
                }
                println!(
                    "{:>6}K  {:>6}ms  {:>6} {:>6}  {:<34} {}",
                    source.metadata().map(|m| m.len()).unwrap_or(0) / 1024,
                    took,
                    before.len(),
                    after.len(),
                    if notes.is_empty() {
                        "nothing".to_string()
                    } else {
                        notes.join("; ")
                    },
                    name
                );
            }
            Ok(Ok(false)) => println!(
                "{:>6}K  {:>6}ms  {:>6} {:>6}  {:<34} {}",
                source.metadata().map(|m| m.len()).unwrap_or(0) / 1024,
                took,
                before.len(),
                before.len(),
                "no paragraph in the first twelve",
                name
            ),
            Ok(Err(error)) => {
                lost += 1;
                println!(
                    "{:>6}K  {:>6}ms  {:>6} {:>6}  {:<34} {}",
                    source.metadata().map(|m| m.len()).unwrap_or(0) / 1024,
                    took,
                    before.len(),
                    0,
                    format!("REFUSED: {error}"),
                    name
                );
            }
            Err(_) => {
                lost += 1;
                println!(
                    "{:>6}K  {:>6}ms  {:>6} {:>6}  {:<34} {}",
                    source.metadata().map(|m| m.len()).unwrap_or(0) / 1024,
                    took,
                    before.len(),
                    0,
                    "PANICKED",
                    name
                );
            }
        }
    }

    println!();
    println!("{clean} edited without losing a part, {lost} with something to answer for");
}
