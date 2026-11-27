//! Gradle's version catalog, `gradle/libs.versions.toml`, read as TOML.
//!
//! The format is documented at
//! <https://docs.gradle.org/current/userguide/version_catalogs.html>. A
//! library is written as a string `"group:name:version"`, or as a table with
//! a `module` (`"group:name"`) or a `group` and a `name`, and a `version`
//! that is a string, a table of `require`, `strictly` and `prefer`, or a
//! `version.ref` naming a key of `[versions]`. A build script uses it by the
//! accessor of its alias: `libs.kotlinx.coroutines.core` for
//! `kotlinx-coroutines-core`. Only the default catalog, named `libs`, is read.

use toml::{Table, Value};

/// One entry of the `[libraries]` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogLibrary {
    /// The key, as written: `kotlinx-coroutines-core`.
    pub alias: String,
    pub group: String,
    pub name: String,
    /// The version asked for, empty when the catalog gives none (a BOM
    /// decides) or gives it in a way that is no single version.
    pub version: String,
}

impl CatalogLibrary {
    /// The name a build script uses: `libs.kotlinx.coroutines.core`.
    pub fn accessor(&self) -> String {
        format!("libs.{}", self.alias.replace(['-', '_'], "."))
    }
}

/// The libraries of a version catalog. A text that is no TOML gives none.
pub fn parse_version_catalog(text: &str) -> Vec<CatalogLibrary> {
    let Ok(table) = text.parse::<Table>() else {
        return Vec::new();
    };
    let versions = table.get("versions").and_then(Value::as_table);
    let Some(libraries) = table.get("libraries").and_then(Value::as_table) else {
        return Vec::new();
    };
    // A version is a string, or a table that holds one of these.
    let version_of = |value: &Value| -> String {
        match value {
            Value::String(version) => version.clone(),
            Value::Table(rich) => ["require", "strictly", "prefer"]
                .iter()
                .find_map(|key| rich.get(*key).and_then(Value::as_str))
                .unwrap_or("")
                .to_string(),
            _ => String::new(),
        }
    };
    let mut out = Vec::new();
    for (alias, value) in libraries {
        let (coordinates, version) = match value {
            Value::String(text) => {
                let mut parts = text.splitn(3, ':');
                let (group, name) = (parts.next(), parts.next());
                let version = parts.next().unwrap_or("").to_string();
                (
                    group.zip(name).map(|(g, n)| (g.to_string(), n.to_string())),
                    version,
                )
            }
            Value::Table(library) => {
                let coordinates = match library.get("module").and_then(Value::as_str) {
                    Some(module) => module
                        .split_once(':')
                        .map(|(g, n)| (g.to_string(), n.to_string())),
                    None => library
                        .get("group")
                        .and_then(Value::as_str)
                        .zip(library.get("name").and_then(Value::as_str))
                        .map(|(g, n)| (g.to_string(), n.to_string())),
                };
                let version = match library.get("version") {
                    // `version.ref = "key"` is a table with one key, `ref`.
                    Some(Value::Table(rich)) if rich.contains_key("ref") => rich
                        .get("ref")
                        .and_then(Value::as_str)
                        .and_then(|key| versions?.get(key))
                        .map(version_of)
                        .unwrap_or_default(),
                    Some(other) => version_of(other),
                    None => String::new(),
                };
                (coordinates, version)
            }
            _ => continue,
        };
        let Some((group, name)) = coordinates else {
            continue;
        };
        out.push(CatalogLibrary {
            alias: alias.clone(),
            group,
            name,
            version,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_form_of_a_library() {
        let catalog = r#"
[versions]
coroutines = "1.7.3"
ktor = { strictly = "2.3.0" }

[libraries]
kotlinx-coroutines-core = { module = "org.jetbrains.kotlinx:kotlinx-coroutines-core", version.ref = "coroutines" }
ktor-client = { group = "io.ktor", name = "ktor-client-core", version.ref = "ktor" }
junit = "junit:junit:4.13.2"
guava = { module = "com.google.guava:guava", version = { require = "32.1.0" } }
bom-managed.module = "org.example:managed"
inline = { module = "org.example:inline", version = "1.0" }
missing-ref = { module = "org.example:missing", version.ref = "nothing" }

[plugins]
kotlin = { id = "org.jetbrains.kotlin.jvm", version = "1.9.0" }
"#;
        let libraries = parse_version_catalog(catalog);
        let find = |alias: &str| {
            libraries
                .iter()
                .find(|l| l.alias == alias)
                .unwrap_or_else(|| panic!("missing {alias}"))
        };
        let coroutines = find("kotlinx-coroutines-core");
        assert_eq!(
            (
                coroutines.group.as_str(),
                coroutines.name.as_str(),
                coroutines.version.as_str()
            ),
            ("org.jetbrains.kotlinx", "kotlinx-coroutines-core", "1.7.3")
        );
        assert_eq!(coroutines.accessor(), "libs.kotlinx.coroutines.core");
        assert_eq!(find("ktor-client").version, "2.3.0");
        assert_eq!(find("ktor-client").name, "ktor-client-core");
        assert_eq!(find("junit").version, "4.13.2");
        assert_eq!(find("guava").version, "32.1.0");
        assert_eq!(find("bom-managed").version, "");
        assert_eq!(find("inline").version, "1.0");
        assert_eq!(find("missing-ref").version, "");
        assert_eq!(libraries.len(), 7);
        assert!(parse_version_catalog("not = [toml").is_empty());
    }
}
