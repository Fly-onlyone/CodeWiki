//! Spike S2: can `merman` render the Mermaid found in real vaults?
//!
//! Usage: spike-s2-mermaid <out-dir> <vault-dir>...
//! Renders every ```mermaid block to SVG, reports failures and timing.

use std::path::{Path, PathBuf};
use std::time::Instant;

use merman::{OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest};

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            markdown_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

fn mermaid_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        match &mut current {
            None if line.trim_start().starts_with("```mermaid") => current = Some(String::new()),
            Some(_) if line.trim_start().starts_with("```") => blocks.extend(current.take()),
            Some(block) => {
                block.push_str(line);
                block.push('\n');
            }
            None => {}
        }
    }
    blocks
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = PathBuf::from(args.next().expect("output directory"));
    std::fs::create_dir_all(&out_dir).expect("create output directory");
    let renderer = Renderer::new();
    let (mut total, mut ok, mut slowest, mut sum) = (0u32, 0u32, 0f64, 0f64);
    for vault in args {
        let mut files = Vec::new();
        markdown_files(Path::new(&vault), &mut files);
        files.sort();
        for file in files {
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            for (index, block) in mermaid_blocks(&text).iter().enumerate() {
                total += 1;
                let started = Instant::now();
                let result = renderer.render(RenderRequest::svg(
                    block,
                    OperationControl::new(),
                    SvgRequest::default(),
                ));
                let ms = started.elapsed().as_secs_f64() * 1000.0;
                sum += ms;
                slowest = slowest.max(ms);
                let stem = file.file_stem().unwrap_or_default().to_string_lossy();
                match result {
                    Ok(RenderOutput::Svg(Some(svg))) => {
                        ok += 1;
                        let name = format!("{total:03}-{stem}-{index}.svg");
                        let _ = std::fs::write(out_dir.join(name), svg.svg());
                    }
                    Ok(_) => println!("NO DIAGRAM  {} #{index}", file.display()),
                    Err(error) => println!("FAILED      {} #{index}: {error}", file.display()),
                }
            }
        }
    }
    // One raster sample so legibility can be judged by eye.
    let sample = "sequenceDiagram\n    autonumber\n    participant p0 as codewiki\n    participant p1 as Service Module\n    participant p2 as Index\n    p0->>p1: update()\n    p1->>p2: replace_facts()\n    p2-->>p1: ok\n";
    if let Ok(RenderOutput::Png(Some(png))) = renderer.render(RenderRequest::png(
        sample,
        OperationControl::new(),
        merman::PngRequest {
            svg: SvgRequest::default(),
            options: Default::default(),
        },
    )) {
        let _ = std::fs::write(out_dir.join("sample.png"), png.bytes);
    }
    println!(
        "{ok}/{total} rendered ({:.1}%), mean {:.1} ms, slowest {:.1} ms",
        if total == 0 { 0.0 } else { ok as f64 * 100.0 / total as f64 },
        if total == 0 { 0.0 } else { sum / total as f64 },
        slowest
    );
}
