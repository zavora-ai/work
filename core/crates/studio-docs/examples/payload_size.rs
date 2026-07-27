//! How much the interface is asked to swallow.
//!
//! The Core hands the renderer one model per document, with the images inlined. That is fine for a
//! contract and a question for a 200MB manuscript: a payload measured in hundreds of megabytes has
//! to cross a channel, be parsed as JSON, and become DOM. A document that reads in a second and
//! then hangs the window has not been handled.

fn main() {
    let folder = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/real-big".to_string());

    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&folder)
        .expect("the folder should be readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "docx"))
        .collect();
    paths.sort_by_key(|path| path.metadata().map(|m| m.len()).unwrap_or(0));

    println!(
        "{:>8}  {:>9}  {:>10}  {:>6}  document",
        "file", "inlined", "referenced", "blocks"
    );

    for path in &paths {
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        let Ok(model) = studio_docs::read(path) else {
            continue;
        };
        let Ok(light) = studio_docs::read_referencing_images(path) else {
            continue;
        };
        let payload = serde_json::to_string(&model)
            .map(|json| json.len())
            .unwrap_or(0);
        let referenced = serde_json::to_string(&light)
            .map(|json| json.len())
            .unwrap_or(0);
        println!(
            "{:>7}K  {:>8}K  {:>7}K  {:>6}  {}",
            size / 1024,
            payload / 1024,
            referenced / 1024,
            model.block_count,
            path.file_name().unwrap_or_default().to_string_lossy()
        );
    }
}
