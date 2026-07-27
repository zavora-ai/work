//! The pictures of the document being looked at.
//!
//! The view references its images rather than carrying them, and asks for each one as it draws.
//! A book has hundreds: this one has 369, and serving them by opening the document each time meant
//! parsing 216MB of file 369 times. The pictures arrived, slowly, over minutes.
//!
//! So the document last asked for is kept — just the one, and just its images. A person looks at one
//! document at a time, and the second they open another the first is no longer worth holding.
//!
//! Keyed by path and modified time together, so a document changed on disk is read again rather
//! than served from a picture of how it used to be.

use std::sync::Mutex;

type Pictures = std::collections::HashMap<String, (String, Vec<u8>)>;

struct Held {
    path: std::path::PathBuf,
    changed_at: std::time::SystemTime,
    pictures: Pictures,
}

static HELD: Mutex<Option<Held>> = Mutex::new(None);

/// When the file was last written, or `None` when that cannot be told — in which case nothing is
/// cached, because serving a stale picture is worse than serving a slow one.
fn changed_at(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// One picture, from the document being looked at.
pub fn picture(path: &std::path::Path, embed_id: &str) -> crate::Result<(String, Vec<u8>)> {
    let Some(changed) = changed_at(path) else {
        return read_one(path, embed_id);
    };

    // Held from a previous request for the same document, unchanged since.
    if let Ok(held) = HELD.lock()
        && let Some(held) = held.as_ref()
        && held.path == path
        && held.changed_at == changed
    {
        return held
            .pictures
            .get(embed_id)
            .cloned()
            .ok_or_else(|| crate::DocError::Open {
                detail: format!("no picture {embed_id} in this document"),
            });
    }

    let pictures = all_pictures(path)?;
    let found = pictures
        .get(embed_id)
        .cloned()
        .ok_or_else(|| crate::DocError::Open {
            detail: format!("no picture {embed_id} in this document"),
        });

    if let Ok(mut held) = HELD.lock() {
        *held = Some(Held {
            path: path.to_path_buf(),
            changed_at: changed,
            pictures,
        });
    }
    found
}

/// Every picture in the document, read once.
fn all_pictures(path: &std::path::Path) -> crate::Result<Pictures> {
    let document = zavora_docx::Document::open(path).map_err(|e| crate::DocError::Open {
        detail: e.to_string(),
    })?;
    Ok(document.all_media())
}

/// Without a modified time to trust, read the one picture and hold nothing.
fn read_one(path: &std::path::Path, embed_id: &str) -> crate::Result<(String, Vec<u8>)> {
    let document = zavora_docx::Document::open(path).map_err(|e| crate::DocError::Open {
        detail: e.to_string(),
    })?;
    document.media(embed_id).ok_or(crate::DocError::Open {
        detail: format!("no picture {embed_id} in this document"),
    })
}

/// Let go of whatever is held. For a test that wants to measure a first read.
pub fn forget() {
    if let Ok(mut held) = HELD.lock() {
        *held = None;
    }
}
