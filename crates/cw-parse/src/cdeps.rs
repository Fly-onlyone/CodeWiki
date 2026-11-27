//! The files a C or C++ project names its libraries in, other than
//! `CMakeLists.txt` (see `cmake.rs`): `vcpkg.json`, `conanfile.txt`,
//! `conanfile.py` and `conan.lock`.
//!
//! Formats, from the tools' own documentation:
//! - vcpkg's manifest (`vcpkg.json`): `dependencies` is a list of names or of
//!   objects with `name` and optionally `version>=`; `overrides` is a list of
//!   `{ "name", "version" }`; `builtin-baseline` is a commit of the registry.
//! - Conan 2's `conanfile.txt`: a `[requires]` section with one
//!   `name/version` per line, an optional `@user/channel` or
//!   `#revision` after it, and `[tool_requires]` for build tools.
//! - `conanfile.py`: `requires = "name/version"` or a tuple or list of
//!   those, and `self.requires("name/version")` in `requirements()`.
//! - `conan.lock`: JSON with `requires` (and `build_requires`), each
//!   `"name/version#revision%timestamp"`.

use serde::Deserialize;

/// A library named in a manifest and the version it asks for, if it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Vcpkg {
    pub dependencies: Vec<Library>,
    pub overrides: Vec<Library>,
    pub baseline: String,
}

#[derive(Deserialize)]
struct VcpkgFile {
    #[serde(default)]
    dependencies: Vec<VcpkgDependency>,
    #[serde(default)]
    overrides: Vec<VcpkgOverride>,
    #[serde(default, rename = "builtin-baseline")]
    baseline: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum VcpkgDependency {
    Name(String),
    Object {
        name: String,
        #[serde(default, rename = "version>=")]
        minimum: String,
    },
}

#[derive(Deserialize)]
struct VcpkgOverride {
    name: String,
    #[serde(default)]
    version: String,
}

/// Read a `vcpkg.json`. A file that is not JSON gives nothing.
pub fn parse_vcpkg(text: &str) -> Vcpkg {
    let Ok(file) = serde_json::from_str::<VcpkgFile>(text) else {
        return Vcpkg::default();
    };
    Vcpkg {
        dependencies: file
            .dependencies
            .into_iter()
            .map(|d| match d {
                VcpkgDependency::Name(name) => Library {
                    name,
                    version: String::new(),
                },
                VcpkgDependency::Object { name, minimum } => Library {
                    name,
                    version: minimum,
                },
            })
            .collect(),
        overrides: file
            .overrides
            .into_iter()
            .map(|o| Library {
                name: o.name,
                version: o.version,
            })
            .collect(),
        baseline: file.baseline,
    }
}

/// `fmt/10.1.1@user/channel#abc` as a library.
fn reference(text: &str) -> Option<Library> {
    let text = text.trim().trim_matches(['"', '\'']);
    let (name, rest) = text.split_once('/')?;
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    let version: String = rest
        .chars()
        .take_while(|c| !matches!(c, '@' | '#' | '%' | '[' | ' ' | '"' | '\''))
        .collect();
    (!version.is_empty()).then(|| Library {
        name: name.to_string(),
        version,
    })
}

/// The `[requires]` of a `conanfile.txt`.
pub fn parse_conanfile_txt(text: &str) -> Vec<Library> {
    let mut section = "";
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        // A revision follows `#` in a reference; a line that starts with
        // one is a comment, and what is before it is read first.
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = match name {
                "requires" => "requires",
                _ => "",
            };
            continue;
        }
        if section == "requires"
            && let Some(library) = reference(line)
        {
            out.push(library);
        }
    }
    out
}

/// The `requires` of a `conanfile.py`: the class attribute and the calls
/// in `requirements()`. Only plain string literals are read.
pub fn parse_conanfile_py(text: &str) -> Vec<Library> {
    let mut out: Vec<Library> = Vec::new();
    let mut in_requires = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let mut pieces: Vec<&str> = Vec::new();
        if let Some(rest) = trimmed
            .strip_prefix("requires")
            .map(str::trim_start)
            .and_then(|r| r.strip_prefix('='))
        {
            in_requires = true;
            pieces.push(rest);
        } else if in_requires {
            pieces.push(trimmed);
        }
        if let Some(at) = trimmed.find("self.requires(") {
            pieces.push(&trimmed[at + "self.requires(".len()..]);
        }
        for piece in pieces {
            for literal in string_literals(piece) {
                if let Some(library) = reference(&literal)
                    && !out.contains(&library)
                {
                    out.push(library);
                }
            }
        }
        // A tuple or list that is not closed on this line goes on.
        if in_requires
            && !trimmed.ends_with(',')
            && !trimmed.ends_with('(')
            && !trimmed.ends_with('[')
        {
            let opens = trimmed.matches(['(', '[']).count();
            let closes = trimmed.matches([')', ']']).count();
            if opens <= closes && !trimmed.starts_with("requires") {
                in_requires = trimmed.ends_with(',');
            } else if trimmed.starts_with("requires") && opens <= closes {
                in_requires = false;
            }
        }
    }
    out
}

fn string_literals(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '"' || c == '\'' {
            let mut value = String::new();
            for d in chars.by_ref() {
                if d == c {
                    break;
                }
                value.push(d);
            }
            out.push(value);
        } else if c == '#' {
            break;
        }
    }
    out
}

/// The libraries a `conan.lock` pins.
pub fn parse_conan_lock(text: &str) -> Vec<Library> {
    #[derive(Deserialize)]
    struct Lock {
        #[serde(default)]
        requires: Vec<String>,
        #[serde(default)]
        build_requires: Vec<String>,
    }
    let Ok(lock) = serde_json::from_str::<Lock>(text) else {
        return Vec::new();
    };
    lock.requires
        .iter()
        .chain(&lock.build_requires)
        .filter_map(|r| reference(r))
        .collect()
}

/// True when an include's first path segment (or a header's own name) is
/// evidently the library `library`: the names agree once case and
/// punctuation are put aside, or a library is one of a family
/// (`boost-asio` of `boost`, `nlohmann_json` of `nlohmann`), or a digit
/// differs (`Eigen3` of `Eigen`).
pub fn names_library(include: &str, library: &str) -> bool {
    let norm = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    };
    let include = norm(include);
    if include.len() < 2 {
        return false;
    }
    let whole = norm(library);
    if whole == include || whole.trim_end_matches(|c: char| c.is_ascii_digit()) == include {
        return true;
    }
    library
        .split(['-', '_', '.', ':', '+'])
        .next()
        .is_some_and(|first| norm(first) == include)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(name: &str, version: &str) -> Library {
        Library {
            name: name.into(),
            version: version.into(),
        }
    }

    #[test]
    fn reads_a_vcpkg_manifest() {
        let manifest = parse_vcpkg(
            r#"{
              "name": "demo",
              "builtin-baseline": "0123abc",
              "dependencies": [
                "fmt",
                { "name": "boost-asio", "version>=": "1.80.0" },
                { "name": "zlib", "platform": "windows" }
              ],
              "overrides": [ { "name": "fmt", "version": "10.1.1" } ]
            }"#,
        );
        assert_eq!(
            manifest.dependencies,
            [lib("fmt", ""), lib("boost-asio", "1.80.0"), lib("zlib", "")]
        );
        assert_eq!(manifest.overrides, [lib("fmt", "10.1.1")]);
        assert_eq!(manifest.baseline, "0123abc");
        assert_eq!(parse_vcpkg("not json"), Vcpkg::default());
    }

    #[test]
    fn reads_conan_files() {
        let txt = parse_conanfile_txt(
            "[requires]\nfmt/10.1.1\nzlib/1.3.1@user/stable  # compression\nboost/1.83.0#abcdef\n\n[tool_requires]\ncmake/3.27\n\n[generators]\nCMakeDeps\n",
        );
        assert_eq!(
            txt,
            [
                lib("fmt", "10.1.1"),
                lib("zlib", "1.3.1"),
                lib("boost", "1.83.0")
            ]
        );
        let py = parse_conanfile_py(
            "from conan import ConanFile\nclass P(ConanFile):\n    requires = \"fmt/10.1.1\"\n    # requires = \"old/1.0\"\n    def requirements(self):\n        self.requires(\"spdlog/1.12.0\")\n        self.requires('zlib/[>=1.2 <2]')\n",
        );
        assert_eq!(py, [lib("fmt", "10.1.1"), lib("spdlog", "1.12.0")]);
        let tuple = parse_conanfile_py(
            "class P:\n    requires = (\n        \"a/1.0\",\n        \"b/2.0@x/y\",\n    )\n    name = \"c/3.0\"\n",
        );
        assert_eq!(tuple, [lib("a", "1.0"), lib("b", "2.0")]);
        let lock = parse_conan_lock(
            r#"{"version":"0.5","requires":["fmt/10.1.1#aa%1700000000.1","zlib/1.3.1#bb%1"],"build_requires":["cmake/3.27.1#cc%2"]}"#,
        );
        assert_eq!(
            lock,
            [
                lib("fmt", "10.1.1"),
                lib("zlib", "1.3.1"),
                lib("cmake", "3.27.1")
            ]
        );
    }

    #[test]
    fn matches_an_include_to_a_declared_library_only_when_evident() {
        for (include, library) in [
            ("fmt", "fmt"),
            ("boost", "Boost"),
            ("boost", "boost-asio"),
            ("gtest", "GTest"),
            ("nlohmann", "nlohmann_json"),
            ("Eigen", "Eigen3"),
            ("zlib", "ZLIB"),
            ("curl", "CURL"),
            ("catch2", "Catch2"),
        ] {
            assert!(names_library(include, library), "{include} {library}");
        }
        for (include, library) in [
            ("gmock", "GTest"),
            ("thread", "Threads"),
            ("a", "a"),
            ("sys", "system"),
            ("lib", "libpng"),
        ] {
            assert!(!names_library(include, library), "{include} {library}");
        }
    }
}
