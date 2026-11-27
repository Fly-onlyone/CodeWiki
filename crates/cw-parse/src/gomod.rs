//! What Go's build files say: `go.mod`, `go.work` and `go.sum`, read as text.
//!
//! The formats are documented at <https://go.dev/ref/mod#go-mod-file> and
//! <https://go.dev/ref/mod#workspaces>. A `go.mod` is a list of directives, one
//! per line or a parenthesised block of lines, with `//` comments; the
//! scanner needs only `module`, `go`, `require` and `replace`.

use std::collections::BTreeMap;

use crate::lang::sanitize;

/// One `require` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoRequire {
    pub path: String,
    pub version: String,
    /// True for a line that ends in `// indirect`.
    pub indirect: bool,
}

/// One `replace` line: `old [version] => new [version]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoReplace {
    pub old: String,
    pub old_version: String,
    pub new: String,
    /// Empty when `new` is a directory of this machine.
    pub new_version: String,
}

/// What a `go.mod` file says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GoMod {
    /// The module path, the prefix of every import path inside the module.
    pub module: String,
    /// The `go` directive, such as `1.22`.
    pub go: String,
    pub requires: Vec<GoRequire>,
    pub replaces: Vec<GoReplace>,
}

/// Text before a `//` comment, trimmed. A `//` inside a quoted path is not a
/// comment, and no module path holds one.
fn code(line: &str) -> &str {
    line.split("//").next().unwrap_or("").trim()
}

/// A path as `go.mod` writes it: plain, or in `"` or backquotes.
fn unquote(text: &str) -> &str {
    text.trim().trim_matches(['"', '`'])
}

/// The directive of a line and what follows it, `require (` giving a block.
fn directive(line: &str) -> Option<(&str, &str)> {
    let line = code(line);
    let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    (!word.is_empty()).then_some((word, rest.trim()))
}

/// Read a `go.mod`. A line it cannot read is left out.
pub fn parse_go_mod(text: &str) -> GoMod {
    let mut out = GoMod::default();
    // The directive whose parenthesised block the lines belong to.
    let mut block: Option<String> = None;
    for raw in text.lines() {
        let indirect = raw.contains("// indirect");
        let line = code(raw);
        if line.is_empty() {
            continue;
        }
        let (word, rest) = match &block {
            Some(open) => {
                if line == ")" {
                    block = None;
                    continue;
                }
                (open.as_str(), line)
            }
            None => match directive(raw) {
                Some((word, "(")) => {
                    block = Some(word.to_string());
                    continue;
                }
                Some(pair) => pair,
                None => continue,
            },
        };
        let parts: Vec<&str> = rest.split_whitespace().collect();
        match word {
            "module" => out.module = unquote(rest).to_string(),
            "go" => out.go = rest.to_string(),
            "require" if parts.len() >= 2 => out.requires.push(GoRequire {
                path: unquote(parts[0]).to_string(),
                version: parts[1].to_string(),
                indirect,
            }),
            "replace" => {
                let Some((old, new)) = rest.split_once("=>") else {
                    continue;
                };
                let old: Vec<&str> = old.split_whitespace().collect();
                let new: Vec<&str> = new.split_whitespace().collect();
                if let (Some(old_path), Some(new_path)) = (old.first(), new.first()) {
                    out.replaces.push(GoReplace {
                        old: unquote(old_path).to_string(),
                        old_version: old.get(1).unwrap_or(&"").to_string(),
                        new: unquote(new_path).to_string(),
                        new_version: new.get(1).unwrap_or(&"").to_string(),
                    });
                }
            }
            _ => {}
        }
    }
    out
}

/// The directories a `go.work` file lists with `use`, as written.
pub fn parse_go_work(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut block = false;
    for raw in text.lines() {
        let line = code(raw);
        if line.is_empty() {
            continue;
        }
        if block {
            if line == ")" {
                block = false;
            } else {
                out.push(unquote(line).to_string());
            }
            continue;
        }
        match directive(raw) {
            Some(("use", "(")) => block = true,
            Some(("use", rest)) if !rest.is_empty() => out.push(unquote(rest).to_string()),
            _ => {}
        }
    }
    out
}

/// The versions `go.sum` holds a source hash for, by module path, oldest
/// first. The `/go.mod` lines hash only the module file and are left out.
pub fn go_sum_versions(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let (Some(path), Some(version), Some(_)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if version.ends_with("/go.mod") {
            continue;
        }
        let versions = out.entry(path.to_string()).or_default();
        if !versions.iter().any(|v| v == version) {
            versions.push(version.to_string());
        }
    }
    for versions in out.values_mut() {
        versions.sort_by_cached_key(|v| version_key(v));
    }
    out
}

/// A sort key that puts `v1.9.0` before `v1.10.0`.
fn version_key(version: &str) -> Vec<u64> {
    let core = version
        .trim_start_matches('v')
        .split(['-', '+'])
        .next()
        .unwrap_or("");
    core.split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

/// The name Go code gives a package imported by `path` when the package's own
/// name is not known: the last segment, without a major-version element
/// (`/v2`), a `.v3` suffix or Go's `go-` and `-go` decoration, with every
/// other non-identifier character turned into `_`.
pub fn guess_package_name(path: &str) -> String {
    let mut segments: Vec<&str> = path.split('/').collect();
    if segments.len() > 1
        && segments.last().is_some_and(|last| {
            last.strip_prefix('v')
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
    {
        segments.pop();
    }
    let last = segments.last().copied().unwrap_or("");
    let last = match last.rsplit_once(".v") {
        Some((name, n)) if n.chars().all(|c| c.is_ascii_digit()) && !name.is_empty() => name,
        _ => last,
    };
    let last = last.strip_prefix("go-").unwrap_or(last);
    let last = last.strip_suffix("-go").unwrap_or(last);
    sanitize(last)
}

/// The Go modules of a repository: the folder of each `go.mod` and the module
/// path it declares.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GoModules {
    /// `(folder, module path)`; the folder is repo-relative, empty for the
    /// root.
    pub list: Vec<(String, String)>,
}

impl GoModules {
    /// The module that holds `file`: the one whose folder is the nearest
    /// above it.
    pub fn of(&self, file: &str) -> Option<&(String, String)> {
        let dir = file.rsplit_once('/').map_or("", |(dir, _)| dir);
        self.list
            .iter()
            .filter(|(root, _)| {
                root.is_empty() || dir == root || dir.starts_with(&format!("{root}/"))
            })
            .max_by_key(|(root, _)| root.len())
    }

    /// The qualified name of the package that holds `file`: the module path
    /// then the folders below the module, every segment made safe. A file
    /// outside any module is named by its folder alone, or `""` at the root.
    ///
    /// This is what an import path of the package turns into, so an import
    /// names the package it finds.
    pub fn package_of(&self, file: &str) -> String {
        let dir = file.rsplit_once('/').map_or("", |(dir, _)| dir);
        let Some((root, path)) = self.of(file) else {
            return dir
                .split('/')
                .filter(|s| !s.is_empty())
                .map(sanitize)
                .collect::<Vec<_>>()
                .join("::");
        };
        let below = dir
            .strip_prefix(root.as_str())
            .unwrap_or("")
            .trim_start_matches('/');
        path.split('/')
            .chain(below.split('/'))
            .filter(|s| !s.is_empty())
            .map(sanitize)
            .collect::<Vec<_>>()
            .join("::")
    }

    /// The folder an import path names inside one of the modules, or `None`
    /// for a path outside all of them. The longest module path wins.
    pub fn folder_of(&self, import: &str) -> Option<String> {
        let (root, path) = self
            .list
            .iter()
            .filter(|(_, path)| import == path || import.starts_with(&format!("{path}/")))
            .max_by_key(|(_, path)| path.len())?;
        let below = import[path.len()..].trim_start_matches('/');
        Some(match (root.is_empty(), below.is_empty()) {
            (true, _) => below.to_string(),
            (false, true) => root.clone(),
            (false, false) => format!("{root}/{below}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GO_MOD: &str = r#"// Module of the app.
module example.com/app // the app

go 1.22.1

require github.com/spf13/cobra v1.8.0

require (
	golang.org/x/sync v0.7.0
	github.com/google/uuid v1.6.0 // indirect
	"gopkg.in/yaml.v3" v3.0.1
)

replace github.com/google/uuid => ../uuid
replace (
	golang.org/x/sync v0.7.0 => github.com/fork/sync v0.7.1
)
exclude example.com/old v1.0.0
"#;

    #[test]
    fn reads_a_go_mod() {
        let m = parse_go_mod(GO_MOD);
        assert_eq!(m.module, "example.com/app");
        assert_eq!(m.go, "1.22.1");
        let requires: Vec<(&str, &str, bool)> = m
            .requires
            .iter()
            .map(|r| (r.path.as_str(), r.version.as_str(), r.indirect))
            .collect();
        assert_eq!(
            requires,
            [
                ("github.com/spf13/cobra", "v1.8.0", false),
                ("golang.org/x/sync", "v0.7.0", false),
                ("github.com/google/uuid", "v1.6.0", true),
                ("gopkg.in/yaml.v3", "v3.0.1", false),
            ]
        );
        assert_eq!(m.replaces.len(), 2);
        assert_eq!(m.replaces[0].old, "github.com/google/uuid");
        assert_eq!(m.replaces[0].new, "../uuid");
        assert_eq!(m.replaces[0].new_version, "");
        assert_eq!(m.replaces[1].old_version, "v0.7.0");
        assert_eq!(m.replaces[1].new, "github.com/fork/sync");
        assert_eq!(m.replaces[1].new_version, "v0.7.1");
    }

    #[test]
    fn reads_a_go_work() {
        let dirs =
            parse_go_work("go 1.22\n\nuse ./api\nuse (\n\t./worker // jobs\n\t../shared\n)\n");
        assert_eq!(dirs, ["./api", "./worker", "../shared"]);
    }

    #[test]
    fn reads_go_sum_without_the_module_file_lines() {
        let sums = go_sum_versions(
            "github.com/a/b v1.10.0 h1:x=\ngithub.com/a/b v1.10.0/go.mod h1:y=\ngithub.com/a/b v1.9.0 h1:z=\ngithub.com/c/d v0.1.0/go.mod h1:w=\n",
        );
        assert_eq!(sums["github.com/a/b"], ["v1.9.0", "v1.10.0"]);
        assert!(!sums.contains_key("github.com/c/d"));
    }

    #[test]
    fn guesses_the_name_a_package_goes_by() {
        for (path, name) in [
            ("github.com/spf13/cobra", "cobra"),
            ("github.com/go-chi/chi/v5", "chi"),
            ("gopkg.in/yaml.v3", "yaml"),
            ("github.com/foo/go-redis", "redis"),
            ("github.com/foo/redis-go", "redis"),
            ("github.com/foo/my-lib", "my_lib"),
            ("fmt", "fmt"),
            ("net/http", "http"),
            ("v2", "v2"),
        ] {
            assert_eq!(guess_package_name(path), name, "{path}");
        }
    }

    #[test]
    fn names_the_package_of_a_file_and_the_folder_of_an_import() {
        let modules = GoModules {
            list: vec![
                (String::new(), "example.com/app".into()),
                ("tools/gen".into(), "example.com/gen".into()),
            ],
        };
        assert_eq!(modules.package_of("main.go"), "example_com::app");
        assert_eq!(
            modules.package_of("internal/store/a.go"),
            "example_com::app::internal::store"
        );
        // The nearest `go.mod` wins.
        assert_eq!(
            modules.package_of("tools/gen/cmd/x.go"),
            "example_com::gen::cmd"
        );
        assert_eq!(
            modules
                .folder_of("example.com/app/internal/store")
                .as_deref(),
            Some("internal/store")
        );
        assert_eq!(modules.folder_of("example.com/app").as_deref(), Some(""));
        assert_eq!(
            modules.folder_of("example.com/gen/cmd").as_deref(),
            Some("tools/gen/cmd")
        );
        assert_eq!(modules.folder_of("example.com/other"), None);
        assert_eq!(modules.folder_of("example.com/application"), None);
        // No module: the folder names the package.
        let none = GoModules::default();
        assert_eq!(none.package_of("cmd/tool/main.go"), "cmd::tool");
        assert_eq!(none.package_of("main.go"), "");
    }
}
