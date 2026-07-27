//! Read every document in a folder and say what happened to each.
//!
//! Run against a folder of real files, which is the only way to find out what real files contain.
//! Reports one line per document: whether it could be read at all, how long it took, how much of it
//! there is, and which constructs arrived — so a failure is a named document with a named cause
//! rather than an impression.

fn main() {
    let folder = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/real-docs".to_string());

    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&folder)
        .expect("the folder should be readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "docx"))
        .collect();
    // Smallest first, so a failure shows up before a long wait.
    paths.sort_by_key(|path| path.metadata().map(|m| m.len()).unwrap_or(0));

    let mut read = 0usize;
    let mut failed = 0usize;
    let mut slowest = (0u128, String::new());

    println!(
        "{:>7}  {:>7}  {:>6}  {:>7}  {:<28} document",
        "size", "took", "blocks", "outline", "what arrived"
    );

    for path in &paths {
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let began = std::time::Instant::now();

        // A panic in a parser is a failure of that document, not of the run: one bad file must not
        // hide the other fifty-five.
        let outcome = std::panic::catch_unwind(|| studio_docs::read(path));
        let took = began.elapsed().as_millis();
        if took > slowest.0 {
            slowest = (took, name.clone());
        }

        match outcome {
            Ok(Ok(model)) => {
                read += 1;
                let mut has = Vec::new();
                for (label, needle) in [
                    ("headings", "<h1"),
                    ("lists", "<li"),
                    ("tables", "<table"),
                    ("images", "<img"),
                    ("links", "href="),
                    ("formulas", "class=\"formula\""),
                ] {
                    if model.html.contains(needle) {
                        has.push(label);
                    }
                }
                println!(
                    "{:>6}K  {:>6}ms  {:>6}  {:>7}  {:<28} {}",
                    size / 1024,
                    took,
                    model.block_count,
                    model.outline.len(),
                    has.join(","),
                    name
                );
            }
            Ok(Err(error)) => {
                failed += 1;
                println!(
                    "{:>6}K  {:>6}ms  {:>6}  {:>7}  {:<28} {}",
                    size / 1024,
                    took,
                    0,
                    0,
                    format!("REFUSED: {error}"),
                    name
                );
            }
            Err(_) => {
                failed += 1;
                println!(
                    "{:>6}K  {:>6}ms  {:>6}  {:>7}  {:<28} {}",
                    size / 1024,
                    took,
                    0,
                    0,
                    "PANICKED",
                    name
                );
            }
        }
    }

    println!();
    println!(
        "read {read} of {} documents, {failed} not read",
        paths.len()
    );
    println!("slowest: {}ms — {}", slowest.0, slowest.1);
}
