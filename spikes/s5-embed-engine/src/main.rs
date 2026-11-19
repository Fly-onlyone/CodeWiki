//! Spike S5: can candle, compiled into a CodeWiki program, run the three
//! embedding models of the catalog on this machine and on a clean Windows?
//!
//! Throwaway code. Commands:
//!
//! ```text
//! spike [--device cuda] check <model-dir>                 the model card's example
//! spike [--device cuda] embed <model-dir> <in.json> <out.json>
//! spike [--device cuda] bench <model-dir> [docs] [tokens] [batch]
//! spike probe                                            hardware and game readings
//! ```

mod engine;
mod probe;

use std::path::Path;
use std::time::Instant;

use candle_core::Device;
use serde_json::{Value, json};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, Error>;

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    // The engine returns unit vectors.
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The example each model card prints, with the scores it prints.
fn check(engine: &mut engine::Engine, dir: &Path) -> Result<()> {
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    match name {
        "gte-modernbert" => {
            let texts = [
                "what is the capital of China?",
                "how to implement quick sort in python?",
                "Beijing",
                "sorting algorithms",
            ]
            .map(str::to_string);
            let vectors = engine.embed(&texts, engine::Kind::Document)?;
            let got: Vec<f32> = (1..4).map(|i| cosine(&vectors[0], &vectors[i])).collect();
            println!("got      {got:.4?}");
            println!("expected [0.4289, 0.7131, 0.3366]");
        }
        "qwen3-0.6b" => {
            let queries = ["What is the capital of China?", "Explain gravity"].map(str::to_string);
            let documents = [
                "The capital of China is Beijing.",
                "Gravity is a force that attracts two bodies towards each other. It gives weight to physical objects and is responsible for the movement of planets around the sun.",
            ]
            .map(str::to_string);
            let q = engine.embed(&queries, engine::Kind::Query)?;
            let d = engine.embed(&documents, engine::Kind::Document)?;
            for query in &q {
                let row: Vec<f32> = d.iter().map(|doc| cosine(query, doc)).collect();
                println!("got      {row:.4?}");
            }
            println!("expected [0.7646, 0.1414] / [0.1355, 0.6000]");
        }
        other => println!("no published example for '{other}'; compare with reference.py"),
    }
    Ok(())
}

fn embed(engine: &mut engine::Engine, input: &Path, output: &Path) -> Result<()> {
    let texts: Value = serde_json::from_str(&std::fs::read_to_string(input)?)?;
    let queries = engine.embed(&strings(&texts["queries"]), engine::Kind::Query)?;
    let documents = engine.embed(&strings(&texts["documents"]), engine::Kind::Document)?;
    std::fs::write(
        output,
        serde_json::to_string(&json!({ "queries": queries, "documents": documents }))?,
    )?;
    Ok(())
}

/// A text of code and prose that fills the token limit asked for; the
/// tokenizer cuts it there.
fn sample(index: usize) -> String {
    let paragraph = "pub fn embed_documents(index: &mut Index, embedder: &dyn Embedder, kinds: &[&str], documents: &[Document]) -> Result<usize> { \
        // Bring the stored vectors of the kinds in line with the documents: embed new and changed \
        documents, drop vectors of removed ones, and return how many documents were embedded. }\n";
    format!("document {index}\n{}", paragraph.repeat(24))
}

fn bench(engine: &mut engine::Engine, docs: usize, tokens: usize, batch: usize) -> Result<()> {
    engine.set_max_tokens(tokens)?;
    let texts: Vec<String> = (0..docs).map(sample).collect();
    // The first batch pays for page faults and thread start; leave it out.
    engine.embed(&texts[..batch.min(docs)], engine::Kind::Document)?;
    let started = Instant::now();
    let mut total_tokens = 0;
    for chunk in texts.chunks(batch) {
        total_tokens += engine.count_tokens(chunk)?;
        engine.embed(chunk, engine::Kind::Document)?;
    }
    let seconds = started.elapsed().as_secs_f64();
    println!(
        "{docs} documents, {total_tokens} tokens ({} per document), batch {batch}: {seconds:.2} s, {:.1} documents/s, {:.0} tokens/s",
        total_tokens / docs.max(1),
        docs as f64 / seconds,
        total_tokens as f64 / seconds,
    );
    Ok(())
}

fn number(args: &[String], index: usize, default: usize) -> usize {
    args.get(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn run() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut device = Device::Cpu;
    if let Some(at) = args.iter().position(|arg| arg == "--device") {
        if args.get(at + 1).map(String::as_str) == Some("cuda") {
            device = Device::new_cuda(0)?;
        }
        args.drain(at..(at + 2).min(args.len()));
    }
    let command = args.first().map(String::as_str).unwrap_or_default();
    if command == "probe" {
        return probe::run();
    }
    let dir = Path::new(args.get(1).ok_or("give a model folder")?);
    let started = Instant::now();
    let mut engine = engine::Engine::load(dir, &device)?;
    println!(
        "loaded {} on {:?} in {:.2} s ({} dimensions)",
        dir.display(),
        device,
        started.elapsed().as_secs_f64(),
        engine.dimensions()
    );
    match command {
        "check" => check(&mut engine, dir),
        "embed" => embed(
            &mut engine,
            Path::new(args.get(2).ok_or("give the input file")?),
            Path::new(args.get(3).ok_or("give the output file")?),
        ),
        "bench" => bench(
            &mut engine,
            number(&args, 2, 32),
            number(&args, 3, 500),
            number(&args, 4, 8),
        ),
        other => Err(format!("unknown command '{other}'").into()),
    }
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
