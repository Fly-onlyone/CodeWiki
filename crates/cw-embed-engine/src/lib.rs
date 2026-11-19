//! The engine of the built-in embedding models.
//!
//! [`Engine`] loads one model of the catalog from its folder and embeds
//! texts with it, with `candle`. [`worker`] is the whole of the hidden child
//! process: it loads the model its arguments name and answers the parent's
//! requests until its input closes (`cw_models::worker` is the protocol).
//!
//! Only the programs that are that child link this crate. Everything else
//! reaches a model through the child, so that a fault in native model code
//! ends the child alone.
//!
//! What loading each family of models needs was worked out, and checked
//! against reference vectors, in spike S5 (`spikes/VERDICTS.md`).

use std::path::Path;
use std::process::ExitCode;

use candle_core::{D, DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::{modernbert, qwen3};
use cw_models::worker::{Hello, Kind, PROTOCOL};
use cw_models::{Entry, Family, Pooling, catalog};
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

/// Any failure of the engine, as text. It never holds a text that was given
/// to embed: the parent process writes it to its log.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

enum Model {
    /// An encoder: a padded batch at once.
    ModernBert(modernbert::ModernBert),
    /// A decoder: one text at a time. candle's model takes no padding mask,
    /// and it keeps a cache between calls, so every text runs on a fresh
    /// copy; a copy shares the weights.
    Qwen3(qwen3::Model),
}

pub struct Engine {
    model: Model,
    tokenizer: Tokenizer,
    device: Device,
    pooling: Pooling,
    query_prefix: String,
    dimensions: usize,
}

/// The weights of a model, read into memory. candle's memory-mapped loader
/// is an `unsafe fn`; this one is not.
fn weights(dir: &Path, device: &Device, first: &str) -> Result<VarBuilder<'static>> {
    let data = std::fs::read(dir.join("model.safetensors"))?;
    // The published files hold half-precision numbers. The engine computes
    // in f32, and the loader converts as it reads.
    let builder = VarBuilder::from_buffered_safetensors(data, DType::F32, device)?;
    // candle asks for `model.<name>`, as a checkpoint with a task head names
    // its tensors. An embedding checkpoint holds the bare model: `<name>`.
    if builder.contains_tensor(first) {
        Ok(builder)
    } else {
        Ok(builder.rename_f(|name: &str| name.strip_prefix("model.").unwrap_or(name).to_string()))
    }
}

/// Scale every row to unit length.
fn unit(rows: &Tensor) -> Result<Tensor> {
    let norms = rows.sqr()?.sum_keepdim(D::Minus1)?.sqrt()?;
    Ok(rows.broadcast_div(&norms)?)
}

impl Engine {
    /// Load the model `entry` from `dir`, the folder an install made.
    pub fn load(entry: &Entry, dir: &Path, device: &Device) -> Result<Self> {
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("config.json"))?)?;
        let mut tokenizer = Tokenizer::from_file(dir.join("tokenizer.json"))?;
        tokenizer.with_truncation(Some(TruncationParams {
            max_length: entry.max_tokens,
            ..Default::default()
        }))?;
        let model = match entry.family {
            Family::ModernBert => {
                let parsed: modernbert::Config = serde_json::from_value(config)?;
                tokenizer.with_padding(Some(PaddingParams {
                    strategy: PaddingStrategy::BatchLongest,
                    pad_id: parsed.pad_token_id,
                    pad_token: "[PAD]".to_string(),
                    ..Default::default()
                }));
                let builder = weights(dir, device, "model.embeddings.tok_embeddings.weight")?;
                Model::ModernBert(modernbert::ModernBert::load(builder, &parsed)?)
            }
            Family::Qwen3 => {
                let parsed: qwen3::Config = serde_json::from_value(config)?;
                let builder = weights(dir, device, "model.embed_tokens.weight")?;
                Model::Qwen3(qwen3::Model::new(&parsed, builder)?)
            }
        };
        Ok(Self {
            model,
            tokenizer,
            device: device.clone(),
            pooling: entry.pooling,
            query_prefix: entry.query_prefix.clone(),
            dimensions: entry.dimensions,
        })
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// One unit vector per text, in order.
    pub fn embed(&self, kind: Kind, texts: &[String]) -> Result<Vec<Vec<f32>>> {
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
        let vectors = match &self.model {
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
                let pooled = match self.pooling {
                    // The state of the first token, [CLS].
                    Pooling::First => states.i((.., 0))?,
                    Pooling::Last => return Err("this model family pools the first token".into()),
                };
                unit(&pooled)?.to_vec2::<f32>()?
            }
            Model::Qwen3(model) => {
                if self.pooling != Pooling::Last {
                    return Err("this model family pools the last token".into());
                }
                let mut out = Vec::with_capacity(texts.len());
                for text in texts {
                    let encoding = self.tokenizer.encode(text, true)?;
                    let ids = encoding.get_ids().to_vec();
                    let length = ids.len();
                    if length == 0 {
                        return Err("a text gave no token".into());
                    }
                    let ids = Tensor::from_vec(ids, (1, length), &self.device)?;
                    let states = model.clone().forward(&ids, 0)?;
                    // The state of the last token, the end-of-text mark the
                    // tokenizer appends.
                    let pooled = states.i((.., length - 1))?.to_dtype(DType::F32)?;
                    out.extend(unit(&pooled)?.to_vec2::<f32>()?);
                }
                out
            }
        };
        Ok(vectors)
    }
}

/// The device this program was built for: the graphics card when it holds
/// the CUDA engine, the processor otherwise.
fn device() -> Result<(Device, cw_models::Device)> {
    if cfg!(feature = "cuda") {
        Ok((Device::new_cuda(0)?, cw_models::Device::Cuda))
    } else {
        Ok((Device::Cpu, cw_models::Device::Cpu))
    }
}

/// The whole of the worker process: load the model `id` from `models_dir`
/// and answer requests on standard input until it closes. Standard output
/// carries the protocol and nothing else.
pub fn worker(id: &str, models_dir: &Path) -> ExitCode {
    cw_core::logs::init("embed-worker", log::LevelFilter::Info);
    let stdout = std::io::stdout();
    let loaded = (|| -> Result<(Engine, cw_models::Device)> {
        let entry = catalog::find(id).ok_or_else(|| format!("unknown model '{id}'"))?;
        let (device, name) = device()?;
        let started = std::time::Instant::now();
        let engine = Engine::load(entry, &models_dir.join(&entry.id), &device)?;
        log::info!(
            "loaded {} on {name:?} in {} ms",
            entry.id,
            started.elapsed().as_millis()
        );
        Ok((engine, name))
    })();
    let (engine, device) = match loaded {
        Ok(loaded) => loaded,
        Err(error) => {
            log::error!("could not load the model {id}: {error}");
            let _ = cw_models::worker::refuse(stdout.lock(), &error.to_string());
            return ExitCode::FAILURE;
        }
    };
    let hello = Hello {
        protocol: PROTOCOL,
        device,
        dimensions: engine.dimensions(),
    };
    let served = cw_models::worker::serve(
        std::io::stdin().lock(),
        stdout.lock(),
        hello,
        &mut |kind, texts| {
            engine.embed(kind, texts).map_err(|error| {
                log::warn!("could not embed {} texts: {error}", texts.len());
                error.to_string()
            })
        },
    );
    match served {
        Ok(()) => ExitCode::SUCCESS,
        // The parent is gone, or closed the pipes: nothing to report to.
        Err(_) => ExitCode::FAILURE,
    }
}
