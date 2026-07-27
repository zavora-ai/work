//! How many pages the document says it has, against how many the file claims in its properties.

fn main() {
    let folder = std::env::args().nth(1).unwrap_or("/tmp/real-small".into());
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&folder)
        .expect("readable")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "docx"))
        .collect();
    paths.sort_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(0));

    println!("  {:>6} {:>8}  document", "pages", "declared");
    for path in &paths {
        let Ok(model) = studio_docs::read_referencing_images(path) else {
            continue;
        };
        let breaks = model.html.matches("page-break rendered").count();
        println!(
            "  {:>6} {:>8}  {}",
            breaks + 1,
            "",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
    }
}
