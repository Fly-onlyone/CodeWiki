//! Semantic search: embeds search documents and blends vector matches with
//! full-text matches.
//!
//! Full-text search finds the words typed. Vectors also find documents that
//! use other forms of those words, or (with a neural embedder) other words
//! for the same idea. Results of both are merged by rank, so neither side's
//! score scale matters.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use cw_core::{Result, sha256_hex};
use cw_embed::Embedder;
use cw_index::{Document, Hit, Index, VectorRow};

/// Vector matches below this similarity are noise, not relatives.
const MIN_SIMILARITY: f64 = 0.18;
/// Rank damping of reciprocal rank fusion; the usual value.
const RRF_K: f64 = 60.0;
/// Characters of a document's opening shown when full-text search gave no snippet.
const OPENING_CHARS: usize = 160;

/// The embedder chosen by the environment, read once per process. A bad
/// setting is reported once and turns semantic search off rather than
/// failing every index update.
pub fn embedder() -> Option<&'static dyn Embedder> {
    static EMBEDDER: OnceLock<Option<Box<dyn Embedder>>> = OnceLock::new();
    EMBEDDER
        .get_or_init(|| match cw_embed::from_env() {
            Ok(embedder) => embedder,
            Err(error) => {
                eprintln!("codewiki: semantic search is off: {error}");
                None
            }
        })
        .as_deref()
}

fn text_of(document: &Document) -> String {
    format!("{}\n{}", document.title, document.body)
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
    Ok(embedded)
}

/// [`embed_documents`] with the process embedder. A failing embedder (for
/// example an unreachable Ollama) is reported and leaves full-text search
/// working.
pub fn refresh(index: &mut Index, kinds: &[&str], documents: &[Document]) {
    let Some(embedder) = embedder() else {
        return;
    };
    if let Err(error) = embed_documents(index, embedder, kinds, documents) {
        eprintln!("codewiki: could not update search vectors: {error}");
    }
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
    let semantic = match embedder.embed(&[text.to_string()]) {
        Ok(vectors) => match vectors.first() {
            Some(query) => index.nearest(query, kind, pool)?,
            None => Vec::new(),
        },
        Err(error) => {
            eprintln!("codewiki: semantic search skipped: {error}");
            Vec::new()
        }
    };
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
    search_with(index, embedder(), text, kind, limit)
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
}
