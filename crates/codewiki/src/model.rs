//! `codewiki model`: the built-in embedding models from the command line.
//!
//! These commands work on the models folder of the data directory, as the
//! backend does. They do not choose the embedder: the app does that, or the
//! environment (`CODEWIKI_EMBED=builtin` with `CODEWIKI_EMBED_MODEL`), since
//! only the backend writes the saved settings.

use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use clap::Subcommand;
use cw_core::settings::{self, Resolved};
use cw_core::{Error, Result};
use cw_models::{Device, Entry, Outcome, Store, catalog};
use serde_json::json;

#[derive(Subcommand)]
pub enum ModelAction {
    /// The models there are, and which are installed.
    List,
    /// The model that fits this machine, and why.
    Recommend,
    /// Download a model. A download that was cut short goes on.
    Install {
        /// A model's id, as `codewiki model list` prints it.
        id: String,
    },
    /// Delete an installed model.
    Remove { id: String },
}

const MB: f64 = 1024.0 * 1024.0;

fn megabytes(bytes: u64) -> String {
    format!("{:.0} MB", bytes as f64 / MB)
}

fn store() -> Store {
    Store::new(
        &cw_core::paths::data_dir(),
        settings::process_env(cw_models::store::BASE_ENV).as_deref(),
    )
}

fn entry(id: &str) -> Result<&'static Entry> {
    catalog::find(id).ok_or_else(|| {
        let known: Vec<&str> = catalog::all().iter().map(|e| e.id.as_str()).collect();
        Error::invalid(format!("unknown model '{id}'; use {}", known.join(", ")))
    })
}

pub fn run(action: &ModelAction, as_json: bool) -> Result<()> {
    if !cw_models::ENGINE {
        return Err(Error::invalid(
            "this build cannot run a built-in model; it runs on Windows",
        ));
    }
    match action {
        ModelAction::List => list(as_json),
        ModelAction::Recommend => recommend(as_json),
        ModelAction::Install { id } => install(entry(id)?),
        ModelAction::Remove { id } => remove(entry(id)?),
    }
}

fn list(as_json: bool) -> Result<()> {
    let store = store();
    if as_json {
        let models: Vec<_> = catalog::all()
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
                })
            })
            .collect();
        println!("{}", json!(models));
        return Ok(());
    }
    for entry in catalog::all() {
        println!(
            "{:<26} {:>8}  {:<12} {:<10} {}",
            entry.id,
            megabytes(entry.size()),
            entry.languages,
            entry.license,
            if store.is_installed(entry) {
                "installed"
            } else {
                ""
            }
        );
    }
    Ok(())
}

fn recommend(as_json: bool) -> Result<()> {
    let hardware = cw_models::hardware::probe(&cw_core::paths::data_dir());
    let recommendation =
        cw_models::recommend(&hardware, cw_models::worker::cuda_program().is_some());
    if as_json {
        println!(
            "{}",
            json!({ "hardware": hardware, "recommended": recommendation })
        );
        return Ok(());
    }
    match recommendation.model.as_deref().and_then(catalog::find) {
        Some(entry) => println!(
            "{} ({}, {}) on the {}. {}",
            entry.id,
            entry.name,
            megabytes(entry.size()),
            match recommendation.device {
                Device::Cpu => "processor",
                Device::Cuda => "graphics card",
            },
            recommendation.reason
        ),
        None => println!("No model is recommended. {}", recommendation.reason),
    }
    Ok(())
}

fn install(entry: &Entry) -> Result<()> {
    let store = store();
    if store.is_installed(entry) {
        println!("{} is installed already.", entry.name);
        return Ok(());
    }
    // One line that is written over, a few times a second.
    let mut shown = Instant::now() - Duration::from_secs(1);
    let mut progress = |done: u64, total: u64| {
        if shown.elapsed() < Duration::from_millis(250) && done < total {
            return;
        }
        shown = Instant::now();
        let percent = (done * 100).checked_div(total).unwrap_or(0);
        eprint!(
            "\r{percent:>3}%  {} of {}   ",
            megabytes(done),
            megabytes(total)
        );
        let _ = std::io::stderr().flush();
    };
    let outcome = store.install(entry, &mut progress, &AtomicBool::new(false));
    eprintln!();
    match outcome? {
        Outcome::Installed => {
            println!(
                "Installed {} in {}.",
                entry.name,
                store.dir(entry).display()
            );
            println!(
                "To search with it, choose it in the app under Settings > Semantic search, or set CODEWIKI_EMBED=builtin and CODEWIKI_EMBED_MODEL={}.",
                entry.id
            );
        }
        // Nothing here asks for a stop; the store reports one all the same.
        Outcome::Cancelled => println!("Stopped. What was downloaded is kept."),
    }
    Ok(())
}

fn remove(entry: &Entry) -> Result<()> {
    let data_dir = cw_core::paths::data_dir();
    let resolved = Resolved::load(&data_dir);
    let in_use = resolved.get(settings::EMBED_PROVIDER.env).as_deref() == Some("builtin")
        && resolved.get(settings::EMBED_MODEL.env).as_deref() == Some(entry.id.as_str());
    store().remove(entry)?;
    println!("Removed {}.", entry.name);
    if in_use {
        eprintln!(
            "warning: the settings name this model as the embedder; search is by words alone until another embedder is chosen"
        );
    }
    Ok(())
}
