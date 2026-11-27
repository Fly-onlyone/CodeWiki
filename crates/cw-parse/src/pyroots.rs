//! Where Python's import names start: the folders an absolute import is
//! taken from, read from the build files (`pyproject.toml`, `setup.cfg`,
//! `setup.py`) and never run.
//!
//! A folder of `.py` files below a root is part of the dotted path whether or
//! not it has an `__init__.py`: that is PEP 420, the namespace package. The
//! roots of a project are the folder of its build file, a `src` folder under
//! it, and what the build file declares:
//!
//! - `[tool.setuptools.packages.find] where = [...]` and
//!   `[tool.setuptools.package-dir]` (`"" = "src"`, or a package named for a
//!   folder: `shop = "lib/shop_impl"`),
//! - `[options] package_dir = =src` and `[options.packages.find] where` of
//!   `setup.cfg`,
//! - the literals `package_dir={"": "src"}` and `where="src"` of `setup.py`,
//! - `[tool.poetry] packages = [{ include, from }]`,
//! - `[tool.hatch.build.targets.wheel] packages` and `sources`,
//! - `[tool.pdm.build] package-dir`, and `[tool.maturin] python-source`.
//!
//! A file that no declared root holds keeps the rule it had before: the name
//! starts at the outermost folder with an `__init__.py`; with none, at the
//! project's folder, or at the repository root (minus a leading `src` or `lib`)
//! when no build file is above it.

use std::collections::{BTreeMap, HashSet};

use crate::jsbuild::{ancestors, join, join_norm, parent_dir};
use crate::lang::sanitize;

/// Build files that make a folder a Python project.
const BUILD_FILES: [&str; 3] = ["pyproject.toml", "setup.py", "setup.cfg"];

/// A folder that import names start at.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceRoot {
    dir: String,
    /// Names the root's content is put under (`shop = "lib/impl"` makes the
    /// files of `lib/impl` the package `shop`).
    prefix: Vec<String>,
}

/// The Python projects of a repository and their import roots.
#[derive(Debug, Default)]
pub struct PythonRoots {
    /// Folder of a build file -> its roots.
    projects: BTreeMap<String, Vec<SourceRoot>>,
}

impl PythonRoots {
    /// Read the build files among `paths` through `read`.
    pub fn read(paths: &[String], read: &dyn Fn(&str) -> Option<String>) -> Self {
        let mut projects: BTreeMap<String, Vec<SourceRoot>> = BTreeMap::new();
        for path in paths {
            let name = path.rsplit('/').next().unwrap_or(path);
            if BUILD_FILES.contains(&name) {
                projects.entry(parent_dir(path).to_string()).or_default();
            }
        }
        for (dir, roots) in &mut projects {
            let mut found: Vec<SourceRoot> = Vec::new();
            let src = join(dir, "src");
            let has_src = paths
                .iter()
                .any(|p| p.ends_with(".py") && p.starts_with(&format!("{src}/")));
            if has_src {
                found.push(SourceRoot {
                    dir: src,
                    prefix: Vec::new(),
                });
            }
            for build in BUILD_FILES {
                if let Some(text) = read(&join(dir, build)) {
                    match build {
                        "pyproject.toml" => found.extend(pyproject_roots(dir, &text)),
                        "setup.cfg" => found.extend(setup_cfg_roots(dir, &text)),
                        _ => found.extend(setup_py_roots(dir, &text)),
                    }
                }
            }
            for root in found {
                // The project's own folder is the default, not a root to list.
                if root.dir != *dir && !roots.contains(&root) {
                    roots.push(root);
                }
            }
        }
        Self { projects }
    }

    /// The folder of the nearest build file above `path`, `""` for the
    /// repository root.
    pub fn project_of(&self, path: &str) -> Option<&str> {
        ancestors(parent_dir(path))
            .into_iter()
            .find_map(|d| self.projects.get_key_value(d).map(|(k, _)| k.as_str()))
    }

    /// True when a folder between `path` and the repository root has an
    /// `__init__.py`: the file is part of a regular package.
    pub fn in_package(files: &HashSet<&str>, path: &str) -> bool {
        let mut dir = parent_dir(path);
        loop {
            if ["py", "pyi"]
                .iter()
                .any(|ext| files.contains(join(dir, &format!("__init__.{ext}")).as_str()))
            {
                return true;
            }
            if dir.is_empty() {
                return false;
            }
            dir = parent_dir(dir);
        }
    }

    /// Module a Python file defines, and whether the file is a package
    /// (`__init__.py`).
    pub fn module(&self, files: &HashSet<&str>, path: &str) -> (String, bool) {
        let stem_path = path.rsplit_once('.').map_or(path, |(stem, _)| stem);
        let mut dirs: Vec<&str> = stem_path.split('/').collect();
        let stem = dirs.pop().unwrap_or("");
        let is_package = stem == "__init__";
        let project = self.project_of(path);
        let root = project
            .and_then(|p| self.projects.get(p))
            .into_iter()
            .flatten()
            .filter(|r| path.starts_with(&format!("{}/", r.dir)))
            .max_by_key(|r| r.dir.len());
        let start = match root {
            Some(root) => root.dir.split('/').filter(|s| !s.is_empty()).count(),
            None => {
                let has_init = |depth: usize| {
                    let dir = dirs[..=depth].join("/");
                    ["py", "pyi"]
                        .iter()
                        .any(|ext| files.contains(format!("{dir}/__init__.{ext}").as_str()))
                };
                (0..dirs.len()).find(|&i| has_init(i)).unwrap_or_else(|| {
                    match project.filter(|p| !p.is_empty()) {
                        Some(project) => project.split('/').count(),
                        None => {
                            usize::from(dirs.first().is_some_and(|d| matches!(*d, "src" | "lib")))
                        }
                    }
                })
            }
        };
        let mut segments: Vec<String> = root
            .map(|r| r.prefix.clone())
            .unwrap_or_default()
            .iter()
            .map(|s| sanitize(s))
            .collect();
        segments.extend(dirs[start.min(dirs.len())..].iter().map(|d| sanitize(d)));
        if !is_package {
            segments.push(sanitize(stem));
        }
        if segments.is_empty() {
            segments.push("__init__".to_string());
        }
        (segments.join("::"), is_package)
    }
}

/// The name a `pyproject.toml` gives its project: `[project] name`, else
/// `[tool.poetry] name`.
pub fn project_name(text: &str) -> Option<String> {
    let table = text.parse::<toml::Table>().ok()?;
    let value = toml::Value::Table(table);
    get(&value, &["project", "name"])
        .or_else(|| get(&value, &["tool", "poetry", "name"]))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|name| !name.is_empty())
}

/// A TOML value that is a string or a list of strings.
fn strings(value: Option<&toml::Value>) -> Vec<String> {
    match value {
        Some(toml::Value::String(one)) => vec![one.clone()],
        Some(toml::Value::Array(many)) => many
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn root(dir: &str, below: &str) -> Option<SourceRoot> {
    Some(SourceRoot {
        dir: join_norm(dir, below)?,
        prefix: Vec::new(),
    })
}

fn get<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    path.iter().try_fold(value, |v, key| v.get(key))
}

fn pyproject_roots(dir: &str, text: &str) -> Vec<SourceRoot> {
    let Ok(table) = text.parse::<toml::Table>() else {
        return Vec::new();
    };
    let value = toml::Value::Table(table);
    let mut out = Vec::new();
    for w in strings(get(
        &value,
        &["tool", "setuptools", "packages", "find", "where"],
    )) {
        out.extend(root(dir, &w));
    }
    if let Some(table) =
        get(&value, &["tool", "setuptools", "package-dir"]).and_then(|v| v.as_table())
    {
        for (name, below) in table {
            let Some(below) = below.as_str() else {
                continue;
            };
            if let Some(mut found) = root(dir, below) {
                found.prefix = name
                    .split('.')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                out.push(found);
            }
        }
    }
    if let Some(packages) = get(&value, &["tool", "poetry", "packages"]).and_then(|v| v.as_array())
    {
        for package in packages {
            if let Some(from) = package.get("from").and_then(|v| v.as_str()) {
                out.extend(root(dir, from));
            }
        }
    }
    let wheel = get(&value, &["tool", "hatch", "build", "targets", "wheel"]);
    for entry in strings(wheel.and_then(|w| w.get("packages"))) {
        // `src/app` names the package `app` inside `src`.
        if let Some((parent, _)) = entry.trim_end_matches('/').rsplit_once('/') {
            out.extend(root(dir, parent));
        }
    }
    for source in strings(wheel.and_then(|w| w.get("sources"))) {
        out.extend(root(dir, &source));
    }
    for below in strings(get(&value, &["tool", "pdm", "build", "package-dir"])) {
        out.extend(root(dir, &below));
    }
    for below in strings(get(&value, &["tool", "maturin", "python-source"])) {
        out.extend(root(dir, &below));
    }
    out
}

/// `package_dir` and `[options.packages.find] where` of `setup.cfg`.
fn setup_cfg_roots(dir: &str, text: &str) -> Vec<SourceRoot> {
    let mut out = Vec::new();
    let mut section = String::new();
    let mut key = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(['#', ';']) || trimmed.is_empty() {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = name.trim().to_string();
            key.clear();
            continue;
        }
        let continued = line.starts_with([' ', '\t']);
        let value = if continued {
            trimmed.to_string()
        } else if let Some((k, v)) = trimmed.split_once(['=', ':']) {
            key = k.trim().to_string();
            v.trim().to_string()
        } else {
            continue;
        };
        match (section.as_str(), key.as_str()) {
            ("options", "package_dir") => {
                // `=src` is the root; `shop = lib/shop` is a named package.
                if let Some(below) = value.strip_prefix('=') {
                    out.extend(root(dir, below.trim()));
                } else if let Some((name, below)) = value.split_once('=')
                    && let Some(mut found) = root(dir, below.trim())
                {
                    found.prefix = name
                        .trim()
                        .split('.')
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect();
                    out.push(found);
                }
            }
            ("options.packages.find", "where") if !value.is_empty() => {
                out.extend(root(dir, &value));
            }
            _ => {}
        }
    }
    out
}

/// The literals `package_dir={"": "src"}` and `where="src"` of a `setup.py`.
/// A path computed in code is not followed.
fn setup_py_roots(dir: &str, text: &str) -> Vec<SourceRoot> {
    let quoted = |rest: &str| -> Option<String> {
        let rest = rest.trim_start();
        let quote = rest.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        let end = rest[1..].find(quote)?;
        Some(rest[1..=end].to_string())
    };
    let mut out = Vec::new();
    for (at, _) in text.match_indices("package_dir") {
        let rest = text[at + "package_dir".len()..].trim_start();
        let Some(rest) = rest.strip_prefix('=').map(str::trim_start) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix('{') else {
            continue;
        };
        if let Some(key) = quoted(rest)
            && key.is_empty()
            && let Some((_, after)) = rest.split_once(':')
            && let Some(below) = quoted(after)
        {
            out.extend(root(dir, &below));
        }
    }
    for (at, _) in text.match_indices("where") {
        let rest = text[at + "where".len()..].trim_start();
        if let Some(rest) = rest.strip_prefix('=')
            && let Some(below) = quoted(rest)
        {
            out.extend(root(dir, &below));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots(files: &[(&str, &str)]) -> (PythonRoots, Vec<String>) {
        let paths: Vec<String> = files.iter().map(|(p, _)| p.to_string()).collect();
        let read = |path: &str| {
            files
                .iter()
                .find(|(p, _)| *p == path)
                .map(|(_, t)| t.to_string())
        };
        (PythonRoots::read(&paths, &read), paths)
    }

    fn module(files: &[(&str, &str)], path: &str) -> String {
        let (roots, paths) = roots(files);
        let set: HashSet<&str> = paths.iter().map(String::as_str).collect();
        roots.module(&set, path).0
    }

    #[test]
    fn a_src_folder_under_the_build_file_is_a_root_without_init_files() {
        let files = [
            ("backend/pyproject.toml", "[project]\nname = \"shop\"\n"),
            ("backend/src/shop/orders.py", ""),
            ("backend/src/shop/api/routes.py", ""),
            ("backend/tests/test_orders.py", ""),
            ("backend/scripts/seed.py", ""),
        ];
        assert_eq!(module(&files, "backend/src/shop/orders.py"), "shop::orders");
        assert_eq!(
            module(&files, "backend/src/shop/api/routes.py"),
            "shop::api::routes"
        );
        // Beside `src`, a file is named from the project's folder.
        assert_eq!(
            module(&files, "backend/tests/test_orders.py"),
            "tests::test_orders"
        );
        assert_eq!(module(&files, "backend/scripts/seed.py"), "scripts::seed");
    }

    #[test]
    fn what_the_build_file_declares_is_a_root_too() {
        let setuptools = [
            (
                "pyproject.toml",
                "[tool.setuptools.packages.find]\nwhere = [\"python\"]\n\n[tool.setuptools.package-dir]\nshop = \"lib/impl\"\n",
            ),
            ("python/app/x.py", ""),
            ("lib/impl/y.py", ""),
        ];
        assert_eq!(module(&setuptools, "python/app/x.py"), "app::x");
        assert_eq!(module(&setuptools, "lib/impl/y.py"), "shop::y");
        let poetry = [
            (
                "pyproject.toml",
                "[tool.poetry]\npackages = [{ include = \"app\", from = \"py\" }]\n",
            ),
            ("py/app/x.py", ""),
        ];
        assert_eq!(module(&poetry, "py/app/x.py"), "app::x");
        let hatch = [
            (
                "pyproject.toml",
                "[tool.hatch.build.targets.wheel]\npackages = [\"source/app\"]\n",
            ),
            ("source/app/x.py", ""),
        ];
        assert_eq!(module(&hatch, "source/app/x.py"), "app::x");
        let cfg = [
            ("setup.cfg", "[options]\npackage_dir =\n    =code\n"),
            ("code/app/x.py", ""),
        ];
        assert_eq!(module(&cfg, "code/app/x.py"), "app::x");
        let find = [
            ("setup.cfg", "[options.packages.find]\nwhere = code\n"),
            ("code/app/x.py", ""),
        ];
        assert_eq!(module(&find, "code/app/x.py"), "app::x");
        let setup_py = [
            (
                "setup.py",
                "from setuptools import setup\nsetup(name='x', package_dir={'': 'code'})\n",
            ),
            ("code/app/x.py", ""),
        ];
        assert_eq!(module(&setup_py, "code/app/x.py"), "app::x");
        let computed = [
            ("setup.py", "setup(package_dir={'': ROOT})\n"),
            ("code/app/x.py", ""),
        ];
        assert_eq!(module(&computed, "code/app/x.py"), "code::app::x");
    }

    #[test]
    fn without_a_build_file_the_old_rules_stand() {
        let files = [
            ("src/pkg/__init__.py", ""),
            ("src/pkg/text.py", ""),
            ("tools/gen.py", ""),
            ("lib/other.py", ""),
        ];
        assert_eq!(module(&files, "src/pkg/text.py"), "pkg::text");
        assert_eq!(module(&files, "tools/gen.py"), "tools::gen");
        assert_eq!(module(&files, "lib/other.py"), "other");
        // The outermost folder with an `__init__.py` starts the name even
        // below a project's folder.
        let nested = [
            ("svc/pyproject.toml", ""),
            ("svc/app/__init__.py", ""),
            ("svc/app/x.py", ""),
            ("svc/loose.py", ""),
        ];
        assert_eq!(module(&nested, "svc/app/x.py"), "app::x");
        assert_eq!(module(&nested, "svc/loose.py"), "loose");
    }

    #[test]
    fn a_regular_package_is_told_from_a_namespace_folder() {
        let files = [("a/__init__.py", ""), ("a/b/c.py", ""), ("d/e.py", "")];
        let set: HashSet<&str> = files.iter().map(|(p, _)| *p).collect();
        assert!(PythonRoots::in_package(&set, "a/b/c.py"));
        assert!(!PythonRoots::in_package(&set, "d/e.py"));
        assert!(!PythonRoots::in_package(&set, "top.py"));
    }
}
