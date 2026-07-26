//! Turning a sentence into a piece of work.
//!
//! "Describe what you need" was the front door of the product and it did nothing: the User typed
//! a request, pressed return, and the screen sat still. Everything else in Work Studio assumes a
//! file already exists, which meant the only way in was to go and find one.
//!
//! This decides what kind of Artefact the request calls for, names a file, creates it, and hands
//! the original request to the specialist that owns that kind. The User asked for a thing, so
//! they get the thing — not a form asking which sort of thing they meant.

// Without the sibling checkouts there is nothing to create a file with, so the endpoint that uses
// these is not compiled. The reading and naming below are still tested, because they are the part
// that decides what the User gets and they are worth holding either way.
#![cfg_attr(not(feature = "adk"), allow(dead_code))]

use serde::{Deserialize, Serialize};

/// What the User typed.
#[derive(Debug, Deserialize)]
pub struct Wanted {
    pub asked: String,
}

/// The piece of work that was started.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    /// Where the file is, so the interface can open it.
    pub path: String,
    /// What to call the conversation about it.
    pub thread: String,
    /// What Work Studio decided to make, in the User's words: "a spreadsheet".
    pub made: String,
}

/// The kinds of thing Work Studio can start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wants {
    Spreadsheet,
    Document,
    Deck,
}

impl Wants {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Spreadsheet => "xlsx",
            Self::Document => "docx",
            Self::Deck => "pptx",
        }
    }

    /// What to tell the User was made. Their words, not the file extension.
    pub fn in_words(self) -> &'static str {
        match self {
            Self::Spreadsheet => "a spreadsheet",
            Self::Document => "a document",
            Self::Deck => "a deck",
        }
    }
}

/// Which kind of Artefact a request calls for.
///
/// Words first, and only the User's own words. A request that says "spreadsheet" wants a
/// spreadsheet whatever else it says, and asking a model to confirm that would add a wait and a
/// cost to a decision already made. Requests that name nothing fall through to the model.
pub fn read_intent(asked: &str) -> Option<Wants> {
    let lower = asked.to_lowercase();

    // Ordered by how strongly each word implies the kind, because a request can mention more
    // than one: "a deck summarising the budget spreadsheet" is a deck.
    const CLUES: &[(&str, Wants)] = &[
        ("deck", Wants::Deck),
        ("slide", Wants::Deck),
        ("presentation", Wants::Deck),
        ("present", Wants::Deck),
        ("spreadsheet", Wants::Spreadsheet),
        ("workbook", Wants::Spreadsheet),
        ("budget", Wants::Spreadsheet),
        ("tracker", Wants::Spreadsheet),
        ("track", Wants::Spreadsheet),
        ("forecast", Wants::Spreadsheet),
        ("model", Wants::Spreadsheet),
        ("calculat", Wants::Spreadsheet),
        ("table of", Wants::Spreadsheet),
        ("document", Wants::Document),
        ("letter", Wants::Document),
        ("contract", Wants::Document),
        ("report", Wants::Document),
        ("memo", Wants::Document),
        ("proposal", Wants::Document),
        ("write", Wants::Document),
        ("draft", Wants::Document),
    ];

    CLUES
        .iter()
        .find(|(clue, _)| lower.contains(clue))
        .map(|(_, wants)| *wants)
}

/// A file name from the request.
///
/// Taken from the User's own sentence rather than generated, because they need to recognise it in
/// a folder next month. "Untitled 4" is a file nobody can find.
pub fn name_from(asked: &str) -> String {
    let stop = [
        "a", "an", "the", "me", "my", "our", "for", "of", "to", "and", "with", "please", "make",
        // "new" is deliberately not here: "the new tenant" is not the same tenant, and dropping
        // it renames the User's work behind their back.
        "create", "build", "draft", "write", "start", "put", "together", "some", "that", "this",
        "i", "need", "want", "would", "like", "can", "you", "up",
    ];

    let words: Vec<String> = asked
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .filter(|word| !stop.contains(&word.to_lowercase().as_str()))
        .take(6)
        .collect();

    if words.is_empty() {
        return "New work".to_string();
    }

    // A name must not end mid-thought. Taking six words from "tracking my Q3 travel costs by
    // month" left "…costs by", which reads like a truncation because it is one.
    const DANGLING: &[&str] = &[
        "by", "in", "on", "at", "from", "into", "per", "over", "under", "about", "before", "after",
        "between", "each", "every",
    ];
    let mut words = words;
    while words
        .last()
        .is_some_and(|last| DANGLING.contains(&last.to_lowercase().as_str()))
    {
        words.pop();
    }
    if words.is_empty() {
        return "New work".to_string();
    }

    // Sentence case: the first word capitalised, the rest as the User wrote them, so "Q3" stays
    // "Q3" rather than becoming "q3".
    let mut name = words.join(" ");
    let mut chars = name.chars();
    if let Some(first) = chars.next() {
        name = first.to_uppercase().collect::<String>() + chars.as_str();
    }
    // Long enough to recognise, short enough to read in a list.
    if name.len() > 60 {
        name.truncate(60);
        name = name.trim_end().to_string();
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_that_names_the_kind_gets_that_kind() {
        assert_eq!(
            read_intent("make me a spreadsheet"),
            Some(Wants::Spreadsheet)
        );
        assert_eq!(read_intent("Draft a contract"), Some(Wants::Document));
        assert_eq!(read_intent("a deck for the board"), Some(Wants::Deck));
    }

    /// A request can mention more than one kind. The one it is asking for wins.
    #[test]
    fn a_deck_about_a_spreadsheet_is_a_deck() {
        assert_eq!(
            read_intent("a deck summarising the budget spreadsheet"),
            Some(Wants::Deck)
        );
        assert_eq!(
            read_intent("slides from the quarterly report"),
            Some(Wants::Deck)
        );
    }

    #[test]
    fn work_implied_rather_than_named_is_still_read() {
        assert_eq!(
            read_intent("track my Q3 expenses"),
            Some(Wants::Spreadsheet)
        );
        assert_eq!(
            read_intent("a letter to the landlord"),
            Some(Wants::Document)
        );
    }

    /// Nothing recognisable must not be guessed at. The caller asks the model instead, because a
    /// wrong file created silently is worse than a moment's wait.
    #[test]
    fn a_request_naming_nothing_is_not_guessed() {
        assert_eq!(read_intent("sort out the thing from yesterday"), None);
        assert_eq!(read_intent(""), None);
    }

    #[test]
    fn the_name_comes_from_what_the_user_said() {
        assert_eq!(
            name_from("Make me a spreadsheet tracking Q3 expenses"),
            "Spreadsheet tracking Q3 expenses"
        );
        assert_eq!(
            name_from("Draft a contract for the new tenant"),
            "Contract new tenant"
        );
    }

    #[test]
    fn a_name_is_never_empty_and_never_endless() {
        assert_eq!(name_from(""), "New work");
        assert_eq!(name_from("please make me a new one"), "New one");
        let long = name_from(&"supercalifragilistic ".repeat(20));
        assert!(long.len() <= 60, "{} characters", long.len());
    }

    /// A file name has to survive being a file name.
    #[test]
    fn punctuation_the_filesystem_dislikes_is_dropped() {
        let name = name_from("Draft a report: Q3/Q4 \"outlook\" (final)");
        assert!(!name.contains('/'), "{name}");
        assert!(!name.contains(':'), "{name}");
        assert!(!name.contains('"'), "{name}");
    }

    /// A name that stops on a preposition reads as a bug, because it is the shape of one.
    #[test]
    fn a_name_does_not_end_mid_thought() {
        assert_eq!(
            name_from("A spreadsheet tracking my Q3 travel costs by month"),
            "Spreadsheet tracking Q3 travel costs"
        );
        assert_eq!(name_from("a report on"), "Report");
    }
}
