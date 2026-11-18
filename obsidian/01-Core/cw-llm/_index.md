---
tags: [moc, core, cw-llm]
---

# Core / cw-llm — Map of Content

> Notes for `cw-llm`: 11 concepts.

## Notes

- [[AgentCli]] — an installed agent CLI run in non-interactive mode.
- [[Answer Module]] — answers a question about the codebase with an LLM, grounded in the vault.
- [[Anthropic]] — `Anthropic` struct defined in `cw_llm::provider` with 5 methods.
- [[cw-llm]] — LLM providers and the prose writer for CodeWiki.
- [[Illustrate Module]] — asks an LLM to draw a note as an OpenUI Lang illustration.
- [[LlmProvider]] — `LlmProvider` trait defined in `cw_llm::provider` with 3 methods.
- [[LlmWriter]] — `LlmWriter` struct defined in `cw_llm::writer` with 6 methods.
- [[Ollama]] — `Ollama` struct defined in `cw_llm::provider` with 4 methods.
- [[OpenAiCompatible]] — any server speaking the OpenAI chat-completions protocol.
- [[Provider Module]] — LLM providers: hosted APIs and locally installed agent CLIs.
- [[Writer Module]] — turns a structural draft into finished prose with an LLM.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
