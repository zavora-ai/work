//! How long it takes to serve every picture in a document, the way a page asks for them.

fn main() {
    let path = std::env::args().nth(1).expect("a document");
    let path = std::path::Path::new(&path);
    let model = studio_docs::read_referencing_images(path).expect("readable");

    let ids: Vec<String> = model
        .html
        .split("data-media=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next().map(str::to_string))
        .collect();
    println!("  pictures referenced: {}", ids.len());

    let began = std::time::Instant::now();
    let mut served = 0usize;
    let mut bytes = 0usize;
    for id in ids.iter().take(60) {
        if let Ok((_, data)) = studio_docs::media(path, id) {
            served += 1;
            bytes += data.len();
        }
    }
    println!(
        "  holding the document : {served} pictures in {}ms ({}KB)",
        began.elapsed().as_millis(),
        bytes / 1024
    );

    // The same again, letting go each time — which is what serving them cost before.
    let began = std::time::Instant::now();
    let mut served = 0usize;
    for id in ids.iter().take(6) {
        studio_docs::forget_pictures();
        if studio_docs::media(path, id).is_ok() {
            served += 1;
        }
    }
    let each = began.elapsed().as_millis() / served.max(1) as u128;
    println!(
        "  reading it each time : {served} pictures in {}ms — {}ms each, so {} for all {}",
        began.elapsed().as_millis(),
        each,
        format_args!("{}s", each * ids.len() as u128 / 1000),
        ids.len()
    );
}
