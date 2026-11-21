//! Machine metadata stored in `obsidian/.codewiki/`.
//!
//! Notes keep the owner's frontmatter convention (tags only), so everything the
//! generator needs to track a note lives here instead. The files are plain text
//! and sorted, so they diff and merge cleanly in git.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cw_core::{Error, MANIFEST_DIR, Result, normalize_text, sha256_hex};
use serde::{Deserialize, Serialize};

use crate::note::NoteKind;
use crate::registry::{Registry, RegistryEntry};

pub const MANIFEST_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub template_pack: String,
    pub template_version: u32,
    /// Commit the vault was last brought up to date with.
    pub last_indexed_commit: Option<String>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            schema: MANIFEST_SCHEMA,
            template_pack: "default".into(),
            template_version: 1,
            last_indexed_commit: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Written by CodeWiki.
    Generated,
    /// Existed before CodeWiki managed the vault.
    Adopted,
}

/// A source file a note describes, with the content hash seen at generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<String>,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteEntry {
    pub id: String,
    /// Vault-relative path.
    pub path: String,
    pub kind: NoteKind,
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceRef>,
    /// Hash of the normalized note text as CodeWiki last wrote or adopted it.
    pub body_hash: String,
    /// Hash of everything that fed generation; unchanged input means no rewrite.
    pub input_hash: String,
    /// Inputs a human rejected a proposal for; never proposed again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejected_inputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Model that wrote the prose; absent for structural notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Vault-relative path of the saved illustration, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub illustration: Option<String>,
}

/// Hash used to tell whether a note changed on disk.
pub fn body_hash(text: &str) -> String {
    sha256_hex(normalize_text(text))
}

/// Everything under `.codewiki/`, loaded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VaultMeta {
    pub manifest: Manifest,
    /// Keyed by note id.
    pub notes: BTreeMap<String, NoteEntry>,
}

impl VaultMeta {
    pub fn dir(vault: &Path) -> PathBuf {
        vault.join(MANIFEST_DIR)
    }

    /// Load metadata; a vault with none yields the default (empty) state.
    pub fn load(vault: &Path) -> Result<Self> {
        let dir = Self::dir(vault);
        let manifest_path = dir.join("manifest.json");
        let manifest = match std::fs::read_to_string(&manifest_path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| Error::external("parsing manifest.json", e))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Manifest::default(),
            Err(e) => return Err(Error::io(manifest_path, e)),
        };
        let notes_path = dir.join("notes.jsonl");
        let mut notes = BTreeMap::new();
        match std::fs::read_to_string(&notes_path) {
            Ok(text) => {
                for (number, line) in text.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let entry: NoteEntry = serde_json::from_str(line).map_err(|e| {
                        Error::external(format!("parsing notes.jsonl line {}", number + 1), e)
                    })?;
                    notes.insert(entry.id.clone(), entry);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(notes_path, e)),
        }
        Ok(Self { manifest, notes })
    }

    /// Write metadata. `registry.json` is derived from the note entries so the
    /// two can never disagree.
    pub fn save(&self, vault: &Path) -> Result<()> {
        let dir = Self::dir(vault);
        std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
        let write = |name: &str, text: String| {
            let path = dir.join(name);
            std::fs::write(&path, text).map_err(|e| Error::io(path, e))
        };
        let pretty = |value: &dyn erased::Json| value.pretty();
        write("manifest.json", pretty(&self.manifest)? + "\n")?;
        let mut lines = String::new();
        for entry in self.notes.values() {
            lines.push_str(
                &serde_json::to_string(entry)
                    .map_err(|e| Error::external("encoding a note entry", e))?,
            );
            lines.push('\n');
        }
        write("notes.jsonl", lines)?;
        let registry: Vec<RegistryEntry> = self.registry()?.entries().cloned().collect();
        write("registry.json", pretty(&registry)? + "\n")
    }

    /// Build the name registry from the note entries.
    pub fn registry(&self) -> Result<Registry> {
        let mut registry = Registry::new();
        for entry in self.notes.values() {
            registry.insert(RegistryEntry {
                id: entry.id.clone(),
                path: entry.path.clone(),
                kind: entry.kind,
            })?;
        }
        Ok(registry)
    }

    pub fn by_path(&self, path: &str) -> Option<&NoteEntry> {
        self.notes.values().find(|e| e.path == path)
    }

    /// Ids of notes that cite `source_path`.
    pub fn notes_for_source(&self, source_path: &str) -> Vec<&NoteEntry> {
        self.notes
            .values()
            .filter(|e| e.sources.iter().any(|s| s.path == source_path))
            .collect()
    }
}

/// Small helper so `save` can pretty-print different types through one closure.
mod erased {
    use cw_core::{Error, Result};

    pub trait Json {
        fn pretty(&self) -> Result<String>;
    }

    impl<T: serde::Serialize> Json for T {
        fn pretty(&self) -> Result<String> {
            serde_json::to_string_pretty(self).map_err(|e| Error::external("encoding metadata", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, path: &str, source: &str) -> NoteEntry {
        NoteEntry {
            id: id.into(),
            path: path.into(),
            kind: NoteKind::Concept,
            origin: Origin::Generated,
            sources: vec![SourceRef {
                path: source.into(),
                symbols: vec!["demo::run".into()],
                hash: "h".into(),
            }],
            body_hash: body_hash("# T\n"),
            input_hash: "i".into(),
            rejected_inputs: vec![],
            commit: Some("abc".into()),
            model: None,
            illustration: None,
        }
    }

    #[test]
    fn saves_and_loads() {
        let dir = tempfile::tempdir().unwrap();
        let mut meta = VaultMeta::default();
        meta.manifest.last_indexed_commit = Some("abc".into());
        for e in [
            entry("b", "01-Core/B.md", "src/b.rs"),
            entry("a", "01-Core/A.md", "src/a.rs"),
        ] {
            meta.notes.insert(e.id.clone(), e);
        }
        meta.save(dir.path()).unwrap();
        assert_eq!(VaultMeta::load(dir.path()).unwrap(), meta);

        let jsonl = std::fs::read_to_string(dir.path().join(".codewiki/notes.jsonl")).unwrap();
        let ids: Vec<&str> = jsonl.lines().map(|l| &l[7..8]).collect();
        assert_eq!(ids, ["a", "b"], "entries are written sorted by id");
        assert!(
            !jsonl.contains("rejected_inputs"),
            "empty fields are omitted"
        );
        assert!(dir.path().join(".codewiki/registry.json").exists());
    }

    #[test]
    fn missing_metadata_loads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(VaultMeta::load(dir.path()).unwrap(), VaultMeta::default());
    }

    #[test]
    fn looks_up_by_path_and_source() {
        let mut meta = VaultMeta::default();
        let e = entry("a", "01-Core/A.md", "src/a.rs");
        meta.notes.insert(e.id.clone(), e);
        assert_eq!(meta.by_path("01-Core/A.md").unwrap().id, "a");
        assert_eq!(meta.notes_for_source("src/a.rs").len(), 1);
        assert!(meta.notes_for_source("src/z.rs").is_empty());
        assert_eq!(meta.registry().unwrap().by_name("A").unwrap().id, "a");
    }

    #[test]
    fn body_hash_ignores_line_endings() {
        assert_eq!(body_hash("# T\r\n\r\nBody\r\n"), body_hash("# T\n\nBody\n"));
        assert_ne!(body_hash("# T\n\nBody\n"), body_hash("# T\n\nBody!\n"));
    }
}
