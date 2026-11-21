//! What the settings routes do: describe the settings in effect, and change
//! the saved ones.
//!
//! A description never holds a secret: a key is reported as unset, saved,
//! given by the environment, or unreadable, with the last characters of a
//! saved one. A setting whose environment variable is set cannot be changed
//! here; the variable wins, so a change would have no effect.

use std::time::Instant;

use cw_core::settings::{self, Key, Resolved, Settings, Source};
use cw_core::{Error, Result, secret};
use serde_json::{Value, json};

use crate::AppState;

/// Settings that decide which embedder semantic search uses.
const EMBEDDER_KEYS: [Key; 12] = [
    settings::EMBED_PROVIDER,
    settings::EMBED_MODEL,
    settings::OPENAI_API_KEY,
    settings::OPENAI_BASE_URL,
    settings::OLLAMA_HOST,
    settings::LMSTUDIO_URL,
    settings::LMSTUDIO_KEY,
    settings::LLAMACPP_URL,
    settings::LLAMACPP_KEY,
    settings::LEMONADE_URL,
    settings::LEMONADE_KEY,
    settings::MISTRALRS_URL,
];

/// Settings that hold the address of a server.
const ADDRESS_KEYS: [Key; 6] = [
    settings::OPENAI_BASE_URL,
    settings::OLLAMA_HOST,
    settings::LMSTUDIO_URL,
    settings::LLAMACPP_URL,
    settings::LEMONADE_URL,
    settings::MISTRALRS_URL,
];

/// The settings in effect for this server.
pub(crate) fn resolved(state: &AppState) -> Resolved {
    let config = &state.0.config;
    Resolved::from_parts(&settings::load(&config.data_dir), config.env)
}

/// An error's text for a person: without the prefix that tells kinds of
/// error apart.
fn message(error: &Error) -> String {
    match error {
        Error::Invalid(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The name of what the settings select, or why they select nothing usable.
fn selected<T: ?Sized>(
    built: Result<Option<Box<T>>>,
    name: impl Fn(&T) -> String,
) -> (Option<String>, Option<String>) {
    match built {
        Ok(chosen) => (chosen.map(|c| name(c.as_ref())), None),
        Err(error) => (None, Some(message(&error))),
    }
}

/// Every setting with its value in effect and where that comes from.
pub(crate) fn describe(state: &AppState) -> Value {
    let resolved = resolved(state);
    let mut out = json!({
        "secrets": if secret::available() { "dpapi" } else { "env_only" },
        "writable": state.0.config.settings_writable,
    });
    for key in settings::ALL {
        out[key.section()][key.field()] = if key.secret {
            let state = resolved.secret_state(key);
            json!({ "state": state.as_str(), "hint": state.hint(), "env": key.env })
        } else {
            json!({
                "value": resolved.get(key.env),
                "source": resolved.source(key).as_str(),
                "env": key.env,
                "default": key.default,
            })
        };
    }
    out["embed"]["provider"]["options"] = json!(cw_embed::PROVIDERS);
    let (name, problem) = selected(cw_embed::from_settings(&resolved), |e| e.name());
    out["embed"]["name"] = json!(name);
    out["embed"]["problem"] = json!(problem);
    out
}

/// What a test of a provider found: `ok`, the provider's name, and either
/// how long it took (with `extra`, what the answer showed) or its error.
fn test_result(name: Option<String>, outcome: Result<Value>, started: Instant) -> Value {
    match outcome {
        Ok(extra) => {
            let mut result = json!({
                "ok": true,
                "name": name,
                "ms": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            });
            if let (Some(result), Some(extra)) = (result.as_object_mut(), extra.as_object()) {
                result.extend(extra.clone());
            }
            result
        }
        Err(error) => json!({ "ok": false, "name": name, "error": message(&error) }),
    }
}

/// Have the chosen embedder embed one word. On success `dimensions` is the
/// length of its vectors, and documents that still wait for a vector are
/// embedded again: a job that failed stays failed until something tries, and
/// a Test that succeeds is that try. Called in a blocking task of a handler.
pub(crate) fn test_embedder(state: &AppState) -> Value {
    let started = Instant::now();
    match cw_embed::from_settings(&resolved(state)) {
        Ok(Some(embedder)) => {
            let probed = embedder.probe();
            let answered = probed.is_ok();
            let result = test_result(
                Some(embedder.name()),
                probed.map(|dimensions| json!({ "dimensions": dimensions })),
                started,
            );
            if answered {
                // Starts a job only when a repository is behind and none runs.
                crate::embed_job::resume(state);
            }
            result
        }
        Ok(None) => test_result(
            None,
            Err(Error::invalid("semantic search is turned off")),
            started,
        ),
        Err(error) => test_result(None, Err(error), started),
    }
}

/// The models the chosen provider offers, for a page that lets a person pick
/// one. `models` is null when the provider lists none; `reachable` is false
/// when the server did not answer, and `problem` says why.
pub(crate) fn embed_models(state: &AppState) -> Value {
    let resolved = resolved(state);
    let provider = resolved.get(settings::EMBED_PROVIDER.env);
    match cw_embed::list_models_in(&resolved) {
        Ok(models) => json!({
            "provider": provider,
            "reachable": true,
            "models": models.map(|models| models
                .into_iter()
                .map(|m| json!({ "id": m.id, "embedding": m.embedding, "loaded": m.loaded }))
                .collect::<Vec<_>>()),
            "problem": null,
        }),
        Err(error) => json!({
            "provider": provider,
            "reachable": false,
            "models": null,
            "problem": message(&error),
        }),
    }
}

/// A value that passed the checks of its setting, as it is saved.
fn checked(key: Key, value: &str) -> Result<String> {
    let value = value.trim();
    let one_of = |options: &[&str], what: &str| {
        let lower = value.to_lowercase();
        if options.contains(&lower.as_str()) {
            Ok(lower)
        } else {
            Err(Error::invalid(format!(
                "unknown {what} '{value}'; use {}",
                options.join(", ")
            )))
        }
    };
    if key == settings::EMBED_PROVIDER {
        one_of(&cw_embed::PROVIDERS, "embedder")
    } else if ADDRESS_KEYS.contains(&key) {
        if value.starts_with("http://") || value.starts_with("https://") {
            Ok(value.trim_end_matches('/').to_string())
        } else {
            Err(Error::invalid(format!(
                "{} must be an address that starts with http:// or https://",
                key.path()
            )))
        }
    } else {
        Ok(value.to_string())
    }
}

/// Apply one field of a change. `null` clears a setting. A plain setting
/// takes text; a secret takes `{"set": "<secret>"}` or `{"remove": true}`.
fn apply(saved: &mut Settings, key: Key, value: &Value) -> Result<()> {
    if value.is_null() {
        saved.clear(key);
        return Ok(());
    }
    if !key.secret {
        let text = value
            .as_str()
            .ok_or_else(|| Error::invalid(format!("{} must be text or null", key.path())))?;
        if text.trim().is_empty() {
            saved.clear(key);
            return Ok(());
        }
        return saved.set(key, &checked(key, text)?);
    }
    if value["remove"] == true {
        saved.clear(key);
        return Ok(());
    }
    let plain = value["set"]
        .as_str()
        .map(str::trim)
        .filter(|plain| !plain.is_empty())
        .ok_or_else(|| {
            Error::invalid(format!(
                "{} takes {{\"set\": \"<secret>\"}} or {{\"remove\": true}}",
                key.path()
            ))
        })?;
    if !secret::available() {
        return Err(Error::invalid(format!(
            "this system cannot save a secret; set the environment variable {}",
            key.env
        )));
    }
    saved.set(key, plain)
}

/// Change the saved settings named in `body`, an object of sections holding
/// fields, and return the new description. Fields not named stay as they
/// are. One field that cannot be applied leaves everything unchanged.
pub(crate) fn change(state: &AppState, body: &Value) -> Result<Value> {
    let sections = body
        .as_object()
        .ok_or_else(|| Error::invalid("the settings to change must be a JSON object"))?;
    let _guard = state
        .0
        .settings_lock
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let before = resolved(state);
    let config = &state.0.config;
    let mut embedder_changed = false;
    let mut model_named = false;
    settings::update(&config.data_dir, |saved| {
        for (section, fields) in sections {
            let fields = fields.as_object().ok_or_else(|| {
                Error::invalid(format!("'{section}' must be an object of settings"))
            })?;
            for (field, value) in fields {
                let key = settings::key_at(section, field).ok_or_else(|| {
                    Error::invalid(format!("unknown setting '{section}.{field}'"))
                })?;
                if before.source(key) == Source::Env {
                    return Err(Error::invalid(format!(
                        "{} is set by the environment variable {}; change or remove the variable",
                        key.path(),
                        key.env
                    )));
                }
                apply(saved, key, value)?;
                embedder_changed |= EMBEDDER_KEYS.contains(&key);
                model_named |= key == settings::EMBED_MODEL;
            }
        }
        if embedder_changed {
            // A model name belongs to one provider: another provider does
            // not serve it. A change of provider that names no model
            // clears it, so that no server is asked for a model it does
            // not have. A model the environment gives stays.
            let provider = |settings: &Resolved| settings.get(settings::EMBED_PROVIDER.env);
            let after = Resolved::from_parts(saved, config.env);
            if provider(&before) != provider(&after)
                && !model_named
                && before.source(settings::EMBED_MODEL) != Source::Env
            {
                saved.clear(settings::EMBED_MODEL);
            }
        }
        Ok(())
    })?;
    if embedder_changed {
        cw_wiki::semantic::configure(&resolved(state));
    }
    Ok(describe(state))
}
