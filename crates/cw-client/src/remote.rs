//! A backend on another machine, or in Docker, that the user chose in the
//! app instead of naming it with `CODEWIKI_URL`.
//!
//! Its address and token are in `remote.json` in the data directory. Only
//! clients write that file: the backend keeps its own settings in another
//! one, so neither can overwrite the other's change. The token is stored
//! encrypted for the current user (`cw_core::secret`).

use std::path::{Path, PathBuf};

use cw_core::secret;
use serde_json::{Value, json};

/// The file's name in the data directory.
pub(crate) const FILE: &str = "remote.json";

/// The saved backend, without its token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    /// Its address, such as `http://nas:7411`.
    pub url: String,
    /// Whether clients connect to it. False once the user went back to the
    /// backend on this machine; the address and token are kept for the next
    /// time.
    pub in_use: bool,
    /// The last characters of the saved token; empty for a short one.
    pub token_hint: String,
}

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}

fn read(data_dir: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(file(data_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

/// The new file replaces the old one in one step, so a reader never sees
/// half of it.
fn write(data_dir: &Path, value: &Value) -> Result<(), String> {
    let failed = |e: std::io::Error| format!("could not save the backend's address: {e}");
    std::fs::create_dir_all(data_dir).map_err(failed)?;
    let draft = data_dir.join(format!("{FILE}.tmp"));
    std::fs::write(&draft, format!("{value:#}\n")).map_err(failed)?;
    std::fs::rename(&draft, file(data_dir)).map_err(failed)
}

/// The saved backend, when there is one.
pub(crate) fn saved_in(data_dir: &Path) -> Option<Remote> {
    let saved = read(data_dir)?;
    let url = saved["url"].as_str()?.trim();
    (!url.is_empty()).then(|| Remote {
        url: url.to_string(),
        in_use: saved["use"].as_bool().unwrap_or(false),
        token_hint: saved["token"]["hint"].as_str().unwrap_or("").to_string(),
    })
}

/// The saved backend's token.
pub(crate) fn token_in(data_dir: &Path) -> Result<String, String> {
    let saved = read(data_dir).unwrap_or_default();
    let protected = saved["token"]["dpapi"]
        .as_str()
        .ok_or("no token is saved for the backend")?;
    secret::unprotect(protected).map_err(|_| {
        "the saved token cannot be read by this user on this machine; enter it again".to_string()
    })
}

/// An address as it is saved and called: with its scheme, without a slash at
/// the end.
pub(crate) fn normalized(url: &str) -> Result<String, String> {
    let url = url.trim().trim_end_matches('/');
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(url.to_string())
    } else {
        Err("the address must start with http:// or https://".to_string())
    }
}

/// Save a backend and make clients use it. `token: None` keeps the token
/// saved for the same address.
pub(crate) fn save_in(data_dir: &Path, url: &str, token: Option<&str>) -> Result<(), String> {
    let url = normalized(url)?;
    let token = match token.map(str::trim).filter(|token| !token.is_empty()) {
        Some(token) => {
            let protected = secret::protect(token).map_err(|_| {
                "this system cannot save the token; set CODEWIKI_URL and CODEWIKI_TOKEN instead"
                    .to_string()
            })?;
            json!({ "dpapi": protected, "hint": secret::hint(token) })
        }
        None => {
            let saved = read(data_dir).unwrap_or_default();
            // A token belongs to one backend: never send it to another.
            if saved["url"].as_str() != Some(url.as_str()) || !saved["token"]["dpapi"].is_string() {
                return Err("enter the token of the backend".to_string());
            }
            saved["token"].clone()
        }
    };
    write(
        data_dir,
        &json!({ "url": url, "use": true, "token": token }),
    )
}

/// Make clients use the saved backend, or the one on this machine. Nothing
/// saved is the same as not using it.
pub(crate) fn set_in_use_in(data_dir: &Path, in_use: bool) -> Result<(), String> {
    let Some(mut saved) = read(data_dir).filter(Value::is_object) else {
        return if in_use {
            Err("no backend is saved".to_string())
        } else {
            Ok(())
        };
    };
    saved["use"] = json!(in_use);
    write(data_dir, &saved)
}

/// Forget the saved backend and its token.
pub(crate) fn forget_in(data_dir: &Path) -> Result<(), String> {
    match std::fs::remove_file(file(data_dir)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("could not forget the backend's address: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "codewiki-remote-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_address_needs_its_scheme_and_loses_its_last_slash() {
        assert_eq!(
            normalized(" http://nas:7411/ ").as_deref(),
            Ok("http://nas:7411")
        );
        assert_eq!(
            normalized("https://wiki.example").as_deref(),
            Ok("https://wiki.example")
        );
        assert!(normalized("nas:7411").is_err());
        assert!(normalized("").is_err());
    }

    #[test]
    fn nothing_saved_means_the_backend_on_this_machine() {
        let dir = temp_dir("empty");
        assert_eq!(saved_in(&dir), None);
        assert!(token_in(&dir).is_err());
        // Going back to this machine's backend needs no file.
        assert_eq!(set_in_use_in(&dir, false), Ok(()));
        assert!(set_in_use_in(&dir, true).is_err());
        assert_eq!(forget_in(&dir), Ok(()));
        // A first save needs a token.
        assert!(save_in(&dir, "http://nas:7411", None).is_err());
        assert!(save_in(&dir, "http://nas:7411", Some("  ")).is_err());
        assert_eq!(saved_in(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_saved_backend_keeps_its_token_encrypted() {
        // Made up: not a real token.
        const TOKEN: &str = "sample-token-for-tests-0042";
        let dir = temp_dir("saved");
        save_in(&dir, "http://nas:7411/", Some(TOKEN)).unwrap();
        assert_eq!(
            saved_in(&dir),
            Some(Remote {
                url: "http://nas:7411".into(),
                in_use: true,
                token_hint: "0042".into(),
            })
        );
        assert_eq!(token_in(&dir).as_deref(), Ok(TOKEN));
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!text.contains(TOKEN));
        assert!(!dir.join(format!("{FILE}.tmp")).exists());

        // Back to this machine's backend: the address and token stay.
        set_in_use_in(&dir, false).unwrap();
        let saved = saved_in(&dir).unwrap();
        assert!(!saved.in_use);
        assert_eq!(saved.url, "http://nas:7411");
        assert_eq!(token_in(&dir).as_deref(), Ok(TOKEN));

        // The same address again needs no token; another address does.
        save_in(&dir, "http://nas:7411", None).unwrap();
        assert!(saved_in(&dir).unwrap().in_use);
        assert_eq!(token_in(&dir).as_deref(), Ok(TOKEN));
        assert!(save_in(&dir, "http://other:7411", None).is_err());
        assert_eq!(saved_in(&dir).unwrap().url, "http://nas:7411");

        forget_in(&dir).unwrap();
        assert_eq!(saved_in(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
