//! The worker that runs a built-in embedding model on an NVIDIA card.
//!
//! The same code as `codewiki embed-worker`, built with the CUDA engine. It
//! is a program of its own because a program that holds that engine does not
//! start where NVIDIA's libraries are missing, and `codewiki` must start
//! everywhere. The backend starts it, with the same arguments, only on a
//! machine with an NVIDIA card:
//!
//! ```text
//! codewiki-embed-cuda embed-worker --model <id> --models-dir <folder>
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

/// The value that follows `name` among the arguments.
fn value(arguments: &[String], name: &str) -> Option<String> {
    arguments
        .iter()
        .position(|argument| argument == name)
        .and_then(|at| arguments.get(at + 1))
        .cloned()
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (Some(model), Some(models_dir)) = (
        value(&arguments, "--model"),
        value(&arguments, "--models-dir"),
    ) else {
        eprintln!("usage: codewiki-embed-cuda embed-worker --model <id> --models-dir <folder>");
        return ExitCode::FAILURE;
    };
    run(&model, &PathBuf::from(models_dir))
}

#[cfg(windows)]
fn run(model: &str, models_dir: &std::path::Path) -> ExitCode {
    cw_embed_engine::worker(model, models_dir)
}

#[cfg(not(windows))]
fn run(_: &str, _: &std::path::Path) -> ExitCode {
    eprintln!("error: this build cannot run a built-in model; it runs on Windows");
    ExitCode::FAILURE
}
