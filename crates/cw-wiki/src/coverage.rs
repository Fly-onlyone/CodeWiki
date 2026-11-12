//! Test coverage imported from LCOV (`lcov.info`) and Cobertura XML reports.
//!
//! A report is parsed, its paths are matched to indexed files, and the result
//! is cached in the index. Reporting then ranks the gaps: an uncovered file
//! matters more when it changes often (hotspot) or much depends on it (rank).
//!
//! Both formats are read by small parsers written against their public
//! descriptions: the `geninfo` tracefile format (`SF`, `DA`, `LF`, `LH`,
//! `end_of_record`) and the Cobertura `coverage-04.dtd` element layout
//! (`sources/source`, `class@filename`, `line@number@hits`).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use cw_core::{Error, Result, SymbolKind};
use cw_index::Index;
use serde::{Deserialize, Serialize};

use crate::gitintel;
use crate::rustscan::is_test_path;

/// Meta key under which imported coverage is cached in the index.
pub const META_KEY: &str = "coverage";

/// Where a report is looked for when no path is given, relative to the
/// repository root.
pub const DEFAULT_REPORTS: [&str; 12] = [
    "lcov.info",
    "coverage/lcov.info",
    "target/lcov.info",
    "target/llvm-cov/lcov.info",
    "target/coverage/lcov.info",
    "coverage.xml",
    "cobertura.xml",
    "coverage/cobertura.xml",
    "coverage/cobertura-coverage.xml",
    "target/cobertura.xml",
    "target/llvm-cov/cobertura.xml",
    "target/tarpaulin/cobertura.xml",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Lcov,
    Cobertura,
}

/// One file as a report describes it, before its path is matched.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RawFile {
    /// Line number to execution count.
    lines: BTreeMap<u32, u64>,
    /// Totals stated by the report, used when it lists no lines.
    found: u32,
    hit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawReport {
    format: Format,
    /// Base directories that relative file names may be relative to.
    sources: Vec<String>,
    files: BTreeMap<String, RawFile>,
}

fn parse_lcov(text: &str) -> Result<RawReport> {
    let mut files: BTreeMap<String, RawFile> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(path) = line.strip_prefix("SF:") {
            files.entry(path.to_string()).or_default();
            current = Some(path.to_string());
        } else if line == "end_of_record" {
            current = None;
        } else if let Some(file) = current.as_ref().and_then(|c| files.get_mut(c)) {
            if let Some(rest) = line.strip_prefix("DA:") {
                // `DA:<line>,<hits>[,<checksum>]`
                let mut cols = rest.split(',');
                if let (Some(Ok(number)), Some(hits)) =
                    (cols.next().map(str::parse::<u32>), cols.next())
                {
                    // Some tools write negative or fractional counts; any
                    // count above zero means the line ran.
                    let hits = hits
                        .trim()
                        .parse::<f64>()
                        .map_or(0, |h| h.max(0.0).ceil() as u64);
                    let entry = file.lines.entry(number).or_insert(0);
                    *entry = entry.saturating_add(hits);
                }
            } else if let Some(rest) = line.strip_prefix("LF:") {
                file.found = file.found.max(rest.trim().parse().unwrap_or(0));
            } else if let Some(rest) = line.strip_prefix("LH:") {
                file.hit = file.hit.max(rest.trim().parse().unwrap_or(0));
            }
        }
    }
    if files.is_empty() {
        return Err(Error::invalid("not an LCOV report: it has no `SF:` record"));
    }
    Ok(RawReport {
        format: Format::Lcov,
        sources: Vec::new(),
        files,
    })
}

fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let Some(semi) = rest.find(';').filter(|s| *s <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .map(|hex| u32::from_str_radix(hex, 16))
                .or_else(|| entity.strip_prefix('#').map(str::parse::<u32>))
                .and_then(|code| code.ok())
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// One XML tag: `<name a="b">`, `</name>` or `<name/>`.
struct Tag<'a> {
    name: &'a str,
    closing: bool,
    attrs: &'a str,
    /// Offset of the `<` and one past the `>`.
    start: usize,
    end: usize,
}

impl Tag<'_> {
    /// Value of attribute `key`, entities decoded.
    fn attr(&self, key: &str) -> Option<String> {
        let mut rest = self.attrs;
        loop {
            rest = rest.trim_start();
            let eq = rest.find('=')?;
            let name = rest[..eq].trim();
            let after = rest[eq + 1..].trim_start();
            let quote = after.chars().next().filter(|c| matches!(c, '"' | '\''))?;
            let close = after[1..].find(quote)?;
            if name == key {
                return Some(unescape(&after[1..1 + close]));
            }
            rest = &after[close + 2..];
        }
    }
}

/// Element tags of `text` in document order. Comments, processing
/// instructions, declarations and CDATA sections are skipped.
fn tags(text: &str) -> Vec<Tag<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(open) = text[i..].find('<').map(|p| i + p) {
        let rest = &text[open..];
        let skip_to = |end: &str| rest.find(end).map_or(text.len(), |p| open + p + end.len());
        if rest.starts_with("<!--") {
            i = skip_to("-->");
        } else if rest.starts_with("<![CDATA[") {
            i = skip_to("]]>");
        } else if rest.starts_with("<?") || rest.starts_with("<!") {
            i = skip_to(">");
        } else {
            // A `>` inside a quoted attribute value does not end the tag.
            let mut quote = None;
            let mut end = text.len();
            for (offset, c) in rest.char_indices().skip(1) {
                match (quote, c) {
                    (None, '"' | '\'') => quote = Some(c),
                    (Some(q), c) if q == c => quote = None,
                    (None, '>') => {
                        end = open + offset + 1;
                        break;
                    }
                    _ => {}
                }
            }
            let inner = text[open + 1..end]
                .trim_end_matches('>')
                .trim_end_matches('/');
            let closing = inner.starts_with('/');
            let inner = inner.trim_start_matches('/');
            let split = inner
                .find(|c: char| c.is_whitespace())
                .unwrap_or(inner.len());
            out.push(Tag {
                name: &inner[..split],
                closing,
                attrs: &inner[split..],
                start: open,
                end,
            });
            i = end;
        }
    }
    out
}

fn parse_cobertura(text: &str) -> Result<RawReport> {
    let mut files: BTreeMap<String, RawFile> = BTreeMap::new();
    let mut sources = Vec::new();
    let mut current: Option<String> = None;
    let mut source_from: Option<usize> = None;
    let mut is_cobertura = false;
    for tag in tags(text) {
        match (tag.name, tag.closing) {
            ("coverage", false) => is_cobertura = true,
            ("source", false) => source_from = Some(tag.end),
            ("source", true) => {
                if let Some(from) = source_from.take() {
                    let dir = unescape(text[from..tag.start].trim());
                    if !dir.is_empty() {
                        sources.push(dir);
                    }
                }
            }
            ("class", false) => {
                current = tag.attr("filename");
                if let Some(name) = &current {
                    files.entry(name.clone()).or_default();
                }
            }
            ("class", true) => current = None,
            // Lines appear under `class/lines` and again under each method;
            // keying by line number counts each once.
            ("line", false) => {
                let file = current.as_ref().and_then(|c| files.get_mut(c));
                let number = tag
                    .attr("number")
                    .and_then(|n| n.trim().parse::<u32>().ok());
                if let (Some(file), Some(number)) = (file, number) {
                    let hits = tag
                        .attr("hits")
                        .and_then(|h| h.trim().parse::<u64>().ok())
                        .unwrap_or(0);
                    let entry = file.lines.entry(number).or_insert(0);
                    *entry = (*entry).max(hits);
                }
            }
            _ => {}
        }
    }
    if !is_cobertura {
        return Err(Error::invalid(
            "not a Cobertura report: it has no `<coverage>` element",
        ));
    }
    Ok(RawReport {
        format: Format::Cobertura,
        sources,
        files,
    })
}

/// Parse a report, telling the format from its first character.
fn parse(text: &str) -> Result<RawReport> {
    let text = text.trim_start_matches('\u{feff}');
    if text.trim_start().starts_with('<') {
        parse_cobertura(text)
    } else {
        parse_lcov(text)
    }
}

fn normalize(path: &str) -> String {
    let path = path.trim().replace('\\', "/");
    let path = path.strip_prefix("//?/").unwrap_or(&path);
    path.trim_start_matches("./").to_string()
}

/// The indexed file a report path refers to.
///
/// Reports carry absolute paths, paths relative to the repository, or paths
/// relative to a package directory. The indexed path that shares the longest
/// run of trailing segments wins; a report path shorter than the indexed one
/// only matches when exactly one file ends with it.
fn resolve<'a>(candidates: &[String], known: &'a [String]) -> Option<&'a str> {
    let ends_with = |long: &str, short: &str| {
        long.len() > short.len()
            && long.ends_with(short)
            && long.as_bytes()[long.len() - short.len() - 1] == b'/'
    };
    // Candidates are tried in order: the most specific spelling comes first.
    for candidate in candidates {
        if let Ok(at) = known.binary_search(candidate) {
            return Some(&known[at]);
        }
        if let Some(best) = known
            .iter()
            .filter(|k| ends_with(candidate, k))
            .max_by_key(|k| k.len())
        {
            return Some(best);
        }
    }
    for candidate in candidates {
        let mut longer = known.iter().filter(|k| ends_with(k, candidate));
        if let (Some(only), None) = (longer.next(), longer.next()) {
            return Some(only);
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileCoverage {
    pub path: String,
    /// Lines the report instruments.
    pub lines_found: u32,
    /// Instrumented lines that ran at least once.
    pub lines_hit: u32,
    /// 0.0 to 100.0.
    pub percent: f64,
    /// Inclusive `[first, last]` runs of lines that never ran.
    pub uncovered: Vec<[u32; 2]>,
}

/// Coverage as imported, with paths matched to the index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageData {
    pub format: Format,
    /// Where the report came from: a path, or `inline` for posted text.
    pub source: String,
    /// By path.
    pub files: Vec<FileCoverage>,
    /// Report entries that match no indexed file (dependencies, generated code).
    pub unmatched: usize,
}

fn percent(hit: u32, found: u32) -> f64 {
    if found == 0 {
        return 100.0;
    }
    (hit as f64 / found as f64 * 1000.0).round() / 10.0
}

fn uncovered_runs(lines: &BTreeMap<u32, u64>) -> Vec<[u32; 2]> {
    let mut runs: Vec<[u32; 2]> = Vec::new();
    let mut open = false;
    for (line, hits) in lines {
        if *hits > 0 {
            open = false;
        } else if open && let Some(run) = runs.last_mut() {
            // Lines between two unexecuted lines that the report does not
            // instrument (blank lines, comments) stay inside the run.
            run[1] = *line;
        } else {
            runs.push([*line, *line]);
            open = true;
        }
    }
    runs
}

/// Parse report `text` and match its files to the index.
pub fn import_text(index: &Index, text: &str, source: &str) -> Result<CoverageData> {
    let raw = parse(text)?;
    let mut known: Vec<String> = index.files()?.into_iter().map(|f| f.path).collect();
    known.sort();
    let bases: Vec<String> = raw.sources.iter().map(|s| normalize(s)).collect();
    let mut merged: BTreeMap<&str, RawFile> = BTreeMap::new();
    let mut unmatched = 0;
    for (name, file) in &raw.files {
        let name = normalize(name);
        let mut candidates: Vec<String> = bases
            .iter()
            .filter(|_| !name.starts_with('/') && !name.contains(":/"))
            .map(|base| format!("{}/{name}", base.trim_end_matches('/')))
            .collect();
        candidates.push(name);
        let Some(path) = resolve(&candidates, &known) else {
            unmatched += 1;
            continue;
        };
        let entry = merged.entry(path).or_default();
        for (line, hits) in &file.lines {
            let slot = entry.lines.entry(*line).or_insert(0);
            *slot = slot.saturating_add(*hits);
        }
        entry.found = entry.found.max(file.found);
        entry.hit = entry.hit.max(file.hit);
    }
    let files = merged
        .into_iter()
        .filter_map(|(path, file)| {
            let (found, hit) = if file.lines.is_empty() {
                (file.found, file.hit.min(file.found))
            } else {
                (
                    file.lines.len() as u32,
                    file.lines.values().filter(|h| **h > 0).count() as u32,
                )
            };
            (found > 0).then(|| FileCoverage {
                path: path.to_string(),
                lines_found: found,
                lines_hit: hit,
                percent: percent(hit, found),
                uncovered: uncovered_runs(&file.lines),
            })
        })
        .collect();
    Ok(CoverageData {
        format: raw.format,
        source: source.to_string(),
        files,
        unmatched,
    })
}

/// Read and import a report file. With no `report`, the first of
/// [`DEFAULT_REPORTS`] that exists under `root` is used.
pub fn import(root: &Path, index: &Index, report: Option<&Path>) -> Result<CoverageData> {
    let path = match report {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => root.join(path),
        None => DEFAULT_REPORTS
            .iter()
            .map(|rel| root.join(rel))
            .find(|p| p.is_file())
            .ok_or_else(|| {
                Error::not_found(format!(
                    "no coverage report; looked for {} in the repository",
                    DEFAULT_REPORTS.join(", ")
                ))
            })?,
    };
    let text = std::fs::read_to_string(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            Error::not_found(format!("coverage report {}", path.display()))
        }
        _ => Error::io(&path, e),
    })?;
    let label = path
        .strip_prefix(root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
    import_text(index, &text, &label)
}

/// Cache imported coverage in the index.
pub fn save(index: &Index, data: &CoverageData) -> Result<()> {
    let json = serde_json::to_string(data).map_err(|e| Error::external("encoding coverage", e))?;
    index.set_meta(META_KEY, &json)
}

/// Coverage cached in the index, if any was imported.
pub fn load(index: &Index) -> Result<Option<CoverageData>> {
    Ok(index
        .meta(META_KEY)?
        .and_then(|json| serde_json::from_str(&json).ok()))
}

/// A poorly covered file and why it matters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gap {
    pub path: String,
    pub percent: f64,
    pub uncovered_lines: u32,
    /// Hotspot score of the file, 0 to 100; 0 without history.
    pub hotspot: f64,
    /// Share of the highest file rank in the dependency graph, 0.0 to 1.0.
    pub rank: f64,
    /// Priority, 0 to 100: uncovered share weighted by hotspot and rank.
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageReport {
    pub format: Format,
    pub source: String,
    pub lines_found: u64,
    pub lines_hit: u64,
    /// Repository line coverage, 0.0 to 100.0.
    pub percent: f64,
    /// Every file in the report, least covered first.
    pub files: Vec<FileCoverage>,
    /// The gaps most worth closing, highest priority first.
    pub least_covered: Vec<Gap>,
    /// Source files with functions that the report does not mention, largest
    /// first.
    pub not_in_report: Vec<String>,
    pub unmatched: usize,
}

/// Uncovered lines at which a file's size stops adding to its priority.
const FULL_SIZE_LINES: f64 = 50.0;
const MAX_NOT_IN_REPORT: usize = 50;

/// Summarise imported coverage and rank the gaps.
pub fn report(index: &Index, data: &CoverageData, limit: usize) -> Result<CoverageReport> {
    let intel = gitintel::load(index)?;
    let hot: HashMap<String, f64> = gitintel::hotspots(index, &intel, usize::MAX)?
        .into_iter()
        .map(|h| (h.path, h.score))
        .collect();
    let symbols = index.symbols()?;
    // Files of declarations and re-exports have no lines to cover.
    let with_code: HashSet<&str> = symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
        .map(|s| s.file.as_str())
        .collect();
    let file_of: HashMap<&str, &str> = symbols
        .iter()
        .map(|s| (s.qualified.as_str(), s.file.as_str()))
        .collect();
    let mut rank: HashMap<&str, f64> = HashMap::new();
    for (node, metrics) in index.metrics()? {
        if let Some(file) = file_of.get(node.as_str()) {
            *rank.entry(file).or_insert(0.0) += metrics.pagerank;
        }
    }
    let top_rank = rank.values().copied().fold(0.0f64, f64::max);

    let mut least_covered: Vec<Gap> = data
        .files
        .iter()
        .filter(|f| f.lines_hit < f.lines_found)
        .map(|f| {
            let uncovered = f.lines_found - f.lines_hit;
            let hotspot = hot.get(&f.path).copied().unwrap_or(0.0);
            let rank = if top_rank > 0.0 {
                rank.get(f.path.as_str()).copied().unwrap_or(0.0) / top_rank
            } else {
                0.0
            };
            let share = uncovered as f64 / f.lines_found as f64;
            let size = ((uncovered as f64).ln_1p() / FULL_SIZE_LINES.ln_1p()).min(1.0);
            // A file with no history and no dependents keeps a fifth of the weight.
            let weight = (1.0 + 2.0 * hotspot / 100.0 + 2.0 * rank) / 5.0;
            Gap {
                path: f.path.clone(),
                percent: f.percent,
                uncovered_lines: uncovered,
                hotspot,
                rank: (rank * 1000.0).round() / 1000.0,
                score: (share * size * weight * 1000.0).round() / 10.0,
            }
        })
        .collect();
    least_covered.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(b.uncovered_lines.cmp(&a.uncovered_lines))
            .then(a.path.cmp(&b.path))
    });
    least_covered.truncate(limit);

    let mut files = data.files.clone();
    files.sort_by(|a, b| a.percent.total_cmp(&b.percent).then(a.path.cmp(&b.path)));
    let mut absent: Vec<cw_core::FileRecord> = index
        .files()?
        .into_iter()
        .filter(|f| f.language.has_symbol_support() && !is_test_path(&f.path))
        .filter(|f| !f.path.ends_with("build.rs") && with_code.contains(f.path.as_str()))
        .filter(|f| data.files.iter().all(|c| c.path != f.path))
        .collect();
    absent.sort_by(|a, b| b.lines.cmp(&a.lines).then(a.path.cmp(&b.path)));
    let lines_found: u64 = data.files.iter().map(|f| f.lines_found as u64).sum();
    let lines_hit: u64 = data.files.iter().map(|f| f.lines_hit as u64).sum();
    Ok(CoverageReport {
        format: data.format,
        source: data.source.clone(),
        lines_found,
        lines_hit,
        percent: if lines_found == 0 {
            0.0
        } else {
            (lines_hit as f64 / lines_found as f64 * 1000.0).round() / 10.0
        },
        files,
        least_covered,
        not_in_report: absent
            .into_iter()
            .take(MAX_NOT_IN_REPORT)
            .map(|f| f.path)
            .collect(),
        unmatched: data.unmatched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lcov_records() {
        let text = "TN:unit\nSF:/work/repo/src/a.rs\nFN:1,main\nDA:1,3\nDA:2,0,abc123\nDA:3,0\nDA:5,1\nLF:4\nLH:2\nend_of_record\nSF:src\\b.rs\nLF:10\nLH:4\nend_of_record\nTN:other\nSF:/work/repo/src/a.rs\nDA:2,2\nend_of_record\n";
        let raw = parse(text).unwrap();
        assert_eq!(raw.format, Format::Lcov);
        let a = &raw.files["/work/repo/src/a.rs"];
        // The second record for the same file adds to the first.
        assert_eq!(a.lines, BTreeMap::from([(1, 3), (2, 2), (3, 0), (5, 1)]));
        let b = &raw.files["src\\b.rs"];
        assert!(b.lines.is_empty());
        assert_eq!((b.found, b.hit), (10, 4));
        assert!(parse("just some text\n").is_err());
    }

    #[test]
    fn parses_cobertura_and_counts_method_lines_once() {
        let text = r#"<?xml version="1.0" ?>
<!DOCTYPE coverage SYSTEM "http://cobertura.sourceforge.net/xml/coverage-04.dtd">
<!-- <class filename="commented/out.rs"> -->
<coverage line-rate="0.5" version="1">
  <sources><source>/work/repo/crates/a</source><source> C:\work &amp; play </source></sources>
  <packages><package name="a">
    <classes>
      <class name="lib" filename="src/lib.rs" line-rate="0.5">
        <methods><method name="f &gt; g" signature="()">
          <lines><line number="1" hits="4"/></lines>
        </method></methods>
        <lines>
          <line number="1" hits="4" branch="false"/>
          <line hits='0' number='2'/>
          <line number="x" hits="1"/>
        </lines>
      </class>
      <class name="empty" filename="src/empty.rs"><lines/></class>
    </classes>
  </package></packages>
</coverage>"#;
        let raw = parse(text).unwrap();
        assert_eq!(raw.format, Format::Cobertura);
        assert_eq!(raw.sources, ["/work/repo/crates/a", "C:\\work & play"]);
        assert_eq!(
            raw.files["src/lib.rs"].lines,
            BTreeMap::from([(1, 4), (2, 0)])
        );
        assert!(raw.files["src/empty.rs"].lines.is_empty());
        assert!(!raw.files.contains_key("commented/out.rs"));
        assert!(parse("<html><body/></html>").is_err());
    }

    #[test]
    fn decodes_entities_and_reads_attributes() {
        assert_eq!(
            unescape("a &lt;b&gt; &amp;&quot;&apos; &#65;&#x42; &bogus; &"),
            "a <b> &\"' AB &bogus; &"
        );
        let all = tags("<a x=\"1 > 0\" y='two'/><b></b>");
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].attr("x").as_deref(), Some("1 > 0"));
        assert_eq!(all[0].attr("y").as_deref(), Some("two"));
        assert_eq!(all[0].attr("z"), None);
        assert!(all[2].closing && all[2].name == "b");
    }

    #[test]
    fn matches_report_paths_to_indexed_files() {
        let known: Vec<String> = [
            "crates/a/src/lib.rs",
            "crates/b/src/lib.rs",
            "src/lib.rs",
            "src/only.rs",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let find = |paths: &[&str]| {
            let candidates: Vec<String> = paths.iter().map(|p| normalize(p)).collect();
            resolve(&candidates, &known)
        };
        assert_eq!(find(&["src/lib.rs"]), Some("src/lib.rs"));
        assert_eq!(find(&["./src/lib.rs"]), Some("src/lib.rs"));
        // Absolute paths match the longest indexed suffix.
        assert_eq!(
            find(&["C:\\work\\repo\\crates\\a\\src\\lib.rs"]),
            Some("crates/a/src/lib.rs")
        );
        assert_eq!(find(&["/work/repo/src/lib.rs"]), Some("src/lib.rs"));
        // A Cobertura source directory disambiguates a package-relative name.
        assert_eq!(
            find(&["/work/repo/crates/b/src/lib.rs", "src/lib.rs"]),
            Some("crates/b/src/lib.rs")
        );
        // Package-relative with no base: only a unique ending matches.
        assert_eq!(find(&["only.rs"]), Some("src/only.rs"));
        assert_eq!(find(&["lib.rs"]), None);
        assert_eq!(find(&["/home/u/.cargo/registry/serde/src/de.rs"]), None);
        // A file name must match whole segments.
        assert_eq!(find(&["/work/repo/xsrc/only.rs"]), None);
    }

    #[test]
    fn groups_unexecuted_lines_into_runs() {
        let lines = BTreeMap::from([(1, 1), (2, 0), (3, 0), (6, 0), (7, 2), (9, 0)]);
        assert_eq!(uncovered_runs(&lines), [[2, 6], [9, 9]]);
        assert!(uncovered_runs(&BTreeMap::from([(1, 1)])).is_empty());
        assert_eq!(percent(1, 3), 33.3);
        assert_eq!(percent(0, 0), 100.0);
    }
}
