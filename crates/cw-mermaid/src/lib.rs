//! Mermaid diagram rendering for CodeWiki.
//!
//! All rendering goes through this crate so the renderer can be replaced in
//! one place. Flowcharts, sequence diagrams and state diagrams are supported;
//! any other diagram type returns an error and the caller shows the source.

use merman::{OperationControl, PngRequest, RenderOutput, RenderRequest, Renderer, SvgRequest};

/// Diagram palette. Chosen to match the surface the diagram is drawn on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    Light,
    Dark,
}

/// Apply a palette with Mermaid's own `init` directive, unless the diagram
/// already carries one.
fn themed(source: &str, palette: Palette) -> String {
    if palette == Palette::Light || source.trim_start().starts_with("%%{") {
        return source.to_string();
    }
    format!("%%{{init: {{\"theme\": \"dark\"}}}}%%\n{source}")
}

/// Repair a mistake common in hand-written flowcharts: a node label that
/// is a URL path, `Agent[/ws/agent]`. Mermaid reads `[/` as the start of a
/// slanted shape and rejects the diagram when no `/]` or `\]` closes it.
/// Such labels are quoted, `Agent["/ws/agent"]`; real slanted shapes and
/// other diagram types are left alone.
pub fn lenient(source: &str) -> String {
    let kind = source
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("%%"))
        .unwrap_or("");
    if !(kind.starts_with("flowchart") || kind.starts_with("graph")) {
        return source.to_string();
    }
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(open) = rest.find("[/") {
        let (before, tail) = rest.split_at(open);
        out.push_str(before);
        let line_end = tail.find('\n').unwrap_or(tail.len());
        match tail[..line_end].find(']') {
            Some(close)
                if !tail[..close].ends_with('/')
                    && !tail[..close].ends_with('\\')
                    && !tail[..close].contains('"') =>
            {
                out.push_str(&format!("[\"{}\"]", &tail[1..close]));
                rest = &tail[close + 1..];
            }
            _ => {
                out.push_str("[/");
                rest = &tail[2..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Render Mermaid source to an SVG document.
pub fn render_svg(source: &str) -> Result<String, String> {
    let source = &lenient(source);
    let output = Renderer::new()
        .render(RenderRequest::svg(
            source,
            OperationControl::new(),
            SvgRequest::default(),
        ))
        .map_err(|e| e.to_string())?;
    match output {
        RenderOutput::Svg(Some(svg)) => Ok(svg.svg().to_string()),
        _ => Err("no Mermaid diagram detected".to_string()),
    }
}

/// Render Mermaid source to PNG bytes.
pub fn render_png(source: &str, palette: Palette) -> Result<Vec<u8>, String> {
    let source = &themed(&lenient(source), palette);
    let output = Renderer::new()
        .render(RenderRequest::png(
            source,
            OperationControl::new(),
            PngRequest {
                svg: SvgRequest::default(),
                options: Default::default(),
            },
        ))
        .map_err(|e| e.to_string())?;
    match output {
        RenderOutput::Png(Some(png)) => Ok(png.bytes),
        _ => Err("no Mermaid diagram detected".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_path_labels_and_keeps_real_shapes() {
        let broken = "flowchart LR\n    Agent[/ws/agent] --> Market[/ws/market]\n";
        assert!(render_svg("flowchart LR\n    A[plain]\n").is_ok());
        let fixed = lenient(broken);
        assert_eq!(
            fixed,
            "flowchart LR\n    Agent[\"/ws/agent\"] --> Market[\"/ws/market\"]\n"
        );
        let svg = render_svg(broken).unwrap();
        assert!(svg.contains("/ws/agent"), "{svg:.80}");
        // Parallelograms, trapezoids and quoted labels are valid as written.
        for valid in [
            "flowchart LR\n    A[/input/] --> B[/wide\\]\n",
            "graph TD\n    A[\"/already/quoted\"]\n",
        ] {
            assert_eq!(lenient(valid), valid);
        }
        // Other diagram kinds are never rewritten.
        let sequence = "sequenceDiagram\n    a->>b: GET [/x]\n";
        assert_eq!(lenient(sequence), sequence);
    }

    #[test]
    fn renders_the_diagram_kinds_notes_use() {
        for source in [
            "flowchart LR\n    n0[cw-core]\n    n1[cw-parse] --> n0\n",
            "sequenceDiagram\n    autonumber\n    participant a as App\n    participant b as Backend\n    a->>b: update()\n",
            "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Busy\n",
        ] {
            let svg = render_svg(source).unwrap_or_else(|e| panic!("{e}: {source}"));
            assert!(svg.starts_with("<svg"), "{svg:.40}");
            let png = render_png(source, Palette::Light).unwrap();
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
            let dark = render_png(source, Palette::Dark).unwrap();
            assert_ne!(png, dark, "the dark palette changes the image");
        }
    }

    #[test]
    fn reports_bad_input_instead_of_panicking() {
        assert!(render_svg("this is not a diagram").is_err());
        assert!(render_png("flowchart LR\n    A[unclosed\n", Palette::Dark).is_err());
        assert!(
            render_svg("pie title Pets\n    \"Dogs\" : 3\n").is_err(),
            "unsupported kinds are errors"
        );
    }
}
