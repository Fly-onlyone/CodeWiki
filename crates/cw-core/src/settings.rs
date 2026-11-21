//! Settings the backend keeps for its user: which embedder serves semantic
//! search, and the keys and addresses it needs.
//!
//! They live in `settings.json` in the data directory, which only the backend
//! writes. Every setting also has an environment variable, and the variable
//! wins: a setup made with variables (Docker, CI, a test) behaves the same
//! whatever the file says. [`Resolved`] holds the outcome for each setting
//! and says where it came from. API keys are stored encrypted (`secret.rs`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::{Error, Result, secret};

/// The file's name in the data directory.
pub const FILE: &str = "settings.json";
/// Where a file that could not be read is moved before a new one is written.
const BROKEN_FILE: &str = "settings.broken.json";
/// Version of the file's layout.
const VERSION: u64 = 1;

/// One setting: the environment variable that names it everywhere, and where
/// it sits in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    /// The environment variable. Also the name code asks for the value by.
    pub env: &'static str,
    section: &'static str,
    field: &'static str,
    /// Stored encrypted and never shown.
    pub secret: bool,
    /// The value in use when neither the variable nor the file gives one.
    /// `None` where the default depends on another setting, as a model's does
    /// on its provider.
    pub default: Option<&'static str>,
}

impl Key {
    /// The setting's name in the file and over HTTP, as `section.field`.
    pub fn path(&self) -> String {
        format!("{}.{}", self.section, self.field)
    }

    pub fn section(&self) -> &'static str {
        self.section
    }

    pub fn field(&self) -> &'static str {
        self.field
    }
}

pub const EMBED_PROVIDER: Key = Key {
    env: "CODEWIKI_EMBED",
    section: "embed",
    field: "provider",
    secret: false,
    default: Some("local"),
};
pub const EMBED_MODEL: Key = Key {
    env: "CODEWIKI_EMBED_MODEL",
    section: "embed",
    field: "model",
    secret: false,
    default: None,
};
/// What runs a built-in embedding model: `auto` (the graphics card when it
/// can, else the processor), `cpu` or `cuda`.
pub const EMBED_DEVICE: Key = Key {
    env: "CODEWIKI_EMBED_DEVICE",
    section: "embed",
    field: "device",
    secret: false,
    default: Some("auto"),
};
pub const OPENAI_API_KEY: Key = Key {
    env: "OPENAI_API_KEY",
    section: "openai",
    field: "api_key",
    secret: true,
    default: None,
};
pub const OPENAI_BASE_URL: Key = Key {
    env: "OPENAI_BASE_URL",
    section: "openai",
    field: "base_url",
    secret: false,
    default: Some("https://api.openai.com/v1"),
};
pub const OLLAMA_HOST: Key = Key {
    env: "OLLAMA_HOST",
    section: "ollama",
    field: "host",
    secret: false,
    default: Some("http://127.0.0.1:11434"),
};

/// Every setting, in the order a page shows them.
pub const ALL: [Key; 6] = [
    OPENAI_API_KEY,
    OPENAI_BASE_URL,
    OLLAMA_HOST,
    EMBED_PROVIDER,
    EMBED_MODEL,
    EMBED_DEVICE,
];

/// Sections of `settings.json` that held the settings of the AI writer
/// CodeWiki once called itself: its provider and model, the Anthropic key,
/// the CLIProxyAPI address and client key. Nothing reads them any more, and
/// two of them hold an API key, encrypted. They are taken out of the file,
/// since a key no program uses should not lie on a disk.
pub const RETIRED_SECTIONS: [&str; 3] = ["llm", "anthropic", "cliproxy"];
/// The environment variables of those settings. A program that finds one
/// set says so in its log: its user expects an effect that no longer comes.
pub const RETIRED_VARIABLES: [&str; 5] = [
    "CODEWIKI_LLM",
    "CODEWIKI_MODEL",
    "ANTHROPIC_API_KEY",
    "CODEWIKI_CLIPROXY_URL",
    "CODEWIKI_CLIPROXY_KEY",
];

/// The setting an environment variable names.
pub fn key(env: &str) -> Option<Key> {
    ALL.into_iter().find(|key| key.env == env)
}

/// The setting at `section.field`.
pub fn key_at(section: &str, field: &str) -> Option<Key> {
    ALL.into_iter()
        .find(|key| key.section == section && key.field == field)
}

/// A secret as the file holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedSecret {
    /// The encrypted secret (`secret::protect`).
    pub protected: String,
    /// The secret's last characters, kept readable so that a page can say
    /// which key is saved without decrypting anything.
    pub hint: String,
}

/// What `settings.json` holds. Parts it does not know are kept as they are,
/// so an older program does not drop what a newer one wrote.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    document: Map<String, Value>,
}

impl Settings {
    fn field(&self, key: Key) -> Option<&Value> {
        self.document.get(key.section)?.get(key.field)
    }

    /// The saved value of a setting that is not a secret.
    pub fn value(&self, key: Key) -> Option<String> {
        if key.secret {
            return None;
        }
        let text = self.field(key)?.as_str()?.trim();
        (!text.is_empty()).then(|| text.to_string())
    }

    /// The saved form of a secret.
    pub fn secret(&self, key: Key) -> Option<SavedSecret> {
        if !key.secret {
            return None;
        }
        let saved = self.field(key)?;
        let protected = saved["dpapi"].as_str()?.to_string();
        (!protected.is_empty()).then(|| SavedSecret {
            protected,
            hint: saved["hint"].as_str().unwrap_or("").to_string(),
        })
    }

    fn put(&mut self, key: Key, value: Value) {
        let section = self
            .document
            .entry(key.section)
            .or_insert_with(|| json!({}));
        if !section.is_object() {
            *section = json!({});
        }
        section[key.field] = value;
    }

    /// Save a value; a blank one clears the setting. A secret is encrypted
    /// first, which fails on a system that cannot protect one.
    pub fn set(&mut self, key: Key, value: &str) -> Result<()> {
        let value = value.trim();
        if value.is_empty() {
            self.clear(key);
            return Ok(());
        }
        let saved = if key.secret {
            json!({ "dpapi": secret::protect(value)?, "hint": secret::hint(value) })
        } else {
            Value::String(value.to_string())
        };
        self.put(key, saved);
        Ok(())
    }

    /// Forget the saved value of a setting.
    pub fn clear(&mut self, key: Key) {
        let Some(section) = self
            .document
            .get_mut(key.section)
            .and_then(Value::as_object_mut)
        else {
            return;
        };
        section.remove(key.field);
        if section.is_empty() {
            self.document.remove(key.section);
        }
    }
}

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}

/// The file's content, or `None` when it is missing or not a JSON object.
fn read(data_dir: &Path) -> Option<Settings> {
    let text = std::fs::read_to_string(file(data_dir)).ok()?;
    match serde_json::from_str(&text) {
        Ok(Value::Object(document)) => Some(Settings { document }),
        _ => None,
    }
}

/// The saved settings. A missing or unreadable file gives none, so a damaged
/// file never stops the backend.
pub fn load(data_dir: &Path) -> Settings {
    read(data_dir).unwrap_or_default()
}

/// Read the file, apply `change` and write it back. The new file replaces the
/// old one in one step, so a reader never sees half of it. When `change`
/// fails, nothing is written.
pub fn update(
    data_dir: &Path,
    change: impl FnOnce(&mut Settings) -> Result<()>,
) -> Result<Settings> {
    let path = file(data_dir);
    let mut settings = match read(data_dir) {
        Some(settings) => settings,
        None => {
            // Keep a file that could not be read: its owner may want it back.
            if path.exists() {
                let aside = data_dir.join(BROKEN_FILE);
                std::fs::rename(&path, &aside).map_err(|e| Error::io(&aside, e))?;
            }
            Settings::default()
        }
    };
    change(&mut settings)?;
    for section in RETIRED_SECTIONS {
        settings.document.remove(section);
    }
    settings.document.insert("version".into(), json!(VERSION));
    let text = serde_json::to_string_pretty(&Value::Object(settings.document.clone()))
        .map_err(|e| Error::external("writing the settings", e))?;
    std::fs::create_dir_all(data_dir).map_err(|e| Error::io(data_dir, e))?;
    let draft = data_dir.join(format!("{FILE}.tmp"));
    std::fs::write(&draft, text).map_err(|e| Error::io(&draft, e))?;
    std::fs::rename(&draft, &path).map_err(|e| Error::io(&path, e))?;
    Ok(settings)
}

/// Take the settings in [`RETIRED_SECTIONS`] out of the file. Returns
/// whether it held any. A file without them is not written, and neither is
/// one that is missing or cannot be read.
pub fn drop_retired(data_dir: &Path) -> Result<bool> {
    let held = read(data_dir).is_some_and(|settings| {
        RETIRED_SECTIONS
            .iter()
            .any(|section| settings.document.contains_key(*section))
    });
    if held {
        update(data_dir, |_| Ok(()))?;
    }
    Ok(held)
}

/// Where the value of a setting comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The environment variable is set; the saved value is not used.
    Env,
    /// The settings file.
    Saved,
    /// Neither: the default, when the setting has one.
    Default,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Saved => "saved",
            Self::Default => "default",
        }
    }
}

/// What is known about a secret without showing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretState {
    Unset,
    /// The environment variable supplies it.
    Env,
    /// Saved in the file; `hint` is its last characters.
    Saved {
        hint: String,
    },
    /// Saved, but this user or machine cannot decrypt it.
    Unreadable {
        hint: String,
    },
}

impl SecretState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unset => "unset",
            Self::Env => "env",
            Self::Saved { .. } => "saved",
            Self::Unreadable { .. } => "unreadable",
        }
    }

    pub fn hint(&self) -> &str {
        match self {
            Self::Saved { hint } | Self::Unreadable { hint } => hint,
            Self::Unset | Self::Env => "",
        }
    }
}

struct Entry {
    value: Option<String>,
    source: Source,
    secret: SecretState,
}

/// The settings in effect: for each one, the environment variable when it is
/// set, otherwise the saved value, otherwise the default.
///
/// It holds decrypted secrets, so it has no `Debug` and should not outlive
/// the work it was loaded for.
pub struct Resolved {
    entries: HashMap<&'static str, Entry>,
}

/// An environment variable of this process. What [`Resolved::load`] reads
/// the environment with; a test passes its own function in its place.
pub fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

impl Resolved {
    /// The settings in effect for this process: its environment over the file
    /// in `data_dir`.
    pub fn load(data_dir: &Path) -> Self {
        Self::from_parts(&load(data_dir), process_env)
    }

    /// The environment alone, as before there was a settings file.
    pub fn from_env() -> Self {
        Self::from_parts(&Settings::default(), process_env)
    }

    /// Resolve `saved` under the environment `env` reads. A variable that is
    /// blank counts as not set.
    pub fn from_parts(saved: &Settings, env: impl Fn(&str) -> Option<String>) -> Self {
        let entries = ALL
            .into_iter()
            .map(|key| {
                let from_env = env(key.env)
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty());
                let entry = match from_env {
                    Some(value) => Entry {
                        value: Some(value),
                        source: Source::Env,
                        secret: if key.secret {
                            SecretState::Env
                        } else {
                            SecretState::Unset
                        },
                    },
                    None if key.secret => Self::saved_secret(saved, key),
                    None => match saved.value(key) {
                        Some(value) => Entry {
                            value: Some(value),
                            source: Source::Saved,
                            secret: SecretState::Unset,
                        },
                        None => Entry {
                            value: key.default.map(str::to_string),
                            source: Source::Default,
                            secret: SecretState::Unset,
                        },
                    },
                };
                (key.env, entry)
            })
            .collect();
        Self { entries }
    }

    fn saved_secret(saved: &Settings, key: Key) -> Entry {
        let Some(stored) = saved.secret(key) else {
            return Entry {
                value: None,
                source: Source::Default,
                secret: SecretState::Unset,
            };
        };
        match secret::unprotect(&stored.protected) {
            Ok(plain) => Entry {
                value: Some(plain),
                source: Source::Saved,
                secret: SecretState::Saved { hint: stored.hint },
            },
            Err(_) => Entry {
                value: None,
                source: Source::Saved,
                secret: SecretState::Unreadable { hint: stored.hint },
            },
        }
    }

    /// The value in effect for the setting an environment variable names.
    /// `None` for a name that is not a setting, and for a setting with
    /// neither a value nor a default.
    pub fn get(&self, env: &str) -> Option<String> {
        self.entries.get(env)?.value.clone()
    }

    /// Where the value of `key` comes from.
    pub fn source(&self, key: Key) -> Source {
        self.entries
            .get(key.env)
            .map_or(Source::Default, |entry| entry.source)
    }

    /// What is known about the secret `key`.
    pub fn secret_state(&self, key: Key) -> SecretState {
        self.entries
            .get(key.env)
            .map_or(SecretState::Unset, |entry| entry.secret.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Made-up values: nothing here is a real key.
    const SAMPLE_KEY: &str = "sample-key-for-tests-0042";

    fn no_env(_: &str) -> Option<String> {
        None
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "codewiki-settings-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn every_setting_has_its_own_variable_and_place() {
        for (index, key) in ALL.iter().enumerate() {
            assert_eq!(super::key(key.env), Some(*key));
            assert_eq!(key_at(key.section, key.field), Some(*key));
            for other in &ALL[index + 1..] {
                assert_ne!(key.env, other.env);
                assert_ne!(key.path(), other.path());
            }
        }
        assert_eq!(super::key("PATH"), None);
        assert_eq!(EMBED_PROVIDER.path(), "embed.provider");
    }

    #[test]
    fn the_variable_wins_then_the_file_then_the_default() {
        let mut saved = Settings::default();
        saved.set(EMBED_PROVIDER, "ollama").unwrap();
        saved.set(EMBED_MODEL, "saved-model").unwrap();
        let env = |name: &str| match name {
            "CODEWIKI_EMBED_MODEL" => Some("env-model".to_string()),
            // Blank: counts as not set.
            "CODEWIKI_EMBED" => Some("  ".to_string()),
            _ => None,
        };
        let resolved = Resolved::from_parts(&saved, env);
        assert_eq!(
            resolved.get("CODEWIKI_EMBED_MODEL").as_deref(),
            Some("env-model")
        );
        assert_eq!(resolved.source(EMBED_MODEL), Source::Env);
        assert_eq!(resolved.get("CODEWIKI_EMBED").as_deref(), Some("ollama"));
        assert_eq!(resolved.source(EMBED_PROVIDER), Source::Saved);
        assert_eq!(
            resolved.get("OLLAMA_HOST").as_deref(),
            Some("http://127.0.0.1:11434")
        );
        assert_eq!(resolved.source(OLLAMA_HOST), Source::Default);
        assert_eq!(resolved.get("PATH"), None);
        // No default of its own: it depends on the provider.
        let resolved = Resolved::from_parts(&saved, no_env);
        assert_eq!(
            resolved.get("CODEWIKI_EMBED_MODEL").as_deref(),
            Some("saved-model")
        );
        let resolved = Resolved::from_parts(&Settings::default(), no_env);
        assert_eq!(resolved.get("CODEWIKI_EMBED_MODEL"), None);
        assert_eq!(resolved.source(EMBED_MODEL), Source::Default);
    }

    #[test]
    fn nothing_saved_and_nothing_set_gives_the_defaults() {
        let resolved = Resolved::from_parts(&Settings::default(), no_env);
        assert_eq!(resolved.get("CODEWIKI_EMBED").as_deref(), Some("local"));
        assert_eq!(resolved.get("OPENAI_API_KEY"), None);
        assert_eq!(resolved.secret_state(OPENAI_API_KEY), SecretState::Unset);
        // A setting CodeWiki once had is no setting.
        assert_eq!(resolved.get("CODEWIKI_LLM"), None);
        assert_eq!(super::key("ANTHROPIC_API_KEY"), None);
        for key in ALL {
            assert_eq!(resolved.source(key), Source::Default);
        }
    }

    #[test]
    fn a_secret_from_the_environment_is_used_and_not_described() {
        let env = |name: &str| (name == "OPENAI_API_KEY").then(|| SAMPLE_KEY.to_string());
        let resolved = Resolved::from_parts(&Settings::default(), env);
        assert_eq!(resolved.get("OPENAI_API_KEY").as_deref(), Some(SAMPLE_KEY));
        let state = resolved.secret_state(OPENAI_API_KEY);
        assert_eq!(state, SecretState::Env);
        assert_eq!(state.hint(), "");
        assert_eq!(state.as_str(), "env");
    }

    #[cfg(windows)]
    #[test]
    fn a_saved_secret_is_encrypted_in_the_file_and_read_back() {
        let dir = temp_dir("secret");
        update(&dir, |s| s.set(OPENAI_API_KEY, SAMPLE_KEY)).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!text.contains(SAMPLE_KEY));
        assert!(text.contains("\"hint\": \"0042\""));

        let saved = load(&dir);
        // A secret is not readable as a plain value.
        assert_eq!(saved.value(OPENAI_API_KEY), None);
        let resolved = Resolved::from_parts(&saved, no_env);
        assert_eq!(resolved.get("OPENAI_API_KEY").as_deref(), Some(SAMPLE_KEY));
        assert_eq!(
            resolved.secret_state(OPENAI_API_KEY),
            SecretState::Saved {
                hint: "0042".into()
            }
        );
        assert_eq!(resolved.source(OPENAI_API_KEY), Source::Saved);

        // The variable still wins over it.
        let env = |name: &str| (name == "OPENAI_API_KEY").then(|| "from-env".to_string());
        let resolved = Resolved::from_parts(&saved, env);
        assert_eq!(resolved.get("OPENAI_API_KEY").as_deref(), Some("from-env"));
        assert_eq!(resolved.secret_state(OPENAI_API_KEY), SecretState::Env);

        update(&dir, |s| {
            s.clear(OPENAI_API_KEY);
            Ok(())
        })
        .unwrap();
        assert_eq!(load(&dir).secret(OPENAI_API_KEY), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_secret_that_cannot_be_decrypted_counts_as_missing() {
        let document = json!({
            "openai": { "api_key": { "dpapi": "00112233445566778899aabbccddeeff", "hint": "1234" } }
        });
        let Value::Object(document) = document else {
            unreachable!()
        };
        let saved = Settings { document };
        let resolved = Resolved::from_parts(&saved, no_env);
        assert_eq!(resolved.get("OPENAI_API_KEY"), None);
        let state = resolved.secret_state(OPENAI_API_KEY);
        assert_eq!(state.as_str(), "unreadable");
        assert_eq!(state.hint(), "1234");
    }

    #[test]
    fn an_update_keeps_what_it_does_not_touch() {
        let dir = temp_dir("update");
        std::fs::write(
            dir.join(FILE),
            r#"{ "version": 1, "embed": { "provider": "ollama", "later": true }, "future": { "x": 1 } }"#,
        )
        .unwrap();
        let after = update(&dir, |s| s.set(EMBED_MODEL, "  nomic  ")).unwrap();
        assert_eq!(after.value(EMBED_MODEL).as_deref(), Some("nomic"));

        let saved = load(&dir);
        assert_eq!(saved, after);
        assert_eq!(saved.value(EMBED_PROVIDER).as_deref(), Some("ollama"));
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["future"]["x"], 1);
        assert_eq!(value["embed"]["later"], true);
        assert!(!dir.join(format!("{FILE}.tmp")).exists());

        // A blank value clears; a section left empty goes too.
        update(&dir, |s| s.set(OLLAMA_HOST, "http://host:1")).unwrap();
        let saved = update(&dir, |s| s.set(OLLAMA_HOST, " ")).unwrap();
        assert_eq!(saved.value(OLLAMA_HOST), None);
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!text.contains("ollama\":"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_settings_of_the_writer_codewiki_once_had_leave_the_file() {
        let dir = temp_dir("retired");
        // Nothing to take out: no file is made.
        assert!(!drop_retired(&dir).unwrap());
        assert!(!dir.join(FILE).exists());

        let old = json!({
            "version": 1,
            "llm": { "provider": "anthropic", "model": "claude" },
            "anthropic": { "api_key": { "dpapi": "00112233445566778899aabbccddeeff", "hint": "1234" } },
            "cliproxy": { "url": "http://127.0.0.1:8317", "api_key": { "dpapi": "ffee", "hint": "abcd" } },
            "openai": { "api_key": { "dpapi": "0011", "hint": "5678" } },
            "embed": { "provider": "ollama" },
            "future": { "x": 1 },
        });
        std::fs::write(dir.join(FILE), old.to_string()).unwrap();
        assert!(drop_retired(&dir).unwrap());

        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        for section in RETIRED_SECTIONS {
            assert_eq!(value.get(section), None, "{section}");
        }
        // No key of the old writer is left on the disk, encrypted or not.
        assert!(!text.contains("00112233445566778899aabbccddeeff"));
        assert!(!text.contains("ffee"));
        // What is still a setting stays, and so does what a newer program
        // may have written.
        assert_eq!(value["openai"]["api_key"]["hint"], "5678");
        assert_eq!(value["embed"]["provider"], "ollama");
        assert_eq!(value["future"]["x"], 1);

        // Done once: the file is not written again.
        assert!(!drop_retired(&dir).unwrap());
        assert_eq!(std::fs::read_to_string(dir.join(FILE)).unwrap(), text);

        // Any later change leaves them out too, should an older program
        // have put them back.
        std::fs::write(dir.join(FILE), old.to_string()).unwrap();
        update(&dir, |s| s.set(EMBED_MODEL, "nomic")).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!text.contains("anthropic") && !text.contains("cliproxy"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_change_writes_nothing() {
        let dir = temp_dir("failed");
        update(&dir, |s| s.set(EMBED_PROVIDER, "ollama")).unwrap();
        let before = std::fs::read_to_string(dir.join(FILE)).unwrap();
        let result = update(&dir, |s| {
            s.set(EMBED_PROVIDER, "openai")?;
            Err(Error::invalid("no"))
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(dir.join(FILE)).unwrap(), before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_or_damaged_file_loads_as_nothing_saved() {
        let dir = temp_dir("damaged");
        assert_eq!(load(&dir), Settings::default());
        std::fs::write(dir.join(FILE), "{ not json").unwrap();
        assert_eq!(load(&dir), Settings::default());
        std::fs::write(dir.join(FILE), "[1, 2]").unwrap();
        assert_eq!(load(&dir), Settings::default());

        // The next save keeps the damaged file beside the new one.
        update(&dir, |s| s.set(EMBED_PROVIDER, "ollama")).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join(BROKEN_FILE)).unwrap(),
            "[1, 2]"
        );
        assert_eq!(load(&dir).value(EMBED_PROVIDER).as_deref(), Some("ollama"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
