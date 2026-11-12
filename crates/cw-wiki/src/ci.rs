//! Findings for continuous integration, as text, SARIF or JUnit.
//!
//! `check` gathers what the other analyses report (dead code, file health,
//! documentation drift, stale decisions, security patterns) into one list a
//! CI system can gate on or annotate a pull request with.

use std::path::Path;

use cw_core::Result;
use cw_index::Index;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::decisions;
use crate::health::{self, Certainty, DriftKind};
pub use crate::level::Level;
use crate::security;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckFinding {
    /// Stable rule id, for example `dead-code` or `doc-drift/broken-link`.
    pub rule: String,
    pub level: Level,
    pub message: String,
    /// Repo-relative file the finding is about.
    pub file: String,
    /// 1-based line, or 0 when the finding is about the whole file.
    pub line: u32,
}

/// What each rule means, shown by tools that list rules.
pub(crate) const RULES: [(&str, &str); 22] = [
    (
        "dead-code",
        "A private symbol that nothing in the repository refers to.",
    ),
    (
        "health/long-file",
        "A source file long enough to be hard to hold in mind.",
    ),
    (
        "health/long-function",
        "A function long enough to hide several responsibilities.",
    ),
    ("health/large-type", "A type with many methods."),
    (
        "health/high-fan-out",
        "A file that depends on many other files.",
    ),
    (
        "health/mutual-dependency",
        "Two files that depend on each other.",
    ),
    (
        "health/many-parameters",
        "A function that takes many parameters.",
    ),
    ("health/wide-api", "A file that exposes many public items."),
    (
        "health/undocumented-api",
        "Public items without doc comments.",
    ),
    (
        "doc-drift/missing-source",
        "A note cites a source file that no longer exists.",
    ),
    (
        "doc-drift/source-changed",
        "A hand-edited note cites code that changed since its last review.",
    ),
    (
        "doc-drift/dead-path",
        "A note names a path that is not in the repository.",
    ),
    ("doc-drift/broken-link", "A wikilink that matches no note."),
    (
        "decision/stale",
        "A decision record cites code that changed after it was last updated.",
    ),
    (
        security::UNSAFE_BLOCK,
        "An `unsafe` block outside test code.",
    ),
    (
        security::HARDCODED_SECRET,
        "A credential written into the source as a string literal.",
    ),
    (
        security::COMMAND_INJECTION,
        "A shell command line or program name built with `format!`.",
    ),
    (
        security::SQL_FORMAT,
        "SQL text built with `format!` instead of bound parameters.",
    ),
    (
        security::TLS_DISABLED,
        "TLS certificate or hostname verification is turned off.",
    ),
    (
        security::ENV_SET_VAR,
        "The process environment is changed at run time.",
    ),
    (
        security::WORLD_WRITABLE,
        "A file mode that lets every user write.",
    ),
    (
        security::INSECURE_HTTP,
        "A plain HTTP URL to a host that is not local.",
    ),
];

/// Run every analysis and return findings sorted by file, line and rule.
pub fn check(root: &Path, index: &Index) -> Result<Vec<CheckFinding>> {
    let mut out = Vec::new();
    for dead in health::dead_code(index)? {
        if dead.certainty == Certainty::Likely {
            out.push(CheckFinding {
                rule: "dead-code".into(),
                level: Level::Warning,
                message: format!("`{}` ({}) is {}", dead.qualified, dead.kind, dead.reason),
                file: dead.file,
                line: dead.line,
            });
        }
    }
    for file in health::file_health(index)? {
        for finding in file.findings {
            // Dead code is reported per symbol above.
            if finding.detector == "dead-code" {
                continue;
            }
            out.push(CheckFinding {
                rule: format!("health/{}", finding.detector),
                level: if finding.penalty >= 2 {
                    Level::Warning
                } else {
                    Level::Note
                },
                message: finding.message,
                file: file.path.clone(),
                line: 0,
            });
        }
    }
    for drift in health::doc_drift(root, index)?.drift {
        let (rule, level) = match drift.kind {
            DriftKind::MissingSource => ("doc-drift/missing-source", Level::Warning),
            DriftKind::SourceChanged => ("doc-drift/source-changed", Level::Note),
            DriftKind::DeadPath => ("doc-drift/dead-path", Level::Warning),
            DriftKind::BrokenLink => ("doc-drift/broken-link", Level::Error),
        };
        out.push(CheckFinding {
            rule: rule.into(),
            level,
            message: drift.detail,
            file: format!("{}/{}", cw_core::VAULT_DIR, drift.note),
            line: 0,
        });
    }
    for decision in decisions::list(root, index)? {
        if decision.stale {
            out.push(CheckFinding {
                rule: "decision/stale".into(),
                level: Level::Warning,
                message: format!("{}: {}", decision.title, decision.stale_reasons.join("; ")),
                file: decision.path,
                line: 0,
            });
        }
    }
    for finding in security::scan(root, index)? {
        out.push(CheckFinding {
            rule: finding.rule,
            level: finding.level,
            message: finding.message,
            file: finding.file,
            line: finding.line,
        });
    }
    out.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.rule.cmp(&b.rule))
            .then(a.message.cmp(&b.message))
    });
    Ok(out)
}

/// True when any finding is at or above `threshold`.
pub fn fails(findings: &[CheckFinding], threshold: Level) -> bool {
    findings.iter().any(|f| f.level >= threshold)
}

pub fn to_text(findings: &[CheckFinding]) -> String {
    let mut out = String::new();
    for f in findings {
        let location = if f.line > 0 {
            format!("{}:{}", f.file, f.line)
        } else {
            f.file.clone()
        };
        out.push_str(&format!(
            "{}: {location}: [{}] {}\n",
            f.level.as_str(),
            f.rule,
            f.message
        ));
    }
    let count = |level| findings.iter().filter(|f| f.level == level).count();
    out.push_str(&format!(
        "{} errors, {} warnings, {} notes\n",
        count(Level::Error),
        count(Level::Warning),
        count(Level::Note)
    ));
    out
}

/// SARIF 2.1.0, the format code-scanning services ingest.
pub fn to_sarif(findings: &[CheckFinding]) -> Value {
    let rules: Vec<Value> = RULES
        .iter()
        .map(|(id, text)| json!({ "id": id, "shortDescription": { "text": text } }))
        .collect();
    let results: Vec<Value> = findings
        .iter()
        .map(|f| {
            let mut location = json!({ "artifactLocation": { "uri": f.file } });
            if f.line > 0 {
                location["region"] = json!({ "startLine": f.line });
            }
            json!({
                "ruleId": f.rule,
                "level": f.level.as_str(),
                "message": { "text": f.message },
                "locations": [{ "physicalLocation": location }],
            })
        })
        .collect();
    json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "codewiki",
                "version": cw_core::VERSION,
                "rules": rules,
            }},
            "results": results,
        }],
    })
}

fn xml_escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            // Control characters are not allowed in XML 1.0.
            c if (c as u32) < 0x20 && !matches!(c, '\n' | '\t') => " ".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// JUnit XML: one test case per rule, failing when the rule has findings at
/// or above `threshold`. Rules with no findings pass, so the report shows
/// what was checked.
pub fn to_junit(findings: &[CheckFinding], threshold: Level) -> String {
    let mut cases = String::new();
    let mut failed = 0;
    for (rule, description) in RULES {
        let hits: Vec<&CheckFinding> = findings.iter().filter(|f| f.rule == rule).collect();
        let blocking = hits.iter().any(|f| f.level >= threshold);
        cases.push_str(&format!(
            "  <testcase classname=\"codewiki\" name=\"{}\">",
            xml_escape(rule)
        ));
        if blocking {
            failed += 1;
            let detail: Vec<String> = hits
                .iter()
                .map(|f| {
                    if f.line > 0 {
                        format!("{}:{}: {}", f.file, f.line, f.message)
                    } else {
                        format!("{}: {}", f.file, f.message)
                    }
                })
                .collect();
            cases.push_str(&format!(
                "\n    <failure message=\"{} findings: {}\">{}</failure>\n  ",
                hits.len(),
                xml_escape(description),
                xml_escape(&detail.join("\n"))
            ));
        }
        cases.push_str("</testcase>\n");
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"codewiki check\" tests=\"{}\" failures=\"{failed}\">\n{cases}</testsuite>\n",
        RULES.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings() -> Vec<CheckFinding> {
        vec![
            CheckFinding {
                rule: "dead-code".into(),
                level: Level::Warning,
                message: "`demo::old` (function) is private & never <used>".into(),
                file: "src/lib.rs".into(),
                line: 12,
            },
            CheckFinding {
                rule: "health/long-file".into(),
                level: Level::Note,
                message: "700 lines".into(),
                file: "src/big.rs".into(),
                line: 0,
            },
            CheckFinding {
                rule: security::TLS_DISABLED.into(),
                level: Level::Error,
                message: "`danger_accept_invalid_certs(true)` turns off TLS certificate checks"
                    .into(),
                file: "src/net.rs".into(),
                line: 40,
            },
        ]
    }

    #[test]
    fn threshold_decides_failure() {
        let f = findings();
        assert!(fails(&f, Level::Note) && fails(&f, Level::Warning));
        assert!(fails(&f, Level::Error));
        assert!(!fails(&f[..2], Level::Error));
        assert!(!fails(&[], Level::Note));
        assert_eq!(Level::parse("warning"), Some(Level::Warning));
        assert_eq!(Level::parse("loud"), None);
    }

    #[test]
    fn text_lists_findings_and_totals() {
        let text = to_text(&findings());
        assert!(text.contains("warning: src/lib.rs:12: [dead-code]"));
        assert!(text.contains("note: src/big.rs: [health/long-file] 700 lines"));
        assert!(text.contains("error: src/net.rs:40: [security/tls-verification-disabled]"));
        assert!(text.ends_with("1 errors, 1 warnings, 1 notes\n"));
    }

    #[test]
    fn sarif_has_the_required_shape() {
        let sarif = to_sarif(&findings());
        assert_eq!(sarif["version"], "2.1.0");
        let run = &sarif["runs"][0];
        assert_eq!(run["tool"]["driver"]["name"], "codewiki");
        assert_eq!(
            run["tool"]["driver"]["rules"].as_array().unwrap().len(),
            RULES.len()
        );
        let first = &run["results"][0];
        assert_eq!(first["ruleId"], "dead-code");
        assert_eq!(first["level"], "warning");
        let location = &first["locations"][0]["physicalLocation"];
        assert_eq!(location["artifactLocation"]["uri"], "src/lib.rs");
        assert_eq!(location["region"]["startLine"], 12);
        // A whole-file finding carries no region.
        assert!(run["results"][1]["locations"][0]["physicalLocation"]["region"].is_null());
        // Every reported rule id is declared.
        let declared: Vec<&str> = RULES.iter().map(|(id, _)| *id).collect();
        assert!(
            findings()
                .iter()
                .all(|f| declared.contains(&f.rule.as_str()))
        );
    }

    #[test]
    fn junit_fails_only_rules_at_the_threshold_and_escapes_text() {
        let xml = to_junit(&findings(), Level::Warning);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"codewiki check\" tests=\"22\" failures=\"2\">"));
        assert!(xml.contains(
            "<testcase classname=\"codewiki\" name=\"security/tls-verification-disabled\">\n    <failure message=\"1 findings: TLS certificate or hostname verification is turned off.\">src/net.rs:40:"
        ));
        assert!(xml.contains("private &amp; never &lt;used&gt;"));
        assert!(!xml.contains("<used>"));
        // The note-level rule passes at a warning threshold.
        assert!(
            xml.contains("<testcase classname=\"codewiki\" name=\"health/long-file\"></testcase>")
        );
        assert!(to_junit(&findings(), Level::Note).contains("failures=\"3\""));
        // Rule ids are unique, so each is one test case.
        let mut ids: Vec<&str> = RULES.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), RULES.len());
        assert!(to_junit(&[], Level::Note).contains("failures=\"0\""));
    }
}
