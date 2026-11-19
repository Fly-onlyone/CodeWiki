//! The built-in embedding models, as the model routes see them: what there
//! is, what is installed, and the one long job that may run.
//!
//! A job installs a model, switches the embedder to it, and embeds the
//! documents of every repository again; or does only the later steps. It
//! runs in the background and reports how far it is in `GET /v1/models`.
//! Nothing is kept on disk about it: a backend that starts looks at the
//! indexes instead, and goes on where documents still wait for a vector.
//!
//! Locks: embedding takes minutes and holds none. The write lock is taken
//! for each batch's write alone, so a note can be saved and a wiki updated
//! while a job runs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cw_core::settings::{self, Source};
use cw_core::{Error, Result};
use cw_embed::Embedder;
use cw_models::{Entry, Hardware, Store, catalog};
use cw_wiki::semantic::{Run, behind};
use cw_wiki::service::{self, DOCUMENT_KINDS};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{AppState, repos};

/// How old a reading of the machine may be before it is taken again. A page
/// asks every second while a job runs.
const HARDWARE_AGE: Duration = Duration::from_secs(60);
/// How often a job looks over the repositories again when updates ran while
/// it worked.
const PASSES: usize = 3;

/// What a job does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    /// Download a model, switch to it, embed again.
    Install,
    /// Switch to an installed model, embed again.
    Use,
    /// Embed again with the embedder in use.
    Reembed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Phase {
    Downloading,
    Switching,
    Embedding,
    /// A game is running; the job goes on by itself when it ends.
    Paused,
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
    kind: Kind,
    phase: Phase,
    /// Catalog id of the model the job is about.
    model: Option<String>,
    bytes_done: u64,
    bytes_total: u64,
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
    /// Stops the job at its next chunk or batch.
    cancel: AtomicBool,
    /// The backend is stopping: the job ends without a word.
    closing: AtomicBool,
    /// A paused job sleeps on this, so that a cancel wakes it at once.
    gate: Mutex<()>,
    wake: Condvar,
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
            gate: Mutex::new(()),
            wake: Condvar::new(),
        }
    }
}

impl Jobs {
    /// Take the place of the running job, or say that one runs.
    fn begin(&self, kind: Kind, model: Option<String>) -> Result<Job> {
        let mut current = lock(&self.current);
        if current.as_ref().is_some_and(|job| !job.phase.over()) {
            return Err(Error::invalid(
                "another model job is running; wait for it or cancel it",
            ));
        }
        self.cancel.store(false, Ordering::SeqCst);
        let job = Job {
            kind,
            phase: match kind {
                Kind::Install => Phase::Downloading,
                Kind::Use => Phase::Switching,
                Kind::Reembed => Phase::Embedding,
            },
            model,
            bytes_done: 0,
            bytes_total: 0,
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
            self.wake.notify_all();
        }
    }

    /// The backend stops: end the job at its next step, also a paused one.
    pub(crate) fn close(&self) {
        self.closing.store(true, Ordering::SeqCst);
        self.cancel.store(true, Ordering::SeqCst);
        self.wake.notify_all();
    }

    /// Sleep, unless a cancel or the backend's stop comes first.
    fn sleep(&self, time: Duration) {
        let gate = lock(&self.gate);
        if !self.stopped() {
            let _ = self.wake.wait_timeout(gate, time);
        }
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

/// The machine as last read, with the time of the reading.
#[derive(Default)]
pub(crate) struct HardwareCache(Mutex<Option<(Instant, Hardware)>>);

fn hardware(state: &AppState) -> Hardware {
    let config = &state.0.config;
    let mut cached = lock(&state.0.hardware.0);
    if let Some((read, hardware)) = cached.as_ref()
        && read.elapsed() < HARDWARE_AGE
    {
        return hardware.clone();
    }
    let hardware = (config.host.hardware)(&config.data_dir);
    *cached = Some((Instant::now(), hardware.clone()));
    hardware
}

fn store(state: &AppState) -> Store {
    let config = &state.0.config;
    Store::new(
        &config.data_dir,
        (config.env)(cw_models::store::BASE_ENV).as_deref(),
    )
}

/// The id of the built-in model the settings name, whether it works or not.
fn chosen(state: &AppState) -> Option<String> {
    let resolved = crate::settings::resolved(state);
    (resolved.get(settings::EMBED_PROVIDER.env).as_deref() == Some("builtin"))
        .then(|| resolved.get(settings::EMBED_MODEL.env))
        .flatten()
}

/// The body of `GET /v1/models`.
pub(crate) fn describe(state: &AppState) -> Value {
    if !cw_models::ENGINE {
        return json!({
            "available": false,
            "reason": "This build cannot run a built-in model; it runs on Windows.",
        });
    }
    let hardware = hardware(state);
    let cuda = cw_models::worker::cuda_program().is_some();
    let store = store(state);
    let chosen = chosen(state);
    let models: Vec<Value> = catalog::all()
        .iter()
        .map(|entry| {
            json!({
                "id": entry.id,
                "name": entry.name,
                "tier": entry.tier,
                "languages": entry.languages,
                "license": entry.license,
                "dimensions": entry.dimensions,
                "size": entry.size(),
                "installed": store.is_installed(entry),
                "active": chosen.as_deref() == Some(entry.id.as_str()),
            })
        })
        .collect();
    json!({
        "available": true,
        "hardware": hardware,
        "cuda": cuda,
        "recommended": cw_models::recommend(&hardware, cuda),
        // The embedder in use, by its provider. A page offers a model only
        // to someone who has chosen no other way to search by meaning.
        "embedder": crate::settings::resolved(state).get(settings::EMBED_PROVIDER.env),
        "models": models,
        "job": state.0.jobs.snapshot(),
        "worker": cw_models::worker::global().status(),
    })
}

/// Why a model route does nothing on this backend; `None` when it works.
pub(crate) fn unavailable() -> Option<Error> {
    (!cw_models::ENGINE)
        .then(|| Error::invalid("this build cannot run a built-in model; it runs on Windows"))
}

fn entry(id: &str) -> Result<&'static Entry> {
    catalog::find(id).ok_or_else(|| Error::not_found(format!("built-in model '{id}'")))
}

/// Start a job in the background and answer with it as it starts. Must be
/// called where the async runtime is known: a handler, or a blocking task
/// of it.
pub(crate) fn start(state: &AppState, kind: Kind, model: Option<&str>) -> Result<Value> {
    if let Some(error) = unavailable() {
        return Err(error);
    }
    let entry = model.map(entry).transpose()?;
    if kind == Kind::Use
        && let Some(entry) = entry
        && !store(state).is_installed(entry)
    {
        return Err(Error::invalid(format!("{} is not installed", entry.name)));
    }
    let job = state
        .0
        .jobs
        .begin(kind, entry.map(|entry| entry.id.clone()))?;
    let state = state.clone();
    tokio::task::spawn_blocking(move || run(&state, kind, entry));
    Ok(json!({ "job": job }))
}

/// How a job's work ended when it did not fail.
#[derive(PartialEq)]
enum End {
    Done,
    /// Cancelled, or the backend is stopping.
    Stopped,
}

fn run(state: &AppState, kind: Kind, entry: Option<&'static Entry>) {
    let jobs = &state.0.jobs;
    let ended = work(state, kind, entry);
    if jobs.closing.load(Ordering::SeqCst) {
        // The backend is stopping. The next one finds what waits.
        return;
    }
    match ended {
        Ok(End::Done) => jobs.change(|job| job.phase = Phase::Done),
        Ok(End::Stopped) => jobs.change(|job| job.phase = Phase::Cancelled),
        Err(error) => {
            log::warn!("model job failed: {error}");
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

fn work(state: &AppState, kind: Kind, entry: Option<&'static Entry>) -> Result<End> {
    let jobs = &state.0.jobs;
    if kind == Kind::Install {
        let entry = entry.ok_or_else(|| Error::invalid("an install names a model"))?;
        log::info!("installing the model {}", entry.id);
        let outcome = store(state).install(
            entry,
            &mut |done, total| {
                jobs.change(|job| {
                    job.bytes_done = done;
                    job.bytes_total = total;
                });
            },
            &jobs.cancel,
        )?;
        if outcome == cw_models::Outcome::Cancelled {
            return Ok(End::Stopped);
        }
    }
    if let Some(entry) = entry {
        jobs.change(|job| job.phase = Phase::Switching);
        switch(state, entry)?;
    }
    embed_all(state)
}

/// Make `entry` the embedder in use. Where the environment names the
/// embedder, the saved settings have no say: the job says so and goes on
/// with the embedder the environment names.
fn switch(state: &AppState, entry: &Entry) -> Result<()> {
    let resolved = crate::settings::resolved(state);
    let by_environment = [settings::EMBED_PROVIDER, settings::EMBED_MODEL]
        .into_iter()
        .find(|key| resolved.source(*key) == Source::Env);
    if let Some(key) = by_environment {
        state.0.jobs.change(|job| {
            job.note = Some(format!(
                "The embedder is set by the environment variable {}, so this model is installed and not in use.",
                key.env
            ));
        });
        // An embedder that named this model before it was installed could
        // not be built; it can now.
        cw_wiki::semantic::configure_in(&resolved, &state.0.config.data_dir);
        return Ok(());
    }
    crate::settings::change(
        state,
        &json!({ "embed": { "provider": "builtin", "model": entry.id } }),
    )?;
    log::info!("the embedder is now the built-in model {}", entry.id);
    Ok(())
}

/// Wait while a game runs. Returns false when the job was stopped meanwhile.
fn wait_out_game(state: &AppState, embedder: &dyn Embedder) -> bool {
    let jobs = &state.0.jobs;
    let host = &state.0.config.host;
    let mut paused = false;
    while (host.game)() {
        if jobs.stopped() {
            return false;
        }
        if !paused {
            paused = true;
            jobs.change(|job| job.phase = Phase::Paused);
            log::info!("embedding paused while a game runs");
            // The game gets the memory back, not only the processor.
            if embedder.name().starts_with("builtin:") {
                cw_models::worker::global().stop();
            }
        }
        jobs.sleep(host.pause_poll);
    }
    if paused {
        jobs.change(|job| job.phase = Phase::Embedding);
        log::info!("embedding goes on");
    }
    !jobs.stopped()
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
            if !wait_out_game(state, embedder.as_ref()) {
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
fn embed_all(state: &AppState) -> Result<End> {
    let jobs = &state.0.jobs;
    let config = &state.0.config;
    let Some(embedder) = (config.host.embedder)() else {
        jobs.change(|job| {
            job.note
                .get_or_insert_with(|| "Semantic search is turned off.".to_string());
        });
        return Ok(End::Done);
    };
    jobs.change(|job| job.phase = Phase::Embedding);
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

/// Delete a model. One that is in use first hands semantic search back to
/// the embedder that needs no model.
pub(crate) fn remove(state: &AppState, id: &str) -> Result<Value> {
    if let Some(error) = unavailable() {
        return Err(error);
    }
    let entry = entry(id)?;
    if state.0.jobs.running() {
        return Err(Error::invalid(
            "a model job is running; wait for it or cancel it",
        ));
    }
    if chosen(state).as_deref() == Some(id) {
        crate::settings::change(
            state,
            &json!({ "embed": { "provider": "local", "model": null } }),
        )?;
        // Windows deletes no file a program holds open.
        cw_models::worker::global().stop();
    }
    store(state).remove(entry)?;
    log::info!("removed the model {id}");
    Ok(describe(state))
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
    if waits && unavailable().is_none() && !state.0.jobs.running() {
        let _ = start(state, Kind::Reembed, None);
    }
}

/// At the backend's start: go on with what a job cut short left waiting.
/// Only for a built-in model, whose embedding is what takes long.
pub(crate) fn resume(state: &AppState) {
    let config = &state.0.config;
    let Some(embedder) = (config.host.embedder)() else {
        return;
    };
    let name = embedder.name();
    if !name.starts_with("builtin:") {
        return;
    }
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
        let _ = start(state, Kind::Reembed, None);
    }
}
