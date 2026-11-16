//! Semantic search: embeds search documents and blends vector matches with
//! full-text matches.
//!
//! Full-text search finds the words typed. Vectors also find documents that
//! use other forms of those words, or (with a neural embedder) other words
//! for the same idea. Results of both are merged by rank, so neither side's
//! score scale matters.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use cw_core::settings::Resolved;
use cw_core::{Result, sha256_hex};
use cw_embed::Embedder;
use cw_index::{Document, Hit, Index, VectorRow};

/// Vector matches below this similarity are noise, not relatives.
const MIN_SIMILARITY: f64 = 0.18;
/// Rank damping of reciprocal rank fusion; the usual value.
const RRF_K: f64 = 60.0;
/// Characters of a document's opening shown when full-text search gave no snippet.
const OPENING_CHARS: usize = 160;
/// Start of the index's meta key that names the embedder which made the
/// vectors of one kind of document.
const MADE_BY: &str = "vectors_embedder:";

/// An embedder, or the fact that semantic search is off. A bad setting is
/// reported once and turns semantic search off rather than failing every
/// index update.
fn built(embedder: Result<Option<Box<dyn Embedder>>>) -> Option<Arc<dyn Embedder>> {
    match embedder {
        Ok(embedder) => embedder.map(Arc::from),
        Err(error) => {
            log::warn!("semantic search is off: {error}");
            None
        }
    }
}

/// The place that holds the embedder in use. It can be given another one
/// while the program runs; work that already took the old one finishes with
/// it.
pub struct Slot {
    /// `None` until something asks: nothing has been chosen yet.
    current: RwLock<Option<Option<Arc<dyn Embedder>>>>,
}

impl Slot {
    pub const fn new() -> Self {
        Self {
            current: RwLock::new(None),
        }
    }

    /// The embedder in use. Before [`Slot::configure`] is called it is the
    /// one the environment alone names, so a program that never reads the
    /// settings file (a test, above all) is not changed by one.
    pub fn get(&self) -> Option<Arc<dyn Embedder>> {
        if let Some(chosen) = &*self.current.read().unwrap_or_else(|p| p.into_inner()) {
            return chosen.clone();
        }
        let mut current = self.current.write().unwrap_or_else(|p| p.into_inner());
        current
            .get_or_insert_with(|| built(cw_embed::from_env()))
            .clone()
    }

    /// Use the embedder the settings in effect name, from now on.
    pub fn configure(&self, resolved: &Resolved) {
        let embedder = built(cw_embed::from_settings(resolved));
        *self.current.write().unwrap_or_else(|p| p.into_inner()) = Some(embedder);
    }
}

impl Default for Slot {
    fn default() -> Self {
        Self::new()
    }
}

static EMBEDDER: Slot = Slot::new();

/// The embedder this process uses; `None` when semantic search is off.
pub fn embedder() -> Option<Arc<dyn Embedder>> {
    EMBEDDER.get()
}

/// Use the embedder the settings in effect name. The programs call this at
/// their start, and the backend again when the settings change, so no restart
/// is needed.
pub fn configure(resolved: &Resolved) {
    EMBEDDER.configure(resolved);
}

fn text_of(document: &Document) -> String {
    format!("{}\n{}", document.title, document.body)
}

/// Whether the vectors of `kind` can be compared with a query that the
/// embedder `name` made. Vectors of an embedder chosen earlier stay in the
/// index until the next update; compared with another embedder's query they
/// would give scores that mean nothing. An index written before the embedder
/// was recorded counts as current.
fn made_by(index: &Index, kind: &str, name: &str) -> bool {
    match index.meta(&format!("{MADE_BY}{kind}")) {
        Ok(Some(recorded)) => recorded == name,
        _ => true,
    }
}

/// Bring the stored vectors of `kinds` in line with `documents`: embed new
/// and changed documents, drop vectors of removed ones. Returns how many
/// documents were embedded.
pub fn embed_documents(
    index: &mut Index,
    embedder: &dyn Embedder,
    kinds: &[&str],
    documents: &[Document],
) -> Result<usize> {
    let model = embedder.name();
    let stored = index.vector_hashes(kinds)?;
    let mut keep = HashSet::new();
    let mut pending: Vec<(&Document, String, String)> = Vec::new();
    for document in documents {
        let id = (document.kind.clone(), document.key.clone());
        let text = text_of(document);
        let hash = sha256_hex(format!("{model}\n{text}").as_bytes());
        if stored.get(&id) != Some(&hash) {
            pending.push((document, text, hash));
        }
        keep.insert(id);
    }
    let texts: Vec<String> = pending.iter().map(|(_, text, _)| text.clone()).collect();
    let vectors = if texts.is_empty() {
        Vec::new()
    } else {
        embedder.embed(&texts)?
    };
    let rows: Vec<VectorRow> = pending
        .into_iter()
        .zip(vectors)
        .map(|((document, _, hash), vector)| VectorRow {
            kind: document.kind.clone(),
            key: document.key.clone(),
            title: document.title.clone(),
            hash,
            vector,
        })
        .collect();
    let embedded = rows.len();
    index.store_vectors(kinds, &keep, &rows)?;
    // Every vector of these kinds now comes from this embedder.
    for kind in kinds {
        index.set_meta(&format!("{MADE_BY}{kind}"), &model)?;
    }
    Ok(embedded)
}

/// [`embed_documents`] with the process embedder. A failing embedder (for
/// example an unreachable Ollama) is reported and leaves full-text search
/// working.
pub fn refresh(index: &mut Index, kinds: &[&str], documents: &[Document]) {
    let Some(embedder) = embedder() else {
        return;
    };
    if let Err(error) = embed_documents(index, embedder.as_ref(), kinds, documents) {
        log::warn!("could not update search vectors: {error}");
    }
}

/// The stored vectors closest to `text`, among the kinds of document whose
/// vectors `embedder` made. Nothing is sent to the embedder when it made
/// none of them.
fn nearest(
    index: &Index,
    embedder: &dyn Embedder,
    text: &str,
    kind: Option<&str>,
    limit: usize,
) -> Result<Vec<Hit>> {
    let name = embedder.name();
    let kinds: Vec<String> = match kind {
        Some(kind) => vec![kind.to_string()],
        None => index.vector_kinds()?,
    }
    .into_iter()
    .filter(|kind| made_by(index, kind, &name))
    .collect();
    if kinds.is_empty() {
        return Ok(Vec::new());
    }
    let query = match embedder.embed(&[text.to_string()]) {
        Ok(vectors) => vectors.into_iter().next(),
        Err(error) => {
            log::warn!("semantic search skipped: {error}");
            None
        }
    };
    let Some(query) = query else {
        return Ok(Vec::new());
    };
    let mut hits = Vec::new();
    for kind in &kinds {
        hits.extend(index.nearest(&query, Some(kind), limit)?);
    }
    hits.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.key.cmp(&b.key)));
    hits.truncate(limit);
    Ok(hits)
}

/// Search with full-text and vector matches merged by reciprocal rank
/// fusion. Without an embedder this is plain full-text search.
pub fn search_with(
    index: &Index,
    embedder: Option<&dyn Embedder>,
    text: &str,
    kind: Option<&str>,
    limit: usize,
) -> Result<Vec<Hit>> {
    let pool = limit.saturating_mul(3).max(10);
    let lexical = index.search(text, kind, pool)?;
    let Some(embedder) = embedder else {
        return Ok(lexical.into_iter().take(limit).collect());
    };
    let semantic = nearest(index, embedder, text, kind, pool)?;
    let mut merged: HashMap<(String, String), (f64, Hit)> = HashMap::new();
    for (rank, hit) in lexical.into_iter().enumerate() {
        let score = 1.0 / (RRF_K + rank as f64 + 1.0);
        merged.insert((hit.kind.clone(), hit.key.clone()), (score, hit));
    }
    for (rank, hit) in semantic
        .into_iter()
        .filter(|h| h.score >= MIN_SIMILARITY)
        .enumerate()
    {
        let score = 1.0 / (RRF_K + rank as f64 + 1.0);
        merged
            .entry((hit.kind.clone(), hit.key.clone()))
            .and_modify(|(total, _)| *total += score)
            .or_insert((score, hit));
    }
    let mut hits: Vec<Hit> = merged
        .into_values()
        .map(|(score, mut hit)| {
            hit.score = score;
            hit
        })
        .collect();
    hits.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.key.cmp(&b.key)));
    hits.truncate(limit);
    for hit in &mut hits {
        if hit.snippet.is_empty() {
            hit.snippet = index.document_opening(&hit.kind, &hit.key, OPENING_CHARS)?;
        }
    }
    Ok(hits)
}

/// [`search_with`] using the process embedder.
pub fn search(index: &Index, text: &str, kind: Option<&str>, limit: usize) -> Result<Vec<Hit>> {
    search_with(index, embedder().as_deref(), text, kind, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_embed::LocalEmbedder;

    fn doc(key: &str, title: &str, body: &str) -> Document {
        Document {
            kind: "note".into(),
            key: key.into(),
            title: title.into(),
            body: body.into(),
        }
    }

    fn corpus() -> Vec<Document> {
        vec![
            doc(
                "resolver.md",
                "Name Resolver",
                "Resolves paths written in source to the symbols they name.",
            ),
            doc(
                "auth.md",
                "Token Middleware",
                "Rejects requests whose bearer token does not match.",
            ),
            doc(
                "layout.md",
                "Graph Layout",
                "Places nodes with a force simulation.",
            ),
        ]
    }

    fn indexed() -> Index {
        let mut index = Index::open_in_memory().unwrap();
        index.replace_documents(&["note"], &corpus()).unwrap();
        let embedded =
            embed_documents(&mut index, &LocalEmbedder::default(), &["note"], &corpus()).unwrap();
        assert_eq!(embedded, 3);
        index
    }

    #[test]
    fn embeds_only_new_and_changed_documents() {
        let mut index = indexed();
        let embedder = LocalEmbedder::default();
        assert_eq!(
            embed_documents(&mut index, &embedder, &["note"], &corpus()).unwrap(),
            0
        );
        let mut changed = corpus();
        changed[0].body.push_str(" Follows re-exports.");
        changed.pop();
        assert_eq!(
            embed_documents(&mut index, &embedder, &["note"], &changed).unwrap(),
            1
        );
        // The removed document's vector is gone.
        assert_eq!(index.vector_hashes(&["note"]).unwrap().len(), 2);
        // Another embedder re-embeds everything.
        let other = LocalEmbedder { dimensions: 64 };
        assert_eq!(
            embed_documents(&mut index, &other, &["note"], &changed).unwrap(),
            2
        );
    }

    #[test]
    fn vectors_find_word_forms_full_text_misses() {
        let index = indexed();
        // No document contains the word "resolving"; prefix matching on the
        // whole word finds nothing.
        assert!(index.search("resolving", None, 5).unwrap().is_empty());
        let embedder = LocalEmbedder::default();
        let hits = search_with(&index, Some(&embedder), "resolving", None, 5).unwrap();
        assert_eq!(hits.first().map(|h| h.key.as_str()), Some("resolver.md"));
        assert!(hits[0].snippet.starts_with("Resolves paths"));
        // Without an embedder the result is exactly full-text search.
        assert!(
            search_with(&index, None, "resolving", None, 5)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_match_on_both_sides_ranks_first() {
        let index = indexed();
        let embedder = LocalEmbedder::default();
        let hits = search_with(&index, Some(&embedder), "bearer token", None, 5).unwrap();
        assert_eq!(hits[0].key, "auth.md");
        assert!(hits.iter().all(|h| h.kind == "note"));
        // Unrelated documents stay out: nothing here is about tokens.
        assert!(hits.iter().all(|h| h.key != "layout.md"));
        assert!(
            search_with(&index, Some(&embedder), "bearer token", Some("symbol"), 5)
                .unwrap()
                .is_empty()
        );
    }

    /// The local embedder under another name: same vectors, same length, but
    /// to the index a different embedder.
    struct Renamed(LocalEmbedder);

    impl Embedder for Renamed {
        fn name(&self) -> String {
            "renamed".into()
        }

        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            self.0.embed(texts)
        }
    }

    #[test]
    fn vectors_of_an_earlier_embedder_are_not_compared() {
        let mut index = indexed();
        let other = Renamed(LocalEmbedder::default());
        // Its vectors would have the right length, so only the recorded name
        // keeps them apart: the search is full-text alone.
        assert!(
            search_with(&index, Some(&other), "resolving", None, 5)
                .unwrap()
                .is_empty()
        );
        assert!(
            search_with(&index, Some(&other), "resolving", Some("note"), 5)
                .unwrap()
                .is_empty()
        );
        // The next update embeds everything again, and they count.
        assert_eq!(
            embed_documents(&mut index, &other, &["note"], &corpus()).unwrap(),
            3
        );
        let hits = search_with(&index, Some(&other), "resolving", None, 5).unwrap();
        assert_eq!(hits.first().map(|h| h.key.as_str()), Some("resolver.md"));
        // The embedder used before is now the stranger.
        let first = LocalEmbedder::default();
        assert!(
            search_with(&index, Some(&first), "resolving", None, 5)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn an_index_from_before_the_embedder_was_recorded_still_matches() {
        let mut index = Index::open_in_memory().unwrap();
        index.replace_documents(&["note"], &corpus()).unwrap();
        let embedder = LocalEmbedder::default();
        let texts: Vec<String> = corpus().iter().map(text_of).collect();
        let rows: Vec<VectorRow> = corpus()
            .iter()
            .zip(embedder.embed(&texts).unwrap())
            .map(|(document, vector)| VectorRow {
                kind: document.kind.clone(),
                key: document.key.clone(),
                title: document.title.clone(),
                hash: "old".into(),
                vector,
            })
            .collect();
        let keep = rows
            .iter()
            .map(|row| (row.kind.clone(), row.key.clone()))
            .collect();
        // Stored the way an older version did: no embedder is recorded.
        index.store_vectors(&["note"], &keep, &rows).unwrap();
        let hits = search_with(&index, Some(&embedder), "resolving", None, 5).unwrap();
        assert_eq!(hits.first().map(|h| h.key.as_str()), Some("resolver.md"));
    }

    #[test]
    fn the_slot_takes_another_embedder_while_the_old_one_is_in_use() {
        use cw_core::settings::{EMBED_PROVIDER, Settings};

        let resolved = |provider: &str| {
            let mut saved = Settings::default();
            saved.set(EMBED_PROVIDER, provider).unwrap();
            Resolved::from_parts(&saved, |_| None)
        };
        let slot = Slot::new();
        slot.configure(&resolved("local"));
        let held = slot.get().expect("the local embedder");
        assert_eq!(held.name(), LocalEmbedder::default().name());

        slot.configure(&resolved("none"));
        assert!(slot.get().is_none());
        // Work that took the embedder before the change finishes with it.
        assert_eq!(held.embed(&["still here".to_string()]).unwrap().len(), 1);

        // A setting that cannot be used turns semantic search off.
        slot.configure(&resolved("no-such-embedder"));
        assert!(slot.get().is_none());
        slot.configure(&resolved("local"));
        assert!(slot.get().is_some());
    }
}
