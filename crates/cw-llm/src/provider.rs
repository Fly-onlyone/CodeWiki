//! LLM providers: hosted APIs and locally installed agent CLIs.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

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

/// Environment variable selecting the provider: `anthropic`, `openai`,
/// `ollama`, `claude-cli`, `codex-cli` or `none`.
pub const PROVIDER_ENV: &str = "CODEWIKI_LLM";
/// Model name for the selected provider.
pub const MODEL_ENV: &str = "CODEWIKI_MODEL";

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// Build the provider chosen by the environment. `Ok(None)` means keyless
/// mode: no provider is configured, which is a supported way to run.
pub fn from_env() -> Result<Option<Box<dyn LlmProvider>>> {
    let model = env(MODEL_ENV);
    let need_model = |provider: &str| {
        model.clone().ok_or_else(|| {
            Error::invalid(format!("set {MODEL_ENV} to use the {provider} provider"))
        })
    };
    let chosen = env(PROVIDER_ENV).map(|v| v.to_lowercase());
    let provider: Box<dyn LlmProvider> = match chosen.as_deref() {
        None | Some("none") => return Ok(None),
        Some("anthropic") => {
            let key = env("ANTHROPIC_API_KEY").ok_or_else(|| {
                Error::invalid("set ANTHROPIC_API_KEY to use the anthropic provider")
            })?;
            Box::new(Anthropic::new(key, model))
        }
        Some("openai") => Box::new(OpenAiCompatible {
            api_key: env("OPENAI_API_KEY")
                .ok_or_else(|| Error::invalid("set OPENAI_API_KEY to use the openai provider"))?,
            model: need_model("openai")?,
            base_url: env("OPENAI_BASE_URL").unwrap_or_else(|| "https://api.openai.com/v1".into()),
        }),
        Some("ollama") => Box::new(Ollama {
            model: need_model("ollama")?,
            base_url: env("OLLAMA_HOST").unwrap_or_else(|| "http://127.0.0.1:11434".into()),
        }),
        Some("claude-cli") => Box::new(AgentCli::claude()),
        Some("codex-cli") => Box::new(AgentCli::codex()),
        Some(other) => {
            return Err(Error::invalid(format!(
                "unknown {PROVIDER_ENV} value '{other}'; use anthropic, openai, ollama, claude-cli, codex-cli or none"
            )));
        }
    };
    Ok(Some(provider))
}
