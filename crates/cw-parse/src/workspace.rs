//! Cargo package discovery and the file-to-module mapping.

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
    pub has_lib: bool,
    pub has_bin: bool,
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
    let mut deps: Vec<String> = table
        .get("dependencies")
        .and_then(|d| d.as_table())
        .map(|t| t.keys().map(|k| k.replace('-', "_")).collect())
        .unwrap_or_default();
    deps.sort();
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
        has_lib: has("src/lib.rs"),
        has_bin: has("src/main.rs") || has_bin_dir || table.contains_key("bin"),
    }))
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
            has_lib,
            has_bin: !has_lib,
        }
    }

    #[test]
    fn parses_package_manifest() {
        let text = "[package]\nname = \"cw-core\"\ndescription = \"Core\"\n[dependencies]\nserde-json = \"1\"\nhex = \"0.4\"\n";
        let files = vec!["crates/cw-core/src/lib.rs".to_string()];
        let c = parse_manifest("crates/cw-core", text, &files)
            .unwrap()
            .unwrap();
        assert_eq!(c.name, "cw_core");
        assert_eq!(c.package, "cw-core");
        assert_eq!(c.deps, ["hex", "serde_json"]);
        assert!(c.has_lib && !c.has_bin);
        assert!(
            parse_manifest("", "[workspace]\nmembers = []\n", &[])
                .unwrap()
                .is_none()
        );
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
