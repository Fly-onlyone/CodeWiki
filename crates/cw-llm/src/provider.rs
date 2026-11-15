//! LLM providers: hosted APIs and locally installed agent CLIs.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use cw_core::settings::{self, Key, Resolved};
use cw_core::{Error, Result};
use serde_json::{Value, json};

/// Default Anthropic model for prose generation.
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-opus-5-5";
const ANTHROPIC_VERSION: &str = "2026-10-21";
/// Lets the API route a refused request to another model instead of failing.
const ANTHROPIC_FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// Non-streaming requests stay under HTTP timeouts at this size; notes are short.
const MAX_TOKENS: u32 = 16000;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Completion {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

pub trait LlmProvider: Send + Sync {
    /// Identifier recorded with generated notes, for example
    /// `anthropic:claude-opus-5-5`.
    fn name(&self) -> String;
    fn complete(&self, system: &str, user: &str) -> Result<Completion>;
}

/// A provider whose calls are logged: how long each took and the tokens it
/// used, never the prompt or the answer. A failure is a warning.
struct Logged(Box<dyn LlmProvider>);

impl LlmProvider for Logged {
    fn name(&self) -> String {
        self.0.name()
    }

    fn complete(&self, system: &str, user: &str) -> Result<Completion> {
        let started = Instant::now();
        let result = self.0.complete(system, user);
        let ms = started.elapsed().as_millis();
        match &result {
            Ok(completion) => log::debug!(
                "{} answered in {ms} ms: {} input and {} output tokens",
                self.0.name(),
                completion.input_tokens,
                completion.output_tokens
            ),
            Err(e) => log::warn!("{} failed after {ms} ms: {e}", self.0.name()),
        }
        result
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Send a JSON request and return the parsed body, turning HTTP errors into
/// messages that include the provider's own explanation.
fn post_json(provider: &str, url: &str, headers: &[(&str, &str)], body: &Value) -> Result<Value> {
    let mut request = agent().post(url);
    for (key, value) in headers {
        request = request.header(*key, *value);
    }
    let mut response = request
        .send_json(body)
        .map_err(|e| Error::external(format!("calling {provider}"), e))?;
    let status = response.status();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| Error::external(format!("reading the {provider} response"), e))?;
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    if !status.is_success() {
        let detail = value["error"]["message"]
            .as_str()
            .or_else(|| value["error"].as_str())
            .unwrap_or("no detail given");
        return Err(Error::external(
            format!("{provider} returned HTTP {}", status.as_u16()),
            detail,
        ));
    }
    Ok(value)
}

pub struct Anthropic {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl Anthropic {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.to_string()),
            base_url: "https://api.anthropic.com".to_string(),
        }
    }
}

impl LlmProvider for Anthropic {
    fn name(&self) -> String {
        format!("anthropic:{}", self.model)
    }

    fn complete(&self, system: &str, user: &str) -> Result<Completion> {
        let body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "system": system,
            "messages": [{ "role": "user", "content": user }],
            "fallbacks": "default",
        });
        let value = post_json(
            "Anthropic",
            &format!("{}/v1/messages", self.base_url),
            &[
                ("x-api-key", &self.api_key),
                ("anthropic-version", ANTHROPIC_VERSION),
                ("anthropic-beta", ANTHROPIC_FALLBACK_BETA),
            ],
            &body,
        )?;
        // A refusal is a successful HTTP response; check before reading content.
        let stop = value["stop_reason"].as_str().unwrap_or("");
        if stop == "refusal" {
            return Err(Error::external(
                "Anthropic declined the request",
                value["stop_details"]["category"]
                    .as_str()
                    .unwrap_or("no category"),
            ));
        }
        if stop == "max_tokens" {
            return Err(Error::external(
                "Anthropic",
                "the response was cut off at max_tokens",
            ));
        }
        let text: String = value["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|block| block["type"] == "text")
            .filter_map(|block| block["text"].as_str())
            .collect();
        Ok(Completion {
            text,
            input_tokens: value["usage"]["input_tokens"].as_u64().unwrap_or(0),
            output_tokens: value["usage"]["output_tokens"].as_u64().unwrap_or(0),
        })
    }
}

/// Any server speaking the OpenAI chat-completions protocol.
pub struct OpenAiCompatible {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl LlmProvider for OpenAiCompatible {
    fn name(&self) -> String {
        format!("openai:{}", self.model)
    }

    fn complete(&self, system: &str, user: &str) -> Result<Completion> {
        let body = json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
        });
        let auth = format!("Bearer {}", self.api_key);
        let value = post_json(
            "the OpenAI-compatible server",
            &format!("{}/chat/completions", self.base_url.trim_end_matches('/')),
            &[("Authorization", &auth)],
            &body,
        )?;
        Ok(Completion {
            text: value["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            input_tokens: value["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
            output_tokens: value["usage"]["completion_tokens"].as_u64().unwrap_or(0),
        })
    }
}

pub struct Ollama {
    pub model: String,
    pub base_url: String,
}

impl LlmProvider for Ollama {
    fn name(&self) -> String {
        format!("ollama:{}", self.model)
    }

    fn complete(&self, system: &str, user: &str) -> Result<Completion> {
        let body = json!({
            "model": self.model,
            "stream": false,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
        });
        let value = post_json(
            "Ollama",
            &format!("{}/api/chat", self.base_url.trim_end_matches('/')),
            &[],
            &body,
        )?;
        Ok(Completion {
            text: value["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            input_tokens: value["prompt_eval_count"].as_u64().unwrap_or(0),
            output_tokens: value["eval_count"].as_u64().unwrap_or(0),
        })
    }
}

/// An installed agent CLI run in non-interactive mode. Uses the user's own
/// subscription and login; no API key is involved.
pub struct AgentCli {
    /// Shown in note metadata, for example `claude-cli`.
    pub label: String,
    pub program: String,
    /// Arguments that make the CLI read the prompt from standard input and
    /// print only the answer.
    pub args: Vec<String>,
}

impl AgentCli {
    /// Claude Code in print mode.
    pub fn claude() -> Self {
        Self {
            label: "claude-cli".into(),
            program: "claude".into(),
            args: vec!["-p".into(), "--output-format".into(), "text".into()],
        }
    }

    /// Codex in non-interactive mode, reading the prompt from standard input.
    pub fn codex() -> Self {
        Self {
            label: "codex-cli".into(),
            program: "codex".into(),
            args: vec!["exec".into(), "-".into()],
        }
    }
}

impl LlmProvider for AgentCli {
    fn name(&self) -> String {
        self.label.clone()
    }

    fn complete(&self, system: &str, user: &str) -> Result<Completion> {
        // On Windows the CLIs are `.cmd` shims, which only a shell resolves.
        let mut command = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&self.program);
            c
        } else {
            Command::new(&self.program)
        };
        let mut child = command
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::external(format!("starting `{}`", self.program), e))?;
        let prompt = format!("{system}\n\n{user}");
        // Write on a separate thread: a CLI that fills its output pipe before
        // reading all input would otherwise deadlock with us.
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let writer = std::thread::spawn(move || stdin.write_all(prompt.as_bytes()));
        let output = child
            .wait_with_output()
            .map_err(|e| Error::external(format!("running `{}`", self.program), e))?;
        let _ = writer.join();
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::external(
                format!("`{}` exited with {}", self.program, output.status),
                stderr.lines().last().unwrap_or("no error output"),
            ));
        }
        Ok(Completion {
            text: String::from_utf8_lossy(&output.stdout).to_string(),
            // Agent CLIs do not report token counts in text mode.
            input_tokens: 0,
            output_tokens: 0,
        })
    }
}

/// Environment variable selecting the provider: one of [`PROVIDERS`].
pub const PROVIDER_ENV: &str = settings::LLM_PROVIDER.env;
/// Model name for the selected provider.
pub const MODEL_ENV: &str = settings::LLM_MODEL.env;
/// The providers a user can choose, in the order a page lists them.
pub const PROVIDERS: [&str; 6] = [
    "none",
    "anthropic",
    "openai",
    "ollama",
    "claude-cli",
    "codex-cli",
];

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// The value of `key`: what `lookup` gives for its variable, otherwise its
/// default. A blank value counts as none.
fn setting(lookup: &impl Fn(&str) -> Option<String>, key: Key) -> Option<String> {
    lookup(key.env)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| key.default.map(str::to_string))
}

/// What to tell a user whose chosen provider lacks the value of `key`.
fn missing(provider: &str, what: &str, key: Key) -> Error {
    Error::invalid(format!(
        "the {provider} provider needs {what}: save one in Settings or set {}",
        key.env
    ))
}

/// Build the provider that `lookup` describes. `lookup` reads a setting by
/// the name of its environment variable. `Ok(None)` means keyless mode: no
/// provider is configured, which is a supported way to run.
pub fn from_config(
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<Option<Box<dyn LlmProvider>>> {
    let value = |key: Key| setting(&lookup, key);
    let model = value(settings::LLM_MODEL);
    let need_model = |provider: &str| {
        model
            .clone()
            .ok_or_else(|| missing(provider, "a model", settings::LLM_MODEL))
    };
    let chosen = value(settings::LLM_PROVIDER).map(|v| v.to_lowercase());
    let provider: Box<dyn LlmProvider> = match chosen.as_deref() {
        None | Some("none") => return Ok(None),
        Some("anthropic") => {
            let key = value(settings::ANTHROPIC_API_KEY)
                .ok_or_else(|| missing("anthropic", "an API key", settings::ANTHROPIC_API_KEY))?;
            Box::new(Anthropic::new(key, model))
        }
        Some("openai") => Box::new(OpenAiCompatible {
            api_key: value(settings::OPENAI_API_KEY)
                .ok_or_else(|| missing("openai", "an API key", settings::OPENAI_API_KEY))?,
            model: need_model("openai")?,
            base_url: value(settings::OPENAI_BASE_URL).unwrap_or_default(),
        }),
        Some("ollama") => Box::new(Ollama {
            model: need_model("ollama")?,
            base_url: value(settings::OLLAMA_HOST).unwrap_or_default(),
        }),
        Some("claude-cli") => Box::new(AgentCli::claude()),
        Some("codex-cli") => Box::new(AgentCli::codex()),
        Some(other) => {
            return Err(Error::invalid(format!(
                "unknown provider '{other}'; use {}",
                PROVIDERS.join(", ")
            )));
        }
    };
    Ok(Some(Box::new(Logged(provider))))
}

/// The provider chosen by the environment alone.
pub fn from_env() -> Result<Option<Box<dyn LlmProvider>>> {
    from_config(env)
}

/// The provider chosen by the settings in effect: the environment over the
/// settings file.
pub fn from_settings(resolved: &Resolved) -> Result<Option<Box<dyn LlmProvider>>> {
    from_config(|name| resolved.get(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The provider that the given `(variable, value)` pairs describe.
    fn configured(pairs: &[(&str, &str)]) -> Result<Option<String>> {
        let lookup = |name: &str| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        };
        Ok(from_config(lookup)?.map(|provider| provider.name()))
    }

    fn problem(pairs: &[(&str, &str)]) -> String {
        configured(pairs).unwrap_err().to_string()
    }

    #[test]
    fn configuration_selects_the_provider() {
        // Made-up keys: nothing here is a real one.
        assert_eq!(configured(&[]).unwrap(), None);
        assert_eq!(configured(&[("CODEWIKI_LLM", "none")]).unwrap(), None);
        // Blank counts as not set.
        assert_eq!(configured(&[("CODEWIKI_LLM", "  ")]).unwrap(), None);
        assert_eq!(
            configured(&[("CODEWIKI_LLM", "Anthropic"), ("ANTHROPIC_API_KEY", "k")]).unwrap(),
            Some(format!("anthropic:{DEFAULT_ANTHROPIC_MODEL}"))
        );
        assert_eq!(
            configured(&[
                ("CODEWIKI_LLM", "anthropic"),
                ("ANTHROPIC_API_KEY", "k"),
                ("CODEWIKI_MODEL", " some-model "),
            ])
            .unwrap()
            .as_deref(),
            Some("anthropic:some-model")
        );
        assert_eq!(
            configured(&[
                ("CODEWIKI_LLM", "openai"),
                ("OPENAI_API_KEY", "k"),
                ("CODEWIKI_MODEL", "m"),
            ])
            .unwrap()
            .as_deref(),
            Some("openai:m")
        );
        assert_eq!(
            configured(&[("CODEWIKI_LLM", "ollama"), ("CODEWIKI_MODEL", "llama")])
                .unwrap()
                .as_deref(),
            Some("ollama:llama")
        );
        assert_eq!(
            configured(&[("CODEWIKI_LLM", "claude-cli")])
                .unwrap()
                .as_deref(),
            Some("claude-cli")
        );
        assert_eq!(
            configured(&[("CODEWIKI_LLM", "codex-cli")])
                .unwrap()
                .as_deref(),
            Some("codex-cli")
        );
    }

    #[test]
    fn a_missing_value_names_the_setting_and_its_variable() {
        let error = problem(&[("CODEWIKI_LLM", "anthropic")]);
        assert!(
            error.contains("an API key") && error.contains("ANTHROPIC_API_KEY"),
            "{error}"
        );
        let error = problem(&[("CODEWIKI_LLM", "openai"), ("CODEWIKI_MODEL", "m")]);
        assert!(error.contains("OPENAI_API_KEY"), "{error}");
        let error = problem(&[("CODEWIKI_LLM", "openai"), ("OPENAI_API_KEY", "k")]);
        assert!(
            error.contains("a model") && error.contains("CODEWIKI_MODEL"),
            "{error}"
        );
        let error = problem(&[("CODEWIKI_LLM", "ollama")]);
        assert!(error.contains("CODEWIKI_MODEL"), "{error}");
        let error = problem(&[("CODEWIKI_LLM", "gpt")]);
        assert!(
            error.contains("unknown provider 'gpt'") && error.contains("codex-cli"),
            "{error}"
        );
    }

    #[test]
    fn addresses_fall_back_to_the_defaults_of_the_settings() {
        let lookup = |name: &str| match name {
            "CODEWIKI_LLM" => Some("ollama".to_string()),
            "CODEWIKI_MODEL" => Some("llama".to_string()),
            _ => None,
        };
        // No address is given, and the provider is still built.
        assert!(from_config(lookup).unwrap().is_some());
        assert_eq!(
            setting(&lookup, settings::OLLAMA_HOST).as_deref(),
            settings::OLLAMA_HOST.default
        );
        assert_eq!(
            setting(&lookup, settings::OPENAI_BASE_URL).as_deref(),
            Some("https://api.openai.com/v1")
        );
        let custom = |name: &str| (name == "OLLAMA_HOST").then(|| " http://box:1 ".to_string());
        assert_eq!(
            setting(&custom, settings::OLLAMA_HOST).as_deref(),
            Some("http://box:1")
        );
    }

    #[test]
    fn the_settings_in_effect_select_the_provider() {
        let mut saved = settings::Settings::default();
        saved.set(settings::LLM_PROVIDER, "ollama").unwrap();
        saved.set(settings::LLM_MODEL, "saved-model").unwrap();
        let resolved = Resolved::from_parts(&saved, |_| None);
        assert_eq!(
            from_settings(&resolved)
                .unwrap()
                .map(|p| p.name())
                .as_deref(),
            Some("ollama:saved-model")
        );
        // The variable wins over the file.
        let env = |name: &str| (name == "CODEWIKI_MODEL").then(|| "env-model".to_string());
        let resolved = Resolved::from_parts(&saved, env);
        assert_eq!(
            from_settings(&resolved)
                .unwrap()
                .map(|p| p.name())
                .as_deref(),
            Some("ollama:env-model")
        );
        // Nothing saved and nothing set: keyless mode.
        let resolved = Resolved::from_parts(&settings::Settings::default(), |_| None);
        assert!(from_settings(&resolved).unwrap().is_none());
    }
}
