//! What each slide's speaker notes say — the talk the deck was written for.
fn main() {
    let path = std::env::args().nth(1).expect("a deck");
    let model = studio_decks::read(std::path::Path::new(&path)).expect("reads");
    let with = model.slides.iter().filter(|s| s.notes.is_some()).count();
    println!("  {} of {} slides carry notes", with, model.slides.len());
    for slide in model.slides.iter().take(4) {
        println!(
            "   {}. {:<34} {}",
            slide.number,
            slide.title.chars().take(34).collect::<String>(),
            slide
                .notes
                .as_deref()
                .map(|n| n.chars().take(56).collect::<String>())
                .unwrap_or_else(|| "—".into())
        );
    }
}
