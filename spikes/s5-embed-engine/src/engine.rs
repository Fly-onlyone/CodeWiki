//! Loads a model folder (config.json, tokenizer.json, model.safetensors and
//! the sentence-transformers pooling files) and embeds texts with candle.

use std::path::Path;

use candle_core::{D, DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::{modernbert, qwen3};
use serde_json::Value;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::Result;

/// Whether a text is what the user asks or what is searched. Some models
/// put an instruction before a query.
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Query,
    Document,
}

enum Model {
    /// Encoder: a whole padded batch at once, the first token's state.
    ModernBert(modernbert::ModernBert),
    /// Decoder: one text at a time, the last token's state. candle's model
    /// takes no padding mask, and it keeps a cache between calls, so every
    /// text runs on a fresh copy.
    Qwen3(qwen3::Model),
}

pub struct Engine {
    model: Model,
    tokenizer: Tokenizer,
    device: Device,
    dimensions: usize,
    query_prefix: String,
}

fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

/// The weights, read into memory. candle's memory-mapped loader is an
/// `unsafe fn`; this one is not.
fn weights(dir: &Path, device: &Device, first: &str) -> Result<VarBuilder<'static>> {
    let data = std::fs::read(dir.join("model.safetensors"))?;
    // The published files hold half-precision numbers; the processor path
    // computes in f32, and the loader converts as it reads.
    let builder = VarBuilder::from_buffered_safetensors(data, DType::F32, device)?;
    // candle asks for `model.<name>`, as a checkpoint with a task head has
    // it. An embedding checkpoint holds the bare model: `<name>`.
    if builder.contains_tensor(first) {
        Ok(builder)
    } else {
        Ok(builder.rename_f(|name: &str| name.strip_prefix("model.").unwrap_or(name).to_string()))
    }
}

impl Engine {
    pub fn load(dir: &Path, device: &Device) -> Result<Self> {
        let config = read_json(&dir.join("config.json"))?;
        let mut tokenizer = Tokenizer::from_file(dir.join("tokenizer.json"))?;
        let dimensions = config["hidden_size"].as_u64().unwrap_or_default() as usize;
        let model = match config["model_type"].as_str() {
            Some("modernbert") => {
                let parsed: modernbert::Config = serde_json::from_value(config.clone())?;
                tokenizer.with_padding(Some(PaddingParams {
                    strategy: PaddingStrategy::BatchLongest,
                    pad_id: parsed.pad_token_id,
                    pad_token: "[PAD]".to_string(),
                    ..Default::default()
                }));
                let builder = weights(dir, device, "model.embeddings.tok_embeddings.weight")?;
                Model::ModernBert(modernbert::ModernBert::load(builder, &parsed)?)
            }
            Some("qwen3") => {
                let parsed: qwen3::Config = serde_json::from_value(config.clone())?;
                let builder = weights(dir, device, "model.embed_tokens.weight")?;
                Model::Qwen3(qwen3::Model::new(&parsed, builder)?)
            }
            other => return Err(format!("unsupported model type {other:?}").into()),
        };
        let prompts = dir.join("config_sentence_transformers.json");
        let query_prefix = if prompts.is_file() {
            read_json(&prompts)?["prompts"]["query"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };
        let mut engine = Self {
            model,
            tokenizer,
            device: device.clone(),
            dimensions,
            query_prefix,
        };
        engine.set_max_tokens(512)?;
        Ok(engine)
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub fn set_max_tokens(&mut self, max_length: usize) -> Result<()> {
        self.tokenizer.with_truncation(Some(TruncationParams {
            max_length,
            ..Default::default()
        }))?;
        Ok(())
    }

    pub fn count_tokens(&self, texts: &[String]) -> Result<usize> {
        let encodings = self.tokenizer.encode_batch(texts.to_vec(), true)?;
        Ok(encodings
            .iter()
            .map(|encoding| {
                encoding
                    .get_attention_mask()
                    .iter()
                    .filter(|&&m| m == 1)
                    .count()
            })
            .sum())
    }

    /// One unit vector per text, in order.
    pub fn embed(&mut self, texts: &[String], kind: Kind) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let texts: Vec<String> = texts
            .iter()
            .map(|text| match kind {
                Kind::Query => format!("{}{text}", self.query_prefix),
                Kind::Document => text.clone(),
            })
            .collect();
        match &self.model {
            Model::ModernBert(model) => {
                let encodings = self.tokenizer.encode_batch(texts, true)?;
                let length = encodings.first().map_or(0, |e| e.get_ids().len());
                let ids: Vec<u32> = encodings
                    .iter()
                    .flat_map(|e| e.get_ids().iter().copied())
                    .collect();
                let mask: Vec<u32> = encodings
                    .iter()
                    .flat_map(|e| e.get_attention_mask().iter().copied())
                    .collect();
                let shape = (encodings.len(), length);
                let ids = Tensor::from_vec(ids, shape, &self.device)?;
                let mask = Tensor::from_vec(mask, shape, &self.device)?;
                let states = model.forward(&ids, &mask)?;
                // The state of the first token, [CLS], for every text.
                let pooled = states.i((.., 0))?;
                Ok(unit(&pooled)?.to_vec2::<f32>()?)
            }
            Model::Qwen3(model) => {
                let mut out = Vec::with_capacity(texts.len());
                for text in texts {
                    let encoding = self.tokenizer.encode(text, true)?;
                    let ids = encoding.get_ids().to_vec();
                    let length = ids.len();
                    let ids = Tensor::from_vec(ids, (1, length), &self.device)?;
                    let states = model.clone().forward(&ids, 0)?;
                    // The state of the last token, the end-of-text mark the
                    // tokenizer appends.
                    let pooled = states.i((.., length - 1))?.to_dtype(DType::F32)?;
                    out.extend(unit(&pooled)?.to_vec2::<f32>()?);
                }
                Ok(out)
            }
        }
    }
}

/// Scale every row to unit length.
fn unit(rows: &Tensor) -> Result<Tensor> {
    let norms = rows.sqr()?.sum_keepdim(D::Minus1)?.sqrt()?;
    Ok(rows.broadcast_div(&norms)?)
}
