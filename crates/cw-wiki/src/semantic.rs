//! Semantic search: embeds search documents and blends vector matches with
//! full-text matches.
//!
//! Full-text search finds the words typed. Vectors also find documents that
//! use other forms of those words, or (with a neural embedder) other words
//! for the same idea. Results of both are merged by rank, so neither side's
//! score scale matters.

use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

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
/// What that key holds while a run replaces another embedder's vectors. No
/// embedder has this name, so the vectors are compared with no query.
const MIXED: &str = "(being replaced)";
/// Start of the meta key that says whether documents of one kind wait for
/// their vector: "1" from the plan of a run until the run is done.
const WAITING: &str = "vectors_waiting:";

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

    /// Use the embedder the settings in effect name, from now on. A built-in
    /// model is looked for under the default data directory.
    pub fn configure(&self, resolved: &Resolved) {
        self.configure_in(resolved, &cw_core::paths::data_dir());
    }

    /// [`Slot::configure`] with the data directory that holds the built-in
    /// models.
    pub fn configure_in(&self, resolved: &Resolved, data_dir: &Path) {
        let embedder = built(cw_embed::from_settings_in(resolved, data_dir));
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

/// [`configure`] with the data directory that holds the built-in models. A
/// backend passes its own, which need not be the default one.
pub fn configure_in(resolved: &Resolved, data_dir: &Path) {
    EMBEDDER.configure_in(resolved, data_dir);
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

/// Whether documents of `kinds` still wait for a vector of the embedder
/// `name`: another embedder made the stored ones, or a run was cut short.
/// The backend embeds such an index in the background.
pub fn behind(index: &Index, name: &str, kinds: &[&str]) -> Result<bool> {
    for kind in kinds {
        let made_by = index.meta(&format!("{MADE_BY}{kind}"))?;
        let waiting = index.meta(&format!("{WAITING}{kind}"))?;
        if made_by.as_deref() != Some(name) || waiting.as_deref() == Some("1") {
            return Ok(true);
        }
    }
    Ok(false)
}

/// How far a run is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Documents embedded and stored by this run.
    pub done: usize,
    /// Documents this run set out to embed.
    pub total: usize,
}

struct Pending {
    kind: String,
    key: String,
    title: String,
    text: String,
    hash: String,
}

/// One pass that brings the stored vectors of some kinds in line with their
/// documents, a batch at a time.
///
/// A neural model takes minutes over a repository. A run stores every batch,
/// so one that is stopped keeps what it did and the next run goes on from
/// there: a document whose vector is stored is not embedded again. The caller
/// drives it, which lets a server embed without holding a lock and take the
/// lock for [`Run::store`] alone:
///
/// ```text
/// let mut run = Run::plan(index, embedder, kinds, documents)?;
/// while !run.is_done() {
///     let vectors = embedder.embed(&run.next())?;
///     run.store(index, vectors)?;
/// }
/// run.finish(index)?;
/// ```
pub struct Run {
    model: String,
    kinds: Vec<String>,
    pending: Vec<Pending>,
    done: usize,
    batch: usize,
}

impl Run {
    /// Compare the stored vectors with `documents`, drop the vectors of
    /// removed documents, and note which documents must be embedded.
    pub fn plan(
        index: &mut Index,
        embedder: &dyn Embedder,
        kinds: &[&str],
        documents: &[Document],
    ) -> Result<Self> {
        let model = embedder.name();
        let stored = index.vector_hashes(kinds)?;
        let mut keep = HashSet::new();
        let mut pending = Vec::new();
        for document in documents {
            let id = (document.kind.clone(), document.key.clone());
            let text = text_of(document);
            let hash = sha256_hex(format!("{model}\n{text}").as_bytes());
            if stored.get(&id) != Some(&hash) {
                pending.push(Pending {
                    kind: document.kind.clone(),
                    key: document.key.clone(),
                    title: document.title.clone(),
                    text,
                    hash,
                });
            }
            keep.insert(id);
        }
        index.prune_vectors(kinds, &keep)?;
        for kind in kinds {
            let waits = pending.iter().any(|document| document.kind == *kind);
            index.set_meta(&format!("{WAITING}{kind}"), if waits { "1" } else { "0" })?;
            // Until the run is done, the vectors of a kind that another
            // embedder made are a mix of two embedders. Record that, so that
            // a run cut short does not leave the other embedder's name, which
            // would make the mix count as current after a switch back to it.
            let key = format!("{MADE_BY}{kind}");
            if waits && index.meta(&key)?.is_some_and(|recorded| recorded != model) {
                index.set_meta(&key, MIXED)?;
            }
        }
        Ok(Self {
            batch: embedder.batch_size().max(1),
            model,
            kinds: kinds.iter().map(|kind| kind.to_string()).collect(),
            pending,
            done: 0,
        })
    }

    pub fn progress(&self) -> Progress {
        Progress {
            done: self.done,
            total: self.pending.len(),
        }
    }

    /// Whether every document has its vector.
    pub fn is_done(&self) -> bool {
        self.done == self.pending.len()
    }

    /// The texts to embed next; empty when the run is done.
    pub fn next(&self) -> Vec<String> {
        self.pending[self.done..]
            .iter()
            .take(self.batch)
            .map(|document| document.text.clone())
            .collect()
    }

    /// Store the vectors of the texts [`Run::next`] gave, in their order.
    pub fn store(&mut self, index: &mut Index, vectors: Vec<Vec<f32>>) -> Result<()> {
        let batch = &self.pending[self.done..(self.done + self.batch).min(self.pending.len())];
        if vectors.len() != batch.len() {
            return Err(cw_core::Error::invalid(format!(
                "{} returned {} vectors for {} texts",
                self.model,
                vectors.len(),
                batch.len()
            )));
        }
        let rows: Vec<VectorRow> = batch
            .iter()
            .zip(vectors)
            .map(|(document, vector)| VectorRow {
                kind: document.kind.clone(),
                key: document.key.clone(),
                title: document.title.clone(),
                hash: document.hash.clone(),
                vector,
            })
            .collect();
        index.upsert_vectors(&rows)?;
        self.done += rows.len();
        Ok(())
    }

    /// Record that every vector of the kinds comes from this run's embedder.
    /// Does nothing while documents still wait.
    pub fn finish(&self, index: &Index) -> Result<()> {
        if !self.is_done() {
            return Ok(());
        }
        for kind in &self.kinds {
            index.set_meta(&format!("{MADE_BY}{kind}"), &self.model)?;
            index.set_meta(&format!("{WAITING}{kind}"), "0")?;
        }
        Ok(())
    }
}

/// Bring the stored vectors of `kinds` in line with `documents`: embed new
/// and changed documents, drop vectors of removed ones. After each batch
/// `on_batch` hears how far the run is and may stop it; what is stored stays,
/// and the next run embeds the rest. Returns where the run ended.
pub fn embed_documents_with(
    index: &mut Index,
    embedder: &dyn Embedder,
    kinds: &[&str],
    documents: &[Document],
    on_batch: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<Progress> {
    let mut run = Run::plan(index, embedder, kinds, documents)?;
    while !run.is_done() {
        let vectors = embedder.embed(&run.next())?;
        run.store(index, vectors)?;
        if on_batch(run.progress()).is_break() {
            break;
        }
    }
    run.finish(index)?;
    Ok(run.progress())
}

/// [`embed_documents_with`] to the end. Returns how many documents were
/// embedded.
pub fn embed_documents(
    index: &mut Index,
    embedder: &dyn Embedder,
    kinds: &[&str],
    documents: &[Document],
) -> Result<usize> {
    let progress = embed_documents_with(index, embedder, kinds, documents, &mut |_| {
        ControlFlow::Continue(())
    })?;
    Ok(progress.done)
}

/// Milliseconds [`refresh`] embeds for before it leaves the rest to a later
/// run; `u64::MAX` is no limit.
static INLINE_BUDGET: AtomicU64 = AtomicU64::new(u64::MAX);

/// Let [`refresh`] embed for about `budget` and leave the rest waiting
/// ([`behind`] then says so); `None` takes the limit away. The backend sets
/// this: an update holds its lock and its client's patience while it
/// embeds, and a first embedding with a neural model takes minutes. The
/// embedder that needs no model is through thousands of documents within
/// any budget, so for it nothing changes. A program that sets nothing
/// embeds everything where it stands.
///
/// It is time and not a count of documents, because a document takes one
/// model a hundred times as long as another.
pub fn set_inline_budget(budget: Option<Duration>) {
    let millis = budget.map_or(u64::MAX, |budget| {
        u64::try_from(budget.as_millis()).unwrap_or(u64::MAX - 1)
    });
    INLINE_BUDGET.store(millis, Ordering::Relaxed);
}

/// [`embed_documents_with`] with the process embedder, within the inline
/// budget. A failing embedder (for example an unreachable Ollama) is
/// reported and leaves full-text search working.
pub fn refresh(index: &mut Index, kinds: &[&str], documents: &[Document]) {
    let Some(embedder) = embedder() else {
        return;
    };
    let budget = match INLINE_BUDGET.load(Ordering::Relaxed) {
        u64::MAX => None,
        millis => Some(Duration::from_millis(millis)),
    };
    match embed_within(index, embedder.as_ref(), kinds, documents, budget) {
        Ok(progress) if progress.done < progress.total => log::info!(
            "{} of {} documents wait for their search vectors",
            progress.total - progress.done,
            progress.total
        ),
        Ok(_) => {}
        Err(error) => log::warn!("could not update search vectors: {error}"),
    }
}

/// [`embed_documents_with`] that stops after the batch during which
/// `budget` ran out; `None` is no limit. Under a budget, an embedder that
/// is slow embeds nothing here: its one batch could take longer than the
/// whole budget, and starting its model longer still. The run is planned,
/// so that the index knows what waits. Any other embeds at least one batch.
fn embed_within(
    index: &mut Index,
    embedder: &dyn Embedder,
    kinds: &[&str],
    documents: &[Document],
    budget: Option<Duration>,
) -> Result<Progress> {
    if budget.is_some() && embedder.slow() {
        let run = Run::plan(index, embedder, kinds, documents)?;
        run.finish(index)?;
        return Ok(run.progress());
    }
    let started = Instant::now();
    embed_documents_with(index, embedder, kinds, documents, &mut |_| {
        if budget.is_some_and(|budget| started.elapsed() >= budget) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
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
    let query = match embedder.embed_query(text) {
        Ok(vector) => vector,
        Err(error) => {
            log::warn!("semantic search skipped: {error}");
            return Ok(Vec::new());
        }
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

    /// An embedder that takes one document at a time, as a slow model does.
    struct OneByOne(LocalEmbedder);

    impl Embedder for OneByOne {
        fn name(&self) -> String {
            "one-by-one".into()
        }

        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            assert!(texts.len() <= 1, "more than its batch size");
            self.0.embed(texts)
        }

        fn batch_size(&self) -> usize {
            1
        }
    }

    fn stop_after(batches: usize) -> impl FnMut(Progress) -> ControlFlow<()> {
        let mut seen = 0;
        move |_| {
            seen += 1;
            if seen >= batches {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }
    }

    #[test]
    fn a_run_that_is_stopped_goes_on_where_it_stopped() {
        let mut index = Index::open_in_memory().unwrap();
        index.replace_documents(&["note"], &corpus()).unwrap();
        let embedder = OneByOne(LocalEmbedder::default());
        let mut seen = Vec::new();
        let mut on_batch = |progress: Progress| {
            seen.push(progress);
            ControlFlow::Break(())
        };
        let first =
            embed_documents_with(&mut index, &embedder, &["note"], &corpus(), &mut on_batch)
                .unwrap();
        assert_eq!(first, Progress { done: 1, total: 3 });
        assert_eq!(seen, [Progress { done: 1, total: 3 }]);
        // The one batch is stored, and the index knows that more wait.
        assert_eq!(index.vector_hashes(&["note"]).unwrap().len(), 1);
        assert!(behind(&index, &embedder.name(), &["note"]).unwrap());

        // The next run embeds the other two and no more.
        let second = embed_documents_with(
            &mut index,
            &embedder,
            &["note"],
            &corpus(),
            &mut stop_after(usize::MAX),
        )
        .unwrap();
        assert_eq!(second, Progress { done: 2, total: 2 });
        assert_eq!(index.vector_hashes(&["note"]).unwrap().len(), 3);
        assert!(!behind(&index, &embedder.name(), &["note"]).unwrap());
        let hits = search_with(&index, Some(&embedder), "resolving", None, 5).unwrap();
        assert_eq!(hits.first().map(|h| h.key.as_str()), Some("resolver.md"));
    }

    #[test]
    fn a_stopped_switch_of_embedder_never_counts_as_current() {
        let mut index = indexed();
        let first = LocalEmbedder::default();
        let other = OneByOne(LocalEmbedder::default());
        embed_documents_with(&mut index, &other, &["note"], &corpus(), &mut stop_after(1)).unwrap();
        // One vector is the other embedder's, two are the first one's. To
        // neither embedder is that mix its own: both search by words alone.
        for embedder in [&first as &dyn Embedder, &other] {
            assert!(
                search_with(&index, Some(embedder), "resolving", None, 5)
                    .unwrap()
                    .is_empty()
            );
            assert!(behind(&index, &embedder.name(), &["note"]).unwrap());
        }
        // Back to the first embedder: only the replaced vector is made again.
        assert_eq!(
            embed_documents(&mut index, &first, &["note"], &corpus()).unwrap(),
            1
        );
        assert!(!behind(&index, &first.name(), &["note"]).unwrap());
        let hits = search_with(&index, Some(&first), "resolving", None, 5).unwrap();
        assert_eq!(hits.first().map(|h| h.key.as_str()), Some("resolver.md"));
    }

    #[test]
    fn an_update_with_the_same_embedder_keeps_the_search_working_meanwhile() {
        let mut index = indexed();
        let embedder = LocalEmbedder::default();
        let mut changed = corpus();
        changed[1]
            .body
            .push_str(" Tokens are compared in constant time.");
        changed[2].body.push_str(" Barnes-Hut keeps it fast.");
        // Planned and not carried out, as when an update stops early.
        let run = Run::plan(&mut index, &embedder, &["note"], &changed).unwrap();
        assert_eq!(run.progress(), Progress { done: 0, total: 2 });
        assert!(behind(&index, &embedder.name(), &["note"]).unwrap());
        // Every stored vector is still this embedder's, so they are compared.
        let hits = search_with(&index, Some(&embedder), "resolving", None, 5).unwrap();
        assert_eq!(hits.first().map(|h| h.key.as_str()), Some("resolver.md"));
    }

    /// An embedder that says it is slow, and counts how often it is asked.
    struct Slow(LocalEmbedder, std::sync::atomic::AtomicUsize);

    impl Embedder for Slow {
        fn name(&self) -> String {
            "slow".into()
        }

        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            self.1.fetch_add(1, Ordering::SeqCst);
            self.0.embed(texts)
        }

        fn slow(&self) -> bool {
            true
        }
    }

    #[test]
    fn under_a_budget_a_slow_embedder_is_left_to_the_background() {
        let mut index = indexed();
        let embedder = Slow(LocalEmbedder::default(), Default::default());
        let budget = Some(Duration::from_secs(60));
        let progress = embed_within(&mut index, &embedder, &["note"], &corpus(), budget).unwrap();
        // Nothing was embedded, however large the budget, and the index
        // knows that all of it waits.
        assert_eq!(progress, Progress { done: 0, total: 3 });
        assert_eq!(embedder.1.load(Ordering::SeqCst), 0);
        assert!(behind(&index, &embedder.name(), &["note"]).unwrap());
        // With no budget it embeds where it stands, as a command does.
        let progress = embed_within(&mut index, &embedder, &["note"], &corpus(), None).unwrap();
        assert_eq!(progress, Progress { done: 3, total: 3 });
        assert!(!behind(&index, &embedder.name(), &["note"]).unwrap());
        // Nothing to embed is not a reason to wait.
        let progress = embed_within(&mut index, &embedder, &["note"], &corpus(), budget).unwrap();
        assert_eq!(progress, Progress { done: 0, total: 0 });
        assert!(!behind(&index, &embedder.name(), &["note"]).unwrap());
    }

    #[test]
    fn a_budget_that_ran_out_leaves_the_rest_waiting() {
        let mut index = Index::open_in_memory().unwrap();
        index.replace_documents(&["note"], &corpus()).unwrap();
        let embedder = OneByOne(LocalEmbedder::default());
        // No time at all still embeds one batch, so that every update gets
        // somewhere.
        let none = Some(Duration::ZERO);
        let progress = embed_within(&mut index, &embedder, &["note"], &corpus(), none).unwrap();
        assert_eq!(progress, Progress { done: 1, total: 3 });
        assert!(behind(&index, &embedder.name(), &["note"]).unwrap());
        // Time enough embeds the rest, as no limit does.
        let plenty = Some(Duration::from_secs(60));
        let progress = embed_within(&mut index, &embedder, &["note"], &corpus(), plenty).unwrap();
        assert_eq!(progress, Progress { done: 2, total: 2 });
        assert!(!behind(&index, &embedder.name(), &["note"]).unwrap());
        let progress = embed_within(&mut index, &embedder, &["note"], &corpus(), None).unwrap();
        assert_eq!(progress, Progress { done: 0, total: 0 });
    }

    #[test]
    fn a_run_refuses_vectors_that_do_not_match_its_batch() {
        let mut index = Index::open_in_memory().unwrap();
        index.replace_documents(&["note"], &corpus()).unwrap();
        let embedder = LocalEmbedder::default();
        let mut run = Run::plan(&mut index, &embedder, &["note"], &corpus()).unwrap();
        assert_eq!(run.next().len(), 3);
        let error = run.store(&mut index, vec![vec![1.0]]).unwrap_err();
        assert!(
            error.to_string().contains("1 vectors for 3 texts"),
            "{error}"
        );
        assert!(!run.is_done());
    }

    /// An embedder whose query vector is not the vector of the query's text,
    /// as with a model that puts an instruction before a query.
    struct Instructed(LocalEmbedder);

    impl Embedder for Instructed {
        fn name(&self) -> String {
            self.0.name()
        }

        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            self.0.embed(texts)
        }

        fn embed_query(&self, _text: &str) -> Result<Vec<f32>> {
            Ok(self.0.embed_one("token middleware bearer"))
        }
    }

    #[test]
    fn a_search_embeds_its_text_as_a_query() {
        let index = indexed();
        let embedder = Instructed(LocalEmbedder::default());
        // No document holds this word; only the query vector can find one.
        let hits = search_with(&index, Some(&embedder), "zzz", None, 5).unwrap();
        assert_eq!(hits.first().map(|h| h.key.as_str()), Some("auth.md"));
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
