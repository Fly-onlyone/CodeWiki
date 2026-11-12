//! LLM providers and the prose writer for CodeWiki.
//!
//! Generation works without any of this (keyless mode). When a provider is
//! configured, the writer replaces structural prose with explained prose.

pub mod answer;
pub mod illustrate;
pub mod provider;
pub mod writer;

pub use provider::{
    AgentCli, Anthropic, Completion, LlmProvider, MODEL_ENV, Ollama, OpenAiCompatible,
    PROVIDER_ENV, from_env,
};
pub use writer::LlmWriter;

/// The writer selected by the environment, or `None` for keyless mode.
pub fn writer_from_env() -> cw_core::Result<Option<LlmWriter>> {
    Ok(from_env()?.map(LlmWriter::new))
}
