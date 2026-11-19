//! Text embeddings for semantic search.
//!
//! Four embedders sit behind one trait:
//!
//! - [`LocalEmbedder`] runs in-process with no model file and no network. It
//!   hashes words, word pairs and character trigrams into a fixed-size
//!   vector, so texts that share vocabulary, word stems or identifier parts
//!   land close together. It does not know that two unrelated words mean the
//!   same thing; a neural model does.
//! - [`BuiltinEmbedder`] has a neural model of CodeWiki's own catalog embed,
//!   in a hidden child process that CodeWiki starts by itself.
//! - [`OllamaEmbedder`] calls a model served by a local Ollama.
//! - [`OpenAiEmbedder`] calls any OpenAI-compatible embeddings endpoint.
//!
//! `CODEWIKI_EMBED` selects one: `local` (default), `builtin`, `ollama`,
//! `openai` or `none`.

use std::path::Path;
use std::time::Duration;

use cw_core::{Error, Result};
use serde_json::{Value, json};

/// Environment variable selecting the embedder.
pub const EMBED_ENV: &str = "CODEWIKI_EMBED";
/// Model name for the HTTP embedders.
pub const EMBED_MODEL_ENV: &str = "CODEWIKI_EMBED_MODEL";
/// What runs a built-in model.
pub const EMBED_DEVICE_ENV: &str = "CODEWIKI_EMBED_DEVICE";
/// The embedders a user can choose, in the order a page lists them.
pub const PROVIDERS: [&str; 5] = ["local", "builtin", "ollama", "openai", "none"];
/// The values of [`EMBED_DEVICE_ENV`].
pub const DEVICES: [&str; 3] = ["auto", "cpu", "cuda"];

/// Texts sent to an HTTP embedder in one request.
const BATCH: usize = 64;
/// Characters of one text sent to an HTTP embedder; models truncate anyway.
const MAX_CHARS: usize = 8_000;
/// How long an HTTP embedder may take over one batch.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
/// How long a probe waits. Long enough for a local server to load its model,
/// short enough for someone watching a "Test" button.
const PROBE_TIMEOUT: Duration = Duration::from_secs(60);
const PROBE_TEXT: &str = "ping";

pub trait Embedder: Send + Sync {
    /// Identifier stored with each vector. Vectors from different embedders
    /// are never compared, so a change of name re-embeds everything.
    fn name(&self) -> String;
    /// One vector per text, in order. Every vector has the same length.
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    /// The vector of what a user searches for. Some models put an
    /// instruction before a query and none before a document; the others
    /// embed both alike, which is the default.
    fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        self.embed(&[text.to_string()])?
            .into_iter()
            .next()
            .ok_or_else(|| Error::invalid(format!("{} returned no vector", self.name())))
    }
    /// How many documents to hand to one [`Embedder::embed`] call when many
    /// wait. Between two calls a long run reports how far it is and can be
    /// stopped, so a slow embedder names a small number.
    fn batch_size(&self) -> usize {
        BATCH
    }
    /// Whether a repository's documents take it minutes. A caller that holds
    /// a lock, and someone's patience, while it embeds then leaves the work
    /// to the background.
    fn slow(&self) -> bool {
        false
    }
    /// Check that the embedder answers: embed one word and return the length
    /// of its vector. An error carries the provider's own explanation.
    fn probe(&self) -> Result<usize> {
        let vectors = self.embed(&[PROBE_TEXT.to_string()])?;
        Ok(vectors.first().map_or(0, Vec::len))
    }
}

/// Scale `vector` to unit length. A zero vector stays zero.
pub fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector.iter_mut() {
            *value /= norm;
        }
    }
}

/// Cosine similarity of two unit vectors; 0 when their lengths differ.
pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Split text into lowercase words, breaking identifiers at underscores,
/// punctuation and camel-case humps: `PageRank_score` gives `page`, `rank`,
/// `score`.
pub fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for c in text.chars() {
        if !c.is_alphanumeric() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            previous_lower = false;
            continue;
        }
        if c.is_uppercase() && previous_lower && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        previous_lower = c.is_lowercase() || c.is_numeric();
        current.extend(c.to_lowercase());
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Words too common to tell two texts apart.
const STOP_WORDS: [&str; 30] = [
    "a", "an", "and", "are", "as", "at", "be", "by", "does", "for", "from", "how", "in", "is",
    "it", "of", "on", "or", "that", "the", "this", "to", "was", "what", "when", "where", "which",
    "who", "why", "with",
];

/// Strip a common English ending so `computes`, `computed` and `computing`
/// count as one word. Crude on purpose: it only has to be consistent.
pub fn stem(word: &str) -> &str {
    let mut root = word;
    for suffix in ["ing", "ed", "es", "s"] {
        if let Some(cut) = word.strip_suffix(suffix)
            && cut.chars().count() >= 3
            && !cut.ends_with('s')
        {
            root = cut;
            break;
        }
    }
    // `compute` and `computing` must meet at `comput`.
    match root.strip_suffix('e') {
        Some(cut) if cut.chars().count() >= 3 => cut,
        _ => root,
    }
}

/// FNV-1a, 64 bit. Stable across runs and platforms, which the stored
/// vectors depend on.
fn hash(feature: &str, salt: u8) -> u64 {
    let mut state: u64 = 0xcbf2_9ce4_8422_2325 ^ u64::from(salt);
    for byte in feature.as_bytes() {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x0000_0100_0000_01b3);
    }
    state
}

/// The built-in embedder: signed feature hashing, no model and no network.
#[derive(Debug, Clone, Copy)]
pub struct LocalEmbedder {
    pub dimensions: usize,
}

impl Default for LocalEmbedder {
    fn default() -> Self {
        Self { dimensions: 384 }
    }
}

impl LocalEmbedder {
    fn add(&self, vector: &mut [f32], feature: &str, salt: u8, weight: f32) {
        let h = hash(feature, salt);
        let slot = (h % self.dimensions as u64) as usize;
        // The sign bit comes from a different part of the hash than the slot,
        // so collisions cancel as often as they add.
        let sign = if (h >> 40) & 1 == 0 { 1.0 } else { -1.0 };
        vector[slot] += sign * weight;
    }

    pub fn embed_one(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0; self.dimensions];
        let all = words(text);
        let kept: Vec<&String> = all
            .iter()
            .filter(|w| !STOP_WORDS.contains(&w.as_str()))
            .collect();
        // Sublinear term frequency: a word repeated ten times is not ten
        // times as telling.
        let mut counts: std::collections::BTreeMap<&str, f32> = std::collections::BTreeMap::new();
        for word in &kept {
            *counts.entry(word.as_str()).or_default() += 1.0;
        }
        for (word, count) in &counts {
            let weight = 1.0 + count.ln();
            self.add(&mut vector, stem(word), 0, weight);
            // Trigrams tie together stems and spelling variants:
            // `resolve`, `resolver`, `resolution`.
            let chars: Vec<char> = word.chars().collect();
            if chars.len() > 3 {
                for window in chars.windows(3) {
                    let gram: String = window.iter().collect();
                    self.add(&mut vector, &gram, 1, 0.35 * weight);
                }
            }
        }
        for pair in kept.windows(2) {
            let feature = format!("{} {}", stem(pair[0]), stem(pair[1]));
            self.add(&mut vector, &feature, 2, 0.6);
        }
        normalize(&mut vector);
        vector
    }
}

impl Embedder for LocalEmbedder {
    fn name(&self) -> String {
        format!("local-hash-v1:{}", self.dimensions)
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| self.embed_one(t)).collect())
    }
}

fn clip(text: &str) -> &str {
    if text.len() <= MAX_CHARS {
        return text;
    }
    let mut end = MAX_CHARS;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn post_json(
    provider: &str,
    url: &str,
    bearer: Option<&str>,
    body: &Value,
    timeout: Duration,
) -> Result<Value> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .into();
    let mut request = agent.post(url);
    if let Some(token) = bearer {
        request = request.header("Authorization", &format!("Bearer {token}"));
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

fn vector_of(value: &Value) -> Option<Vec<f32>> {
    let mut vector: Vec<f32> = value
        .as_array()?
        .iter()
        .map(|v| v.as_f64().map(|f| f as f32))
        .collect::<Option<_>>()?;
    if vector.is_empty() {
        return None;
    }
    normalize(&mut vector);
    Some(vector)
}

/// Read `count` vectors of equal length from a provider's response.
fn checked(provider: &str, vectors: Option<Vec<Vec<f32>>>, count: usize) -> Result<Vec<Vec<f32>>> {
    let vectors = vectors.ok_or_else(|| {
        Error::invalid(format!("{provider} returned a response without embeddings"))
    })?;
    let same_length = vectors.windows(2).all(|w| w[0].len() == w[1].len());
    if vectors.len() != count || !same_length {
        return Err(Error::invalid(format!(
            "{provider} returned {} embeddings for {count} texts",
            vectors.len()
        )));
    }
    Ok(vectors)
}

/// Vectors from an Ollama `/api/embed` response.
fn parse_ollama(value: &Value) -> Option<Vec<Vec<f32>>> {
    value["embeddings"]
        .as_array()?
        .iter()
        .map(vector_of)
        .collect()
}

/// Vectors from an OpenAI-style `/embeddings` response, ordered by `index`.
fn parse_openai(value: &Value) -> Option<Vec<Vec<f32>>> {
    let mut rows: Vec<(u64, Vec<f32>)> = value["data"]
        .as_array()?
        .iter()
        .enumerate()
        .map(|(position, row)| {
            Some((
                row["index"].as_u64().unwrap_or(position as u64),
                vector_of(&row["embedding"])?,
            ))
        })
        .collect::<Option<_>>()?;
    rows.sort_by_key(|(index, _)| *index);
    Some(rows.into_iter().map(|(_, vector)| vector).collect())
}

/// A model served by Ollama on this machine or another.
pub struct OllamaEmbedder {
    pub model: String,
    pub base_url: String,
}

impl OllamaEmbedder {
    fn embed_within(&self, texts: &[String], timeout: Duration) -> Result<Vec<Vec<f32>>> {
        let url = format!("{}/api/embed", self.base_url.trim_end_matches('/'));
        let mut out = Vec::with_capacity(texts.len());
        for batch in texts.chunks(BATCH) {
            let input: Vec<&str> = batch.iter().map(|t| clip(t)).collect();
            let value = post_json(
                "Ollama",
                &url,
                None,
                &json!({ "model": self.model, "input": input }),
                timeout,
            )?;
            out.extend(checked("Ollama", parse_ollama(&value), batch.len())?);
        }
        Ok(out)
    }
}

impl Embedder for OllamaEmbedder {
    fn name(&self) -> String {
        format!("ollama:{}", self.model)
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.embed_within(texts, REQUEST_TIMEOUT)
    }

    fn probe(&self) -> Result<usize> {
        let vectors = self.embed_within(&[PROBE_TEXT.to_string()], PROBE_TIMEOUT)?;
        Ok(vectors.first().map_or(0, Vec::len))
    }
}

/// Any endpoint that speaks the OpenAI embeddings API.
pub struct OpenAiEmbedder {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl OpenAiEmbedder {
    fn embed_within(&self, texts: &[String], timeout: Duration) -> Result<Vec<Vec<f32>>> {
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));
        let mut out = Vec::with_capacity(texts.len());
        for batch in texts.chunks(BATCH) {
            let input: Vec<&str> = batch.iter().map(|t| clip(t)).collect();
            let value = post_json(
                "the embeddings API",
                &url,
                Some(&self.api_key),
                &json!({ "model": self.model, "input": input }),
                timeout,
            )?;
            out.extend(checked(
                "the embeddings API",
                parse_openai(&value),
                batch.len(),
            )?);
        }
        Ok(out)
    }
}

impl Embedder for OpenAiEmbedder {
    fn name(&self) -> String {
        format!("openai:{}", self.model)
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.embed_within(texts, REQUEST_TIMEOUT)
    }

    fn probe(&self) -> Result<usize> {
        let vectors = self.embed_within(&[PROBE_TEXT.to_string()], PROBE_TIMEOUT)?;
        Ok(vectors.first().map_or(0, Vec::len))
    }
}

/// Video memory from which `auto` puts a built-in model on the graphics
/// card. The largest model of the catalog takes about 2.5 GB there.
const CUDA_VRAM: u64 = 4 * 1024 * 1024 * 1024;

/// A neural model of CodeWiki's catalog, installed under the data directory
/// and run by a hidden child process that CodeWiki starts on the first
/// request and stops when it is idle.
///
/// An embedder of this kind is only a name and an address: making one starts
/// nothing, so the many places that build an embedder to ask its name cost
/// nothing. All of them reach the one child of this process through
/// [`cw_models::worker::global`].
pub struct BuiltinEmbedder {
    entry: &'static cw_models::Entry,
    spec: cw_models::worker::Spec,
}

impl BuiltinEmbedder {
    /// The embedder for the catalog model `id`, installed under `data_dir`,
    /// on `device` (one of [`DEVICES`]).
    pub fn new(id: &str, device: &str, data_dir: &Path) -> Result<Self> {
        use cw_models::{Device, catalog, hardware, worker};

        if !cw_models::ENGINE {
            return Err(Error::invalid(
                "this build cannot run a built-in model; it runs on Windows",
            ));
        }
        let entry = catalog::find(id).ok_or_else(|| {
            let known: Vec<&str> = catalog::all().iter().map(|e| e.id.as_str()).collect();
            Error::invalid(format!(
                "unknown built-in model '{id}'; use {}",
                known.join(", ")
            ))
        })?;
        let store = cw_models::Store::new(data_dir, None);
        if !store.is_installed(entry) {
            return Err(Error::invalid(format!(
                "the built-in model {} is not installed",
                entry.name
            )));
        }
        let card = hardware::nvidia_card;
        let device = match device {
            "cpu" => Device::Cpu,
            "cuda" if worker::cuda_program().is_none() => {
                return Err(Error::invalid(
                    "this installation has no worker for the graphics card",
                ));
            }
            "cuda" if card().is_none() => {
                return Err(Error::invalid("no NVIDIA graphics card was found"));
            }
            "cuda" => Device::Cuda,
            "auto" => {
                let fits = worker::cuda_program().is_some()
                    && card().is_some_and(|card| card.vram_total >= CUDA_VRAM);
                if fits { Device::Cuda } else { Device::Cpu }
            }
            other => {
                return Err(Error::invalid(format!(
                    "unknown device '{other}'; use {}",
                    DEVICES.join(", ")
                )));
            }
        };
        Ok(Self {
            entry,
            spec: worker::Spec {
                model: entry.id.clone(),
                models_dir: store.root().to_path_buf(),
                device,
            },
        })
    }

    /// The device this embedder asks for. The child may still run on the
    /// processor, once the graphics card failed too often.
    pub fn device(&self) -> cw_models::Device {
        self.spec.device
    }
}

impl Embedder for BuiltinEmbedder {
    fn name(&self) -> String {
        // The same on the processor and on the card: their vectors are equal.
        self.entry.embedder_name()
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        cw_models::worker::global().embed(&self.spec, cw_models::worker::Kind::Document, texts)
    }

    fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        cw_models::worker::global()
            .embed(
                &self.spec,
                cw_models::worker::Kind::Query,
                &[text.to_string()],
            )?
            .into_iter()
            .next()
            .ok_or_else(|| Error::invalid(format!("{} returned no vector", self.name())))
    }

    fn batch_size(&self) -> usize {
        self.entry.batch
    }

    fn slow(&self) -> bool {
        true
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// The embedder for a provider name. `lookup` reads configuration values;
/// production code passes the process environment. A built-in model is
/// looked for under the default data directory.
pub fn from_config(lookup: impl Fn(&str) -> Option<String>) -> Result<Option<Box<dyn Embedder>>> {
    from_config_in(lookup, &cw_core::paths::data_dir())
}

/// [`from_config`] with the data directory that holds the built-in models.
pub fn from_config_in(
    lookup: impl Fn(&str) -> Option<String>,
    data_dir: &Path,
) -> Result<Option<Box<dyn Embedder>>> {
    let model = |default: &str| lookup(EMBED_MODEL_ENV).unwrap_or_else(|| default.to_string());
    let chosen = lookup(EMBED_ENV).map(|v| v.to_lowercase());
    Ok(match chosen.as_deref() {
        None | Some("local") => Some(Box::new(LocalEmbedder::default())),
        Some("none") => None,
        Some("builtin") => {
            let id = lookup(EMBED_MODEL_ENV).ok_or_else(|| {
                Error::invalid(format!(
                    "the built-in embedder needs a model; set {EMBED_MODEL_ENV}"
                ))
            })?;
            let device = lookup(EMBED_DEVICE_ENV)
                .map_or_else(|| "auto".to_string(), |device| device.to_lowercase());
            Some(Box::new(BuiltinEmbedder::new(&id, &device, data_dir)?))
        }
        Some("ollama") => Some(Box::new(OllamaEmbedder {
            model: model("embeddinggemma"),
            base_url: lookup("OLLAMA_HOST").unwrap_or_else(|| "http://127.0.0.1:11434".into()),
        })),
        Some("openai") => Some(Box::new(OpenAiEmbedder {
            api_key: lookup("OPENAI_API_KEY")
                .ok_or_else(|| Error::invalid("set OPENAI_API_KEY to use the openai embedder"))?,
            model: model("text-embedding-3-small"),
            base_url: lookup("OPENAI_BASE_URL")
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
        })),
        Some(other) => {
            return Err(Error::invalid(format!(
                "unknown embedder '{other}'; use {}",
                PROVIDERS.join(", ")
            )));
        }
    })
}

/// The embedder chosen by the environment; `None` turns semantic search off.
pub fn from_env() -> Result<Option<Box<dyn Embedder>>> {
    from_config(env)
}

/// The embedder chosen by the settings in effect: the environment over the
/// settings file.
pub fn from_settings(resolved: &cw_core::settings::Resolved) -> Result<Option<Box<dyn Embedder>>> {
    from_config(|name| resolved.get(name))
}

/// [`from_settings`] with the data directory that holds the built-in models.
/// A backend passes its own, which need not be the default one.
pub fn from_settings_in(
    resolved: &cw_core::settings::Resolved,
    data_dir: &Path,
) -> Result<Option<Box<dyn Embedder>>> {
    from_config_in(|name| resolved.get(name), data_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sim(a: &str, b: &str) -> f32 {
        let e = LocalEmbedder::default();
        similarity(&e.embed_one(a), &e.embed_one(b))
    }

    #[test]
    fn splits_identifiers_into_words() {
        assert_eq!(
            words("PageRank_score in cw-graph v2"),
            ["page", "rank", "score", "in", "cw", "graph", "v2"]
        );
        assert_eq!(words("HTTPServer"), ["httpserver"]);
        assert!(words("  ..  ").is_empty());
    }

    #[test]
    fn local_vectors_are_unit_length_and_stable() {
        let e = LocalEmbedder::default();
        let v = e.embed_one("Weighted PageRank by power iteration");
        assert_eq!(v.len(), 384);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);
        assert_eq!(v, e.embed_one("Weighted PageRank by power iteration"));
        assert!(e.embed_one("").iter().all(|x| *x == 0.0));
        assert_eq!(e.name(), "local-hash-v1:384");
    }

    #[test]
    fn related_texts_score_above_unrelated_ones() {
        // Identifier form against prose form of the same idea.
        let related = sim(
            "fn page_rank(graph) computes node importance",
            "how is PageRank computed for nodes",
        );
        let unrelated = sim(
            "fn page_rank(graph) computes node importance",
            "bearer token middleware rejects the request",
        );
        assert!(related > 0.3, "{related}");
        assert!(unrelated < 0.1, "{unrelated}");
        assert_eq!(stem("computes"), stem("computed"));
        assert_eq!(stem("nodes"), stem("node"));
        assert_eq!(stem("computing"), stem("compute"));
        assert_eq!(stem("class"), "class");
        assert_eq!(stem("is"), "is");
        // Stem variants share trigrams even though no whole word matches.
        assert!(sim("resolver", "resolution of names") > sim("resolver", "theme picker"));
        assert!((sim("same text", "same text") - 1.0).abs() < 1e-4);
    }

    #[test]
    fn parses_provider_responses() {
        let ollama = json!({ "model": "m", "embeddings": [[3.0, 4.0], [0.0, 2.0]] });
        let vectors = parse_ollama(&ollama).unwrap();
        assert_eq!(vectors, [vec![0.6, 0.8], vec![0.0, 1.0]]);

        // Rows may arrive out of order; `index` restores it.
        let openai = json!({ "data": [
            { "index": 1, "embedding": [0.0, 5.0] },
            { "index": 0, "embedding": [5.0, 0.0] },
        ]});
        assert_eq!(
            parse_openai(&openai).unwrap(),
            [vec![1.0, 0.0], vec![0.0, 1.0]]
        );
        assert!(parse_openai(&json!({ "error": "x" })).is_none());
        assert!(parse_ollama(&json!({ "embeddings": [["a"]] })).is_none());
    }

    #[test]
    fn rejects_a_wrong_number_of_vectors() {
        assert!(checked("p", Some(vec![vec![1.0]]), 2).is_err());
        assert!(checked("p", Some(vec![vec![1.0], vec![1.0, 0.0]]), 2).is_err());
        assert!(checked("p", None, 1).is_err());
        assert_eq!(checked("p", Some(vec![vec![1.0]]), 1).unwrap().len(), 1);
    }

    #[test]
    fn configuration_selects_the_embedder() {
        let with = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| value.to_string())
            }
        };
        let name = |pairs| from_config(with(pairs)).unwrap().map(|e| e.name());
        assert_eq!(name(&[]).as_deref(), Some("local-hash-v1:384"));
        assert_eq!(name(&[("CODEWIKI_EMBED", "none")]), None);
        assert_eq!(
            name(&[("CODEWIKI_EMBED", "Ollama")]).as_deref(),
            Some("ollama:embeddinggemma")
        );
        assert_eq!(
            name(&[
                ("CODEWIKI_EMBED", "openai"),
                ("OPENAI_API_KEY", "k"),
                ("CODEWIKI_EMBED_MODEL", "m")
            ])
            .as_deref(),
            Some("openai:m")
        );
        assert!(from_config(with(&[("CODEWIKI_EMBED", "openai")])).is_err());
        assert!(from_config(with(&[("CODEWIKI_EMBED", "magic")])).is_err());
    }

    /// Put a model of the catalog under `data_dir` as an install leaves it,
    /// with empty files of the right sizes.
    fn installed(data_dir: &Path, id: &str) {
        let entry = cw_models::catalog::find(id).unwrap();
        let dir = cw_models::Store::new(data_dir, None).dir(entry);
        std::fs::create_dir_all(&dir).unwrap();
        for file in &entry.files {
            std::fs::File::create(dir.join(&file.path))
                .unwrap()
                .set_len(file.size)
                .unwrap();
        }
        std::fs::write(
            dir.join("manifest.json"),
            json!({ "revision": entry.revision }).to_string(),
        )
        .unwrap();
    }

    #[test]
    fn a_built_in_model_is_chosen_by_its_catalog_id() {
        if !cw_models::ENGINE {
            return;
        }
        let data = tempfile::tempdir().unwrap();
        let configured = |pairs: &'static [(&'static str, &'static str)]| {
            from_config_in(
                |name: &str| {
                    pairs
                        .iter()
                        .find(|(key, _)| *key == name)
                        .map(|(_, value)| value.to_string())
                },
                data.path(),
            )
        };
        let problem = |pairs| match configured(pairs) {
            Ok(_) => panic!("{pairs:?} was accepted"),
            Err(error) => error.to_string(),
        };

        // No model named, a model the catalog does not have, one not on disk.
        let error = problem(&[("CODEWIKI_EMBED", "builtin")]);
        assert!(error.contains("CODEWIKI_EMBED_MODEL"), "{error}");
        let error = problem(&[
            ("CODEWIKI_EMBED", "builtin"),
            ("CODEWIKI_EMBED_MODEL", "embeddinggemma"),
        ]);
        assert!(
            error.contains("unknown built-in model 'embeddinggemma'")
                && error.contains("gte-modernbert-base"),
            "{error}"
        );
        let small: &[(&str, &str)] = &[
            ("CODEWIKI_EMBED", "builtin"),
            ("CODEWIKI_EMBED_MODEL", "granite-small-english-r2"),
            ("CODEWIKI_EMBED_DEVICE", "cpu"),
        ];
        let error = problem(small);
        assert!(error.contains("is not installed"), "{error}");

        installed(data.path(), "granite-small-english-r2");
        let embedder = configured(small).unwrap().unwrap();
        // Its name holds the revision, so other weights embed again.
        assert_eq!(
            embedder.name(),
            "builtin:granite-small-english-r2@2ab6fa8ea2d6"
        );
        assert_eq!(embedder.batch_size(), 16);
        // Making it started no child process.
        assert_eq!(cw_models::worker::global().status().running, None);

        let error = problem(&[
            ("CODEWIKI_EMBED", "builtin"),
            ("CODEWIKI_EMBED_MODEL", "granite-small-english-r2"),
            ("CODEWIKI_EMBED_DEVICE", "quantum"),
        ]);
        assert!(error.contains("unknown device 'quantum'"), "{error}");
        // A test program has no worker for the graphics card beside it.
        let error = problem(&[
            ("CODEWIKI_EMBED", "builtin"),
            ("CODEWIKI_EMBED_MODEL", "granite-small-english-r2"),
            ("CODEWIKI_EMBED_DEVICE", "cuda"),
        ]);
        assert!(error.contains("no worker for the graphics card"), "{error}");
        // `auto` then means the processor.
        let auto = BuiltinEmbedder::new("granite-small-english-r2", "auto", data.path()).unwrap();
        assert_eq!(auto.device(), cw_models::Device::Cpu);
    }

    #[test]
    fn clip_respects_character_boundaries() {
        let long = "é".repeat(MAX_CHARS);
        assert!(clip(&long).len() <= MAX_CHARS);
        assert!(clip(&long).chars().all(|c| c == 'é'));
        assert_eq!(clip("short"), "short");
    }
}
