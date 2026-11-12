//! The note-name registry.
//!
//! Wikilinks name notes by title, so titles must be unique across the vault and
//! fixed before any note body is written. The registry is that fixed list.

use std::collections::BTreeMap;

use cw_core::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::note::NoteKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Stable id; survives renames.
    pub id: String,
    /// Vault-relative path with forward slashes, for example
    /// `01-Core/cw_core/Hashing.md`.
    pub path: String,
    pub kind: NoteKind,
}

/// Maps note names (file stem, compared case-insensitively because Windows
/// file names and Obsidian links are) to entries. Map notes (`_index`, `_HOME`) are
/// keyed by path instead, because every folder has one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    names: BTreeMap<String, RegistryEntry>,
    maps: BTreeMap<String, RegistryEntry>,
}

/// Characters Obsidian or Windows forbid in file names or link targets.
const FORBIDDEN: [char; 12] = ['[', ']', '|', '#', '^', ':', '\\', '/', '*', '?', '"', '<'];

/// Make a title safe to use as a file name and link target.
pub fn sanitize_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if FORBIDDEN.contains(&c) || c == '>' {
                ' '
            } else {
                c
            }
        })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.trim_matches('.').to_string()
}

fn stem(path: &str) -> &str {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.strip_suffix(".md").unwrap_or(file)
}

fn folder(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a note. Fails when another note already owns the name.
    pub fn insert(&mut self, entry: RegistryEntry) -> Result<()> {
        if matches!(entry.kind, NoteKind::Home | NoteKind::Index) {
            self.maps.insert(entry.path.clone(), entry);
            return Ok(());
        }
        let name = stem(&entry.path).to_lowercase();
        if let Some(existing) = self.names.get(&name)
            && existing.path != entry.path
        {
            return Err(Error::invalid(format!(
                "note name '{name}' is used by both {} and {}",
                existing.path, entry.path
            )));
        }
        self.names.insert(name, entry);
        Ok(())
    }

    /// Reserve a unique name for `title` in `folder`, appending the folder's
    /// last segment and then a counter when the plain title is taken.
    pub fn reserve(&mut self, id: &str, dir: &str, title: &str, kind: NoteKind) -> RegistryEntry {
        let base = sanitize_name(title);
        let qualifier = dir.rsplit('/').next().unwrap_or("");
        let mut candidates = vec![base.clone()];
        if !qualifier.is_empty() {
            candidates.push(format!("{base} ({qualifier})"));
        }
        let mut n = 2;
        let name = loop {
            if let Some(free) = candidates
                .iter()
                .find(|c| !self.names.contains_key(&c.to_lowercase()))
            {
                break free.clone();
            }
            candidates = vec![format!("{base} {n}")];
            n += 1;
        };
        let path = if dir.is_empty() {
            format!("{name}.md")
        } else {
            format!("{dir}/{name}.md")
        };
        let entry = RegistryEntry {
            id: id.to_string(),
            path,
            kind,
        };
        self.names.insert(name.to_lowercase(), entry.clone());
        entry
    }

    pub fn by_name(&self, name: &str) -> Option<&RegistryEntry> {
        self.names.get(&name.to_lowercase())
    }

    pub fn by_id(&self, id: &str) -> Option<&RegistryEntry> {
        self.entries().find(|e| e.id == id)
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.names.contains_key(&name.to_lowercase())
    }

    /// All entries, named notes first, each group in path order.
    pub fn entries(&self) -> impl Iterator<Item = &RegistryEntry> {
        self.names.values().chain(self.maps.values())
    }

    pub fn len(&self) -> usize {
        self.names.len() + self.maps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Resolve a wikilink target written in the note at `from_path`.
    pub fn resolve(&self, target: &str, from_path: &str) -> Option<&RegistryEntry> {
        let target = target.trim().trim_end_matches(".md");
        if target.contains('/') {
            let path = format!("{target}.md");
            return self
                .maps
                .get(&path)
                .or_else(|| self.names.values().find(|e| e.path == path));
        }
        match target {
            // A bare `_index` means the map of the note's own folder; from a
            // map note it means the parent folder's map.
            "_index" => {
                let dir = folder(from_path);
                let own = join(dir, "_index.md");
                if from_path != own {
                    return self.maps.get(&own);
                }
                self.maps.get(&join(folder(dir), "_index.md"))
            }
            "_HOME" => self.maps.get("_HOME.md"),
            name => self.names.get(&name.to_lowercase()),
        }
    }
}

fn join(dir: &str, file: &str) -> String {
    if dir.is_empty() {
        file.to_string()
    } else {
        format!("{dir}/{file}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        let mut r = Registry::new();
        for (id, path, kind) in [
            ("home", "_HOME.md", NoteKind::Home),
            ("core-idx", "01-Core/_index.md", NoteKind::Index),
            ("hash-idx", "01-Core/hash/_index.md", NoteKind::Index),
            ("hashing", "01-Core/hash/Hashing.md", NoteKind::Concept),
        ] {
            r.insert(RegistryEntry {
                id: id.into(),
                path: path.into(),
                kind,
            })
            .unwrap();
        }
        r
    }

    #[test]
    fn resolves_names_paths_and_maps() {
        let r = registry();
        let from = "01-Core/hash/Hashing.md";
        assert_eq!(r.resolve("Hashing", "_HOME.md").unwrap().id, "hashing");
        assert_eq!(r.resolve("_index", from).unwrap().id, "hash-idx");
        assert_eq!(
            r.resolve("_index", "01-Core/hash/_index.md").unwrap().id,
            "core-idx"
        );
        assert_eq!(r.resolve("_HOME", from).unwrap().id, "home");
        assert_eq!(r.resolve("01-Core/_index", from).unwrap().id, "core-idx");
        assert_eq!(
            r.resolve("01-Core/hash/Hashing", from).unwrap().id,
            "hashing"
        );
        assert!(r.resolve("Missing", from).is_none());
        assert!(r.resolve("_index", "Loose.md").is_none());
    }

    #[test]
    fn rejects_duplicate_names_and_reserves_unique_ones() {
        let mut r = registry();
        let dup = RegistryEntry {
            id: "x".into(),
            path: "02-Frontend/Hashing.md".into(),
            kind: NoteKind::Concept,
        };
        assert!(r.insert(dup).is_err());

        let a = r.reserve("a", "02-Frontend/ui", "Hashing", NoteKind::Concept);
        assert_eq!(a.path, "02-Frontend/ui/Hashing (ui).md");
        let b = r.reserve("b", "02-Frontend/ui", "Hashing", NoteKind::Concept);
        assert_eq!(b.path, "02-Frontend/ui/Hashing 2.md");
        assert_eq!(r.by_id("b").unwrap().path, b.path);
    }

    #[test]
    fn sanitizes_titles() {
        assert_eq!(sanitize_name("Vec<T>: a/b [x] | y?"), "Vec T a b x y");
        assert_eq!(sanitize_name("  spaced   out. "), "spaced out");
    }

    #[test]
    fn serializes_round_trip() {
        let r = registry();
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<Registry>(&json).unwrap(), r);
    }
}
