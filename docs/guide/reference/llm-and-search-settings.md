---
description: The embedders, their addresses and keys, and the environment variables.
icon: sparkles
---

# Semantic search settings

Search combines full-text matching with vectors. CodeWiki runs no model of its own, installs no server and downloads no model. You run an embedding server, and CodeWiki asks it for vectors. The vectors are for search only: nothing writes notes with them.

For the steps, see [Set up semantic search](../getting-started/semantic-search.md). This page lists every setting.

You can choose each setting in the app, under Settings → Semantic search, and it takes effect at once. Each also has an environment variable. A variable that is set wins over the saved value, and the page then shows the setting locked, with the variable's name.

## The embedders

`CODEWIKI_EMBED` selects where vectors come from.

| Value       | Name in Settings            | What it is                                                               |
| ----------- | --------------------------- | ------------------------------------------------------------------------ |
| `local`     | Word forms: no model needed | The default. It matches word forms and needs no model and no network.    |
| `lmstudio`  | LM Studio                   | A server on your machine.                                                |
| `ollama`    | Ollama                      | A server on your machine.                                                |
| `llamacpp`  | Llama / llama.cpp           | Llama for Windows, or `llama-server`.                                    |
| `lemonade`  | Lemonade                    | A server on your machine.                                                |
| `mistralrs` | mistral.rs                  | A server on your machine.                                                |
| `openai`    | OpenAI-compatible           | A service with an OpenAI-compatible embeddings route. It runs elsewhere. |
| `none`      | Off: full-text search only  | Leaves full-text search alone.                                           |

Every server except Ollama is asked through the OpenAI-style route `POST /v1/embeddings`. A server needs a model. Without one, search stays full-text alone and the page says so.

## Addresses, keys and models

| Setting            | Variable                 | Default                     |
| ------------------ | ------------------------ | --------------------------- |
| Embedder           | `CODEWIKI_EMBED`         | `local`                     |
| Model              | `CODEWIKI_EMBED_MODEL`   | none                        |
| LM Studio address  | `CODEWIKI_LMSTUDIO_URL`  | `http://127.0.0.1:1234`     |
| LM Studio key      | `CODEWIKI_LMSTUDIO_KEY`  | none, optional              |
| Ollama address     | `OLLAMA_HOST`            | `http://127.0.0.1:11434`    |
| Llama address      | `CODEWIKI_LLAMACPP_URL`  | `http://127.0.0.1:9931`     |
| Llama key          | `CODEWIKI_LLAMACPP_KEY`  | none, optional              |
| Lemonade address   | `CODEWIKI_LEMONADE_URL`  | `http://127.0.0.1:13305`    |
| Lemonade key       | `CODEWIKI_LEMONADE_KEY`  | none, optional              |
| mistral.rs address | `CODEWIKI_MISTRALRS_URL` | `http://127.0.0.1:1234`     |
| OpenAI API key     | `OPENAI_API_KEY`         | none, required for `openai` |
| OpenAI address     | `OPENAI_BASE_URL`        | `https://api.openai.com/v1` |

Ollama and mistral.rs need no key. An address pasted with `/v1` at the end, as the servers show it, works like the bare address. A server that wants a key and gets none, or a wrong one, answers with an error that names the setting to fill in.

The model names do not carry across servers. When you change the embedder to one that names no model, CodeWiki clears the model, unless the environment sets it.

## Saved settings

The backend keeps the saved settings in `settings.json` in its data folder, so the command line and an agent's MCP bridge use them too.

An API key is stored encrypted for your Windows user. The page shows only its last four characters, and nothing sends it back.

**Test** sends the chosen embedder one small request and shows its answer or its error.

A backend that listens beyond its own machine, as the one in Docker does, takes its settings from its environment variables only. The app shows them and cannot change them.

## How texts are sent

* Texts go in batches, and each text is cut at 8,000 characters.
* A batch that a server refuses as too long is sent again with each text cut to 2,000 characters. Refused again, the texts are sent one at a time, and a text refused alone is sent once more cut to 400 characters. A text refused even then ends the job with the server's error.
* The first request to a server may wait up to five minutes while it loads the model. Later requests wait two minutes.
* Some models expect a short text in front of a query or a document: `embeddinggemma`, `qwen3-embedding`, `nomic-embed-text`, `mxbai-embed`, the `bge` v1.5 models and the `e5` models. CodeWiki adds it for them, whichever server serves the model. It adds `+<name>` to the name its vectors are stored under, so vectors made with and without it are never compared.

## After a change

After a change of embedder or model, search is full-text alone until the documents are embedded again. The backend does that in the background after every sync. The Settings page shows the progress with a **Cancel** button.

A server that is down, or a model it does not know, ends the job with that error, shown as a warning. Nothing retries by itself. The next sync (**Sync now**), **Test** or a new request to embed tries once more.

## Earlier versions

Earlier versions could download a model and run it themselves. That is gone, with its worker, its `cuda` folder, the command `codewiki model` and the device setting. When the backend starts, it moves a saved choice of the built-in model to `local`, forgets the model and the device, and deletes the downloaded models in the data folder `models`. An update removes an old `cuda` folder. Choose a server in Settings → Semantic search to search by meaning again. A variable that still names the built-in embedder cannot be changed for you: search is full-text alone, and the page says that the built-in embedder was removed.

Earlier versions could also call a language model to write notes. CodeWiki no longer does. Those settings have no effect, and the backend removes them from `settings.json` when it starts, the saved keys with them. The explanations of notes are written by your coding agent. See [Agent-written notes](../agents/writing-notes.md).

## Other variables

| Variable                         | Meaning                                                                               |
| -------------------------------- | ------------------------------------------------------------------------------------- |
| `CODEWIKI_URL`, `CODEWIKI_TOKEN` | Use a backend on another machine or in Docker instead of a local one.                 |
| `CODEWIKI_DATA_DIR`              | Where the index and the settings go. Default: `%LOCALAPPDATA%\CodeWiki`.              |
| `CODEWIKI_LOG`                   | `debug` writes more to the logs. See [Logs and diagnostics](logs-and-diagnostics.md). |
| `CODEWIKI_LOG_DIR`               | Where the logs go.                                                                    |
| `CODEWIKI_WEBHOOK_SECRET`        | Lets a git host trigger updates on a backend you run yourself.                        |

Set them where the program runs: in your terminal for the command line, or in your Windows user environment for the app and for coding agents.
