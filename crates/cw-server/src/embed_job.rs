//! The background embedding job: the one long job that may run.
//!
//! A job embeds the documents of every repository that still wait for a
//! vector of the embedder in use, whichever embedder that is. An update of a
//! wiki leaves it what does not fit the inline budget, a backend starts one
//! for what waits, and `POST /v1/embed/reembed` starts one by hand. It runs in
//! the background and reports how far it is in `GET /v1/embed/job`. Nothing
//! is kept on disk about it: a backend that starts looks at the indexes
//! instead, and goes on where documents still wait for a vector.
//!
//! A job whose embedder fails ends with the reason, and the documents keep
//! waiting. Nothing tries again by itself: the next update or reembed does.
//!
//! Locks: embedding takes minutes and holds none. The write lock is taken
//! for each batch's write alone, so a note can be saved and a wiki updated
//! while a job runs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use cw_core::{Error, Result};
use cw_embed::Embedder;
use cw_wiki::semantic::{Run, behind};
use cw_wiki::service::{self, DOCUMENT_KINDS};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{AppState, repos};

/// How often a job looks over the repositories again when updates ran while
/// it worked.
const PASSES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Phase {
    Embedding,
    Done,
    Failed,
    Cancelled,
}

impl Phase {
    fn over(self) -> bool {
        matches!(self, Phase::Done | Phase::Failed | Phase::Cancelled)
    }
}

/// A job as a page shows it.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Job {
    phase: Phase,
    /// Name of the embedder the job embeds with; `None` when semantic search
    /// is off.
    model: Option<String>,
    /// Name of the repository being embedded.
    repo: Option<String>,
    /// Its place among the repositories of this pass, from 1.
    repo_index: usize,
    repo_count: usize,
    docs_done: usize,
    docs_total: usize,
    /// Repositories left out, each with the reason.
    skipped: Vec<Value>,
    /// What a person should know about a job that did not fail.
    note: Option<String>,
    error: Option<String>,
}

/// The job that runs, or the last one that ran.
pub(crate) struct Jobs {
    current: Mutex<Option<Job>>,
    /// Stops the job at its next batch.
    cancel: AtomicBool,
    /// The backend is stopping: the job ends without a word.
    closing: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Default for Jobs {
    fn default() -> Self {
        Self {
            current: Mutex::new(None),
            cancel: AtomicBool::new(false),
            closing: AtomicBool::new(false),
        }
    }
}

impl Jobs {
    /// Take the place of the running job, or say that one runs.
    fn begin(&self, model: Option<String>) -> Result<Job> {
        let mut current = lock(&self.current);
        if current.as_ref().is_some_and(|job| !job.phase.over()) {
            return Err(Error::invalid(
                "another embedding job is running; wait for it or cancel it",
            ));
        }
        self.cancel.store(false, Ordering::SeqCst);
        let job = Job {
            phase: Phase::Embedding,
            model,
            repo: None,
            repo_index: 0,
            repo_count: 0,
            docs_done: 0,
            docs_total: 0,
            skipped: Vec::new(),
            note: None,
            error: None,
        };
        *current = Some(job.clone());
        Ok(job)
    }

    fn change(&self, change: impl FnOnce(&mut Job)) {
        if let Some(job) = lock(&self.current).as_mut() {
            change(job);
        }
    }

    pub(crate) fn snapshot(&self) -> Option<Job> {
        lock(&self.current).clone()
    }

    fn running(&self) -> bool {
        lock(&self.current)
            .as_ref()
            .is_some_and(|job| !job.phase.over())
    }

    fn stopped(&self) -> bool {
        self.cancel.load(Ordering::SeqCst) || self.closing.load(Ordering::SeqCst)
    }

    /// Ask the running job to stop. What it stored stays.
    pub(crate) fn cancel(&self) {
        if self.running() {
            self.cancel.store(true, Ordering::SeqCst);
        }
    }

    /// The backend stops: end the job at its next step.
    pub(crate) fn close(&self) {
        self.closing.store(true, Ordering::SeqCst);
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// One line for `GET /v1/status`, where the tray reads it.
    pub(crate) fn status(&self) -> Value {
        match self.snapshot() {
            Some(job) if !job.phase.over() => json!({
                "phase": job.phase,
                "model": job.model,
                "repo": job.repo,
            }),
            _ => Value::Null,
        }
    }
}

/// Start a job in the background and answer with it as it starts. Must be
/// called where the async runtime is known: a handler, or a blocking task
/// of it.
pub(crate) fn start(state: &AppState) -> Result<Value> {
    let embedder = (state.0.config.host.embedder)();
    let job = state
        .0
        .jobs
        .begin(embedder.as_ref().map(|embedder| embedder.name()))?;
    let state = state.clone();
    tokio::task::spawn_blocking(move || run(&state, embedder));
    Ok(json!({ "job": job }))
}

/// How a job's work ended when it did not fail.
#[derive(PartialEq)]
enum End {
    Done,
    /// Cancelled, or the backend is stopping.
    Stopped,
}

fn run(state: &AppState, embedder: Option<Arc<dyn Embedder>>) {
    let jobs = &state.0.jobs;
    let ended = embed_all(state, embedder);
    if jobs.closing.load(Ordering::SeqCst) {
        // The backend is stopping. The next one finds what waits.
        return;
    }
    match ended {
        Ok(End::Done) => jobs.change(|job| job.phase = Phase::Done),
        Ok(End::Stopped) => jobs.change(|job| job.phase = Phase::Cancelled),
        Err(error) => {
            log::warn!("embedding job failed: {error}");
            let text = match &error {
                Error::Invalid(text) => text.clone(),
                other => other.to_string(),
            };
            jobs.change(|job| {
                job.phase = Phase::Failed;
                job.error = Some(text);
            });
        }
    }
}

/// Why a repository's embedding failed. An index that cannot be read or
/// written is that repository's problem, and the job goes on with the next;
/// an embedder that fails would fail for the next one too.
enum Failure {
    Index(Error),
    Embedder(Error),
}

impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Failure::Index(error)
    }
}

/// Embed one repository's documents. `Ok(false)`: the job was stopped.
fn embed_repo(
    state: &AppState,
    embedder: &Arc<dyn Embedder>,
    repo: &repos::RepoEntry,
) -> std::result::Result<bool, Failure> {
    let jobs = &state.0.jobs;
    let config = &state.0.config;
    let root = std::path::Path::new(&repo.path);
    let Some(mut index) = service::open_existing_index_in(&config.data_dir, root)? else {
        // Never indexed: its first update embeds it.
        return Ok(true);
    };
    if !behind(&index, &embedder.name(), &DOCUMENT_KINDS)? {
        return Ok(true);
    }
    loop {
        // An update of a wiki changes the documents under a run. The run
        // then starts over from what the index holds now.
        let updates = state.0.updates.load(Ordering::SeqCst);
        let mut run = {
            let _guard = lock(&state.0.write_lock);
            let documents = index.documents(&DOCUMENT_KINDS)?;
            Run::plan(&mut index, embedder.as_ref(), &DOCUMENT_KINDS, &documents)?
        };
        jobs.change(|job| {
            job.docs_done = 0;
            job.docs_total = run.progress().total;
        });
        let mut overtaken = false;
        while !run.is_done() {
            if jobs.stopped() {
                return Ok(false);
            }
            // The slow part, with no lock held.
            let vectors = embedder.embed(&run.next()).map_err(Failure::Embedder)?;
            let _guard = lock(&state.0.write_lock);
            if state.0.updates.load(Ordering::SeqCst) != updates {
                overtaken = true;
                break;
            }
            run.store(&mut index, vectors)?;
            let done = run.progress().done;
            jobs.change(|job| job.docs_done = done);
        }
        if overtaken {
            continue;
        }
        let _guard = lock(&state.0.write_lock);
        run.finish(&index)?;
        return Ok(true);
    }
}

/// Embed the documents of every repository that waits, one after the other.
fn embed_all(state: &AppState, embedder: Option<Arc<dyn Embedder>>) -> Result<End> {
    let jobs = &state.0.jobs;
    let config = &state.0.config;
    let Some(embedder) = embedder else {
        jobs.change(|job| {
            job.note
                .get_or_insert_with(|| "Semantic search is turned off.".to_string());
        });
        return Ok(End::Done);
    };
    for _ in 0..PASSES {
        let updates = state.0.updates.load(Ordering::SeqCst);
        let present: Vec<repos::RepoEntry> = repos::load(&config.data_dir)?
            .into_iter()
            .filter(|repo| std::path::Path::new(&repo.path).is_dir())
            .collect();
        for (index, repo) in present.iter().enumerate() {
            if jobs.stopped() {
                return Ok(End::Stopped);
            }
            jobs.change(|job| {
                job.repo = Some(repo.name.clone());
                job.repo_index = index + 1;
                job.repo_count = present.len();
                job.docs_done = 0;
                job.docs_total = 0;
            });
            let started = Instant::now();
            match embed_repo(state, &embedder, repo) {
                Ok(true) => log::info!(
                    "search vectors of {} are up to date ({} ms)",
                    repo.name,
                    started.elapsed().as_millis()
                ),
                Ok(false) => return Ok(End::Stopped),
                Err(Failure::Index(error)) => {
                    log::warn!("search vectors of {} were left out: {error}", repo.name);
                    jobs.change(|job| {
                        job.skipped
                            .push(json!({ "repo": repo.name, "error": error.to_string() }));
                    });
                }
                Err(Failure::Embedder(error)) => return Err(error),
            }
        }
        // An update that ran meanwhile may have left documents waiting in a
        // repository this pass was already through with.
        if state.0.updates.load(Ordering::SeqCst) == updates {
            break;
        }
    }
    Ok(End::Done)
}

/// What an update of a wiki leaves to do: count it, so that a run under way
/// starts over, and embed in the background what the update left waiting.
pub(crate) fn after_update(state: &AppState, root: &std::path::Path) {
    state.0.updates.fetch_add(1, Ordering::SeqCst);
    let config = &state.0.config;
    let Some(embedder) = (config.host.embedder)() else {
        return;
    };
    let waits = service::open_existing_index_in(&config.data_dir, root)
        .and_then(|index| match index {
            Some(index) => behind(&index, &embedder.name(), &DOCUMENT_KINDS),
            None => Ok(false),
        })
        .unwrap_or(false);
    // A job that runs already picks the repository up in its next pass.
    if waits && !state.0.jobs.running() {
        let _ = start(state);
    }
}

/// At the backend's start, and after a Test that succeeds: go on with what
/// waits, for any embedder. It tries once; a job that fails stays failed.
pub(crate) fn resume(state: &AppState) {
    let config = &state.0.config;
    let Some(embedder) = (config.host.embedder)() else {
        return;
    };
    let name = embedder.name();
    let waits = repos::load(&config.data_dir)
        .unwrap_or_default()
        .iter()
        .any(|repo| {
            service::open_existing_index_in(&config.data_dir, std::path::Path::new(&repo.path))
                .ok()
                .flatten()
                .is_some_and(|index| behind(&index, &name, &DOCUMENT_KINDS).unwrap_or(false))
        });
    if waits {
        log::info!("documents wait for their search vectors; embedding in the background");
        let _ = start(state);
    }
}
