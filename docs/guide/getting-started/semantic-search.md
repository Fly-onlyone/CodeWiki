---
description: Run an embedding server, choose it in Settings and search by meaning.
icon: magnifying-glass
---

# Set up semantic search

Search in CodeWiki always combines full-text matching with vectors. Without a model server, the vectors match word forms. That works without setup and finds more than exact words.

To search by meaning, you run a model server on your machine and tell CodeWiki to use it. CodeWiki runs no model itself and downloads none. It sends the server short texts and gets vectors back. These vectors are for search only: nothing writes notes with them.

No key is needed for a server on your machine, and no text leaves it.

## Choose a server

| Server            | How to serve an embedding model                                                            | Port  |
| ----------------- | ------------------------------------------------------------------------------------------ | ----- |
| LM Studio         | Download an embedding model in LM Studio and start the server on the Developer tab.        | 1234  |
| Ollama            | Run `ollama pull embeddinggemma`, or name another embedding model.                         | 11434 |
| Llama / llama.cpp | Load an embedding model (GGUF) in Llama for Windows, or start `llama-server` with one.     | 9931  |
| Lemonade          | Pull an embedding model in Lemonade, such as `nomic-embed-text-v1-GGUF`.                   | 13305 |
| mistral.rs        | Run `mistralrs serve -m Qwen/Qwen3-Embedding-0.6B`.                                        | 1234  |
| OpenAI-compatible | Any service with an OpenAI-compatible embeddings route. It runs elsewhere and needs a key. | none  |

LM Studio and mistral.rs both use port 1234. Llama for Windows and `llama-server` both use port 9931. Run one of each pair at a time, or give one of them another address.

## Set it up

{% stepper %}
{% step %}
### Start the server and load a model

Follow the row of your server in the table. Start the server before you go on. CodeWiki can wait for a server that is not running yet, but it cannot choose a model from it.
{% endstep %}

{% step %}
### Choose the embedder

Open Settings → Semantic search. Under **Embedder**, choose your server. The choices are **Word forms: no model needed**, **LM Studio**, **Ollama**, **Llama / llama.cpp**, **Lemonade**, **mistral.rs**, **OpenAI-compatible** and **Off: full-text search only**.

The page then shows the address of the server, with its default. Change it if your server listens elsewhere. For **OpenAI-compatible**, enter the **OpenAI API key** and the **OpenAI address**. A key is stored encrypted for your Windows user and is never shown again.
{% endstep %}

{% step %}
### Choose the model

The **Model** row lists the models that the running server offers, with the embedding ones first. Pick one. If the server lists none, type the name the server knows the model by. **Refresh** asks the server again.

Without a model, search stays full-text alone, and the page says so.
{% endstep %}

{% step %}
### Test it

Press **Test**. CodeWiki sends the server one small request and shows its answer or its error.

![Settings, Semantic search, with Llama / llama.cpp chosen, its address, a model chosen from the server's list and a Works result after Test. The server in this picture is a stand-in used for the screenshot, and its model name is not a real model. Your page shows your own server's models.](../.gitbook/assets/settings-search.png)
{% endstep %}

{% step %}
### Embed the notes

A new embedder serves the next search, but the stored vectors are rebuilt at the next sync. Until then search is full-text alone. Press **Sync now**, or wait for the next sync.

The backend embeds the documents in the background after every sync. The page shows the progress, with a **Cancel** button. What was embedded is kept, and the next sync goes on from there.
{% endstep %}
{% endstepper %}

## If the server is not running

When the server is down, or does not know the model, the page shows a warning that names the server, and the embedding job ends with that error. The page asks the server again by itself, so you can start it after you opened Settings. Nothing else retries: the next sync, **Test** or **Sync now** tries once more.

## On a clean Windows

* `llama-server`, Lemonade's own `llama-server` and mistral.rs need the Microsoft Visual C++ Redistributable. A clean Windows has none. Without it, Windows reports a missing DLL, and Lemonade answers **llama-server failed to start**. Install the x64 redistributable from Microsoft first: [vc\_redist.x64.exe](https://aka.ms/vs/17/release/vc_redist.x64.exe).
* On the processor, mistral.rs loads `Qwen/Qwen3-Embedding-0.6B` as F32, about 2.3 GB. If less is free, it refuses with **This model does not fit on the devices**. Add `--isq 8` to load it 8-bit.
* For long texts with `llama-server`, start it with `-ub 2048 -b 2048`, or add those flags to the custom serve arguments of Llama for Windows.
* Ollama takes about two seconds for a request whose length in tokens it has not seen since it loaded the model. A search by meaning can take that long with Ollama. The other servers answered in under 100 ms.

## Settings and variables

Every setting on this page has an environment variable. See [Semantic search settings](../reference/llm-and-search-settings.md).
