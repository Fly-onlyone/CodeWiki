//! What the CLIProxyAPI route does: describe the proxy the settings name.
//!
//! The user runs the proxy and manages its accounts in it, through its own
//! web panel or its `-claude-login` and `-codex-login` flags. This backend
//! only asks whether it answers and which models its accounts serve, with
//! the client key. A backend that listens beyond its machine answers no
//! such route, as with the settings.

use cw_core::Error;
use cw_core::settings::{self, Resolved};
use cw_llm::cliproxy;
use serde_json::{Value, json};

use crate::AppState;
use crate::settings::resolved;

fn url(resolved: &Resolved) -> String {
    resolved
        .get(settings::CLIPROXY_URL.env)
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string()
}

/// An error's text for a person.
fn message(error: &Error) -> String {
    match error {
        Error::Invalid(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The proxy as the settings name it: whether it answers, the models its
/// accounts serve, and where its own panel is. A part that cannot be read
/// says why in its own error field.
pub(crate) fn describe(state: &AppState) -> Value {
    let resolved = resolved(state);
    let url = url(&resolved);
    let mut out = json!({
        "url": url,
        "panel": cliproxy::panel(&url),
        "reachable": false,
        "models": [],
        "error": null,
        "models_error": null,
    });
    if !cliproxy::reachable(&url) {
        out["error"] = json!(format!(
            "CLIProxyAPI not reachable at {url}: start it, or check the address"
        ));
        return out;
    }
    out["reachable"] = json!(true);
    match resolved.get(settings::CLIPROXY_API_KEY.env) {
        None => {
            out["models_error"] =
                json!("save a client key: one of access.api-keys in the proxy's config.yaml")
        }
        Some(key) => match cliproxy::models(&url, &key) {
            Ok(models) => out["models"] = json!(models),
            Err(error) => out["models_error"] = json!(message(&error)),
        },
    }
    out
}
