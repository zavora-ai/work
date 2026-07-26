//! Where the User's work lives.
//!
//! The interface tells the User "Your files live in Documents › Work Studio on your Mac"
//! and "Folders here are real folders on your Mac". Both were untrue: the folder did not
//! exist and the listing was invented. This makes them true, which matters more than any
//! feature — a product that misdescribes the User's own filesystem cannot be trusted about
//! anything else.
//!
//! Folders are real folders and kinds are filters, never containers (Correctness Property
//! 31). So this lists what is actually on disk and says what kind each thing is; it never
//! invents a folder to group them by.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Kind;

#[derive(Debug, thiserror::Error)]
pub enum HomeError {
    #[error("Work Studio could not find your Documents folder")]
    NoDocuments,
    #[error("Work Studio could not use that folder")]
    Unusable { detail: String },
}

impl HomeError {
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Unusable { detail } => Some(detail),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, HomeError>;

/// The folder name the interface names to the User. Changing this changes a promise.
pub const FOLDER_NAME: &str = "Work Studio";

/// One entry in the User's folder: a real file, or a real folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// What the User called it.
    pub name: String,
    /// The full path, so opening it needs no guesswork.
    pub path: String,
    /// A folder, or one of the kinds Work Studio can open. `None` for a file it cannot.
    pub kind: Option<String>,
    pub is_folder: bool,
    /// Bytes. Absent for a folder.
    pub size: Option<u64>,
    /// When it last changed, as seconds since the epoch, for "changed" in the User's terms.
    pub changed: Option<i64>,
    /// How many things are in it. Only for a folder.
    pub count: Option<usize>,
}

/// The User's own folder, and what is in it.
#[derive(Debug, Clone)]
pub struct Home {
    root: PathBuf,
}

impl Home {
    /// The folder under the User's Documents, created if it is not there yet.
    ///
    /// Creating it on first use is what makes the interface's claim true. It is created
    /// empty and never populated with samples, because a folder of files the User did not
    /// put there is its own kind of lie.
    pub fn open_default() -> Result<Self> {
        let documents = documents_dir().ok_or(HomeError::NoDocuments)?;
        Self::open_at(documents.join(FOLDER_NAME))
    }

    /// A specific folder, created if needed. Used by Settings when the User moves it, and
    /// by tests.
    pub fn open_at(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|e| HomeError::Unusable {
            detail: e.to_string(),
        })?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// What the interface shows as the location, in the User's terms rather than as a path.
    pub fn described(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for part in self.root.iter().rev().take(2) {
            parts.push(part.to_string_lossy().into_owned());
        }
        parts.reverse();
        parts.join(" › ")
    }

    /// Everything in a folder, folders first then files, each by name.
    ///
    /// `within` is relative to the home folder. A path that climbs out of it is refused
    /// rather than followed, because the renderer supplies it.
    pub fn list(&self, within: Option<&str>) -> Result<Vec<Entry>> {
        let target = self.resolve(within)?;
        let reading = std::fs::read_dir(&target).map_err(|e| HomeError::Unusable {
            detail: e.to_string(),
        })?;

        let mut folders = Vec::new();
        let mut files = Vec::new();
        for item in reading.flatten() {
            let path = item.path();
            let name = item.file_name().to_string_lossy().into_owned();
            // What the operating system keeps for itself is not the User's work.
            if name.starts_with('.') {
                continue;
            }
            let meta = match item.metadata() {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            let changed = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);

            if meta.is_dir() {
                let count = std::fs::read_dir(&path)
                    .map(|entries| {
                        entries
                            .flatten()
                            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                            .count()
                    })
                    .unwrap_or(0);
                folders.push(Entry {
                    name,
                    path: path.to_string_lossy().into_owned(),
                    kind: None,
                    is_folder: true,
                    size: None,
                    changed,
                    count: Some(count),
                });
            } else {
                files.push(Entry {
                    name,
                    path: path.to_string_lossy().into_owned(),
                    kind: Kind::of_path(&path).map(|k| k.as_str().to_string()),
                    is_folder: false,
                    size: Some(meta.len()),
                    changed,
                    count: None,
                });
            }
        }

        folders.sort_by_key(|entry| entry.name.to_lowercase());
        files.sort_by_key(|entry| entry.name.to_lowercase());
        folders.extend(files);
        Ok(folders)
    }

    /// A new folder, made where the User asked for it.
    pub fn create_folder(&self, within: Option<&str>, name: &str) -> Result<Entry> {
        let clean = name.trim();
        if clean.is_empty() || clean.contains('/') || clean.starts_with('.') {
            return Err(HomeError::Unusable {
                detail: format!("not a usable folder name: {name:?}"),
            });
        }
        let parent = self.resolve(within)?;
        let path = parent.join(clean);
        std::fs::create_dir_all(&path).map_err(|e| HomeError::Unusable {
            detail: e.to_string(),
        })?;
        Ok(Entry {
            name: clean.to_string(),
            path: path.to_string_lossy().into_owned(),
            kind: None,
            is_folder: true,
            size: None,
            changed: None,
            count: Some(0),
        })
    }

    /// A path for a file Work Studio is about to make for the User.
    pub fn path_for(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// Whether a path is inside the User's folder.
    ///
    /// Used before opening anything the renderer names, so a path from outside cannot be
    /// used to read elsewhere on the disk.
    pub fn contains(&self, path: &Path) -> bool {
        let root = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        match path.canonicalize() {
            Ok(real) => real.starts_with(&root),
            // A file that does not exist yet is inside if its parent is.
            Err(_) => path
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .map(|p| p.starts_with(&root))
                .unwrap_or(false),
        }
    }

    fn resolve(&self, within: Option<&str>) -> Result<PathBuf> {
        let Some(within) = within.map(str::trim).filter(|w| !w.is_empty()) else {
            return Ok(self.root.clone());
        };
        let candidate = if Path::new(within).is_absolute() {
            PathBuf::from(within)
        } else {
            self.root.join(within)
        };
        // Refused rather than followed: the renderer supplies this.
        if !self.contains(&candidate) {
            return Err(HomeError::Unusable {
                detail: format!("{within:?} is outside the User's folder"),
            });
        }
        Ok(candidate)
    }
}

/// The User's Documents folder.
fn documents_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let documents = home.join("Documents");
    documents.is_dir().then_some(documents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zws-home-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_folder_is_made_so_the_promise_is_true() {
        let root = temp("made");
        assert!(!root.exists());
        let home = Home::open_at(&root).unwrap();
        assert!(root.is_dir(), "the folder the interface names must exist");
        assert!(home.list(None).unwrap().is_empty(), "and start empty");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn it_lists_what_is_really_there_folders_first() {
        let root = temp("listing");
        let home = Home::open_at(&root).unwrap();
        std::fs::write(root.join("model.xlsx"), b"x").unwrap();
        std::fs::write(root.join("notes.txt"), b"x").unwrap();
        std::fs::create_dir(root.join("Board packs")).unwrap();
        std::fs::write(root.join("Board packs").join("deck.pptx"), b"x").unwrap();
        // What the operating system leaves behind is not the User's work.
        std::fs::write(root.join(".DS_Store"), b"x").unwrap();

        let entries = home.list(None).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Board packs", "model.xlsx", "notes.txt"]);
        assert!(entries[0].is_folder);
        assert_eq!(entries[0].count, Some(1));
        assert_eq!(entries[1].kind.as_deref(), Some("spreadsheet"));
        assert_eq!(
            entries[2].kind, None,
            "a file Work Studio cannot open says so rather than pretending"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_the_user_makes_is_a_real_folder() {
        let root = temp("newfolder");
        let home = Home::open_at(&root).unwrap();
        let made = home.create_folder(None, "Contracts").unwrap();
        assert!(
            Path::new(&made.path).is_dir(),
            "Property 30: the folder shown must exist on disk"
        );
        assert!(home.create_folder(None, "  ").is_err());
        assert!(home.create_folder(None, "a/b").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The renderer names paths, so a path that climbs out must be refused rather than
    /// followed.
    #[test]
    fn a_path_outside_the_users_folder_is_refused() {
        let root = temp("escape");
        let home = Home::open_at(&root).unwrap();
        assert!(home.list(Some("../..")).is_err());
        assert!(home.list(Some("/etc")).is_err());
        assert!(!home.contains(Path::new("/etc/hosts")));
        assert!(home.contains(&root.join("anything.xlsx")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_location_is_described_in_the_users_terms() {
        let home = Home::open_at(temp("described").join(FOLDER_NAME)).unwrap();
        let described = home.described();
        assert!(described.contains(FOLDER_NAME));
        assert!(described.contains('›'), "read as a place: {described}");
        assert!(!described.starts_with('/'), "not as a path: {described}");
        let _ = std::fs::remove_dir_all(home.root());
    }
}
