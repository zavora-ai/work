//! The sections of a document that never used heading styles.
//!
//! Plenty of real documents have no styled headings at all. Their author made the title big and
//! bold and moved on; a converter threw the styles away; the file came out of a tool that only
//! writes direct formatting. Those documents have sections — a reader can see them at a glance —
//! and Work Studio showed an empty list, so there was no way to move around a long one.
//!
//! What a reader uses to tell a heading from a sentence is what is used here: it is set larger
//! than the body text around it, it is short, and it does not end in a full stop. All three,
//! because any one alone turns emphasis into structure — a bold sentence is not a heading, and a
//! long line in a large font is a pull quote.
//!
//! Only ever consulted when the document has no styled headings of its own. A document that says
//! what its headings are is believed.

use crate::OutlineItem;

/// Longest a line can be and still be a heading. Long enough for a real one — "Limitation of
/// liability and indemnities" is 40 — and short enough to exclude a sentence.
const LONGEST: usize = 90;

/// How much larger than the body a heading must be. Word's own Heading 3 is only slightly larger
/// than body text, so this is deliberately close to 1.
const LARGER_BY: f64 = 1.08;

/// One paragraph, as far as this needs to know about it.
struct Block {
    index: usize,
    text: String,
    size: Option<f64>,
    bold: bool,
}

/// The sections of a document with no headings of its own.
pub fn inferred_sections(html: &str) -> Vec<OutlineItem> {
    let blocks = blocks_of(html);
    if blocks.is_empty() {
        return Vec::new();
    }

    // The body size is the one most of the document is set in, by how much text is in it rather
    // than by how many paragraphs — a document with one long chapter and forty captions has body
    // text the size of the chapter.
    let Some(body_size) = dominant_size(&blocks) else {
        return Vec::new();
    };

    let mut candidates: Vec<&Block> = blocks
        .iter()
        .filter(|block| looks_like_a_heading(block, body_size))
        .collect();

    // A document where everything qualifies has told us nothing. That happens with a file whose
    // every paragraph is a short bold line — a form, or a list of names — and an outline listing
    // every line is worse than none.
    if candidates.len() > blocks.len() / 2 {
        return Vec::new();
    }
    candidates.sort_by_key(|block| block.index);

    // Levels from size: the largest heading in the document is its first level, the next size down
    // its second. Real headings are numbered this way and readers read them this way.
    let mut sizes: Vec<f64> = candidates
        .iter()
        .map(|block| block.size.unwrap_or(body_size))
        .collect();
    sizes.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    sizes.dedup_by(|a, b| (*a - *b).abs() < 0.51);

    candidates
        .into_iter()
        .map(|block| {
            let size = block.size.unwrap_or(body_size);
            let level = sizes
                .iter()
                .position(|candidate| (candidate - size).abs() < 0.51)
                .unwrap_or(0);
            OutlineItem {
                index: block.index,
                // Capped at 6, which is as deep as a heading goes.
                level: (level as u8 + 1).min(6),
                text: block.text.clone(),
            }
        })
        .collect()
}

fn looks_like_a_heading(block: &Block, body_size: f64) -> bool {
    if block.text.is_empty() || block.text.chars().count() > LONGEST {
        return false;
    }
    // A full stop is the strongest signal of a sentence. A colon is not — "Definitions:" is a
    // heading.
    if block.text.ends_with('.') || block.text.ends_with('?') || block.text.ends_with('!') {
        return false;
    }
    let larger = block.size.is_some_and(|size| size >= body_size * LARGER_BY);
    // Bold alone is emphasis. Bold and larger is a heading; larger alone is one too, since a
    // heading in a light font is still a heading.
    larger || (block.bold && block.size.is_some_and(|size| size > body_size))
}

/// The size most of the document's text is set in.
fn dominant_size(blocks: &[Block]) -> Option<f64> {
    let mut weighted: Vec<(f64, usize)> = Vec::new();
    for block in blocks {
        let size = block.size?;
        let characters = block.text.chars().count();
        match weighted
            .iter_mut()
            .find(|(candidate, _)| (*candidate - size).abs() < 0.51)
        {
            Some((_, total)) => *total += characters,
            None => weighted.push((size, characters)),
        }
    }
    weighted
        .into_iter()
        .max_by_key(|(_, total)| *total)
        .map(|(size, _)| size)
}

/// Every paragraph in the fragment, with the size and weight it is set in.
fn blocks_of(html: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut rest = html;

    while let Some(at) = rest.find("<p ") {
        rest = &rest[at..];
        let Some(open_end) = rest.find('>') else {
            break;
        };
        let attributes = &rest[..open_end];
        let Some(index) = attribute(attributes, "data-p").and_then(|v| v.parse::<usize>().ok())
        else {
            rest = &rest[open_end + 1..];
            continue;
        };
        let body = &rest[open_end + 1..];
        let Some(close) = body.find("</p>") else {
            break;
        };
        let inner = &body[..close];

        blocks.push(Block {
            index,
            text: crate::strip_tags(inner).trim().to_string(),
            size: font_size_in(inner),
            bold: inner.contains("<strong") || inner.contains("<b>"),
        });
        rest = &body[close + 4..];
    }
    blocks
}

/// The value of an attribute in a tag's text.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let tail = &tag[start..];
    let end = tail.find('"')?;
    Some(tail[..end].to_string())
}

/// The first font size in a fragment, in points.
fn font_size_in(fragment: &str) -> Option<f64> {
    let at = fragment.find("font-size:")? + "font-size:".len();
    let tail = &fragment[at..];
    let end = tail.find("pt")?;
    tail[..end].trim().parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a document from a tool that keeps no styles looks like.
    const NO_STYLES: &str = r#"
<p data-p="0"><span style="font-family:'Times';font-size:24pt"><strong>Master Services Agreement</strong></span></p>
<p data-p="1"><span style="font-size:12pt">This agreement is made between the parties named below, and sets out what each of them will do.</span></p>
<p data-p="2"><span style="font-size:16pt"><strong>1. Definitions</strong></span></p>
<p data-p="3"><span style="font-size:12pt">The term Services means the work described in Schedule A.</span></p>
<p data-p="4"><span style="font-size:16pt"><strong>2. Obligations</strong></span></p>
<p data-p="5"><span style="font-size:12pt">Each party shall perform its obligations with reasonable skill and care at all times.</span></p>
"#;

    #[test]
    fn a_document_with_no_styles_still_has_sections() {
        let found = inferred_sections(NO_STYLES);
        let texts: Vec<&str> = found.iter().map(|item| item.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "Master Services Agreement",
                "1. Definitions",
                "2. Obligations"
            ]
        );
    }

    #[test]
    fn the_biggest_heading_is_the_first_level() {
        let found = inferred_sections(NO_STYLES);
        assert_eq!(found[0].level, 1, "the title");
        assert_eq!(found[1].level, 2, "and the sections under it");
        assert_eq!(found[2].level, 2);
    }

    #[test]
    fn the_index_points_at_the_paragraph_it_came_from() {
        let found = inferred_sections(NO_STYLES);
        assert_eq!(
            found.iter().map(|item| item.index).collect::<Vec<_>>(),
            [0, 2, 4]
        );
    }

    /// The failure that matters: emphasis is not structure.
    #[test]
    fn a_bold_sentence_is_not_a_heading() {
        let html = r#"
<p data-p="0"><span style="font-size:12pt">Ordinary text here, of which there is a good deal.</span></p>
<p data-p="1"><span style="font-size:12pt"><strong>This sentence is bold for emphasis.</strong></span></p>
<p data-p="2"><span style="font-size:12pt">More ordinary text, continuing the paragraph above it.</span></p>
"#;
        assert!(
            inferred_sections(html).is_empty(),
            "bold at body size is emphasis, not a section"
        );
    }

    #[test]
    fn a_long_line_in_a_big_font_is_not_a_heading() {
        let html = r#"
<p data-p="0"><span style="font-size:12pt">Ordinary text here, of which there is a good deal indeed.</span></p>
<p data-p="1"><span style="font-size:18pt">A pull quote is set large and runs on at length, well past what any heading would, and so it is not one</span></p>
<p data-p="2"><span style="font-size:12pt">More ordinary text, continuing after the quotation above.</span></p>
"#;
        assert!(
            inferred_sections(html).is_empty(),
            "too long to be a heading"
        );
    }

    #[test]
    fn a_sentence_ending_in_a_full_stop_is_not_a_heading() {
        let html = r#"
<p data-p="0"><span style="font-size:12pt">Ordinary text, at some length, so that the body size is clear.</span></p>
<p data-p="1"><span style="font-size:16pt"><strong>This is short and large.</strong></span></p>
"#;
        assert!(inferred_sections(html).is_empty());
    }

    /// A document where everything looks like a heading has said nothing.
    #[test]
    fn a_document_of_short_bold_lines_gets_no_outline() {
        let html = r#"
<p data-p="0"><span style="font-size:16pt"><strong>Name</strong></span></p>
<p data-p="1"><span style="font-size:16pt"><strong>Address</strong></span></p>
<p data-p="2"><span style="font-size:16pt"><strong>Telephone</strong></span></p>
<p data-p="3"><span style="font-size:12pt">One line of body text</span></p>
"#;
        assert!(
            inferred_sections(html).is_empty(),
            "a form is not a document with three sections"
        );
    }

    #[test]
    fn a_document_with_nothing_in_it_is_no_trouble() {
        assert!(inferred_sections("").is_empty());
        assert!(inferred_sections("<p data-p=\"0\"></p>").is_empty());
    }
}
