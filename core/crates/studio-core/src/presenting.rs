//! Presenting a deck out loud.
//!
//! Two things a presenter does that a deck cannot: decide what to say about a slide, and say it.
//!
//! What to say comes from the deck where the deck says — the speaker notes are the talk it was
//! written for — and from the slide itself where it does not, because a deck without notes is the
//! common case and refusing to present it would be refusing to do the thing that was asked. What
//! comes back is marked as written rather than read, so the User is never told the deck said
//! something it did not.
//!
//! Saying it is a voice provider, reached with the same credential as everything else. The audio is
//! handed to the interface as bytes to play, which keeps the sound on the User's own machine rather
//! than in a browser somewhere.

use serde::Serialize;

/// What to say over one slide.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Talk {
    pub slide: usize,
    pub words: String,
    /// True when the deck's own notes were used, false when the words were written from the slide.
    ///
    /// The difference matters to the User: one is what they wrote, the other is a suggestion.
    pub from_the_deck: bool,
}

/// The talk for every slide in a deck.
///
/// Notes where there are notes. Where there are none, the slide's own words in the order they are
/// drawn — which is a fair reading of the slide and is honestly marked as not being the author's.
pub fn talk_for(model: &studio_decks::DeckModel) -> Vec<Talk> {
    model
        .slides
        .iter()
        .map(|slide| match slide.notes.as_deref() {
            Some(notes) if !notes.trim().is_empty() => Talk {
                slide: slide.number,
                words: notes.trim().to_string(),
                from_the_deck: true,
            },
            _ => Talk {
                slide: slide.number,
                words: read_aloud(slide),
                from_the_deck: false,
            },
        })
        .collect()
}

/// A slide read as a person would read it out: the title, then what is on it.
///
/// Deliberately plain. Inventing commentary would put words in the User's mouth in front of an
/// audience, which is the one place a wrong sentence cannot be taken back.
fn read_aloud(slide: &studio_decks::Slide) -> String {
    let mut said = String::new();
    if !slide.title.trim().is_empty() {
        said.push_str(slide.title.trim());
        said.push('.');
    }

    // The text drawn on the slide, minus the title. A title is often drawn as several wrapped
    // fragments, so a fragment of it is skipped too — otherwise the talk opens by saying the title
    // and then saying it again in pieces.
    let title = slide.title.trim();
    let mut rest: Vec<String> = text_in(&slide.svg)
        .into_iter()
        .map(|line| said_aloud(&line))
        .filter(|line| {
            let line = line.trim();
            !line.is_empty()
                && !(!title.is_empty()
                    && (line == title || title.contains(line) || line.contains(title)))
        })
        .collect();
    rest.dedup();

    for line in rest {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        said.push(' ');
        said.push_str(line);
        if !line.ends_with('.') && !line.ends_with('?') && !line.ends_with('!') {
            said.push('.');
        }
    }
    said
}

/// A line as it should be said rather than as it is drawn.
///
/// A bullet is a mark on the page, not a word: read out, "bullet 70% of children" is not what the
/// slide says. The same for the dashes and arrows decks use in their place.
fn said_aloud(line: &str) -> String {
    line.trim_start_matches(|c: char| {
        matches!(
            c,
            '•' | '·' | '‣' | '▪' | '◦' | '–' | '—' | '-' | '*' | '>' | '→'
        ) || c.is_whitespace()
    })
    .trim()
    .to_string()
}

/// The words drawn on a slide, in the order they are drawn.
fn text_in(svg: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = svg;
    while let Some(at) = rest.find("<text") {
        rest = &rest[at..];
        let Some(open) = rest.find('>') else { break };
        let Some(close) = rest[open..].find("</text>") else {
            break;
        };
        let inner = &rest[open + 1..open + close];
        // Nested spans, which is how a wrapped line is drawn.
        let plain = strip_tags(inner);
        if !plain.trim().is_empty() {
            found.push(plain.trim().to_string());
        }
        rest = &rest[open + close..];
    }
    found
}

fn strip_tags(fragment: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for character in fragment.chars() {
        match character {
            '<' => inside = true,
            '>' => {
                inside = false;
                // A tag boundary is a word boundary: two spans run together would otherwise be
                // read as one word.
                out.push(' ');
            }
            _ if !inside => out.push(character),
            _ => {}
        }
    }
    let plain = out.split_whitespace().collect::<Vec<_>>().join(" ");
    unescaped(&plain)
}

/// A drawing's text as words rather than as markup.
///
/// The slide is drawn as XML, so an ampersand arrives as `&amp;`. Said aloud that is "amp", which
/// is not a word the author wrote.
fn unescaped(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_slide(number: usize, title: &str, svg: &str, notes: Option<&str>) -> studio_decks::Slide {
        studio_decks::Slide {
            number,
            title: title.to_string(),
            svg: svg.to_string(),
            item_count: 0,
            targets: Vec::new(),
            notes: notes.map(str::to_string),
        }
    }

    #[test]
    fn the_decks_own_notes_are_the_talk() {
        let model = studio_decks::DeckModel {
            file_name: "deck.pptx".into(),
            slides: vec![a_slide(
                1,
                "The Problem",
                "<text>The Problem</text>",
                Some("Open with the shortfall, then the plan."),
            )],
            active: 0,
        };
        let talk = talk_for(&model);
        assert_eq!(talk[0].words, "Open with the shortfall, then the plan.");
        assert!(
            talk[0].from_the_deck,
            "the User wrote this, and should be told so"
        );
    }

    #[test]
    fn a_slide_without_notes_is_read_as_it_stands() {
        let model = studio_decks::DeckModel {
            file_name: "deck.pptx".into(),
            slides: vec![a_slide(
                1,
                "Revenue by region",
                "<text>Revenue by region</text><text>North is behind</text><text>South is ahead</text>",
                None,
            )],
            active: 0,
        };
        let talk = talk_for(&model);
        assert_eq!(
            talk[0].words,
            "Revenue by region. North is behind. South is ahead."
        );
        assert!(
            !talk[0].from_the_deck,
            "these are our words, not the author's, and saying otherwise would be a lie in front of an audience"
        );
    }

    /// The title is said once, not twice.
    #[test]
    fn the_title_is_not_repeated() {
        let model = studio_decks::DeckModel {
            file_name: "deck.pptx".into(),
            slides: vec![a_slide(
                1,
                "Highlights",
                "<text>Highlights</text><text>Two new customers</text>",
                None,
            )],
            active: 0,
        };
        assert_eq!(talk_for(&model)[0].words, "Highlights. Two new customers.");
    }

    #[test]
    fn a_wrapped_line_reads_as_a_sentence_rather_than_a_run_of_words() {
        assert_eq!(
            strip_tags("<tspan>Revenue by</tspan><tspan>region</tspan>"),
            "Revenue by region"
        );
    }

    #[test]
    fn an_empty_slide_says_nothing_rather_than_something() {
        let model = studio_decks::DeckModel {
            file_name: "deck.pptx".into(),
            slides: vec![a_slide(1, "", "", None)],
            active: 0,
        };
        assert_eq!(talk_for(&model)[0].words, "");
    }

    /// A title drawn as wrapped fragments must not be said twice.
    #[test]
    fn a_wrapped_title_is_said_once() {
        let model = studio_decks::DeckModel {
            file_name: "deck.pptx".into(),
            slides: vec![a_slide(
                1,
                "Revolutionizing Digital Content",
                "<text>Revolutionizing Digital Content</text><text>Revolutionizing Digital</text><text>Content</text><text>Two new customers</text>",
                None,
            )],
            active: 0,
        };
        assert_eq!(
            talk_for(&model)[0].words,
            "Revolutionizing Digital Content. Two new customers."
        );
    }

    /// A bullet is a mark on the page, not a word.
    #[test]
    fn bullet_marks_are_not_read_out() {
        let model = studio_decks::DeckModel {
            file_name: "deck.pptx".into(),
            slides: vec![a_slide(
                1,
                "The Problem",
                "<text>The Problem</text><text>• 70% of children lack access</text><text>— and fees are rising</text>",
                None,
            )],
            active: 0,
        };
        assert_eq!(
            talk_for(&model)[0].words,
            "The Problem. 70% of children lack access. and fees are rising."
        );
    }

    /// An ampersand in a title is an ampersand, not the word "amp".
    #[test]
    fn markup_is_not_read_out_as_words() {
        let model = studio_decks::DeckModel {
            file_name: "deck.pptx".into(),
            slides: vec![a_slide(
                1,
                "Content & Education",
                "<text>Content &amp; Education</text><text>Two markets</text>",
                None,
            )],
            active: 0,
        };
        let words = &talk_for(&model)[0].words;
        assert!(!words.contains("amp;"), "reads as markup: {words}");
        assert_eq!(words, "Content & Education. Two markets.");
    }
}
