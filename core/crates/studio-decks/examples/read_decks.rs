//! Read every presentation in a folder and say what arrived.

fn main() {
    let folder = std::env::args().nth(1).unwrap_or("/tmp/real-decks".into());
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&folder)
        .expect("readable")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pptx"))
        .collect();
    paths.sort_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(0));

    println!(
        "  {:>7} {:>7} {:>6} {:>6} {:<22} document",
        "size", "took", "slides", "items", "what arrived"
    );
    for path in &paths {
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        let began = std::time::Instant::now();
        let outcome = std::panic::catch_unwind(|| studio_decks::read(path));
        let took = began.elapsed().as_millis();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        match outcome {
            Ok(Ok(model)) => {
                let svg: String = model.slides.iter().map(|s| s.svg.clone()).collect();
                let mut has = Vec::new();
                for (label, needle) in [
                    ("text", "<text"),
                    ("images", "<image"),
                    ("shapes", "<rect"),
                    ("lines", "<path"),
                ] {
                    if svg.contains(needle) {
                        has.push(label);
                    }
                }
                println!(
                    "  {:>6}K {:>6}ms {:>6} {:>6} {:<22} {}",
                    size / 1024,
                    took,
                    model.slides.len(),
                    model.slides.iter().map(|s| s.item_count).sum::<usize>(),
                    has.join(","),
                    name
                );
            }
            Ok(Err(e)) => println!(
                "  {:>6}K {:>6}ms {:>6} {:>6} {:<22} {}",
                size / 1024,
                took,
                0,
                0,
                format!("REFUSED: {e}"),
                name
            ),
            Err(_) => println!(
                "  {:>6}K {:>6}ms {:>6} {:>6} {:<22} {}",
                size / 1024,
                took,
                0,
                0,
                "PANICKED",
                name
            ),
        }
    }
}
