//! Reading a deck for the interface.
//!
//! The third of the three, and the same shape as the other two: the Core reads the file
//! and the renderer draws what it is given.
//!
//! ## What was missing, and what was added
//!
//! Documents were already addressable — `zavora-docx-html` emits `data-p` per block.
//! Slides were not: `scene_to_svg` produced a faithful drawing in which nothing could be
//! named, so a click on a shape could not be traced to anything and a change could not be
//! attributed. That was the real blocker task 13.6 described.
//!
//! It is now fixed upstream by `SvgOptions { identify: true }`, which wraps each scene
//! item in `<g data-item="{index}">`. The option is off by default, so every existing
//! caller renders byte-for-byte the same drawing — which is why all 739 of
//! `zavora-slide`'s own tests still pass.

use serde::{Deserialize, Serialize};
use zavora_slide::Presentation;

#[derive(Debug, thiserror::Error)]
pub enum DeckError {
    #[error("that file could not be opened — it may not be a presentation")]
    Open { detail: String },
    #[error("there is no slide {0} in this deck")]
    NoSuchSlide(usize),
}

impl DeckError {
    /// The underlying cause, for support. Never shown on a primary surface.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Open { detail } => Some(detail),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, DeckError>;

/// One slide, drawn and addressable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slide {
    /// Position in the deck, from one, as the User counts them.
    pub number: usize,
    /// A title for the thumbnail strip, taken from the slide's own text.
    pub title: String,
    /// The drawing. Every element carries `data-item` so it can be selected.
    pub svg: String,
    /// How many things are on the slide and therefore addressable.
    pub item_count: usize,
    /// What each drawn element refers to, by position. `None` where the drawing came from
    /// something the User cannot yet change.
    pub targets: Vec<Option<Target>>,
    /// What the speaker is meant to say over this slide, where the deck says.
    ///
    /// The presenter needs it in front of them, and it is what an agent presenting the deck has to
    /// work from — without it the agent would be inventing the talk rather than giving the one the
    /// deck was written for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl Slide {
    /// What the drawn element at `index` refers to.
    ///
    /// This is the whole point of the identifiers: a click resolves to something a
    /// specialist can be asked to change.
    pub fn target_at(&self, index: usize) -> Option<Target> {
        self.targets.get(index).copied().flatten()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckModel {
    pub file_name: String,
    pub slides: Vec<Slide>,
    /// Which slide the interface should show first.
    pub active: usize,
}

impl DeckModel {
    pub fn slide(&self, number: usize) -> Option<&Slide> {
        self.slides.iter().find(|slide| slide.number == number)
    }

    pub fn active_slide(&self) -> Option<&Slide> {
        self.slides.get(self.active)
    }
}

/// The width the interface draws at.
pub const RENDER_WIDTH: u32 = 1280;

/// Read a deck into a model the interface can draw and select in.
pub fn read(path: &std::path::Path) -> Result<DeckModel> {
    let presentation = Presentation::open(path).map_err(|e| DeckError::Open {
        detail: e.to_string(),
    })?;

    let mut slides = Vec::new();
    for index in 0..presentation.slide_count() {
        let scene = presentation
            .slide(index)
            .map_err(|e| DeckError::Open {
                detail: e.to_string(),
            })?
            .scene();
        let svg = zavora_slide_render::scene_to_svg_with(
            &scene,
            RENDER_WIDTH,
            zavora_slide_render::SvgOptions { identify: true },
        );
        let targets = (0..scene.items.len())
            .map(|item| {
                scene.source_of(item).map(|source| match source {
                    zavora_slide_layout::ItemSource::Shape(i) => Target::Shape(i),
                    zavora_slide_layout::ItemSource::Image(i) => Target::Picture(i),
                    zavora_slide_layout::ItemSource::Table(i) => Target::Table(i),
                    zavora_slide_layout::ItemSource::Chart(i) => Target::Chart(i),
                    zavora_slide_layout::ItemSource::Background => Target::Background,
                })
            })
            .collect();
        let notes = presentation
            .slide(index)
            .ok()
            .and_then(|slide| slide.notes().map(str::to_string))
            .filter(|notes| !notes.trim().is_empty());

        slides.push(Slide {
            number: index + 1,
            notes,
            title: title_of(&scene, index),
            item_count: scene.items.len(),
            targets,
            svg,
        });
    }

    Ok(DeckModel {
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        slides,
        active: 0,
    })
}

/// What a click on a drawn element refers to.
///
/// The interface hands back what the User pointed at; this says what may be changed. A
/// shape is the unit the engine's own edit calls take, which is why it is the one named
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "refers_to", content = "position")]
pub enum Target {
    Shape(usize),
    Picture(usize),
    Table(usize),
    Chart(usize),
    /// The slide itself, behind everything on it.
    Background,
}

/// What each drawn element on a slide refers to, by element index.
pub fn targets(slide: &Slide) -> Vec<Option<Target>> {
    slide.targets.clone()
}

/// Every element index present in a rendered slide, in order.
///
/// The interface uses this to know what it may select; the test below uses it to prove a
/// slide is addressable at all.
pub fn item_indices(svg: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut rest = svg;
    while let Some(at) = rest.find("data-item=\"") {
        rest = &rest[at + 11..];
        if let Some(end) = rest.find('"') {
            if let Ok(index) = rest[..end].parse::<usize>() {
                found.push(index);
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    found
}

/// A name for the thumbnail strip, from the slide's own first line of text.
fn title_of(scene: &zavora_slide_layout::Scene, index: usize) -> String {
    for item in &scene.items {
        if let zavora_slide_layout::Item::Text { lines, .. } = item {
            for line in lines {
                let trimmed = line.text.trim();
                if !trimmed.is_empty() {
                    return trimmed.chars().take(40).collect();
                }
            }
        }
    }
    format!("Slide {}", index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("zws-deck-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn write_deck(path: &std::path::Path) {
        use zavora_slide::{Emu, Layout};

        let mut presentation = Presentation::new();
        for title in ["Revenue by region — Q3", "What we are asking for"] {
            let index = presentation.add_slide(Layout::Blank);
            presentation.slide_mut(index).unwrap().add_text_box(
                title,
                Emu(914_400),
                Emu(914_400),
                Emu(6_400_800),
                Emu(1_000_000),
            );
        }
        presentation.save(path).unwrap();
    }

    #[test]
    fn every_slide_is_read_and_drawn() {
        let path = fixture("deck.pptx");
        write_deck(&path);
        let model = read(&path).expect("reads");

        assert_eq!(model.file_name, "deck.pptx");
        assert_eq!(model.slides.len(), 2);
        assert_eq!(model.slides[0].number, 1, "the User counts slides from one");
        assert_eq!(model.active, 0);
        assert!(model.slides[0].svg.starts_with("<svg"));
        assert!(model.slide(2).is_some());
        assert!(model.slide(9).is_none());
    }

    /// The whole point of the upstream change: a slide's elements can be named.
    #[test]
    fn every_element_on_a_slide_is_addressable() {
        let path = fixture("addressable.pptx");
        write_deck(&path);
        let model = read(&path).unwrap();
        let slide = model.active_slide().expect("a first slide");

        assert!(
            slide.item_count > 0,
            "the slide should have something on it"
        );
        let indices = item_indices(&slide.svg);
        assert_eq!(
            indices.len(),
            slide.item_count,
            "every element must be addressable, or a click cannot be traced to anything"
        );
        assert_eq!(
            indices,
            (0..slide.item_count).collect::<Vec<_>>(),
            "indices must be the item positions, in order"
        );
    }

    #[test]
    fn a_slide_is_named_from_its_own_text() {
        let path = fixture("titles.pptx");
        write_deck(&path);
        let model = read(&path).unwrap();
        assert_eq!(model.slides[0].title, "Revenue by region — Q3");
        assert_eq!(model.slides[1].title, "What we are asking for");
    }

    #[test]
    fn a_file_that_is_not_a_presentation_says_so_without_technical_detail() {
        let path = fixture("not-a-deck.pptx");
        std::fs::write(&path, b"this is not a presentation").unwrap();
        let error = read(&path).expect_err("must fail");
        let message = error.to_string();
        assert!(message.starts_with("that file could not be opened"));
        assert!(
            !message.to_lowercase().contains("zip") && !message.contains("EOCD"),
            "the User must never be shown the cause: {message}"
        );
        assert!(
            error.detail().is_some(),
            "but support must be able to see it"
        );
    }

    #[test]
    fn the_model_survives_a_round_trip_through_json() {
        let path = fixture("json.pptx");
        write_deck(&path);
        let model = read(&path).unwrap();
        let json = serde_json::to_string(&model).unwrap();
        let back: DeckModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model, back);
        assert!(
            json.contains("\"itemCount\""),
            "the renderer reads camelCase"
        );
    }

    #[test]
    fn indices_are_read_back_out_of_a_drawing() {
        assert_eq!(
            item_indices(r#"<g data-item="0"><rect/></g><g data-item="2"><text/></g>"#),
            vec![0, 2]
        );
        assert_eq!(item_indices("<svg><rect/></svg>"), Vec::<usize>::new());
    }

    /// Without this, an identifier is decoration: it says which drawn element was clicked
    /// but not what could be changed.
    #[test]
    fn a_drawn_element_resolves_to_something_that_can_be_changed() {
        let path = fixture("targets.pptx");
        write_deck(&path);
        let model = read(&path).unwrap();
        let slide = model.active_slide().unwrap();

        assert_eq!(
            slide.targets.len(),
            slide.item_count,
            "every drawn element must resolve to something, or to a known nothing"
        );
        let resolved: Vec<_> = slide.targets.iter().flatten().collect();
        assert!(
            !resolved.is_empty(),
            "a slide with a text box must have at least one changeable thing"
        );
        assert!(
            matches!(slide.target_at(0), Some(Target::Shape(_))),
            "the text box must resolve to a shape, got {:?}",
            slide.target_at(0)
        );
    }

    #[test]
    fn what_a_click_refers_to_survives_the_crossing_to_the_interface() {
        let path = fixture("targets-json.pptx");
        write_deck(&path);
        let model = read(&path).unwrap();
        let json = serde_json::to_string(&model).unwrap();
        let back: DeckModel = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.active_slide().unwrap().target_at(0),
            model.active_slide().unwrap().target_at(0)
        );
    }
}

#[cfg(test)]
mod notes_tests {
    use super::*;

    /// An agent asked to present a deck has to say what the deck says. Without the notes it would
    /// be inventing the talk.
    #[test]
    fn the_speaker_notes_are_read() {
        let path = std::env::temp_dir().join(format!("zws-notes-{}.pptx", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            use zavora_slide::{Emu, Layout};
            let mut deck = Presentation::new();
            let index = deck.add_slide(Layout::Blank);
            let mut slide = deck.slide_mut(index).unwrap();
            slide.add_text_box(
                "Revenue by region",
                Emu(914_400),
                Emu(914_400),
                Emu(6_400_800),
                Emu(1_000_000),
            );
            slide.set_notes("Start with the shortfall in the north, then the plan.");
            deck.save(&path).unwrap();
        }

        let model = read(&path).expect("reads");
        assert_eq!(
            model.slides[0].notes.as_deref(),
            Some("Start with the shortfall in the north, then the plan."),
            "the talk the deck was written for is missing"
        );
    }

    /// A deck with no notes says so, rather than offering an empty string to talk over.
    #[test]
    fn a_deck_without_notes_says_nothing() {
        let path = std::env::temp_dir().join(format!("zws-nonotes-{}.pptx", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            use zavora_slide::{Emu, Layout};
            let mut deck = Presentation::new();
            let index = deck.add_slide(Layout::Blank);
            deck.slide_mut(index).unwrap().add_text_box(
                "Only a title",
                Emu(914_400),
                Emu(914_400),
                Emu(6_400_800),
                Emu(1_000_000),
            );
            deck.save(&path).unwrap();
        }
        assert_eq!(read(&path).expect("reads").slides[0].notes, None);
    }
}
