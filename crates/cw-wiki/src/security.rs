//! Security pattern findings for Rust source.
//!
//! Each rule is a narrow lexical pattern over code with comments and string
//! contents masked out ([`crate::rustscan`]), so a pattern never fires on
//! prose or on text inside a literal. Test code is skipped: files under
//! `tests/`, `benches/` and `examples/`, and items behind `#[cfg(test)]` or
//! `#[test]`. The rules favour silence over recall; they flag what is worth a
//! reviewer's look, not everything that could go wrong.
//!
//! A finding is silenced by a comment on its line or the line above:
//! `// codewiki-allow: security/<rule>`, or `// codewiki-allow` for any rule.

use std::path::Path;

use cw_core::{Language, Result};
use cw_index::Index;
use serde::{Deserialize, Serialize};

use crate::level::Level;
use crate::rustscan::{Literal, Scanned, is_test_path};

/// An `unsafe { … }` block. `unsafe fn`, `impl` and `trait` are declarations
/// and are not reported.
pub const UNSAFE_BLOCK: &str = "security/unsafe-block";
/// A recognisable credential, or a high-entropy literal assigned to a name
/// such as `token`, `secret`, `password` or `api_key`.
pub const HARDCODED_SECRET: &str = "security/hardcoded-secret";
/// `Command::new` on a shell with a `format!`-built command line, or with a
/// `format!`-built program name.
pub const COMMAND_INJECTION: &str = "security/command-injection";
/// A SQL statement passed through `format!` with interpolated values.
pub const SQL_FORMAT: &str = "security/sql-format";
/// Certificate or hostname verification switched off on a TLS client.
pub const TLS_DISABLED: &str = "security/tls-verification-disabled";
/// `std::env::set_var` or `remove_var`.
pub const ENV_SET_VAR: &str = "security/env-set-var";
/// A Unix file mode whose "other" digit includes write.
pub const WORLD_WRITABLE: &str = "security/world-writable";
/// An `http://` URL literal whose host is not local, private or reserved.
pub const INSECURE_HTTP: &str = "security/insecure-http";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityFinding {
    /// Stable rule id, for example `security/hardcoded-secret`.
    pub rule: String,
    pub level: Level,
    /// Repo-relative file.
    pub file: String,
    /// 1-based line.
    pub line: u32,
    pub message: String,
}

/// A finding before its offset is turned into a line.
struct Hit {
    rule: &'static str,
    level: Level,
    offset: usize,
    message: String,
}

/// Text inside the bracket that opens at or after `from`, when only
/// whitespace separates the two.
fn call_args(s: &Scanned, from: usize) -> Option<(usize, &str)> {
    let rest = &s.code[from..];
    let open = from + (rest.len() - rest.trim_start().len());
    if !matches!(s.code.as_bytes().get(open), Some(b'(' | b'[' | b'{')) {
        return None;
    }
    let close = s.matching(open);
    Some((open, s.code.get(open + 1..close.saturating_sub(1))?))
}

/// The string literal that starts the text at `from`, allowing a `&` and a
/// raw or byte prefix.
fn leading_literal(s: &Scanned, from: usize) -> Option<&Literal> {
    let rest = &s.code[from..];
    let quote = rest.find('"')?;
    rest[..quote]
        .chars()
        .all(|c| c.is_whitespace() || matches!(c, '&' | 'r' | 'b' | 'c' | '#'))
        .then(|| s.literal_at(from + quote))
        .flatten()
}

fn unsafe_blocks(s: &Scanned, out: &mut Vec<Hit>) {
    for pos in s.find("unsafe") {
        if s.code[pos + 6..].trim_start().starts_with('{') {
            out.push(Hit {
                rule: UNSAFE_BLOCK,
                level: Level::Note,
                offset: pos,
                message: "`unsafe` block; check that its safety conditions are stated and hold"
                    .into(),
            });
        }
    }
}

const SECRET_WORDS: [&str; 10] = [
    "token",
    "secret",
    "password",
    "passwd",
    "api_key",
    "apikey",
    "private_key",
    "access_key",
    "auth_key",
    "credential",
];

/// Name endings that say the value names a secret rather than being one.
const NOT_A_SECRET_SUFFIXES: [&str; 14] = [
    "_env", "_var", "_name", "_header", "_file", "_path", "_url", "_field", "_prefix", "_label",
    "_param", "_id", "_type", "_kind",
];

const PLACEHOLDERS: [&str; 9] = [
    "example",
    "placeholder",
    "changeme",
    "dummy",
    "sample",
    "your",
    "xxxx",
    "redacted",
    "****",
];

fn names_a_secret(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SECRET_WORDS.iter().any(|w| lower.contains(w))
        && !NOT_A_SECRET_SUFFIXES.iter().any(|s| lower.ends_with(s))
}

/// Shannon entropy in bits per byte.
fn entropy(text: &str) -> f64 {
    let mut counts = [0u32; 256];
    for byte in text.bytes() {
        counts[byte as usize] += 1;
    }
    let total = text.len() as f64;
    counts
        .iter()
        .filter(|c| **c > 0)
        .map(|c| {
            let p = *c as f64 / total;
            -p * p.log2()
        })
        .sum()
}

/// Minimum length and entropy for a value to read as generated rather than
/// written. English words sit near 2.5 to 3 bits per character; hex and
/// base64 keys of 16 characters or more sit above 3.5.
const MIN_SECRET_LEN: usize = 16;
const MIN_SECRET_ENTROPY: f64 = 3.5;

fn looks_generated(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    (MIN_SECRET_LEN..=512).contains(&value.len())
        && value.bytes().all(|b| b.is_ascii_graphic())
        && value.bytes().any(|b| b.is_ascii_digit())
        && value.bytes().any(|b| b.is_ascii_alphabetic())
        && !value.contains(['{', '<', '$'])
        && !value.contains("://")
        && !value.starts_with(['/', '.'])
        && !PLACEHOLDERS.iter().any(|p| lower.contains(p))
        // `AWS_SECRET_KEY_2` is the name of a variable, not a key.
        && !(value.contains('_')
            && value
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'))
        && entropy(value) >= MIN_SECRET_ENTROPY
}

/// Credential formats that are recognisable whatever they are assigned to.
fn known_credential(value: &str) -> Option<&'static str> {
    let token = |prefix: &str, min: usize| {
        value.starts_with(prefix)
            && value.len() >= min
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    };
    if value.contains("-----BEGIN ") && value.contains("PRIVATE KEY-----") {
        Some("a private key")
    } else if value.len() == 20
        && value.starts_with("AKIA")
        && value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        Some("an AWS access key id")
    } else if ["ghp_", "gho_", "ghs_", "ghu_", "github_pat_"]
        .iter()
        .any(|p| token(p, 36))
    {
        Some("a GitHub token")
    } else if ["xoxb-", "xoxp-", "xoxa-"].iter().any(|p| token(p, 24)) {
        Some("a Slack token")
    } else {
        None
    }
}

/// Name a literal is assigned to: `let NAME = "…"`, `const NAME: &str = "…"`,
/// `self.NAME = "…"` or the struct field `NAME: "…"`.
fn assigned_name(code: &str, open: usize) -> Option<&str> {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    let mut before = code[..open].trim_end_matches('#');
    for prefix in ["br", "cr", "r", "b", "c"] {
        if let Some(rest) = before.strip_suffix(prefix)
            && !rest.ends_with(is_ident)
        {
            before = rest;
            break;
        }
    }
    loop {
        let trimmed = before.trim_end();
        let stripped = ["&", "String::from(", "Some("]
            .iter()
            .find_map(|wrap| trimmed.strip_suffix(wrap))
            .unwrap_or(trimmed);
        if stripped.len() == before.len() {
            break;
        }
        before = stripped;
    }
    let (head, is_field) = if let Some(head) = before.strip_suffix('=') {
        if head.ends_with(['=', '!', '<', '>', '+', '-', '*', '/', '%', '^', '|', '&']) {
            return None;
        }
        (head, false)
    } else {
        let head = before.strip_suffix(':')?;
        if head.ends_with(':') {
            return None;
        }
        (head, true)
    };
    let start = head.rfind([';', '{', '}', '(', ',']).map_or(0, |p| p + 1);
    let mut segment = &head[start..];
    if !is_field {
        // Drop a type annotation: the first `:` that is not part of `::`.
        let bytes = segment.as_bytes();
        if let Some(colon) = (0..bytes.len()).find(|i| {
            bytes[*i] == b':'
                && bytes.get(i + 1) != Some(&b':')
                && (*i == 0 || bytes[i - 1] != b':')
        }) {
            segment = &segment[..colon];
        }
    }
    let segment = segment.trim_end();
    let name_start = segment
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident(*c))
        .last()
        .map(|(i, _)| i)?;
    Some(&segment[name_start..])
}

fn hardcoded_secrets(s: &Scanned, out: &mut Vec<Hit>) {
    for literal in &s.literals {
        let message = if let Some(what) = known_credential(&literal.value) {
            format!(
                "string literal looks like {what}; load it from the environment or a secret store"
            )
        } else if let Some(name) = assigned_name(&s.code, literal.open)
            && names_a_secret(name)
            && looks_generated(&literal.value)
        {
            format!(
                "`{name}` is assigned a high-entropy string literal; load it from the environment or a secret store"
            )
        } else {
            continue;
        };
        out.push(Hit {
            rule: HARDCODED_SECRET,
            level: Level::Error,
            offset: literal.open,
            message,
        });
    }
}

const SHELLS: [&str; 9] = [
    "sh",
    "bash",
    "zsh",
    "dash",
    "ksh",
    "fish",
    "cmd",
    "powershell",
    "pwsh",
];

fn is_shell(program: &str) -> bool {
    let name = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    SHELLS.contains(&name.strip_suffix(".exe").unwrap_or(&name))
}

/// True when `name` is bound by `let name = format!(…)` earlier in `code`.
fn bound_to_format(s: &Scanned, before: usize, name: &str) -> bool {
    s.find(name).take_while(|pos| *pos < before).any(|pos| {
        let head = s.code[..pos].trim_end();
        let declared = head.ends_with("let") || head.ends_with("let mut");
        let value = s.code[pos + name.len()..].trim_start();
        declared
            && value.strip_prefix('=').is_some_and(|v| {
                v.trim_start()
                    .trim_start_matches('&')
                    .starts_with("format!")
            })
    })
}

fn command_injection(s: &Scanned, out: &mut Vec<Hit>) {
    for pos in s.find("Command::new") {
        let Some((open, args)) = call_args(s, pos + "Command::new".len()) else {
            continue;
        };
        let program = args.trim_start().trim_start_matches('&');
        if program.starts_with("format!") {
            out.push(Hit {
                rule: COMMAND_INJECTION,
                level: Level::Warning,
                offset: pos,
                message: "program name is built with `format!`; a caller-controlled value picks what runs".into(),
            });
            continue;
        }
        let Some(shell) = leading_literal(s, open + 1).filter(|l| is_shell(&l.value)) else {
            continue;
        };
        let close = s.matching(open);
        let chain = &s.code[close..s.statement_end(close)];
        let mut interpolated = chain.contains("format!");
        for (at, _) in chain.match_indices(".arg") {
            let Some((_, inner)) = call_args(s, close + at + 4) else {
                continue;
            };
            let name = inner.trim().trim_start_matches('&').trim();
            let is_name = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
            interpolated |= is_name && bound_to_format(s, pos, name);
        }
        if interpolated {
            out.push(Hit {
                rule: COMMAND_INJECTION,
                level: Level::Warning,
                offset: pos,
                message: format!(
                    "`{}` runs a command line built with `format!`; pass the program and each argument separately instead",
                    shell.value
                ),
            });
        }
    }
}

/// Does the text start like a SQL statement with a table clause?
fn is_sql(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    let Some(first) = words.first() else {
        return false;
    };
    // Statements are written `SELECT` or `select`; `Select` starts a sentence.
    let upper = first.to_ascii_uppercase();
    if *first != upper && *first != first.to_ascii_lowercase() {
        return false;
    }
    let has = |word: &str| words.iter().skip(1).any(|w| w.eq_ignore_ascii_case(word));
    match upper.as_str() {
        "SELECT" => has("FROM"),
        "INSERT" | "REPLACE" => has("INTO"),
        "UPDATE" => has("SET"),
        "DELETE" => has("FROM"),
        "DROP" | "ALTER" | "TRUNCATE" => has("TABLE"),
        _ => false,
    }
}

/// A constant by naming convention: `MAX_ROWS`, `COLUMNS`.
fn is_constant_name(name: &str) -> bool {
    name.bytes().any(|b| b.is_ascii_uppercase())
        && name
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// True when a format string interpolates a run-time value. Escaped braces
/// (`{{`) are text, and a constant (`{COLUMNS}`, or `{}` filled from one)
/// cannot carry input; `rest` is the macro's remaining arguments.
fn interpolates(text: &str, rest: &str) -> bool {
    let bytes = text.as_bytes();
    let mut positional = false;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
        } else if bytes.get(i + 1) == Some(&b'{') {
            i += 2;
        } else {
            let end = text[i..].find('}').map_or(bytes.len(), |p| i + p);
            let name = text[i + 1..end].split(':').next().unwrap_or("");
            if name.is_empty() || name.bytes().all(|b| b.is_ascii_digit()) {
                positional = true;
            } else if !is_constant_name(name) {
                return true;
            }
            i = end;
        }
    }
    positional
        && !rest
            .split(',')
            .map(str::trim)
            .filter(|arg| !arg.is_empty())
            .all(is_constant_name)
}

fn sql_format(s: &Scanned, out: &mut Vec<Hit>) {
    for pos in s.find("format!") {
        let Some((open, args)) = call_args(s, pos + "format!".len()) else {
            continue;
        };
        if let Some(literal) = leading_literal(s, open + 1)
            && is_sql(&literal.value)
            && interpolates(
                &literal.value,
                args.get(literal.close - open - 1..).unwrap_or(""),
            )
        {
            out.push(Hit {
                rule: SQL_FORMAT,
                level: Level::Warning,
                offset: pos,
                message: "SQL text is built with `format!`; pass values as bound parameters".into(),
            });
        }
    }
}

fn tls_disabled(s: &Scanned, out: &mut Vec<Hit>) {
    for method in [
        ".danger_accept_invalid_certs",
        ".danger_accept_invalid_hostnames",
        ".disable_verification",
    ] {
        for pos in s.find(method) {
            if call_args(s, pos + method.len()).is_some_and(|(_, args)| args.trim() == "true") {
                out.push(Hit {
                    rule: TLS_DISABLED,
                    level: Level::Error,
                    offset: pos,
                    message: format!(
                        "`{}(true)` turns off TLS certificate checks; any server can impersonate the peer",
                        &method[1..]
                    ),
                });
            }
        }
    }
    for pos in s.find("SslVerifyMode::NONE") {
        out.push(Hit {
            rule: TLS_DISABLED,
            level: Level::Error,
            offset: pos,
            message: "`SslVerifyMode::NONE` turns off TLS certificate checks".into(),
        });
    }
}

fn env_set_var(s: &Scanned, out: &mut Vec<Hit>) {
    for call in ["env::set_var", "env::remove_var"] {
        for pos in s.find(call) {
            out.push(Hit {
                rule: ENV_SET_VAR,
                level: Level::Warning,
                offset: pos,
                message: format!(
                    "`std::{call}` changes the environment of the whole process and races with any thread that reads it"
                ),
            });
        }
    }
}

fn world_writable(s: &Scanned, out: &mut Vec<Hit>) {
    for call in ["from_mode", "set_mode", ".mode"] {
        for pos in s.find(call) {
            let Some((_, args)) = call_args(s, pos + call.len()) else {
                continue;
            };
            let Some(digits) = args.trim().strip_prefix("0o") else {
                continue;
            };
            let digits = digits.replace('_', "");
            let other = digits
                .bytes()
                .last()
                .filter(|_| digits.bytes().all(|b| (b'0'..=b'7').contains(&b)));
            if other.is_some_and(|d| (d - b'0') & 2 != 0) {
                out.push(Hit {
                    rule: WORLD_WRITABLE,
                    level: Level::Warning,
                    offset: pos,
                    message: format!(
                        "mode `0o{digits}` lets every user on the machine write the file"
                    ),
                });
            }
        }
    }
}

/// Hosts that appear in `http://` identifiers nobody connects to: XML
/// namespaces, schema ids and license URLs.
const IDENTIFIER_HOSTS: [&str; 12] = [
    "www.w3.org",
    "json-schema.org",
    "www.apache.org",
    "purl.org",
    "ns.adobe.com",
    "www.sitemaps.org",
    "cobertura.sourceforge.net",
    "docbook.org",
    "relaxng.org",
    "www.opengis.net",
    "java.sun.com",
    "maven.apache.org",
];

const RESERVED_TLDS: [&str; 6] = [
    ".localhost",
    ".local",
    ".test",
    ".internal",
    ".example",
    ".invalid",
];

/// True when plain HTTP to `host` crosses a network someone else could be on.
fn is_remote_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let private_172 = host
        .strip_prefix("172.")
        .and_then(|rest| rest.split('.').next()?.parse::<u8>().ok())
        .is_some_and(|second| (16..=31).contains(&second));
    // A single label (`backend`, `db`) is a name on a private network.
    host.contains('.')
        && !host.contains(['{', '}', '$', '%', '[', '*', '<'])
        && !RESERVED_TLDS.iter().any(|tld| host.ends_with(tld))
        && !["example.com", "example.org", "example.net"]
            .iter()
            .any(|e| host == *e || host.ends_with(&format!(".{e}")))
        && !["127.", "10.", "192.168.", "169.254.", "0.0.0.0"]
            .iter()
            .any(|p| host.starts_with(p))
        && !private_172
        && !IDENTIFIER_HOSTS.contains(&host.as_str())
        && !host.starts_with("schemas.")
        && !host.starts_with("xmlns.")
}

fn insecure_http(s: &Scanned, out: &mut Vec<Hit>) {
    for literal in &s.literals {
        let host = literal.value.match_indices("http://").find_map(|(at, _)| {
            let rest = &literal.value[at + 7..];
            // Credentials before the host (`user:pass@host`) are not the host.
            let authority = rest.split(['/', '?', '#', '"', '\'', ' ', '\\']).next()?;
            let host = authority.rsplit('@').next()?.split(':').next()?;
            is_remote_host(host).then_some(host)
        });
        if let Some(host) = host {
            out.push(Hit {
                rule: INSECURE_HTTP,
                level: Level::Warning,
                offset: literal.open,
                message: format!(
                    "plain HTTP to `{host}`; traffic can be read and changed in transit, use https"
                ),
            });
        }
    }
}

/// True when a `codewiki-allow` comment on `line` covers `rule`.
fn allows(line: &str, rule: &str) -> bool {
    line.split_once("codewiki-allow")
        .is_some_and(|(before, after)| {
            before.contains("//") && (!after.contains("security/") || after.contains(rule))
        })
}

/// Findings in one Rust source file. `path` is repo-relative.
pub fn scan_source(path: &str, source: &str) -> Vec<SecurityFinding> {
    if is_test_path(path) {
        return Vec::new();
    }
    let s = Scanned::new(source);
    let mut hits = Vec::new();
    unsafe_blocks(&s, &mut hits);
    hardcoded_secrets(&s, &mut hits);
    command_injection(&s, &mut hits);
    sql_format(&s, &mut hits);
    tls_disabled(&s, &mut hits);
    env_set_var(&s, &mut hits);
    world_writable(&s, &mut hits);
    insecure_http(&s, &mut hits);

    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<SecurityFinding> = hits
        .into_iter()
        .filter(|hit| !s.in_test(hit.offset))
        .map(|hit| SecurityFinding {
            rule: hit.rule.to_string(),
            level: hit.level,
            file: path.to_string(),
            line: s.line_of(hit.offset),
            message: hit.message,
        })
        .filter(|f| {
            let at = f.line as usize;
            ![at.checked_sub(1), at.checked_sub(2)]
                .into_iter()
                .flatten()
                .filter_map(|i| lines.get(i))
                .any(|line| allows(line, &f.rule))
        })
        .collect();
    out.sort_by(|a, b| a.line.cmp(&b.line).then(a.rule.cmp(&b.rule)));
    out.dedup_by(|a, b| a.line == b.line && a.rule == b.rule);
    out
}

/// Findings across every indexed Rust file, by file, line and rule.
pub fn scan(root: &Path, index: &Index) -> Result<Vec<SecurityFinding>> {
    let mut out = Vec::new();
    let mut files = index.files()?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    for file in files {
        if file.language != Language::Rust || is_test_path(&file.path) {
            continue;
        }
        // A file removed since the last index has nothing left to report.
        if let Ok(text) = std::fs::read_to_string(root.join(&file.path)) {
            out.extend(scan_source(&file.path, &text));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rules that fire on `source`, as `(rule, line)`.
    fn rules(source: &str) -> Vec<(String, u32)> {
        scan_source("src/lib.rs", source)
            .into_iter()
            .map(|f| (f.rule, f.line))
            .collect()
    }

    fn fires(rule: &str, source: &str) {
        let found = rules(source);
        assert!(
            found.iter().any(|(r, _)| r == rule),
            "{rule} should fire on:\n{source}\ngot {found:?}"
        );
    }

    fn silent(source: &str) {
        let found = rules(source);
        assert!(
            found.is_empty(),
            "nothing should fire on:\n{source}\ngot {found:?}"
        );
    }

    #[test]
    fn unsafe_blocks_but_not_unsafe_signatures() {
        assert_eq!(
            rules("fn f(p: *const u8) -> u8 {\n    unsafe { *p }\n}\n"),
            [(UNSAFE_BLOCK.to_string(), 2)]
        );
        silent("unsafe fn f() {}\nunsafe impl Send for X {}\npub unsafe trait T {}\n");
        silent("// unsafe { }\nconst DOC: &str = \"unsafe { }\";\nfn not_unsafe_here() {}\n");
    }

    #[test]
    fn hardcoded_secrets_need_a_secret_name_and_a_generated_value() {
        for source in [
            "const API_KEY: &str = \"9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c\";\n",
            "fn f() { let auth_token = String::from(\"Zx8kQ2mVb7Lp0Rt5Ny3Wd6Hs\"); }\n",
            "fn f(c: &mut C) { c.client_secret = Some(\"q7Rz2Lm9Xw4Tb6Yc1Nd8Vk3P\".into()); }\n",
            "fn f() -> C { C { password: \"hG7#kL2@pQ9!zX4&mN6v\", port: 1 } }\n",
            "static DB_PASSWORD: &'static str = r#\"aB3dE5gH7jK9mN1pQ2rS4t\"#;\n",
        ] {
            fires(HARDCODED_SECRET, source);
        }
        // Recognisable credential formats fire whatever the name.
        fires(
            HARDCODED_SECRET,
            "const ID: &str = \"AKIAIOSFODNN7EXAMPLE\";\n",
        );
        fires(
            HARDCODED_SECRET,
            "const PEM: &str = \"-----BEGIN RSA PRIVATE KEY-----\\nMIIE\\n-----END RSA PRIVATE KEY-----\";\n",
        );
        for source in [
            // The name of an environment variable, not a secret.
            "const TOKEN_ENV: &str = \"CODEWIKI_TOKEN\";\nconst KEY: &str = \"AWS_SECRET_ACCESS_KEY_2\";\n",
            // Low entropy, placeholders and templates.
            "const PASSWORD: &str = \"password\";\nlet api_key = \"your-api-key-goes-here-1234\";\n",
            "let token = format!(\"Bearer {raw_token_value_1234}\");\nlet secret = \"${VAULT_SECRET_ID_12345}\";\n",
            // A generated value under a name that is not a secret.
            "const BUILD_ID: &str = \"9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c\";\n",
            // Comparisons and lookups are not assignments.
            "fn f(token: &str) -> bool { token == \"9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c\" }\n",
            "fn f() { let token = std::env::var(\"9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c\"); }\n",
            "#[serde(rename = \"9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c\")]\nstruct Token;\n",
        ] {
            silent(source);
        }
    }

    #[test]
    fn names_and_values_are_judged_separately() {
        assert_eq!(
            assigned_name("let mut api_token = \"", 20),
            Some("api_token")
        );
        assert_eq!(
            assigned_name("pub const KEY: &'static str = r#\"", 32),
            Some("KEY")
        );
        assert_eq!(assigned_name("X { a: 1, secret: &\"", 19), Some("secret"));
        assert_eq!(assigned_name("call(\"", 5), None);
        assert_eq!(assigned_name("a != \"", 5), None);
        assert_eq!(assigned_name("Path::\"", 6), None);
        assert!(names_a_secret("GITHUB_TOKEN") && names_a_secret("dbPassword"));
        assert!(!names_a_secret("token_env") && !names_a_secret("tokenizer_kind"));
        assert!(!names_a_secret("timeout"));
        assert!(entropy("aaaaaaaaaaaaaaaa") < 0.1);
        assert!(entropy("9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c") > 3.5);
        assert!(!looks_generated("short1"));
        assert!(!looks_generated("correct horse battery staple 1"));
    }

    #[test]
    fn commands_built_from_interpolated_shell_strings() {
        fires(
            COMMAND_INJECTION,
            "fn f(dir: &str) {\n    Command::new(\"sh\").arg(\"-c\").arg(format!(\"ls {dir}\")).status();\n}\n",
        );
        fires(
            COMMAND_INJECTION,
            "fn f(dir: &str) {\n    let line = format!(\"dir {dir}\");\n    std::process::Command::new(\"C:\\\\Windows\\\\System32\\\\cmd.exe\")\n        .args([\"/C\", &line])\n        .arg(&line)\n        .output();\n}\n",
        );
        fires(
            COMMAND_INJECTION,
            "fn f(tool: &str) { Command::new(format!(\"/opt/{tool}/bin/run\")).spawn(); }\n",
        );
        for source in [
            // Separate arguments are never parsed by a shell.
            "fn f(root: &Path) { Command::new(\"git\").arg(\"-C\").arg(root).arg(format!(\"-n{MAX}\")).output(); }\n",
            // A shell running a fixed line.
            "fn f() { Command::new(\"sh\").arg(\"-c\").arg(\"ls -la\").status(); }\n",
            // The `format!` belongs to a later statement.
            "fn f(n: u8) { Command::new(\"bash\").arg(\"x.sh\").status(); let s = format!(\"{n}\"); }\n",
            "fn f(binary: &Path) { Command::new(&binary).arg(\"serve\").spawn(); }\n",
        ] {
            silent(source);
        }
    }

    #[test]
    fn sql_built_with_format() {
        for source in [
            "fn f(id: &str) -> String { format!(\"SELECT * FROM users WHERE id = '{id}'\") }\n",
            "fn f(t: &str) { let q = format!(\n        \"delete from {} where done = 1\",\n        t\n    ); }\n",
            "fn f(v: u8) { run(&format!(r#\"UPDATE jobs SET state = {v} WHERE id = 1\"#)); }\n",
            "fn f(n: &str) { let q = format!(\"INSERT INTO log (name) VALUES ('{}')\", n); }\n",
            "fn f(t: &str) { let q = format!(\"SELECT {COLUMNS} FROM {} LIMIT {MAX}\", t); }\n",
        ] {
            fires(SQL_FORMAT, source);
        }
        for source in [
            // Bound parameters, no interpolation.
            "fn f() { conn.execute(\"SELECT * FROM users WHERE id = ?1\", [id]); }\n",
            "fn f() { let q = format!(\"SELECT * FROM users WHERE id = ?1\"); }\n",
            // Escaped braces are not interpolation.
            "fn f() { let q = format!(\"SELECT '{{}}' FROM t\"); }\n",
            // Constants cannot carry input.
            "fn f() { let q = format!(\"SELECT {COLUMNS} FROM symbols WHERE {FILTER:} = ?1\"); }\n",
            "fn f() { let q = format!(\"SELECT {} FROM {0}\", SYMBOL_COLUMNS, ); }\n",
            // Prose that happens to start with a verb.
            "fn f(n: u8) { let m = format!(\"Select {n} items from the list\"); }\n",
            "fn f(n: u8) { let m = format!(\"update {n} of them\"); }\n",
        ] {
            silent(source);
        }
    }

    #[test]
    fn disabled_tls_verification() {
        fires(
            TLS_DISABLED,
            "fn f() { reqwest::Client::builder().danger_accept_invalid_certs(true).build(); }\n",
        );
        fires(
            TLS_DISABLED,
            "fn f(b: B) { b\n    .danger_accept_invalid_hostnames( true ); }\n",
        );
        fires(
            TLS_DISABLED,
            "fn f(b: &mut B) { b.set_verify(SslVerifyMode::NONE); }\n",
        );
        silent(
            "fn f(b: B, insecure: bool) { b.danger_accept_invalid_certs(false).danger_accept_invalid_hostnames(insecure); }\n",
        );
        silent("fn f(b: &mut B) { b.set_verify(SslVerifyMode::PEER); }\n");
    }

    #[test]
    fn environment_mutation() {
        fires(ENV_SET_VAR, "fn f() { std::env::set_var(\"A\", \"b\"); }\n");
        fires(
            ENV_SET_VAR,
            "use std::env;\nfn f() { unsafe { env::remove_var(\"A\") } }\n",
        );
        silent(
            "fn f() { let _ = std::env::var(\"A\"); cmd.env(\"A\", \"b\"); my_env::set_variable(); }\n",
        );
    }

    #[test]
    fn world_writable_modes() {
        fires(
            WORLD_WRITABLE,
            "fn f() { fs::set_permissions(p, Permissions::from_mode(0o777)); }\n",
        );
        fires(
            WORLD_WRITABLE,
            "fn f(perm: &mut P) { perm.set_mode(0o666); }\n",
        );
        fires(
            WORLD_WRITABLE,
            "fn f() { OpenOptions::new().mode(0o0_646).open(p); }\n",
        );
        silent(
            "fn f(perm: &mut P) { perm.set_mode(0o755); Permissions::from_mode(0o600); o.mode(0o644); }\n",
        );
        silent("fn f(perm: &mut P, m: u32) { perm.set_mode(m); let mode = 0o777; }\n");
    }

    #[test]
    fn plain_http_to_remote_hosts() {
        assert_eq!(
            rules("const API: &str = \"http://api.partner.io/v1\";\n"),
            [(INSECURE_HTTP.to_string(), 1)]
        );
        fires(
            INSECURE_HTTP,
            "fn f() { get(\"http://user:pw@updates.vendor.net:8080/feed\"); }\n",
        );
        for source in [
            "const A: &str = \"https://api.partner.io/v1\";\n",
            "const A: &str = \"http://localhost:8080/v1\";\nconst B: &str = \"http://127.0.0.1:1\";\n",
            "const A: &str = \"http://[::1]:8080\";\nconst B: &str = \"http://192.168.1.4/\";\n",
            "const A: &str = \"http://172.20.0.3:9000\";\nconst B: &str = \"http://backend:9000\";\n",
            "const A: &str = \"http://example.com/x\";\nconst B: &str = \"http://svc.internal/\";\n",
            // Namespace and schema identifiers are never fetched.
            "const NS: &str = \"http://www.w3.org/2000/svg\";\nconst S: &str = \"http://json-schema.org/draft-07/schema#\";\n",
            // The host comes from a variable; nothing to judge.
            "fn f(h: &str) { let u = format!(\"http://{h}/x\"); let p = u.starts_with(\"http://\"); }\n",
            "// see http://api.partner.io/docs\nfn f() {}\n",
        ] {
            silent(source);
        }
        assert!(is_remote_host("172.32.0.1") && !is_remote_host("172.16.0.1"));
    }

    #[test]
    fn test_code_and_allowed_lines_are_skipped() {
        let source = "pub fn real() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        unsafe { std::env::set_var(\"A\", \"http://api.partner.io\") };\n        let api_key = \"9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c\";\n    }\n}\n";
        silent(source);
        let risky = "fn f() { std::env::set_var(\"A\", \"b\"); }\n";
        assert!(scan_source("crates/x/tests/it.rs", risky).is_empty());
        assert!(scan_source("examples/demo.rs", risky).is_empty());
        assert_eq!(scan_source("build.rs", risky).len(), 1);

        silent(
            "// codewiki-allow: security/env-set-var\nfn f() { std::env::set_var(\"A\", \"b\"); }\n",
        );
        silent("fn f() { std::env::set_var(\"A\", \"b\"); } // codewiki-allow\n");
        // An allowance for another rule does not cover this one.
        fires(
            ENV_SET_VAR,
            "// codewiki-allow: security/unsafe-block\nfn f() { std::env::set_var(\"A\", \"b\"); }\n",
        );
        // The marker only counts inside a comment.
        fires(
            ENV_SET_VAR,
            "fn f() { std::env::set_var(\"codewiki-allow\", \"b\"); }\n",
        );
    }

    #[test]
    fn findings_carry_levels_and_declared_rule_ids() {
        let source = "fn f(p: *const u8) {\n    unsafe { *p };\n    std::env::set_var(\"A\", \"b\");\n    b.danger_accept_invalid_certs(true);\n}\n";
        let found = scan_source("src/net.rs", source);
        let levels: Vec<(&str, Level, u32)> = found
            .iter()
            .map(|f| (f.rule.as_str(), f.level, f.line))
            .collect();
        assert_eq!(
            levels,
            [
                (UNSAFE_BLOCK, Level::Note, 2),
                (ENV_SET_VAR, Level::Warning, 3),
                (TLS_DISABLED, Level::Error, 4),
            ]
        );
        assert!(found.iter().all(|f| f.file == "src/net.rs"));
        for rule in [
            UNSAFE_BLOCK,
            HARDCODED_SECRET,
            COMMAND_INJECTION,
            SQL_FORMAT,
            TLS_DISABLED,
            ENV_SET_VAR,
            WORLD_WRITABLE,
            INSECURE_HTTP,
        ] {
            assert!(
                crate::ci::RULES.iter().any(|(id, _)| *id == rule),
                "{rule} is not declared in the CI rule table"
            );
        }
    }
}
