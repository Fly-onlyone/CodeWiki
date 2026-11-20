//! Cargo package discovery and the file-to-module mapping.

use std::collections::BTreeMap;

use cw_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// One Cargo package found in the repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateInfo {
    /// Name as used in paths (`-` replaced by `_`).
    pub name: String,
    /// Name as written in `Cargo.toml`.
    pub package: String,
    /// Repo-relative directory, forward slashes, empty for the repo root.
    pub dir: String,
    pub description: String,
    /// Dependency names as used in paths, from `[dependencies]` only.
    pub deps: Vec<String>,
    /// Every crate the package's own code can name, as used in paths: its
    /// normal, dev and build dependencies, those of `[target.*]` tables too.
    #[serde(default)]
    pub code_deps: Vec<String>,
    pub has_lib: bool,
    pub has_bin: bool,
}

/// The tables of a manifest, or of one of its `[target.*]` tables, that
/// name dependencies.
const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// The dependency names in one table, as used in paths.
fn dependency_names(table: Option<&toml::Value>) -> Vec<String> {
    table
        .and_then(|d| d.as_table())
        .map(|t| t.keys().map(|k| k.replace('-', "_")).collect())
        .unwrap_or_default()
}

/// Parse a `Cargo.toml`. Returns `None` for a virtual workspace manifest.
pub fn parse_manifest(dir: &str, text: &str, files: &[String]) -> Result<Option<CrateInfo>> {
    let table: toml::Table = text
        .parse()
        .map_err(|e| Error::external(format!("parsing {dir}/Cargo.toml"), e))?;
    let Some(package) = table.get("package").and_then(|p| p.as_table()) else {
        return Ok(None);
    };
    let Some(name) = package.get("name").and_then(|n| n.as_str()) else {
        return Ok(None);
    };
    let description = package
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("")
        .to_string();
    let mut deps = dependency_names(table.get("dependencies"));
    deps.sort();
    let targets = table
        .get("target")
        .and_then(|t| t.as_table())
        .into_iter()
        .flat_map(|t| t.values());
    let mut code_deps: Vec<String> = std::iter::once(&table)
        .chain(targets.filter_map(|t| t.as_table()))
        .flat_map(|t| DEPENDENCY_TABLES.iter().map(|name| t.get(*name)))
        .flat_map(dependency_names)
        .collect();
    code_deps.sort();
    code_deps.dedup();
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    let has = |rel: &str| files.iter().any(|f| *f == format!("{prefix}{rel}"));
    let has_bin_dir = files
        .iter()
        .any(|f| f.starts_with(&format!("{prefix}src/bin/")));
    Ok(Some(CrateInfo {
        name: name.replace('-', "_"),
        package: name.to_string(),
        dir: dir.to_string(),
        description,
        deps,
        code_deps,
        has_lib: has("src/lib.rs"),
        has_bin: has("src/main.rs") || has_bin_dir || table.contains_key("bin"),
    }))
}

/// What a manifest asks of one dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepSpec {
    /// Name as used in paths (`-` replaced by `_`).
    pub name: String,
    /// Name the package is published under: the key, or `package = "..."`
    /// when the dependency is renamed.
    pub package: String,
    /// Version requirement as written; empty for a path or git dependency
    /// that names none.
    pub req: String,
    /// Features the manifest turns on, in name order.
    pub features: Vec<String>,
    /// False when the manifest says `default-features = false`.
    pub default_features: bool,
}

/// One entry of a dependency table, with what `[workspace.dependencies]`
/// says when the entry is `workspace = true`.
fn dep_spec(key: &str, value: &toml::Value, workspace: Option<&toml::Table>) -> DepSpec {
    let mut spec = DepSpec {
        name: key.replace('-', "_"),
        package: key.to_string(),
        req: String::new(),
        features: Vec::new(),
        default_features: true,
    };
    let mut read = |value: &toml::Value| match value {
        toml::Value::String(req) => spec.req = req.clone(),
        toml::Value::Table(table) => {
            if let Some(req) = table.get("version").and_then(|v| v.as_str()) {
                spec.req = req.to_string();
            }
            if let Some(package) = table.get("package").and_then(|v| v.as_str()) {
                spec.package = package.to_string();
            }
            if let Some(on) = table.get("default-features").and_then(|v| v.as_bool()) {
                spec.default_features = on;
            }
            let features = table.get("features").and_then(|v| v.as_array());
            for feature in features.into_iter().flatten().filter_map(|f| f.as_str()) {
                spec.features.push(feature.to_string());
            }
        }
        _ => {}
    };
    let inherits = value
        .as_table()
        .and_then(|t| t.get("workspace"))
        .and_then(|w| w.as_bool())
        .unwrap_or(false);
    // The workspace's entry first: the member adds features to it.
    if inherits && let Some(shared) = workspace.and_then(|w| w.get(key)) {
        read(shared);
    }
    read(value);
    spec.features.sort();
    spec.features.dedup();
    spec
}

/// Every dependency a `Cargo.toml` names, in name order: its normal, dev
/// and build dependencies, those of `[target.*]` tables too. A dependency
/// named in several tables is one entry. `workspace_manifest` is the text of
/// the workspace's root manifest, which `workspace = true` refers to. A
/// manifest that does not parse names none.
pub fn dependency_specs(manifest: &str, workspace_manifest: Option<&str>) -> Vec<DepSpec> {
    let Ok(table) = manifest.parse::<toml::Table>() else {
        return Vec::new();
    };
    let root: Option<toml::Table> = workspace_manifest.and_then(|text| text.parse().ok());
    // A workspace root that is also a package refers to itself.
    let shared = root
        .as_ref()
        .or(Some(&table))
        .and_then(|t| t.get("workspace"))
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table());
    let targets = table
        .get("target")
        .and_then(|t| t.as_table())
        .into_iter()
        .flat_map(|t| t.values())
        .filter_map(|t| t.as_table());
    let mut specs: BTreeMap<String, DepSpec> = BTreeMap::new();
    for owner in std::iter::once(&table).chain(targets) {
        for name in DEPENDENCY_TABLES {
            let entries = owner.get(name).and_then(|d| d.as_table());
            for (key, value) in entries.into_iter().flatten() {
                let spec = dep_spec(key, value, shared);
                match specs.get_mut(&spec.name) {
                    Some(known) => {
                        if known.req.is_empty() {
                            known.req = spec.req;
                        }
                        known.features.extend(spec.features);
                        known.features.sort();
                        known.features.dedup();
                        known.default_features &= spec.default_features;
                    }
                    None => {
                        specs.insert(spec.name.clone(), spec);
                    }
                }
            }
        }
    }
    specs.into_values().collect()
}

/// The versions a `Cargo.lock` holds of one package.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Locked {
    /// Name the package is published under.
    pub package: String,
    /// Exact versions, in the order of the lock file (lowest first).
    pub versions: Vec<String>,
}

/// The packages of a `Cargo.lock`, by name as used in paths. A lock file
/// that does not parse holds none.
pub fn locked_versions(lock: &str) -> BTreeMap<String, Locked> {
    let mut locked: BTreeMap<String, Locked> = BTreeMap::new();
    let Ok(table) = lock.parse::<toml::Table>() else {
        return locked;
    };
    let packages = table.get("package").and_then(|p| p.as_array());
    for package in packages.into_iter().flatten() {
        let name = package.get("name").and_then(|n| n.as_str());
        let version = package.get("version").and_then(|v| v.as_str());
        if let (Some(name), Some(version)) = (name, version) {
            let entry = locked.entry(name.replace('-', "_")).or_default();
            entry.package = name.to_string();
            entry.versions.push(version.to_string());
        }
    }
    locked
}

/// The `description` of the package a `Cargo.toml` defines, on one line.
pub fn package_description(manifest: &str) -> Option<String> {
    let table: toml::Table = manifest.parse().ok()?;
    let text = table.get("package")?.get("description")?.as_str()?;
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!line.is_empty()).then_some(line)
}

/// The crate that owns `file`: the one with the longest matching directory.
pub fn owning_crate<'a>(crates: &'a [CrateInfo], file: &str) -> Option<&'a CrateInfo> {
    crates
        .iter()
        .filter(|c| c.dir.is_empty() || file.starts_with(&format!("{}/", c.dir)))
        .max_by_key(|c| c.dir.len())
}

/// Qualified module path defined by a Rust source file.
pub fn module_path(crates: &[CrateInfo], file: &str) -> String {
    let Some(krate) = owning_crate(crates, file) else {
        return segments_of(file).join("::");
    };
    let rel = if krate.dir.is_empty() {
        file
    } else {
        &file[krate.dir.len() + 1..]
    };
    let mut segs = vec![krate.name.clone()];
    match rel {
        "src/lib.rs" => {}
        // With a library present the binary is a separate crate root.
        "src/main.rs" if krate.has_lib => segs.push("main".into()),
        "src/main.rs" => {}
        _ => {
            let inner = rel.strip_prefix("src/").unwrap_or(rel);
            let mut parts = segments_of(inner);
            if parts.last().is_some_and(|p| p == "mod") {
                parts.pop();
            }
            if parts.first().is_some_and(|p| p == "bin")
                && parts.last().is_some_and(|p| p == "main")
            {
                parts.pop();
            }
            segs.extend(parts);
        }
    }
    segs.join("::")
}

fn segments_of(path: &str) -> Vec<String> {
    let stem = path.strip_suffix(".rs").unwrap_or(path);
    stem.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.replace(['-', '.'], "_"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(name: &str, dir: &str, has_lib: bool) -> CrateInfo {
        CrateInfo {
            name: name.into(),
            package: name.replace('_', "-"),
            dir: dir.into(),
            description: String::new(),
            deps: vec![],
            code_deps: vec![],
            has_lib,
            has_bin: !has_lib,
        }
    }

    #[test]
    fn parses_package_manifest() {
        let text = "[package]\nname = \"cw-core\"\ndescription = \"Core\"\n[dependencies]\nserde-json = \"1\"\nhex = \"0.4\"\n[dev-dependencies]\ntempfile = \"3\"\n[target.'cfg(windows)'.dependencies]\nwindows-sys = \"0.61\"\nhex = \"0.4\"\n";
        let files = vec!["crates/cw-core/src/lib.rs".to_string()];
        let c = parse_manifest("crates/cw-core", text, &files)
            .unwrap()
            .unwrap();
        assert_eq!(c.name, "cw_core");
        assert_eq!(c.package, "cw-core");
        assert_eq!(c.deps, ["hex", "serde_json"]);
        assert_eq!(
            c.code_deps,
            ["hex", "serde_json", "tempfile", "windows_sys"]
        );
        assert!(c.has_lib && !c.has_bin);
        assert!(
            parse_manifest("", "[workspace]\nmembers = []\n", &[])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn reads_what_a_manifest_asks_of_its_dependencies() {
        let root = "[workspace]\nmembers = [\"crates/*\"]\n[workspace.dependencies]\nserde = { version = \"1.0.200\", features = [\"derive\"] }\ntokio = \"1\"\n";
        let member = "[package]\nname = \"app\"\n[dependencies]\nserde = { workspace = true, features = [\"rc\"] }\ntokio.workspace = true\naxum = { version = \"0.8\", default-features = false, features = [\"macros\", \"json\"] }\njson = { package = \"serde-json\", version = \"1\" }\nlocal = { path = \"../local\" }\n[dev-dependencies]\ntempfile = \"3\"\n[target.'cfg(windows)'.dependencies]\naxum = { version = \"0.8\", features = [\"ws\"] }\n";
        let specs = dependency_specs(member, Some(root));
        let spec = |name: &str| {
            specs
                .iter()
                .find(|s| s.name == name)
                .unwrap_or_else(|| panic!("no {name}"))
        };
        let names: Vec<&str> = specs.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            ["axum", "json", "local", "serde", "tempfile", "tokio"]
        );
        // The workspace's entry, with the member's features added.
        assert_eq!(spec("serde").req, "1.0.200");
        assert_eq!(spec("serde").features, ["derive", "rc"]);
        assert_eq!(spec("tokio").req, "1");
        // One entry for a dependency two tables name.
        assert_eq!(spec("axum").features, ["json", "macros", "ws"]);
        assert!(!spec("axum").default_features);
        assert!(spec("serde").default_features);
        assert_eq!(spec("json").package, "serde-json");
        assert_eq!(spec("local").req, "");
        // Without the workspace's manifest the shared entry is not known.
        assert_eq!(dependency_specs(member, None)[3].req, "");
        assert!(dependency_specs("not toml [", None).is_empty());
    }

    #[test]
    fn reads_a_lock_file_and_a_description() {
        let lock = "version = 4\n\n[[package]]\nname = \"windows-sys\"\nversion = \"0.52.0\"\n\n[[package]]\nname = \"windows-sys\"\nversion = \"0.61.2\"\n\n[[package]]\nname = \"axum\"\nversion = \"0.8.9\"\n";
        let locked = locked_versions(lock);
        assert_eq!(locked["windows_sys"].package, "windows-sys");
        assert_eq!(locked["windows_sys"].versions, ["0.52.0", "0.61.2"]);
        assert_eq!(locked["axum"].versions, ["0.8.9"]);
        assert!(locked_versions("not toml [").is_empty());

        assert_eq!(
            package_description("[package]\nname = \"x\"\ndescription = \"Web framework\\nthat focuses on ergonomics\"\n")
                .as_deref(),
            Some("Web framework that focuses on ergonomics")
        );
        assert_eq!(package_description("[package]\nname = \"x\"\n"), None);
    }

    #[test]
    fn maps_files_to_modules() {
        let crates = vec![
            krate("cw_core", "crates/cw-core", true),
            krate("app", "", false),
        ];
        let m = |f: &str| module_path(&crates, f);
        assert_eq!(m("crates/cw-core/src/lib.rs"), "cw_core");
        assert_eq!(m("crates/cw-core/src/hash.rs"), "cw_core::hash");
        assert_eq!(m("crates/cw-core/src/a/mod.rs"), "cw_core::a");
        assert_eq!(m("crates/cw-core/src/a/b.rs"), "cw_core::a::b");
        assert_eq!(m("crates/cw-core/src/main.rs"), "cw_core::main");
        assert_eq!(m("crates/cw-core/tests/it.rs"), "cw_core::tests::it");
        assert_eq!(m("src/main.rs"), "app");
        assert_eq!(m("src/bin/tool.rs"), "app::bin::tool");
        assert_eq!(m("src/bin/tool/main.rs"), "app::bin::tool");
        assert_eq!(m("build.rs"), "app::build");
    }
}
