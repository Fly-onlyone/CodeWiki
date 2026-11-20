//! The models CodeWiki offers, as a list inside the program.
//!
//! A model enters the list only after the engine reproduced its reference
//! vectors (spike S5 in `spikes/VERDICTS.md`). The list changes with a
//! release, never at run time: nothing is fetched to learn what exists.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// How much machine a model wants; the recommendation picks by this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Small,
    Balanced,
    Large,
}

/// The architecture, which names the loader the engine uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    /// An encoder that takes a padded batch.
    ModernBert,
    /// A decoder that takes one text at a time.
    Qwen3,
}

/// Which token's state is the text's vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Pooling {
    First,
    Last,
}

/// One file of a model, as its repository holds it at the pinned revision.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct File {
    /// Path inside the repository and inside the model's folder.
    pub path: String,
    pub size: u64,
    /// Lowercase hex.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Entry {
    /// The name a setting holds; never changes once released.
    pub id: String,
    /// The name a person reads.
    pub name: String,
    pub tier: Tier,
    pub family: Family,
    /// `owner/name` on Hugging Face.
    pub repository: String,
    /// The commit the files are taken from, 40 hex characters.
    pub revision: String,
    /// SPDX identifier.
    pub license: String,
    pub languages: String,
    /// Length of a vector.
    pub dimensions: usize,
    /// Tokens of a text the model is given; the rest is cut off.
    pub max_tokens: usize,
    pub pooling: Pooling,
    /// Put before a query, and before no document.
    pub query_prefix: String,
    /// Documents embedded in one step of a long run.
    pub batch: usize,
    pub files: Vec<File>,
}

impl Entry {
    /// Bytes to download.
    pub fn size(&self) -> u64 {
        self.files.iter().map(|file| file.size).sum()
    }

    /// The name stored with each vector. It holds the revision, so that a
    /// release that moves a model to other weights embeds everything again.
    pub fn embedder_name(&self) -> String {
        format!("builtin:{}@{}", self.id, &self.revision[..12])
    }
}

/// Every model, smallest tier first.
pub fn all() -> &'static [Entry] {
    static CATALOG: OnceLock<Vec<Entry>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        // The file is part of the program, and a test reads it.
        serde_json::from_str(include_str!("../catalog.json")).expect("catalog.json is valid")
    })
}

/// The model with this id.
pub fn find(id: &str) -> Option<&'static Entry> {
    all().iter().find(|entry| entry.id == id)
}

/// The model of a tier.
pub fn of_tier(tier: Tier) -> Option<&'static Entry> {
    all().iter().find(|entry| entry.tier == tier)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn is_hex(text: &str, length: usize) -> bool {
        text.len() == length
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }

    #[test]
    fn every_model_is_pinned_and_complete() {
        let models = all();
        assert_eq!(models.len(), 3);
        let ids: HashSet<&str> = models.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids.len(), models.len(), "an id is used twice");
        for entry in models {
            let id = &entry.id;
            assert!(
                id.bytes().all(|b| b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || b == b'-'
                    || b == b'.'),
                "{id}: an id is lowercase letters, digits, '-' and '.'"
            );
            assert!(
                is_hex(&entry.revision, 40),
                "{id}: revision is not a commit"
            );
            assert_eq!(entry.repository.split('/').count(), 2, "{id}: repository");
            assert!(entry.dimensions > 0 && entry.max_tokens > 0 && entry.batch > 0);
            for needed in ["config.json", "tokenizer.json", "model.safetensors"] {
                assert!(
                    entry.files.iter().any(|file| file.path == needed),
                    "{id}: {needed} is missing"
                );
            }
            for file in &entry.files {
                assert!(is_hex(&file.sha256, 64), "{id}: sha256 of {}", file.path);
                assert!(file.size > 0, "{id}: size of {}", file.path);
                // A path is joined to a folder on disk and to an address.
                assert!(
                    !file.path.contains("..")
                        && !file.path.starts_with('/')
                        && !file.path.contains('\\'),
                    "{id}: path {}",
                    file.path
                );
            }
        }
    }

    #[test]
    fn every_license_is_one_the_project_allows() {
        // deny.toml allows these for code; a model's weights are held to the
        // same list, and none may be gated or non-commercial.
        for entry in all() {
            assert!(
                ["Apache-2.0", "MIT"].contains(&entry.license.as_str()),
                "{}: {}",
                entry.id,
                entry.license
            );
        }
    }

    #[test]
    fn each_tier_has_one_model() {
        for tier in [Tier::Small, Tier::Balanced, Tier::Large] {
            assert_eq!(
                all().iter().filter(|entry| entry.tier == tier).count(),
                1,
                "{tier:?}"
            );
        }
        assert_eq!(of_tier(Tier::Balanced).unwrap().id, "gte-modernbert-base");
        assert!(find("gte-modernbert-base").is_some());
        assert!(find("no-such-model").is_none());
    }

    #[test]
    fn the_embedder_name_changes_with_the_revision() {
        let entry = find("gte-modernbert-base").unwrap();
        assert_eq!(
            entry.embedder_name(),
            "builtin:gte-modernbert-base@e7f32e3c00f9"
        );
        let mut moved = entry.clone();
        moved.revision = "0".repeat(40);
        assert_ne!(moved.embedder_name(), entry.embedder_name());
        assert_eq!(entry.size(), 1184 + 3_583_228 + 298_041_568);
    }
}
