//! What the scan decides about C and C++ before and after it parses a file:
//! which language a `.h` is, which header and source form one module, what an
//! `#include` names, and how the parts of a declaration become one symbol.
//! The walker itself is `c.rs`; this is the part that needs the other files.
//!
//! # The rules
//!
//! - **Language.** `.c` is C. `.cc`, `.cpp`, `.cxx`, `.c++`, `.hpp`, `.hh`,
//!   `.hxx`, `.ipp`, `.tpp` and `.inl` are C++. A `.h` is C++ when it uses
//!   what only C++ has (`class`, `namespace`, `template`, an `#include` of a
//!   header with no extension...), outside the `#ifdef __cplusplus` blocks C
//!   headers keep for C++ callers; otherwise when the source of the same name
//!   is C++, or its folder holds C++ sources and no C; otherwise C.
//! - **Module.** A header and the source of its name are one module
//!   (`net/socket.h` and `net/socket.c`), named from the header's path
//!   without its extension and without a leading `src`, `lib` or `include`.
//!   A header and a source of one name in different folders (`include/x/a.h`
//!   and `src/a.cpp`) are one module when each is the only one of its kind
//!   with that name. Every other file is a module of its own.
//! - **Declaration and definition** of one name in a module are one symbol,
//!   whose file is the header and which lists the sources that define it
//!   (`impl:<file>`). `void Foo::bar() {}` joins the class `Foo`, wherever
//!   the class is declared. A function a header declares and another module
//!   defines is merged when the defining module includes that header.
//! - **Vendored trees** (`third_party`, `vendor`, `external`, `extern`,
//!   `deps`, `_deps`, `3rdparty`) are not parsed when the repository has
//!   C or C++ outside them: they would dwarf the project in every ranking
//!   and in the time of a scan. An include that reaches into one names the
//!   library by the folder it sits in.

use std::collections::{BTreeMap, HashMap, HashSet};

use cw_core::{Language, Symbol, SymbolKind, Visibility};

use crate::c::{DECLARATION_ATTR, IMPL_ATTR, Include, OUTLINE_ATTR};
use crate::cdeps::{
    names_library, parse_conan_lock, parse_conanfile_py, parse_conanfile_txt, parse_vcpkg,
};
use crate::cmake::{Cmake, read_cmake};
use crate::lang::sanitize;
use crate::rust::ParsedFile;

const HEADER_EXTENSIONS: [&str; 9] = ["h", "hpp", "hh", "hxx", "h++", "hp", "inl", "ipp", "tpp"];
const SOURCE_EXTENSIONS: [&str; 5] = ["c", "cc", "cpp", "cxx", "c++"];

/// Folders that hold other people's code.
pub const VENDORED_DIRS: [&str; 9] = [
    "third_party",
    "thirdparty",
    "third-party",
    "3rdparty",
    "vendor",
    "external",
    "extern",
    "deps",
    "_deps",
];

/// Folders whose name is no part of a module's name.
const CONTAINER_DIRS: [&str; 6] = ["src", "lib", "include", "inc", "source", "sources"];

/// C library headers and the POSIX and Windows ones that code names most:
/// the platform, never a library the repository could name.
const PLATFORM_HEADERS: &[&str] = &[
    "assert.h",
    "complex.h",
    "ctype.h",
    "errno.h",
    "fenv.h",
    "float.h",
    "inttypes.h",
    "iso646.h",
    "limits.h",
    "locale.h",
    "math.h",
    "setjmp.h",
    "signal.h",
    "stdalign.h",
    "stdarg.h",
    "stdatomic.h",
    "stdbool.h",
    "stddef.h",
    "stdint.h",
    "stdio.h",
    "stdlib.h",
    "stdnoreturn.h",
    "string.h",
    "tgmath.h",
    "threads.h",
    "time.h",
    "uchar.h",
    "wchar.h",
    "wctype.h",
    "unistd.h",
    "fcntl.h",
    "pthread.h",
    "dirent.h",
    "dlfcn.h",
    "poll.h",
    "regex.h",
    "sched.h",
    "semaphore.h",
    "termios.h",
    "netdb.h",
    "syslog.h",
    "strings.h",
    "getopt.h",
    "windows.h",
    "winsock2.h",
    "ws2tcpip.h",
    "windef.h",
    "winbase.h",
    "tchar.h",
    "process.h",
    "io.h",
    "direct.h",
    "malloc.h",
    "memory.h",
    "conio.h",
    "shlobj.h",
    "wincrypt.h",
    "objbase.h",
    "ole2.h",
    "shellapi.h",
    "psapi.h",
];

/// Folders of the platform's headers (`sys/types.h`, `netinet/in.h`).
const PLATFORM_DIRS: &[&str] = &[
    "sys",
    "bits",
    "linux",
    "netinet",
    "arpa",
    "asm",
    "net",
    "mach",
    "os",
    "cuda",
    "x86_64-linux-gnu",
    "machine",
];

pub fn is_header(path: &str) -> bool {
    extension(path).is_some_and(|e| HEADER_EXTENSIONS.contains(&e.as_str()))
}

pub fn is_source(path: &str) -> bool {
    extension(path).is_some_and(|e| SOURCE_EXTENSIONS.contains(&e.as_str()))
}

fn extension(path: &str) -> Option<String> {
    let file = path.rsplit('/').next()?;
    file.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

fn stem_of(path: &str) -> &str {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(s, _)| s)
}

fn is_vendored(path: &str) -> bool {
    let mut parts = path.split('/').collect::<Vec<_>>();
    parts.pop();
    parts
        .iter()
        .any(|d| VENDORED_DIRS.contains(&d.to_ascii_lowercase().as_str()))
}

/// The folder of a vendored file that names its library: the one under the
/// vendor folder (`third_party/fmt/include/fmt/core.h` is `fmt`).
fn vendored_library(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    let at = parts
        .iter()
        .position(|d| VENDORED_DIRS.contains(&d.to_ascii_lowercase().as_str()))?;
    parts
        .get(at + 1)
        .filter(|_| at + 2 < parts.len())
        .map(|s| s.to_string())
}

/// Whether a header's text uses what only C++ has. The blocks C headers
/// keep for C++ callers (`#ifdef __cplusplus`) are left out.
pub fn looks_cpp(text: &str) -> bool {
    let mut guards: Vec<(bool, bool)> = Vec::new();
    let mut in_cpp = false;
    let mut in_comment = false;
    for line in text.lines() {
        let mut line = line.trim();
        if in_comment {
            match line.find("*/") {
                Some(end) => {
                    line = line[end + 2..].trim();
                    in_comment = false;
                }
                None => continue,
            }
        }
        // Comments and the strings in the line are no code.
        let code = strip_comments(line, &mut in_comment);
        let code = code.trim();
        if let Some(rest) = code.strip_prefix('#') {
            let directive = rest.trim_start();
            let word = directive
                .split(|c: char| !c.is_alphanumeric())
                .next()
                .unwrap_or("");
            match word {
                "if" | "ifdef" | "ifndef" => {
                    let mentions = directive.contains("__cplusplus");
                    let negated = word == "ifndef" || directive.contains("!defined");
                    // (is a test of __cplusplus, the branch now read is C++)
                    guards.push((mentions, mentions && !negated));
                    in_cpp = guards.iter().any(|g| g.1);
                    continue;
                }
                "else" | "elif" => {
                    if let Some(last) = guards.last_mut()
                        && last.0
                    {
                        last.1 = word == "else" && !last.1;
                    }
                    in_cpp = guards.iter().any(|g| g.1);
                    continue;
                }
                "endif" => {
                    guards.pop();
                    in_cpp = guards.iter().any(|g| g.1);
                    continue;
                }
                "include" => {
                    // `#include <vector>`: a standard header has no extension.
                    if let Some(name) = directive
                        .split_once('<')
                        .and_then(|(_, r)| r.split_once('>'))
                        .map(|(n, _)| n.trim())
                        && !name.is_empty()
                        && !name.contains(['.', '/'])
                        && !in_cpp
                    {
                        return true;
                    }
                    continue;
                }
                _ => continue,
            }
        }
        if in_cpp {
            continue;
        }
        let words: Vec<&str> = code
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|w| !w.is_empty())
            .collect();
        if words.iter().any(|w| {
            matches!(
                *w,
                "class"
                    | "namespace"
                    | "template"
                    | "typename"
                    | "constexpr"
                    | "nullptr"
                    | "noexcept"
                    | "virtual"
                    | "override"
                    | "static_cast"
                    | "reinterpret_cast"
                    | "dynamic_cast"
            )
        }) {
            return true;
        }
        if code.contains("std::")
            || code.starts_with("public:")
            || code.starts_with("private:")
            || code.starts_with("protected:")
        {
            return true;
        }
    }
    false
}

/// `code` of a line without its comments and string contents.
fn strip_comments(line: &str, in_comment: &mut bool) -> String {
    let mut out = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => break,
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut closed = false;
                while let Some(d) = chars.next() {
                    if d == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    *in_comment = true;
                }
            }
            '"' => {
                out.push(' ');
                while let Some(d) = chars.next() {
                    if d == '\\' {
                        chars.next();
                    } else if d == '"' {
                        break;
                    }
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// What the scan knows about the C and C++ files of one repository.
#[derive(Debug, Default)]
pub struct CPlan {
    /// Language of every C or C++ file, `.h` decided.
    pub language: HashMap<String, Language>,
    /// Module of every file that is parsed.
    pub module: HashMap<String, String>,
    /// Files in a vendored tree that are not parsed.
    pub vendored: HashSet<String>,
    all: HashSet<String>,
    /// Header file name -> paths.
    by_name: HashMap<String, Vec<String>>,
    cmake: Cmake,
    libraries: Vec<String>,
}

impl CPlan {
    /// `None` when the repository has no C or C++ file.
    pub fn build(paths: &[String], read: &dyn Fn(&str) -> Option<String>) -> Option<Self> {
        let family: Vec<&String> = paths
            .iter()
            .filter(|p| is_header(p) || is_source(p))
            .collect();
        if family.is_empty() {
            return None;
        }
        let mut plan = Self::default();
        let has_own = family.iter().any(|p| !is_vendored(p));
        for path in &family {
            plan.all.insert((*path).clone());
            if has_own && is_vendored(path) {
                plan.vendored.insert((*path).clone());
            }
        }
        // The language of each file.
        let own: Vec<&&String> = family
            .iter()
            .filter(|p| !plan.vendored.contains(**p))
            .collect();
        let mut sources_in: HashMap<&str, (usize, usize)> = HashMap::new();
        let mut stems: HashMap<(&str, &str), (bool, bool)> = HashMap::new();
        for path in &own {
            if is_source(path) {
                let cpp = Language::from_path(path) == Language::Cpp;
                let counts = sources_in.entry(dir_of(path)).or_default();
                if cpp {
                    counts.1 += 1;
                } else {
                    counts.0 += 1;
                }
                let entry = stems.entry((dir_of(path), stem_of(path))).or_default();
                if cpp {
                    entry.1 = true;
                } else {
                    entry.0 = true;
                }
            }
        }
        for path in &family {
            let path: &str = path.as_str();
            let language = match Language::from_path(path) {
                Language::C if is_header(path) => {
                    let text = read(path).unwrap_or_default();
                    if looks_cpp(&text) {
                        Language::Cpp
                    } else {
                        match stems.get(&(dir_of(path), stem_of(path))) {
                            Some((true, false)) => Language::C,
                            Some((_, true)) => Language::Cpp,
                            _ => match sources_in.get(dir_of(path)) {
                                Some((0, cpp)) if *cpp > 0 => Language::Cpp,
                                _ => Language::C,
                            },
                        }
                    }
                }
                other => other,
            };
            plan.language.insert(path.to_string(), language);
        }
        // Headers by file name, for includes that no folder rule finds.
        for path in &family {
            if is_header(path) {
                let name = path.rsplit('/').next().unwrap_or(path);
                plan.by_name
                    .entry(name.to_string())
                    .or_default()
                    .push((*path).clone());
            }
        }
        plan.module = module_names(&own);
        plan.cmake = read_cmake(paths, read);
        plan.libraries = declared_libraries(paths, read, &plan.cmake);
        Some(plan)
    }

    /// Directories an include of `file` may be written relative to, besides
    /// its own folder.
    pub fn include_dirs(&self, file: &str) -> HashSet<String> {
        let mut dirs: HashSet<String> = self.cmake.include_dirs(file).into_iter().collect();
        let mut here = dir_of(file);
        loop {
            for root in ["include", "inc", "src", "lib", "source"] {
                dirs.insert(if here.is_empty() {
                    root.to_string()
                } else {
                    format!("{here}/{root}")
                });
            }
            if here.is_empty() {
                break;
            }
            here = dir_of(here);
        }
        dirs.insert(String::new());
        dirs
    }

    pub fn cmake(&self) -> &Cmake {
        &self.cmake
    }

    /// What `#include` `spec` means in `from`, given the directories
    /// [`Self::include_dirs`] named for it.
    pub fn include(&self, from: &str, dirs: &HashSet<String>, spec: &str, quoted: bool) -> Include {
        if let Some(target) = self.find(from, dirs, spec, quoted) {
            if self.vendored.contains(&target) {
                return match vendored_library(&target) {
                    Some(name) => Include::Library {
                        package: name,
                        name: spec.to_string(),
                    },
                    None => Include::Unknown,
                };
            }
            return match self.module.get(&target) {
                Some(module) => Include::Repo(module.split("::").map(str::to_string).collect()),
                None => Include::Unknown,
            };
        }
        self.library(spec)
    }

    fn find(&self, from: &str, dirs: &HashSet<String>, spec: &str, quoted: bool) -> Option<String> {
        // Most includes are the platform's: no header of that name, no file.
        let name = spec.rsplit('/').next()?;
        let named = self.by_name.get(name)?;
        let relative = |base: &str| -> Option<String> {
            let joined = if base.is_empty() {
                spec.to_string()
            } else {
                format!("{base}/{spec}")
            };
            let path = normalize(&joined)?;
            self.all.contains(&path).then_some(path)
        };
        if quoted && let Some(found) = relative(dir_of(from)) {
            return Some(found);
        }
        // A header at the end of an include directory: the path is the
        // directory and the spec.
        let spec = normalize(spec)?;
        let suffix = format!("/{spec}");
        let mut hits: Vec<(&str, &String)> = named
            .iter()
            .filter_map(|p| {
                let base = if *p == spec {
                    ""
                } else {
                    p.strip_suffix(&suffix)?
                };
                dirs.contains(base).then_some((base, p))
            })
            .collect();
        hits.sort();
        if let Some((_, found)) = hits.first() {
            return Some((*found).clone());
        }
        // A name that only one file in the repository can mean.
        let found: Vec<&String> = named
            .iter()
            .filter(|p| **p == spec || p.ends_with(&suffix))
            .collect();
        (found.len() == 1).then(|| found[0].clone())
    }

    /// A library the repository declares and `spec` is evidently from.
    fn library(&self, spec: &str) -> Include {
        let platform = PLATFORM_HEADERS.contains(&spec)
            || (!spec.contains(['.', '/']))
            || spec
                .split_once('/')
                .is_some_and(|(first, _)| PLATFORM_DIRS.contains(&first));
        if platform {
            return Include::Unknown;
        }
        let (first, rest) = match spec.split_once('/') {
            Some((first, rest)) => (first, rest),
            None => (spec.rsplit_once('.').map_or(spec, |(s, _)| s), ""),
        };
        for library in &self.libraries {
            if names_library(first, library) {
                return Include::Library {
                    package: library.clone(),
                    name: rest.to_string(),
                };
            }
        }
        Include::Unknown
    }
}

/// The module of every C and C++ file of a repository, by path; a file in a
/// vendored tree has none. What the scan and the units both go by.
pub fn c_module_map(paths: &[String]) -> HashMap<String, String> {
    let family: Vec<&String> = paths
        .iter()
        .filter(|p| is_header(p) || is_source(p))
        .collect();
    let has_own = family.iter().any(|p| !is_vendored(p));
    let own: Vec<&&String> = family
        .iter()
        .filter(|p| !(has_own && is_vendored(p)))
        .collect();
    module_names(&own)
}

fn module_names(own: &[&&String]) -> HashMap<String, String> {
    let mut module: HashMap<String, String> = HashMap::new();
    // Files of one folder and name are one group.
    let mut groups: BTreeMap<(String, String), Vec<&str>> = BTreeMap::new();
    for path in own {
        groups
            .entry((dir_of(path).to_string(), stem_of(path).to_string()))
            .or_default()
            .push(path.as_str());
    }
    // A header alone and a source alone of one name in two folders pair.
    let mut by_stem: BTreeMap<&str, Vec<(&String, &String)>> = BTreeMap::new();
    for key in groups.keys() {
        by_stem
            .entry(key.1.as_str())
            .or_default()
            .push((&key.0, &key.1));
    }
    let mut merged: Vec<((String, String), (String, String))> = Vec::new();
    for (_, keys) in by_stem {
        let only = |header: bool| -> Vec<&(&String, &String)> {
            keys.iter()
                .filter(|(d, s)| {
                    let files = &groups[&((*d).clone(), (*s).clone())];
                    files.iter().any(|f| is_header(f)) == header
                        && files.iter().any(|f| is_source(f)) != header
                })
                .collect()
        };
        let headers = only(true);
        let sources = only(false);
        if headers.len() == 1 && sources.len() == 1 {
            merged.push((
                ((*headers[0].0).clone(), (*headers[0].1).clone()),
                ((*sources[0].0).clone(), (*sources[0].1).clone()),
            ));
        }
    }
    for (header, source) in merged {
        if let Some(files) = groups.remove(&source) {
            groups.entry(header).or_default().extend(files);
        }
    }
    // Names: from the header when there is one, else the first file.
    let mut named: Vec<(Vec<&str>, String, String)> = Vec::new();
    for files in groups.values() {
        let mut sorted = files.clone();
        sorted.sort_by_key(|f| (!is_header(f), *f));
        let first = sorted[0];
        named.push((sorted, module_name(first, true), module_name(first, false)));
    }
    let mut count: HashMap<String, usize> = HashMap::new();
    for (_, short, _) in &named {
        *count.entry(short.clone()).or_default() += 1;
    }
    let mut taken: HashSet<String> = HashSet::new();
    for (files, short, long) in named {
        let mut name = if count[&short] > 1 { long } else { short };
        let mut n = 2;
        let base = name.clone();
        while !taken.insert(name.clone()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        for file in files {
            module.insert(file.to_string(), name.clone());
        }
    }
    module
}

/// The libraries the repository declares, in the order it does.
fn declared_libraries(
    paths: &[String],
    read: &dyn Fn(&str) -> Option<String>,
    cmake: &Cmake,
) -> Vec<String> {
    let mut out: Vec<String> = cmake.packages.iter().map(|p| p.name.clone()).collect();
    for path in paths {
        let file = path.rsplit('/').next().unwrap_or(path);
        let Some(text) = (match file {
            "vcpkg.json" | "conanfile.txt" | "conanfile.py" | "conan.lock" => read(path),
            _ => None,
        }) else {
            continue;
        };
        match file {
            "vcpkg.json" => out.extend(parse_vcpkg(&text).dependencies.into_iter().map(|d| d.name)),
            "conanfile.txt" => out.extend(parse_conanfile_txt(&text).into_iter().map(|l| l.name)),
            "conanfile.py" => out.extend(parse_conanfile_py(&text).into_iter().map(|l| l.name)),
            _ => out.extend(parse_conan_lock(&text).into_iter().map(|l| l.name)),
        }
    }
    let mut seen = HashSet::new();
    out.retain(|n| seen.insert(n.to_lowercase()));
    out
}

fn normalize(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// The module a file's name makes: its path without the extension and, when
/// `strip`, without the folders that only say where sources and headers go
/// (`src`, `lib`, `include`).
fn module_name(path: &str, strip: bool) -> String {
    let stem = path.rsplit_once('.').map_or(path, |(s, _)| s);
    let mut parts: Vec<String> = stem.split('/').map(sanitize).collect();
    if strip {
        // `proj/include/proj/x` is `proj::proj::x`, not `proj::include::proj::x`.
        let last = parts.len() - 1;
        let mut at = 0;
        parts.retain(|p| {
            at += 1;
            at - 1 == last || !CONTAINER_DIRS.contains(&p.as_str())
        });
    }
    parts.join("::")
}

/// True for a file of a header's kind, where a name is declared for others.
pub fn declares_for_others(file: &str) -> bool {
    is_header(file)
}

fn home_of<'a>(symbol: &Symbol, plan: &'a CPlan) -> Option<&'a str> {
    plan.module.get(&symbol.file).map(String::as_str)
}

/// Apply what only all the files together tell: the class an out-of-line
/// definition belongs to, one symbol for a declaration and its definition,
/// and the modules. Returns the symbols of every C and C++ file; the
/// facts that mention a symbol that was renamed are rewritten in `files`.
pub fn merge(files: &mut [(String, ParsedFile)], plan: &CPlan) -> Vec<Symbol> {
    let is_c = |path: &str| plan.language.contains_key(path);
    let mut all: Vec<Symbol> = files
        .iter()
        .filter(|(path, _)| is_c(path))
        .flat_map(|(_, parsed)| parsed.symbols.iter().cloned())
        .collect();
    // Types by module and name, and by name alone.
    let mut by_module: HashMap<(String, String), Vec<String>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    for symbol in &all {
        if symbol.kind.is_type()
            && !symbol.attrs.iter().any(|a| a.starts_with(OUTLINE_ATTR))
            && let Some(home) = home_of(symbol, plan)
        {
            by_module
                .entry((home.to_string(), symbol.name.clone()))
                .or_default()
                .push(symbol.qualified.clone());
            by_name
                .entry(symbol.name.clone())
                .or_default()
                .push(symbol.qualified.clone());
        }
    }
    for list in by_module.values_mut().chain(by_name.values_mut()) {
        list.sort();
        list.dedup();
    }
    // An out-of-line definition names its class.
    let mut renames: HashMap<String, String> = HashMap::new();
    for symbol in &mut all {
        let Some(at) = symbol
            .attrs
            .iter()
            .position(|a| a.starts_with(OUTLINE_ATTR))
        else {
            continue;
        };
        let scope = symbol.attrs.remove(at)[OUTLINE_ATTR.len()..].to_string();
        let Some(home) = home_of(symbol, plan).map(str::to_string) else {
            continue;
        };
        let class_name = scope.rsplit("::").next().unwrap_or(&scope).to_string();
        let wanted = format!("::{scope}");
        let pick = |list: &Vec<String>| -> Option<String> {
            list.iter()
                .find(|q| q.ends_with(&wanted))
                .or_else(|| (list.len() == 1).then(|| &list[0]))
                .cloned()
        };
        let class = by_module
            .get(&(home.clone(), class_name.clone()))
            .and_then(pick)
            .or_else(|| by_name.get(&class_name).and_then(pick));
        let old = symbol.qualified.clone();
        match class {
            Some(class) => {
                symbol.qualified = format!("{class}::{}", symbol.name);
                symbol.parent = Some(class);
            }
            None => {
                // `ns::f` and not `Class::f`: a function of the module.
                if symbol.kind == SymbolKind::Method {
                    symbol.kind = SymbolKind::Function;
                }
                symbol.qualified = format!("{home}::{}", symbol.name);
                symbol.parent = Some(home);
            }
        }
        if old != symbol.qualified {
            renames.insert(old, symbol.qualified.clone());
        }
    }
    // One symbol for every part: headers first, so that the symbol stands
    // where the declaration is.
    let mut order: Vec<usize> = (0..all.len()).collect();
    order.sort_by_key(|&i| (!declares_for_others(&all[i].file), all[i].file.as_str(), i));
    let mut at: HashMap<String, usize> = HashMap::new();
    let mut merged: Vec<Symbol> = Vec::new();
    for i in order {
        let part = all[i].clone();
        match at.get(&part.qualified) {
            None => {
                at.insert(part.qualified.clone(), merged.len());
                merged.push(part);
            }
            Some(&first) => join_parts(&mut merged[first], &part),
        }
    }
    // A definition in a module of its own, of a function another module's
    // header declares, is that function.
    let includes = includes_of(files, plan);
    let declared: HashMap<(String, SymbolKind), Vec<usize>> = {
        let mut map: HashMap<(String, SymbolKind), Vec<usize>> = HashMap::new();
        for (i, s) in merged.iter().enumerate() {
            if s.attrs.iter().any(|a| a == DECLARATION_ATTR)
                && matches!(s.kind, SymbolKind::Function)
                && declares_for_others(&s.file)
            {
                map.entry((s.name.clone(), s.kind)).or_default().push(i);
            }
        }
        map
    };
    let mut gone: HashSet<usize> = HashSet::new();
    for i in 0..merged.len() {
        let symbol = &merged[i];
        if symbol.kind != SymbolKind::Function
            || symbol.visibility == Visibility::Private
            || symbol.attrs.iter().any(|a| a == DECLARATION_ATTR)
            || declares_for_others(&symbol.file)
        {
            continue;
        }
        let Some(candidates) = declared.get(&(symbol.name.clone(), symbol.kind)) else {
            continue;
        };
        let Some(home) = home_of(symbol, plan) else {
            continue;
        };
        let seen = reach(&includes, home);
        let matching: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&c| {
                let other = &merged[c];
                other.qualified != symbol.qualified
                    && home_of(other, plan).is_some_and(|m| seen.contains(m))
            })
            .collect();
        if let [target] = matching[..] {
            let part = merged[i].clone();
            renames.insert(part.qualified.clone(), merged[target].qualified.clone());
            join_parts(&mut merged[target], &part);
            gone.insert(i);
        }
    }
    let mut merged: Vec<Symbol> = merged
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !gone.contains(i))
        .map(|(_, s)| s)
        .collect();
    // A class whose methods are defined in a source lists that source too.
    let mut defined_in: HashMap<String, Vec<String>> = HashMap::new();
    for symbol in &merged {
        let Some(parent) = &symbol.parent else {
            continue;
        };
        for attr in &symbol.attrs {
            if attr.starts_with(IMPL_ATTR) {
                let list = defined_in.entry(parent.clone()).or_default();
                if !list.contains(attr) {
                    list.push(attr.clone());
                }
            }
        }
    }
    for symbol in &mut merged {
        if symbol.kind.is_type()
            && let Some(attrs) = defined_in.get(&symbol.qualified)
        {
            for attr in attrs {
                if attr[IMPL_ATTR.len()..] != symbol.file && !symbol.attrs.contains(attr) {
                    symbol.attrs.push(attr.clone());
                }
            }
        }
    }
    // What is left in a source and declared nowhere is internal; a macro in
    // a source is private to it.
    for symbol in &mut merged {
        let in_header = declares_for_others(&symbol.file);
        let is_entry = crate::c::ENTRY_NAMES.contains(&symbol.name.as_str());
        if !in_header && symbol.visibility == Visibility::Public {
            match symbol.kind {
                SymbolKind::Macro => symbol.visibility = Visibility::Private,
                SymbolKind::Const if symbol.attrs.iter().any(|a| a == "macro") => {
                    symbol.visibility = Visibility::Private;
                }
                SymbolKind::Function | SymbolKind::Static | SymbolKind::Const
                    if !is_entry && symbol.parent.as_deref() == home_of(symbol, plan) =>
                {
                    symbol.visibility = Visibility::Crate;
                }
                _ => {}
            }
        }
        symbol.attrs.retain(|a| a != DECLARATION_ATTR);
    }
    // The facts that name a renamed symbol.
    if !renames.is_empty() {
        for (path, parsed) in files.iter_mut() {
            if !is_c(path) {
                continue;
            }
            for call in &mut parsed.calls {
                if let Some(new) = renames.get(&call.caller) {
                    call.caller.clone_from(new);
                }
            }
            for r in &mut parsed.type_refs {
                if let Some(new) = renames.get(&r.from) {
                    r.from.clone_from(new);
                }
            }
            for v in &mut parsed.value_refs {
                if let Some(new) = renames.get(&v.from) {
                    v.from.clone_from(new);
                }
            }
        }
    }
    for (path, parsed) in files.iter_mut() {
        if is_c(path) {
            parsed.symbols.clear();
        }
    }
    merged
}

/// Fold `part`, another declaration or definition of the symbol `base`.
fn join_parts(base: &mut Symbol, part: &Symbol) {
    if base.kind == SymbolKind::TypeAlias && part.kind != SymbolKind::TypeAlias {
        // A typedef of an opaque struct and the struct that defines it.
        let doc = base.doc.clone();
        let file = base.file.clone();
        let header = declares_for_others(&file);
        let attrs = base.attrs.clone();
        *base = part.clone();
        if base.doc.is_empty() {
            base.doc = doc;
        }
        if header && !declares_for_others(&base.file) {
            base.attrs.extend(attrs);
        }
        return;
    }
    let part_defines = !part.attrs.iter().any(|a| a == DECLARATION_ATTR);
    let base_declares = base.attrs.iter().any(|a| a == DECLARATION_ATTR);
    if part_defines && part.file != base.file {
        let tag = format!("{IMPL_ATTR}{}", part.file);
        if !base.attrs.contains(&tag) {
            base.attrs.push(tag);
        }
    }
    if base.doc.is_empty() {
        base.doc.clone_from(&part.doc);
    }
    for attr in &part.attrs {
        if !base.attrs.contains(attr) && attr != DECLARATION_ATTR && !attr.starts_with(IMPL_ATTR) {
            base.attrs.push(attr.clone());
        }
    }
    for attr in &part.attrs {
        if attr.starts_with(IMPL_ATTR) && !base.attrs.contains(attr) {
            base.attrs.push(attr.clone());
        }
    }
    if base_declares && part_defines {
        base.attrs.retain(|a| a != DECLARATION_ATTR);
        // A definition outside a header leaves the declaration's place.
        if !declares_for_others(&base.file) {
            let attrs = base.attrs.clone();
            let doc = base.doc.clone();
            *base = part.clone();
            base.attrs = attrs;
            base.doc = if doc.is_empty() {
                part.doc.clone()
            } else {
                doc
            };
        }
    }
    // One module symbol, whose end covers every part.
    if base.kind == SymbolKind::Module && part.file == base.file {
        base.end_line = base.end_line.max(part.end_line);
    }
}

/// Module -> modules its files include directly.
fn includes_of(files: &[(String, ParsedFile)], plan: &CPlan) -> HashMap<String, HashSet<String>> {
    let mut out: HashMap<String, HashSet<String>> = HashMap::new();
    for (path, parsed) in files {
        if !plan.language.contains_key(path) {
            continue;
        }
        for u in &parsed.uses {
            if u.glob {
                out.entry(u.scope.clone())
                    .or_default()
                    .insert(u.path.join("::"));
            }
        }
    }
    out
}

/// The modules reachable from `from` through includes, three levels deep.
fn reach(includes: &HashMap<String, HashSet<String>>, from: &str) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(from.to_string());
    let mut frontier = vec![from.to_string()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for module in frontier {
            for included in includes.get(&module).into_iter().flatten() {
                if seen.insert(included.clone()) {
                    next.push(included.clone());
                }
            }
        }
        frontier = next;
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_cpp_headers_by_what_only_cpp_has() {
        for text in [
            "class Foo {};\n",
            "namespace a { int f(); }\n",
            "template <typename T> T id(T x);\n",
            "#include <vector>\nint x;\n",
            "struct S {\npublic:\n  int a;\n};\n",
            "auto f() noexcept -> int;\n",
            "std::string name();\n",
        ] {
            assert!(looks_cpp(text), "{text}");
        }
        for text in [
            "#ifndef X_H\n#define X_H\n#include <stdio.h>\n#include <sys/types.h>\nint f(int class_id);\n#endif\n",
            // The block C++ callers see is not C++ in a C header.
            "#ifdef __cplusplus\nextern \"C\" {\nnamespace n { class A; }\n#endif\nint f(void);\n#ifdef __cplusplus\n}\n#endif\n",
            "/* a class of its own, a namespace of names */\nint f(void); // template\n",
            "const char *s = \"std::vector class\";\n",
        ] {
            assert!(!looks_cpp(text), "{text}");
        }
        // Outside the guard it is C++ again.
        assert!(looks_cpp(
            "#ifdef __cplusplus\nnamespace a {}\n#endif\nclass B {};\n"
        ));
    }

    fn plan(files: &[(&str, &str)]) -> CPlan {
        let mut paths: Vec<String> = files.iter().map(|(p, _)| p.to_string()).collect();
        paths.sort();
        let texts: HashMap<String, String> = files
            .iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect();
        CPlan::build(&paths, &|p| texts.get(p).cloned()).unwrap()
    }

    #[test]
    fn decides_the_language_of_a_dot_h() {
        let p = plan(&[
            ("c/a.h", "int a(void);\n"),
            ("c/a.c", ""),
            ("c/lonely.h", "int l(void);\n"),
            ("cpp/b.h", "int b(void);\n"),
            ("cpp/b.cpp", ""),
            ("cpp/other.h", "int o(void);\n"),
            ("cpp/other2.cpp", ""),
            ("mixed/m.h", "int m(void);\n"),
            ("mixed/m.c", ""),
            ("mixed/m2.cpp", ""),
            ("mixed/own.h", "class X {};\n"),
            ("x/up.hpp", ""),
        ]);
        let language = |path: &str| p.language[path];
        assert_eq!(language("c/a.h"), Language::C);
        assert_eq!(language("c/lonely.h"), Language::C);
        // The source of its name is C++.
        assert_eq!(language("cpp/b.h"), Language::Cpp);
        // A folder with C++ sources and no C.
        assert_eq!(language("cpp/other.h"), Language::Cpp);
        // A folder of both: the source of its name decides, else C.
        assert_eq!(language("mixed/m.h"), Language::C);
        // Its own text decides first.
        assert_eq!(language("mixed/own.h"), Language::Cpp);
        assert_eq!(language("x/up.hpp"), Language::Cpp);
    }

    #[test]
    fn pairs_a_header_with_its_source_into_one_module() {
        let p = plan(&[
            ("src/net/socket.h", ""),
            ("src/net/socket.c", ""),
            ("src/net/other.c", ""),
            ("include/proj/util.h", ""),
            ("src/util.cpp", ""),
            ("src/dup.c", ""),
            ("tools/dup.c", ""),
            ("lib/a/x.h", ""),
            ("lib/b/x.h", ""),
        ]);
        let m = |path: &str| p.module[path].as_str();
        assert_eq!(m("src/net/socket.h"), "net::socket");
        assert_eq!(m("src/net/socket.c"), "net::socket");
        assert_eq!(m("src/net/other.c"), "net::other");
        // Each the only one of its name, in different folders.
        assert_eq!(m("include/proj/util.h"), "proj::util");
        assert_eq!(m("src/util.cpp"), "proj::util");
        // Two sources of one name with no header: modules of their own,
        // named in full.
        assert_ne!(m("src/dup.c"), m("tools/dup.c"));
        assert_eq!(m("lib/a/x.h"), "a::x");
        assert_eq!(m("lib/b/x.h"), "b::x");
    }

    #[test]
    fn resolves_includes_to_files_libraries_and_the_platform() {
        let p = plan(&[
            (
                "CMakeLists.txt",
                "find_package(fmt REQUIRED)\nfind_package(Boost 1.80)\nadd_library(core src/core.c)\ntarget_include_directories(core PUBLIC include)\n",
            ),
            ("vcpkg.json", "{\"dependencies\": [\"zlib\"]}"),
            ("include/core/api.h", ""),
            ("src/core.c", ""),
            ("src/local.h", ""),
            ("third_party/png/png.h", ""),
            ("third_party/png/png.c", ""),
        ]);
        let dirs = p.include_dirs("src/core.c");
        let inc = |spec: &str, quoted: bool| p.include("src/core.c", &dirs, spec, quoted);
        assert_eq!(inc("local.h", true), Include::Repo(vec!["local".into()]));
        assert_eq!(
            inc("core/api.h", false),
            Include::Repo(vec!["core".into(), "api".into()])
        );
        assert_eq!(
            inc("fmt/core.h", false),
            Include::Library {
                package: "fmt".into(),
                name: "core.h".into()
            }
        );
        assert_eq!(
            inc("boost/asio.hpp", false),
            Include::Library {
                package: "Boost".into(),
                name: "asio.hpp".into()
            }
        );
        assert_eq!(
            inc("zlib.h", false),
            Include::Library {
                package: "zlib".into(),
                name: String::new()
            }
        );
        // Into a vendored tree: the folder names the library.
        assert_eq!(
            inc("png/png.h", false),
            Include::Library {
                package: "png".into(),
                name: "png/png.h".into()
            }
        );
        // The platform and what nothing declares are not named.
        assert_eq!(inc("stdio.h", false), Include::Unknown);
        assert_eq!(inc("vector", false), Include::Unknown);
        assert_eq!(inc("sys/types.h", false), Include::Unknown);
        assert_eq!(inc("curl/curl.h", false), Include::Unknown);
        assert_eq!(inc("missing.h", true), Include::Unknown);
        assert!(p.vendored.contains("third_party/png/png.h"));
        assert!(!p.module.contains_key("third_party/png/png.h"));
    }

    #[test]
    fn a_repository_of_only_vendored_code_is_parsed() {
        let p = plan(&[("third_party/x/a.c", ""), ("third_party/x/a.h", "")]);
        assert!(p.vendored.is_empty());
        assert!(p.module.contains_key("third_party/x/a.c"));
    }
}
