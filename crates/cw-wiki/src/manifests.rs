//! What a repository's build files say about a package from outside it: the
//! version asked for, the version locked, the features turned on, and what
//! the package is.
//!
//! The first three are read from files of the repository, so every machine
//! reads the same. What the package is stands in no file of the repository.
//! A [`PackageInfo`] takes it from files the machine may have (cargo's
//! registry cache, `node_modules`), and [`describe`] keeps what was found in
//! `packages.json` beside the vault's manifest, so that a machine without
//! those files writes the same notes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cw_core::{Error, Language, MANIFEST_DIR, Result};
use cw_parse::workspace::{dependency_specs, locked_versions, package_description};
use cw_parse::{Locked, go_sum_versions, parse_go_mod};
use serde::{Deserialize, Serialize};

use crate::plan::Repo;
use crate::units::{Manifest, Unit, xml_attribute, xml_element, xml_elements};

/// File under the vault's manifest folder that keeps the descriptions.
const SAVED: &str = "packages.json";

/// What one build file asks of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// Repo-relative path of the build file.
    pub manifest: String,
    /// Version requirement as written; empty when the file gives none or
    /// was not read.
    pub requirement: String,
    /// Exact version from the lock file; empty when not known.
    pub locked: String,
    /// Features the build file turns on, in name order.
    pub features: Vec<String>,
    /// False when the build file turns the package's default features off.
    pub default_features: bool,
}

impl Declared {
    fn bare(manifest: &str) -> Self {
        Self {
            manifest: manifest.to_string(),
            requirement: String::new(),
            locked: String::new(),
            features: Vec::new(),
            default_features: true,
        }
    }
}

/// What the repository's own files say about outside packages, beyond each
/// unit's build file, and what is known of each package.
#[derive(Debug, Default)]
pub struct Packages {
    /// Text of the root `Cargo.toml`, which `workspace = true` refers to.
    workspace: Option<String>,
    /// `Cargo.lock`, by crate name as used in paths.
    locked: BTreeMap<String, Locked>,
    /// Package -> what it is, in one line. Filled by [`describe`].
    pub descriptions: BTreeMap<String, String>,
}

impl Packages {
    /// Read the files at the repository's root through `read`.
    pub fn read(read: &dyn Fn(&str) -> Option<String>) -> Self {
        Self {
            workspace: read("Cargo.toml"),
            locked: read("Cargo.lock")
                .map(|lock| locked_versions(&lock))
                .unwrap_or_default(),
            descriptions: BTreeMap::new(),
        }
    }

    /// The locked version of the crate `name` that `requirement` asks for.
    /// With several versions in the lock file, the one the requirement
    /// accepts; `None` when that cannot be told.
    pub fn locked(&self, name: &str, requirement: &str) -> Option<&str> {
        let versions = &self.locked.get(name)?.versions;
        let asked = numbers(requirement.trim().trim_start_matches(['^', '=', '~']));
        let fitting = asked.and_then(|asked| {
            versions
                .iter()
                .rfind(|version| numbers(version).is_some_and(|v| accepts(&asked, &v)))
        });
        match (fitting, versions.as_slice()) {
            (Some(version), _) => Some(version),
            (None, [only]) => Some(only),
            _ => None,
        }
    }

    /// Name the crate `name` is published under, which a registry folder
    /// carries: `tree-sitter` for `tree_sitter`.
    pub fn published<'a>(&'a self, name: &'a str) -> &'a str {
        self.locked.get(name).map_or(name, |l| l.package.as_str())
    }
}

/// The leading numbers of a version or of a plain requirement: `1.0.229`,
/// `0.8`. `None` for anything else, such as `>=1, <3` or `*`.
fn numbers(text: &str) -> Option<Vec<u64>> {
    let core = text.split(['-', '+']).next().unwrap_or("").trim();
    let parts: Vec<u64> = core
        .split('.')
        .map(|part| part.parse().ok())
        .collect::<Option<_>>()?;
    (!parts.is_empty() && parts.len() <= 3).then_some(parts)
}

/// Cargo's default rule: `version` is at least `asked` and equal to it up
/// to the first number that is not zero.
fn accepts(asked: &[u64], version: &[u64]) -> bool {
    let fixed = asked
        .iter()
        .position(|n| *n != 0)
        .unwrap_or(asked.len() - 1);
    let mut padded = asked.to_vec();
    padded.resize(3, 0);
    version.len() == 3 && version[..=fixed] == asked[..=fixed] && version >= padded.as_slice()
}

/// What each build file that names `package` asks of it, one entry for
/// each file. `package` is the name integrations go by: a crate as used in
/// paths, an npm package, the first segment of a Python import, the first
/// two of a Java one.
pub fn declared(repo: &Repo, package: &str) -> Vec<Declared> {
    let mut found: Vec<Declared> = Vec::new();
    for unit in &repo.units.list {
        let of_unit = if unit.is_rust() {
            cargo(unit, package, &repo.packages).into_iter().collect()
        } else {
            other(unit, package)
        };
        for declared in of_unit {
            if !found.iter().any(|d| d.manifest == declared.manifest) {
                found.push(declared);
            }
        }
    }
    found
}

fn cargo(unit: &Unit, package: &str, packages: &Packages) -> Option<Declared> {
    let manifest = unit.manifests.first()?;
    if manifest.text.is_empty() {
        // Not read: the scan still knows the names under `[dependencies]`.
        return unit
            .deps
            .iter()
            .any(|d| d == package)
            .then(|| Declared::bare(&manifest.path));
    }
    let spec = dependency_specs(&manifest.text, packages.workspace.as_deref())
        .into_iter()
        .find(|spec| spec.name == package)?;
    Some(Declared {
        manifest: manifest.path.clone(),
        locked: packages
            .locked(package, &spec.req)
            .unwrap_or_default()
            .to_string(),
        requirement: spec.req,
        features: spec.features,
        default_features: spec.default_features,
    })
}

fn other(unit: &Unit, package: &str) -> Vec<Declared> {
    if unit.language == Language::CSharp {
        return nuget(unit, package);
    }
    if unit.language == Language::Go {
        return go(unit, package);
    }
    let named = unit.declares(package);
    unit.manifests
        .iter()
        .filter(|m| named.contains(&m.path.as_str()))
        .map(|m| {
            let (requirement, features) = asked_in(m, package).unwrap_or_default();
            Declared {
                requirement,
                features,
                ..Declared::bare(&m.path)
            }
        })
        .collect()
}

/// The requirement and the extras `manifest` gives for `package`, read by
/// the rules of the file's kind.
fn asked_in(manifest: &Manifest, package: &str) -> Option<(String, Vec<String>)> {
    let file = manifest.path.rsplit('/').next().unwrap_or(&manifest.path);
    match file {
        "package.json" => npm(&manifest.text, package).map(|req| (req, Vec::new())),
        "pom.xml" => maven(&manifest.text, package).map(|req| (req, Vec::new())),
        "build.gradle" | "build.gradle.kts" => {
            gradle(&manifest.text, package).map(|req| (req, Vec::new()))
        }
        _ => python(&manifest.text, package),
    }
}

/// What a Go module's `go.mod` asks of the module `package` (an import path
/// outside the repository, cut to the module that holds it): the version of
/// its `require` line, marked when the line is `// indirect` or a `replace`
/// sends the module elsewhere, and the version `go.sum` holds a hash of.
fn go(unit: &Unit, package: &str) -> Vec<Declared> {
    let Some(manifest) = unit
        .manifests
        .first()
        .filter(|m| m.path.ends_with("go.mod"))
    else {
        return Vec::new();
    };
    let go_mod = parse_go_mod(&manifest.text);
    let Some(require) = go_mod.requires.iter().find(|r| r.path == package) else {
        return Vec::new();
    };
    let mut requirement = require.version.clone();
    if require.indirect {
        requirement.push_str(" (indirect)");
    }
    let replaced = go_mod.replaces.iter().find(|r| {
        r.old == package && (r.old_version.is_empty() || r.old_version == require.version)
    });
    if let Some(replace) = replaced {
        requirement.push_str(&format!(
            ", replaced by {}{}{}",
            replace.new,
            if replace.new_version.is_empty() {
                ""
            } else {
                " "
            },
            replace.new_version
        ));
    }
    let locked = unit
        .lock
        .as_ref()
        .and_then(|lock| go_sum_versions(&lock.text).remove(package))
        .map(|versions| {
            // The version asked for, when go.sum has it; else the newest.
            versions
                .iter()
                .find(|v| **v == require.version)
                .or_else(|| versions.last())
                .cloned()
                .unwrap_or_default()
        })
        .unwrap_or_default();
    vec![Declared {
        requirement,
        locked,
        ..Declared::bare(&manifest.path)
    }]
}

/// The NuGet package a C# namespace root names, from the ids a build file
/// references: the id equal to it, else the id that continues it
/// (`Serilog.Sinks` is `Serilog.Sinks.File`), else the id it continues
/// (`Avalonia.Controls` is part of `Avalonia`). Case does not matter.
fn nuget_id<'a>(ids: &[&'a str], package: &str) -> Option<&'a str> {
    let wanted = package.to_lowercase();
    let lower = |id: &str| id.to_lowercase();
    let continues = |long: &str, short: &str| {
        long.strip_prefix(short)
            .is_some_and(|rest| rest.starts_with('.'))
    };
    ids.iter()
        .find(|id| lower(id) == wanted)
        .or_else(|| ids.iter().find(|id| continues(&lower(id), &wanted)))
        .or_else(|| {
            ids.iter()
                .filter(|id| continues(&wanted, &lower(id)))
                .max_by_key(|id| id.len())
        })
        .copied()
}

/// The `PackageReference` lines of a project or props file: id and the
/// version written there, empty when the version is not.
fn package_references(text: &str) -> Vec<(String, String)> {
    xml_elements(text, "PackageReference")
        .into_iter()
        .filter_map(|(attributes, inner)| {
            let id = xml_attribute(attributes, "Include")
                .or_else(|| xml_attribute(attributes, "Update"))?;
            let version = xml_attribute(attributes, "Version")
                .or_else(|| xml_attribute(attributes, "VersionOverride"))
                .or_else(|| xml_element(inner, "Version").map(str::to_string))
                .unwrap_or_default();
            Some((id, version))
        })
        .collect()
}

/// The version `Directory.Packages.props` gives package `id` (central
/// package management).
fn central_version(unit: &Unit, id: &str) -> Option<String> {
    unit.manifests
        .iter()
        .filter(|m| m.path.ends_with("Directory.Packages.props"))
        .flat_map(|m| xml_elements(&m.text, "PackageVersion"))
        .find_map(|(attributes, _)| {
            let named = xml_attribute(attributes, "Include")?;
            named
                .eq_ignore_ascii_case(id)
                .then(|| xml_attribute(attributes, "Version"))
                .flatten()
        })
}

/// The version of package `id` that the unit's `packages.lock.json` pins.
fn locked_nuget(unit: &Unit, id: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(&unit.lock.as_ref()?.text).ok()?;
    value
        .get("dependencies")?
        .as_object()?
        .values()
        .find_map(|framework| {
            framework
                .as_object()?
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(id))?
                .1
                .get("resolved")?
                .as_str()
                .map(str::to_string)
        })
}

/// What the project files of a C# unit ask of `package`: the version written
/// beside the reference or, for a reference with none, the central one, and
/// the version the lock file pins.
fn nuget(unit: &Unit, package: &str) -> Vec<Declared> {
    let mut out = Vec::new();
    for manifest in &unit.manifests {
        let name = manifest.path.rsplit('/').next().unwrap_or(&manifest.path);
        if !(name.ends_with(".csproj") || name == "Directory.Build.props") {
            continue;
        }
        let references = package_references(&manifest.text);
        let ids: Vec<&str> = references.iter().map(|(id, _)| id.as_str()).collect();
        let Some(id) = nuget_id(&ids, package) else {
            continue;
        };
        let written = references
            .iter()
            .find(|(named, _)| named == id)
            .map(|(_, version)| version.clone())
            .unwrap_or_default();
        let requirement = if written.is_empty() {
            central_version(unit, id).unwrap_or_default()
        } else {
            written
        };
        out.push(Declared {
            requirement,
            locked: locked_nuget(unit, id).unwrap_or_default(),
            ..Declared::bare(&manifest.path)
        });
    }
    out
}

fn npm(text: &str, package: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ]
    .iter()
    .find_map(|table| value.get(table)?.get(package)?.as_str())
    .map(str::to_string)
}

/// True when the Maven group `group` is the one a Java import of `package`
/// (its first two segments) comes from.
fn in_group(group: &str, package: &str) -> bool {
    group
        .strip_prefix(package)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}

/// Text of the first `<tag>` element in `xml`.
fn element<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&format!("</{tag}>"))?;
    Some(xml[start..start + end].trim())
}

fn maven(text: &str, package: &str) -> Option<String> {
    text.split("<dependency>")
        .skip(1)
        .filter_map(|rest| rest.split("</dependency>").next())
        .find(|block| element(block, "groupId").is_some_and(|g| in_group(g, package)))
        .map(|block| element(block, "version").unwrap_or("").to_string())
}

/// Gradle names a dependency in a string, `"group:artifact:version"`.
fn gradle(text: &str, package: &str) -> Option<String> {
    text.split(['"', '\''])
        // Every second piece stands between two quotes.
        .skip(1)
        .step_by(2)
        .map(|quoted| quoted.split(':').collect::<Vec<_>>())
        .find(|parts| parts.len() >= 2 && in_group(parts[0], package))
        .map(|parts| parts.get(2).copied().unwrap_or("").to_string())
}

/// Python build files are of many kinds (`requirements.txt`,
/// `pyproject.toml`, `setup.py`, `Pipfile`), so the text after the name is
/// read as it stands: extras in brackets, then the version specifier.
fn python(text: &str, package: &str) -> Option<(String, Vec<String>)> {
    let lower = text.to_lowercase();
    let needle = package.to_lowercase();
    let part = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-');
    let mut first: Option<(String, Vec<String>)> = None;
    for (at, _) in lower.match_indices(&needle) {
        let after = at + needle.len();
        if lower[..at].chars().next_back().is_some_and(part)
            || lower[after..].chars().next().is_some_and(part)
        {
            continue;
        }
        // The lowercase text has the offsets of the original only when no
        // letter changed its length; read from it, then, since a version
        // has no case that matters.
        let mut rest = &lower[after..];
        let mut extras = Vec::new();
        if let Some(inner) = rest.strip_prefix('[')
            && let Some(end) = inner.find(']')
        {
            extras = inner[..end]
                .split(',')
                .map(|e| e.trim().to_string())
                .filter(|e| !e.is_empty())
                .collect();
            rest = &inner[end + 1..];
        }
        rest = rest.trim_start_matches([' ', '\t']);
        // `name = "^2.31"` in a TOML table: an assignment, not `==`.
        if rest.starts_with('=') && !rest.starts_with("==") {
            rest = rest[1..].trim_start_matches([' ', '\t', '"', '\'']);
        }
        let spec: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || "=<>!~^*.,+ ".contains(*c))
            .collect();
        // A word after a space is no part of a version: `requests and ...`.
        let words: Vec<&str> = spec.split(' ').collect();
        let end = words
            .iter()
            .position(|w| w.starts_with(|c: char| c.is_ascii_alphabetic()))
            .unwrap_or(words.len());
        let spec = words[..end]
            .join(" ")
            .trim()
            .trim_end_matches(',')
            .to_string();
        let starts_right = spec.starts_with(|c: char| c.is_ascii_digit() || "=<>!~^".contains(c));
        let spec = if starts_right { spec } else { String::new() };
        if !spec.is_empty() {
            return Some((spec, extras));
        }
        first.get_or_insert((spec, extras));
    }
    first
}

/// Says what a package from outside the repository is.
pub trait PackageInfo {
    /// What `package` is, in one line, or `None` when it is not known.
    /// `package` is the name it is published under, `version` the locked
    /// one when the repository has a lock file.
    fn describe(&self, language: Language, package: &str, version: Option<&str>) -> Option<String>;
}

/// Knows no package. A run with it writes what `packages.json` holds.
pub struct NoPackageInfo;

impl PackageInfo for NoPackageInfo {
    fn describe(&self, _: Language, _: &str, _: Option<&str>) -> Option<String> {
        None
    }
}

/// Reads the files a build on this machine left behind: the sources cargo
/// unpacked into its registry cache, and `node_modules`. It never asks the
/// network, and it knows nothing on a machine that has not built the
/// repository.
pub struct Machine {
    root: PathBuf,
    /// The folders of `<cargo home>/registry/src`, one for each registry.
    registries: Vec<PathBuf>,
    /// The folder NuGet unpacks packages into, when the machine has one.
    nuget: Option<PathBuf>,
}

impl Machine {
    /// For the repository at `root`, with the cargo home the environment
    /// names (`CARGO_HOME`, or `.cargo` in the user's home folder).
    pub fn new(root: &Path) -> Self {
        let home = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .map(|home| PathBuf::from(home).join(".cargo"))
            });
        let nuget = std::env::var_os("NUGET_PACKAGES")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .map(|home| PathBuf::from(home).join(".nuget").join("packages"))
            });
        Self::with_cargo_home(root, home.as_deref()).with_nuget(nuget)
    }

    /// The same machine with NuGet's package folder given.
    pub fn with_nuget(mut self, folder: Option<PathBuf>) -> Self {
        self.nuget = folder.filter(|f| f.is_dir());
        self
    }

    /// As [`Machine::new`], with the cargo home given.
    pub fn with_cargo_home(root: &Path, cargo_home: Option<&Path>) -> Self {
        let mut registries: Vec<PathBuf> = cargo_home
            .and_then(|home| std::fs::read_dir(home.join("registry").join("src")).ok())
            .into_iter()
            .flatten()
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|path| path.is_dir())
            .collect();
        registries.sort();
        Self {
            root: root.to_path_buf(),
            registries,
            nuget: None,
        }
    }

    /// The `description` of the NuGet package a namespace root names, from
    /// the `.nuspec` NuGet unpacked next to the package. NuGet names its
    /// folders by the lowercase id, so the package is the folder named like
    /// `package`, else the first one that continues it.
    fn nuget_description(&self, package: &str, version: Option<&str>) -> Option<String> {
        let folder = self.nuget.as_ref()?;
        let wanted = package.to_lowercase();
        let mut ids: Vec<String> = std::fs::read_dir(folder)
            .ok()?
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|name| name == &wanted || name.starts_with(&format!("{wanted}.")))
            .collect();
        ids.sort();
        let id = ids
            .iter()
            .find(|id| **id == wanted)
            .or_else(|| ids.first())?;
        let package_dir = folder.join(id);
        let version_dir = match version {
            Some(version) if package_dir.join(version).is_dir() => package_dir.join(version),
            _ => {
                let mut versions: Vec<PathBuf> = std::fs::read_dir(&package_dir)
                    .ok()?
                    .filter_map(|entry| Some(entry.ok()?.path()))
                    .filter(|path| path.is_dir())
                    .collect();
                versions.sort();
                versions.pop()?
            }
        };
        let nuspec = std::fs::read_dir(&version_dir)
            .ok()?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .find(|path| path.extension().is_some_and(|e| e == "nuspec"))?;
        let text = std::fs::read_to_string(nuspec).ok()?;
        let line = xml_element(&text, "description")?
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        (!line.is_empty()).then_some(line)
    }

    fn crate_manifest(&self, package: &str, version: Option<&str>) -> Option<String> {
        self.registries.iter().find_map(|registry| {
            let folder = match version {
                Some(version) => registry.join(format!("{package}-{version}")),
                // No lock file: the newest copy the cache holds.
                None => {
                    let prefix = format!("{package}-");
                    let mut copies: Vec<PathBuf> = std::fs::read_dir(registry)
                        .ok()?
                        .filter_map(|entry| Some(entry.ok()?.path()))
                        .filter(|path| {
                            path.file_name()
                                .and_then(|n| n.to_str())
                                .and_then(|n| n.strip_prefix(&prefix))
                                .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()))
                        })
                        .collect();
                    copies.sort();
                    copies.pop()?
                }
            };
            std::fs::read_to_string(folder.join("Cargo.toml")).ok()
        })
    }
}

impl PackageInfo for Machine {
    fn describe(&self, language: Language, package: &str, version: Option<&str>) -> Option<String> {
        match language {
            Language::Rust => package_description(&self.crate_manifest(package, version)?),
            Language::TypeScript | Language::JavaScript => {
                let path = self
                    .root
                    .join("node_modules")
                    .join(package)
                    .join("package.json");
                let value: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
                let line = value
                    .get("description")?
                    .as_str()?
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                (!line.is_empty()).then_some(line)
            }
            Language::CSharp => self.nuget_description(package, version),
            _ => None,
        }
    }
}

/// One package in `packages.json`.
#[derive(Debug, Serialize, Deserialize)]
struct Saved {
    description: String,
}

fn saved_path(vault: &Path) -> PathBuf {
    vault.join(MANIFEST_DIR).join(SAVED)
}

fn load(vault: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(saved_path(vault))
        .ok()
        .and_then(|text| serde_json::from_str::<BTreeMap<String, Saved>>(&text).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|(name, saved)| (name, saved.description))
        .collect()
}

/// Say what each of `packages` is and keep the answers in the vault at
/// `vault`. What `info` finds on this machine wins; what it does not find
/// is taken from the file an earlier run wrote. The file then holds exactly
/// the packages named here that have a description, and is written only
/// when that differs from what it held.
pub fn describe(
    repo: &Repo,
    vault: &Path,
    packages: &[&str],
    info: &dyn PackageInfo,
) -> Result<BTreeMap<String, String>> {
    let before = load(vault);
    let mut known: BTreeMap<String, String> = BTreeMap::new();
    for package in packages {
        let language = repo
            .facts
            .external_uses
            .iter()
            .find(|u| u.krate == *package)
            .map_or(Language::Other, |u| Language::from_path(&u.file));
        let version = declared(repo, package)
            .into_iter()
            .map(|d| d.locked)
            .find(|locked| !locked.is_empty());
        let found = info
            .describe(
                language,
                repo.packages.published(package),
                version.as_deref(),
            )
            .or_else(|| before.get(*package).cloned());
        if let Some(description) = found {
            known.insert(package.to_string(), description);
        }
    }
    if known != before {
        let path = saved_path(vault);
        if known.is_empty() {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(Error::io(&path, e)),
            }
        } else {
            let saved: BTreeMap<&String, Saved> = known
                .iter()
                .map(|(name, description)| {
                    let description = description.clone();
                    (name, Saved { description })
                })
                .collect();
            let mut text = serde_json::to_string_pretty(&saved)
                .map_err(|e| Error::external("encoding package descriptions", e))?;
            text.push('\n');
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            std::fs::write(&path, text).map_err(|e| Error::io(&path, e))?;
        }
    }
    Ok(known)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packages(lock: &str) -> Packages {
        Packages::read(&|path| (path == "Cargo.lock").then(|| lock.to_string()))
    }

    #[test]
    fn picks_the_locked_version_a_requirement_accepts() {
        let lock = "[[package]]\nname = \"windows-sys\"\nversion = \"0.52.0\"\n\n[[package]]\nname = \"windows-sys\"\nversion = \"0.61.2\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.230\"\n\n[[package]]\nname = \"rand\"\nversion = \"0.8.5\"\n\n[[package]]\nname = \"rand\"\nversion = \"0.9.2\"\n";
        let p = packages(lock);
        assert_eq!(p.locked("windows_sys", "0.61"), Some("0.61.2"));
        assert_eq!(p.locked("windows_sys", "0.52.0"), Some("0.52.0"));
        // A newer patch of the version asked for.
        assert_eq!(p.locked("serde", "1.0.229"), Some("1.0.230"));
        assert_eq!(p.locked("serde", "^1"), Some("1.0.230"));
        // The one version there is, whatever the requirement says.
        assert_eq!(p.locked("serde", ">=1, <3"), Some("1.0.230"));
        // Two versions and a requirement that names neither: not known.
        assert_eq!(p.locked("rand", ">=0.8"), None);
        assert_eq!(p.locked("rand", "0.7"), None);
        assert_eq!(p.locked("missing", "1"), None);
        assert_eq!(p.published("windows_sys"), "windows-sys");
        assert_eq!(p.published("unknown_crate"), "unknown_crate");
    }

    /// Knows `axum` at the version the lock file holds, and nothing else.
    struct OnlyAxum;

    impl PackageInfo for OnlyAxum {
        fn describe(
            &self,
            language: Language,
            package: &str,
            version: Option<&str>,
        ) -> Option<String> {
            (language == Language::Rust && package == "axum" && version == Some("0.8.9"))
                .then(|| "Web framework".to_string())
        }
    }

    #[test]
    fn a_description_found_once_is_kept_in_the_vault() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write(
            "Cargo.toml",
            "[package]\nname = \"demo\"\n[dependencies]\naxum = { version = \"0.8\", features = [\"macros\"] }\n[dev-dependencies]\ntempfile = \"3\"\n",
        );
        write(
            "Cargo.lock",
            "[[package]]\nname = \"axum\"\nversion = \"0.8.9\"\n",
        );
        write("src/lib.rs", "use axum::Router;\nuse tempfile::tempdir;\n");
        let facts = cw_parse::scan_repo(root).unwrap();
        let repo = Repo::at(root, &facts, &[]);

        assert_eq!(
            declared(&repo, "axum"),
            [Declared {
                manifest: "Cargo.toml".into(),
                requirement: "0.8".into(),
                locked: "0.8.9".into(),
                features: vec!["macros".into()],
                default_features: true,
            }]
        );
        // A dependency of the tests is declared too, and not locked here.
        assert_eq!(declared(&repo, "tempfile")[0].requirement, "3");
        assert_eq!(declared(&repo, "tempfile")[0].locked, "");
        assert!(declared(&repo, "serde").is_empty());

        let vault = root.join(cw_core::VAULT_DIR);
        let saved = saved_path(&vault);
        let wanted = ["axum", "tempfile"];
        let known = describe(&repo, &vault, &wanted, &OnlyAxum).unwrap();
        assert_eq!(known.get("axum").map(String::as_str), Some("Web framework"));
        assert!(!known.contains_key("tempfile"));
        let text = std::fs::read_to_string(&saved).unwrap();
        assert!(
            text.contains("\"description\": \"Web framework\""),
            "{text}"
        );

        // A machine that knows nothing writes the same.
        let again = describe(&repo, &vault, &wanted, &NoPackageInfo).unwrap();
        assert_eq!(again, known);
        assert_eq!(std::fs::read_to_string(&saved).unwrap(), text);

        // A package that is no integration any more leaves the file.
        let none = describe(&repo, &vault, &[], &NoPackageInfo).unwrap();
        assert!(none.is_empty());
        assert!(!saved.exists());
    }

    #[test]
    fn reads_requirements_of_other_languages() {
        let npm_text = "{\"dependencies\": {\"react\": \"^18.2.0\"}, \"devDependencies\": {\"vitest\": \"1.6.0\"}}";
        assert_eq!(npm(npm_text, "react").as_deref(), Some("^18.2.0"));
        assert_eq!(npm(npm_text, "vitest").as_deref(), Some("1.6.0"));
        assert_eq!(npm(npm_text, "vue"), None);

        let pom = "<dependencies>\n<dependency>\n<groupId>org.slf4j</groupId>\n<artifactId>slf4j-api</artifactId>\n<version>2.0.9</version>\n</dependency>\n<dependency>\n<groupId>org.junit.jupiter</groupId>\n<artifactId>junit-jupiter</artifactId>\n</dependency>\n</dependencies>";
        assert_eq!(maven(pom, "org.slf4j").as_deref(), Some("2.0.9"));
        // A version the parent manages is not written here.
        assert_eq!(maven(pom, "org.junit").as_deref(), Some(""));
        assert_eq!(maven(pom, "org.slf"), None);

        let build = "dependencies {\n    implementation 'org.slf4j:slf4j-api:2.0.9'\n    testImplementation(\"org.junit.jupiter:junit-jupiter\")\n}\n";
        assert_eq!(gradle(build, "org.slf4j").as_deref(), Some("2.0.9"));
        assert_eq!(gradle(build, "org.junit").as_deref(), Some(""));
        assert_eq!(gradle(build, "com.google"), None);

        let asked = |text: &str| python(text, "requests");
        let plain = |spec: &str| Some((spec.to_string(), Vec::new()));
        assert_eq!(asked("flask\nrequests==2.31.0\n"), plain("==2.31.0"));
        assert_eq!(
            asked("requests >= 2.0, < 3  # http\n"),
            plain(">= 2.0, < 3")
        );
        assert_eq!(
            asked("dependencies = [\"requests[security,socks]>=2.28\"]\n"),
            Some((
                ">=2.28".to_string(),
                vec!["security".into(), "socks".into()]
            ))
        );
        assert_eq!(asked("[packages]\nrequests = \"^2.31\"\n"), plain("^2.31"));
        // Named with no version, and not named at all.
        assert_eq!(asked("requests\n"), plain(""));
        assert_eq!(asked("# uses requests and more\n"), plain(""));
        assert_eq!(asked("requests-oauthlib==1.0\n"), None);
    }

    #[test]
    fn reads_a_description_from_the_cargo_cache_and_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        let registry = "cargo/registry/src/index.crates.io-1949cf8c6b5b557f";
        write(
            &format!("{registry}/tree-sitter-0.24.7/Cargo.toml"),
            "[package]\nname = \"tree-sitter\"\ndescription = \"Rust bindings to the Tree-sitter parsing library\"\n",
        );
        write(
            &format!("{registry}/tree-sitter-0.25.1/Cargo.toml"),
            "[package]\nname = \"tree-sitter\"\ndescription = \"Newer bindings\"\n",
        );
        write(
            &format!("{registry}/tree-sitter-rust-0.24.0/Cargo.toml"),
            "[package]\nname = \"tree-sitter-rust\"\ndescription = \"Rust grammar\"\n",
        );
        write(
            "repo/node_modules/@scope/sdk/package.json",
            "{\"name\": \"@scope/sdk\", \"description\": \"Client for\\n the service\"}",
        );
        let machine =
            Machine::with_cargo_home(&dir.path().join("repo"), Some(&dir.path().join("cargo")));
        let rust = |version| machine.describe(Language::Rust, "tree-sitter", version);
        assert_eq!(
            rust(Some("0.24.7")).as_deref(),
            Some("Rust bindings to the Tree-sitter parsing library")
        );
        // With no lock file, the newest copy; `tree-sitter-rust` is another
        // package.
        assert_eq!(rust(None).as_deref(), Some("Newer bindings"));
        assert_eq!(rust(Some("9.9.9")), None);
        assert_eq!(
            machine
                .describe(Language::TypeScript, "@scope/sdk", None)
                .as_deref(),
            Some("Client for the service")
        );
        assert_eq!(machine.describe(Language::Python, "requests", None), None);
        // No cargo home at all.
        let bare = Machine::with_cargo_home(dir.path(), None);
        assert_eq!(bare.describe(Language::Rust, "tree-sitter", None), None);
    }

    fn csharp_unit(manifests: &[(&str, &str)], lock: Option<&str>) -> Unit {
        Unit {
            name: "core".into(),
            package: "Core".into(),
            slug: "core".into(),
            dir: "src/Core".into(),
            description: String::new(),
            deps: Vec::new(),
            has_lib: false,
            has_bin: false,
            language: Language::CSharp,
            manifests: manifests
                .iter()
                .map(|(path, text)| Manifest {
                    path: path.to_string(),
                    text: text.to_string(),
                })
                .collect(),
            root: None,
            strip: 0,
            entries: Vec::new(),
            scripts: Vec::new(),
            tests: false,
            lock: lock.map(|text| Manifest {
                path: "src/Core/packages.lock.json".into(),
                text: text.into(),
            }),
        }
    }

    #[test]
    fn reads_nuget_requirements_from_the_project_the_props_and_the_lock() {
        let project = r#"<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <PackageReference Include="Serilog" Version="4.2.0" />
    <PackageReference Include="Serilog.Sinks.File" />
    <PackageReference Include="Avalonia" />
    <PackageReference Include="xunit.v3">
      <Version>3.2.2</Version>
    </PackageReference>
  </ItemGroup>
</Project>"#;
        let central = r#"<Project><ItemGroup>
  <PackageVersion Include="Serilog.Sinks.File" Version="6.0.0" />
  <PackageVersion Include="Avalonia" Version="11.3.0" />
</ItemGroup></Project>"#;
        let lock = r#"{"version": 2, "dependencies": {"net10.0": {
  "Serilog.Sinks.File": {"type": "Direct", "requested": "[6.0.0, )", "resolved": "6.0.1"},
  "Avalonia": {"type": "Direct", "requested": "[11.3.0, )", "resolved": "11.3.2"}
}}}"#;
        let unit = csharp_unit(
            &[
                ("src/Core/Core.csproj", project),
                ("Directory.Packages.props", central),
            ],
            Some(lock),
        );
        let asked = |package: &str| -> Vec<(String, String, String)> {
            nuget(&unit, package)
                .into_iter()
                .map(|d| (d.manifest, d.requirement, d.locked))
                .collect()
        };
        let csproj = "src/Core/Core.csproj".to_string();
        // A version beside the reference; no lock entry for it.
        assert_eq!(
            asked("Serilog"),
            [(csproj.clone(), "4.2.0".into(), "".into())]
        );
        // A namespace root two segments long names the package that continues
        // it, and the central file gives the version the reference lacks.
        assert_eq!(
            asked("Serilog.Sinks"),
            [(csproj.clone(), "6.0.0".into(), "6.0.1".into())]
        );
        assert_eq!(
            asked("Avalonia.Controls"),
            [(csproj.clone(), "11.3.0".into(), "11.3.2".into())]
        );
        // The id and the namespace differ in case and in shape.
        assert_eq!(
            asked("Xunit"),
            [(csproj.clone(), "3.2.2".into(), "".into())]
        );
        assert!(asked("Newtonsoft.Json").is_empty());
        // The central file and the lock are no declarations of their own.
        assert!(
            nuget(&unit, "Avalonia")
                .iter()
                .all(|d| d.manifest == csproj)
        );
    }

    fn go_unit(go_mod: &str, go_sum: Option<&str>) -> Unit {
        Unit {
            name: "app".into(),
            package: "app".into(),
            slug: "app".into(),
            dir: String::new(),
            description: String::new(),
            deps: Vec::new(),
            has_lib: false,
            has_bin: false,
            language: Language::Go,
            manifests: vec![Manifest {
                path: "go.mod".into(),
                text: go_mod.into(),
            }],
            root: None,
            strip: 0,
            entries: Vec::new(),
            scripts: Vec::new(),
            tests: false,
            lock: go_sum.map(|text| Manifest {
                path: "go.sum".into(),
                text: text.into(),
            }),
        }
    }

    #[test]
    fn reads_go_requirements_from_the_go_mod_and_the_go_sum() {
        let unit = go_unit(
            "module example.com/app\n\nrequire (\n\tgithub.com/spf13/cobra v1.8.0\n\tgolang.org/x/sync v0.7.0 // indirect\n\tgithub.com/google/uuid v1.6.0\n\tgithub.com/local/lib v0.1.0\n)\n\nreplace github.com/google/uuid => github.com/fork/uuid v1.6.1\nreplace github.com/local/lib => ../lib\n",
            Some(
                "github.com/spf13/cobra v1.8.0 h1:a=\ngithub.com/spf13/cobra v1.8.0/go.mod h1:b=\ngithub.com/spf13/cobra v1.7.0 h1:c=\ngolang.org/x/sync v0.6.0 h1:d=\n",
            ),
        );
        let asked = |package: &str| -> Vec<(String, String, String)> {
            go(&unit, package)
                .into_iter()
                .map(|d| (d.manifest, d.requirement, d.locked))
                .collect()
        };
        assert_eq!(
            asked("github.com/spf13/cobra"),
            [("go.mod".into(), "v1.8.0".into(), "v1.8.0".into())]
        );
        // `// indirect` is said; go.sum holds another version than go.mod
        // asks for, and the newest of those stands.
        assert_eq!(
            asked("golang.org/x/sync"),
            [("go.mod".into(), "v0.7.0 (indirect)".into(), "v0.6.0".into())]
        );
        // A replace by another module, and by a folder of this machine.
        assert_eq!(
            asked("github.com/google/uuid"),
            [(
                "go.mod".into(),
                "v1.6.0, replaced by github.com/fork/uuid v1.6.1".into(),
                "".into()
            )]
        );
        assert_eq!(
            asked("github.com/local/lib"),
            [(
                "go.mod".into(),
                "v0.1.0, replaced by ../lib".into(),
                "".into()
            )]
        );
        assert!(asked("github.com/other/thing").is_empty());
        // Without a go.sum nothing is locked.
        let bare = go_unit("module x\nrequire a.com/b/c v1.0.0\n", None);
        assert_eq!(go(&bare, "a.com/b/c")[0].locked, "");
        // A package from outside has no description on a machine without one.
        let machine = Machine::with_cargo_home(Path::new("."), None);
        assert_eq!(
            machine.describe(Language::Go, "github.com/spf13/cobra", None),
            None
        );
    }

    #[test]
    fn a_nuget_description_comes_from_the_unpacked_package() {
        let dir = tempfile::tempdir().unwrap();
        let write = |rel: &str, text: &str| {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write(
            "nuget/serilog.sinks.file/6.0.0/serilog.sinks.file.nuspec",
            "<package><metadata><id>Serilog.Sinks.File</id><description>Write log events\n  to files.</description></metadata></package>",
        );
        write(
            "nuget/serilog.sinks.file/5.0.0/serilog.sinks.file.nuspec",
            "<package><metadata><description>Old.</description></metadata></package>",
        );
        write(
            "nuget/avalonia/11.3.2/avalonia.nuspec",
            "<package><metadata><description>Cross-platform UI.</description></metadata></package>",
        );
        let machine =
            Machine::with_cargo_home(dir.path(), None).with_nuget(Some(dir.path().join("nuget")));
        let describe = |package, version| machine.describe(Language::CSharp, package, version);
        assert_eq!(
            describe("Serilog.Sinks", Some("6.0.0")).as_deref(),
            Some("Write log events to files.")
        );
        // The locked version, else the newest unpacked.
        assert_eq!(
            describe("Serilog.Sinks", Some("5.0.0")).as_deref(),
            Some("Old.")
        );
        assert_eq!(
            describe("Serilog.Sinks", None).as_deref(),
            Some("Write log events to files.")
        );
        assert_eq!(
            describe("Avalonia", None).as_deref(),
            Some("Cross-platform UI.")
        );
        assert_eq!(describe("Newtonsoft.Json", None), None);
        // A machine with no NuGet folder knows nothing.
        let bare = Machine::with_cargo_home(dir.path(), None).with_nuget(None);
        assert_eq!(bare.describe(Language::CSharp, "Avalonia", None), None);
    }
}
