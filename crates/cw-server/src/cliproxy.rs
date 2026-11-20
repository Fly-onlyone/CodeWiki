//! What the CLIProxyAPI routes do: describe the proxy the settings name, and
//! add, watch and remove the subscription accounts it rotates.
//!
//! The user runs the proxy; this backend only talks to it, with the client
//! key for models and the management key for accounts. A backend that
//! listens beyond its machine answers none of these routes, as with the
//! settings: someone holding its token could otherwise manage the accounts.

use cw_core::settings::{self, Resolved};
use cw_core::{Error, Result};
use cw_llm::cliproxy::{self, LoginStatus, Management};
use serde_json::{Value, json};

use crate::AppState;
use crate::settings::resolved;

/// The vendors whose accounts can be added from CodeWiki.
pub(crate) const VENDORS: [&str; 2] = ["claude", "codex"];

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

fn management(resolved: &Resolved) -> Result<Management> {
    let key = resolved
        .get(settings::CLIPROXY_MANAGEMENT_KEY.env)
        .ok_or_else(|| {
            Error::invalid(
                "save the proxy's management key: management.secret-key in its config.yaml",
            )
        })?;
    Ok(Management {
        base_url: url(resolved),
        key,
    })
}

/// The proxy as the settings name it: whether it answers, whether it is on
/// this machine, the models it can serve and the accounts it holds. A part
/// that cannot be read says why in its own error field.
pub(crate) fn describe(state: &AppState) -> Value {
    let resolved = resolved(state);
    let url = url(&resolved);
    let mut out = json!({
        "url": url,
        "loopback": cliproxy::is_loopback(&url),
        "reachable": false,
        "models": [],
        "accounts": [],
        "error": null,
        "models_error": null,
        "accounts_error": null,
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
    match management(&resolved).and_then(|m| m.accounts()) {
        Ok(accounts) => out["accounts"] = json!(accounts),
        Err(error) => out["accounts_error"] = json!(message(&error)),
    }
    out
}

/// Start signing in an account of `vendor`: the page the user opens, and the
/// state that names the sign-in.
pub(crate) fn add(state: &AppState, vendor: &str) -> Result<Value> {
    let vendor = vendor.trim().to_lowercase();
    if !VENDORS.contains(&vendor.as_str()) {
        return Err(Error::invalid(format!(
            "unknown vendor '{vendor}'; use {}",
            VENDORS.join(" or ")
        )));
    }
    let resolved = resolved(state);
    let url = url(&resolved);
    // The vendor sends the browser back to a port on the proxy's machine.
    if !cliproxy::is_loopback(&url) {
        return Err(Error::invalid(format!(
            "accounts can be added only through a CLIProxyAPI on this machine; {url} is elsewhere, so add them there with its --{vendor}-login"
        )));
    }
    let login = management(&resolved)?.start_login(&vendor)?;
    Ok(json!({ "url": login.url, "state": login.state }))
}

/// How the sign-in named by `login` goes: `wait`, `ok`, or `error` with why.
pub(crate) fn login_status(state: &AppState, login: &str) -> Result<Value> {
    Ok(match management(&resolved(state))?.login_status(login)? {
        LoginStatus::Waiting => json!({ "status": "wait" }),
        LoginStatus::Done => json!({ "status": "ok" }),
        LoginStatus::Failed(error) => json!({ "status": "error", "error": error }),
    })
}

pub(crate) fn cancel_login(state: &AppState, login: &str) -> Result<Value> {
    management(&resolved(state))?.cancel_login(login)?;
    Ok(json!({ "cancelled": true }))
}

pub(crate) fn remove(state: &AppState, name: &str) -> Result<Value> {
    management(&resolved(state))?.remove(name)?;
    Ok(json!({ "removed": name }))
}
