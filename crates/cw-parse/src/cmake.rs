//! A reader of `CMakeLists.txt`, for what a repository's units need: the
//! targets and the files each lists, the include directories, the links
//! between targets, the libraries the project asks for and the tests it
//! registers with CTest.
//!
//! CMake is a language, and this reads the part of it that projects write
//! as data. It follows `set` and `list(APPEND)` of variables, `file(GLOB)`
//! with a literal pattern, `add_subdirectory`, and the branches of every
//! `if` (none is evaluated, as the preprocessor's are not). It does not run
//! functions or macros, does not read `include()`d files, and does not
//! evaluate generator expressions other than `$<BUILD_INTERFACE:...>`. An
//! argument it cannot resolve is left out: a source it cannot name is a
//! source that falls to its folder's unit.

use std::collections::{BTreeMap, HashMap, HashSet};

/// What kind of thing a target builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Executable,
    Library,
}

/// An `add_library` or `add_executable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    pub kind: TargetKind,
    /// Folder of the `CMakeLists.txt` that defines it, repo-relative.
    pub dir: String,
    /// The `CMakeLists.txt` that defines it.
    pub file: String,
    /// Repo-relative files it lists, that exist in the repository.
    pub sources: Vec<String>,
    /// Repo-relative include directories given for it.
    pub include_dirs: Vec<String>,
    /// Names given to `target_link_libraries`.
    pub links: Vec<String>,
}

/// Where a library the project asks for is declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageSource {
    FindPackage,
    FetchContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    /// As written (`1.2`, `v3.4.0`, a commit), empty when none was given.
    pub version: String,
    pub source: PackageSource,
    /// The `CMakeLists.txt` that asks for it.
    pub file: String,
}

/// A test registered with CTest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Test {
    /// For `add_test` its name; for a discovery command the target whose
    /// tests CTest learns of when the project is built.
    pub name: String,
    /// The target it runs, when known.
    pub target: Option<String>,
    /// `gtest`, `catch` or `doctest` when a discovery command registers the
    /// tests of `target` by their own names; `None` for a plain `add_test`.
    pub discovered: Option<&'static str>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cmake {
    pub targets: Vec<Target>,
    /// `include_directories` calls: the folder they apply to, and the
    /// directories.
    pub dir_includes: Vec<(String, Vec<String>)>,
    pub packages: Vec<Package>,
    pub tests: Vec<Test>,
    /// Every `CMakeLists.txt` that was read.
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Command {
    name: String,
    args: Vec<String>,
}

/// Split a CMake file into its commands, comments left out. A quoted
/// argument loses its quotes; `[[...]]` arguments are kept as written.
fn commands(text: &str) -> Vec<Command> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'#' {
            i = skip_comment(b, i);
            continue;
        }
        if !(c.is_ascii_alphabetic() || c == b'_') {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
            i += 1;
        }
        let name = text[start..i].to_ascii_lowercase();
        let mut j = i;
        while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
            j += 1;
        }
        if j >= b.len() || b[j] != b'(' {
            continue;
        }
        let (args, end) = arguments(text, j + 1);
        out.push(Command { name, args });
        i = end;
    }
    out
}

fn skip_comment(b: &[u8], mut i: usize) -> usize {
    // `#[[ ... ]]` and `#[=[ ... ]=]` run over lines.
    if b.get(i + 1) == Some(&b'[') {
        let mut k = i + 2;
        let mut level = 0;
        while b.get(k) == Some(&b'=') {
            level += 1;
            k += 1;
        }
        if b.get(k) == Some(&b'[') {
            let close = format!("]{}]", "=".repeat(level));
            let rest = &b[k + 1..];
            return match find(rest, close.as_bytes()) {
                Some(at) => k + 1 + at + close.len(),
                None => b.len(),
            };
        }
    }
    while i < b.len() && b[i] != b'\n' {
        i += 1;
    }
    i
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The arguments after an opening parenthesis, and where the command ends.
fn arguments(text: &str, mut i: usize) -> (Vec<String>, usize) {
    let b = text.as_bytes();
    let mut args = Vec::new();
    let mut depth = 1;
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'#' => i = skip_comment(b, i),
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return (args, i);
                }
            }
            b'"' => {
                let mut value = String::new();
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' && i + 1 < b.len() {
                        i += 1;
                    }
                    let ch = text[i..].chars().next().unwrap_or(' ');
                    value.push(ch);
                    i += ch.len_utf8();
                }
                i += 1;
                // Quoted arguments stay one item: mark them so that an
                // expansion does not split them at `;`.
                args.push(format!("\u{1}{value}"));
            }
            b'[' if matches!(b.get(i + 1), Some(b'[') | Some(b'=')) => {
                let mut k = i + 1;
                let mut level = 0;
                while b.get(k) == Some(&b'=') {
                    level += 1;
                    k += 1;
                }
                if b.get(k) == Some(&b'[') {
                    let close = format!("]{}]", "=".repeat(level));
                    let rest = &b[k + 1..];
                    let end = find(rest, close.as_bytes()).map_or(b.len(), |at| k + 1 + at);
                    args.push(format!("\u{1}{}", &text[k + 1..end.min(text.len())]));
                    i = (end + close.len()).min(b.len());
                } else {
                    i += 1;
                }
            }
            _ => {
                let start = i;
                // `$<...>` and `${...}` may hold spaces in neither case that
                // matters here; parentheses end a bare word.
                while i < b.len()
                    && !matches!(
                        b[i],
                        b' ' | b'\t' | b'\r' | b'\n' | b'(' | b')' | b'#' | b'"'
                    )
                {
                    i += 1;
                }
                if i == start {
                    i += 1;
                } else {
                    args.push(text[start..i].to_string());
                }
            }
        }
    }
    (args, i)
}

/// Marks a path in a variable as named from the repository root, which is
/// how CMake's own folder variables read.
const ROOT_MARK: char = '\u{5}';

fn here_var(dir: &str) -> String {
    format!("{ROOT_MARK}{dir}")
}

/// A path as written in a `CMakeLists.txt` in `dir`, repo-relative.
fn locate(dir: &str, item: &str) -> Option<String> {
    match item.strip_prefix(ROOT_MARK) {
        Some(rest) => normalize(rest.trim_start_matches('/')),
        None => normalize(&join(dir, item)),
    }
}

fn join(dir: &str, path: &str) -> String {
    if dir.is_empty() || dir == "." {
        path.to_string()
    } else {
        format!("{dir}/{path}")
    }
}

/// `a/./b/../c` as `a/c`; `None` when the path leaves the repository or is
/// absolute.
fn normalize(path: &str) -> Option<String> {
    if path.starts_with('/') || path.contains(':') && path.chars().nth(1) == Some(':') {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split(['/', '\\']) {
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

type Vars = HashMap<String, Vec<String>>;

struct Reader<'a> {
    files: &'a HashSet<&'a str>,
    paths: &'a [String],
    read: &'a dyn Fn(&str) -> Option<String>,
    out: Cmake,
    visited: HashSet<String>,
    /// Target name -> index in `out.targets`.
    at: BTreeMap<String, usize>,
    /// `add_library(ns::name ALIAS real)`.
    aliases: HashMap<String, String>,
}

/// Read every `CMakeLists.txt` of the repository. `paths` are all its files,
/// sorted; `read` gives the text of one.
pub fn read_cmake(paths: &[String], read: &dyn Fn(&str) -> Option<String>) -> Cmake {
    let files: HashSet<&str> = paths.iter().map(String::as_str).collect();
    let lists: Vec<&str> = paths
        .iter()
        .map(String::as_str)
        .filter(|p| *p == "CMakeLists.txt" || p.ends_with("/CMakeLists.txt"))
        .collect();
    if lists.is_empty() {
        return Cmake::default();
    }
    // The folders another file adds as a subdirectory are not roots.
    let mut added: HashSet<String> = HashSet::new();
    for list in &lists {
        let dir = list
            .strip_suffix("CMakeLists.txt")
            .unwrap()
            .trim_end_matches('/');
        let Some(text) = read(list) else {
            continue;
        };
        for command in commands(&text) {
            if command.name == "add_subdirectory"
                && let Some(first) = command.args.first()
                && !first.contains("${")
                && let Some(sub) = locate(dir, first.trim_start_matches('\u{1}'))
            {
                added.insert(sub);
            }
        }
    }
    let mut reader = Reader {
        files: &files,
        paths,
        read,
        out: Cmake::default(),
        visited: HashSet::new(),
        at: BTreeMap::new(),
        aliases: HashMap::new(),
    };
    for list in &lists {
        let dir = list
            .strip_suffix("CMakeLists.txt")
            .unwrap()
            .trim_end_matches('/');
        if !added.contains(dir) {
            let mut vars = Vars::new();
            reader.directory(dir, &mut vars, dir, 0);
        }
    }
    // A folder that only a cycle reaches is still read.
    for list in &lists {
        let dir = list
            .strip_suffix("CMakeLists.txt")
            .unwrap()
            .trim_end_matches('/');
        if !reader.visited.contains(dir) {
            let mut vars = Vars::new();
            reader.directory(dir, &mut vars, dir, 0);
        }
    }
    let mut out = reader.out;
    // A target linked under an alias name links the real one.
    for target in &mut out.targets {
        for link in &mut target.links {
            if let Some(real) = reader.aliases.get(link.as_str()) {
                *link = real.clone();
            }
        }
        target.links.sort();
        target.links.dedup();
        target.sources.sort();
        target.sources.dedup();
        target.include_dirs.sort();
        target.include_dirs.dedup();
    }
    out
}

impl Reader<'_> {
    fn directory(&mut self, dir: &str, vars: &mut Vars, root: &str, depth: usize) {
        if depth > 16 || !self.visited.insert(dir.to_string()) {
            return;
        }
        let file = join(dir, "CMakeLists.txt");
        let Some(text) = (self.read)(&file) else {
            return;
        };
        self.out.files.push(file.clone());
        let here = here_var(dir);
        for name in [
            "CMAKE_CURRENT_SOURCE_DIR",
            "CMAKE_CURRENT_LIST_DIR",
            "PROJECT_SOURCE_DIR",
        ] {
            vars.entry(name.to_string())
                .and_modify(|v| {
                    if name != "PROJECT_SOURCE_DIR" {
                        *v = vec![here.clone()];
                    }
                })
                .or_insert_with(|| vec![here.clone()]);
        }
        vars.entry("CMAKE_SOURCE_DIR".to_string())
            .or_insert_with(|| vec![here_var(root)]);
        vars.entry("CMAKE_CURRENT_SOURCE_DIR".to_string())
            .or_default()
            .clone_from(&vec![here.clone()]);
        let mut parent: Vec<(String, Vec<String>)> = Vec::new();
        let mut skipping = 0usize;
        for command in commands(&text) {
            // Functions and macros are definitions, not calls.
            match command.name.as_str() {
                "function" | "macro" => {
                    skipping += 1;
                    continue;
                }
                "endfunction" | "endmacro" => {
                    skipping = skipping.saturating_sub(1);
                    continue;
                }
                _ => {}
            }
            if skipping > 0 {
                continue;
            }
            self.command(&command, dir, vars, root, depth, &mut parent, &file);
        }
        // `set(X ... PARENT_SCOPE)` reaches the caller through the caller's
        // own bookkeeping: handed back in `vars` under a reserved key.
        for (name, value) in parent {
            vars.entry("\u{2}parent".to_string())
                .or_default()
                .push(format!("{name}\u{2}{}", value.join("\u{3}")));
        }
    }

    /// Expand `${NAME}` in one argument into the items it stands for.
    /// `None` when a variable is unknown or a generator expression cannot be
    /// read.
    fn expand(&self, arg: &str, vars: &Vars) -> Option<Vec<String>> {
        let (quoted, text) = match arg.strip_prefix('\u{1}') {
            Some(rest) => (true, rest),
            None => (false, arg),
        };
        let mut text = text.to_string();
        if let Some(inner) = text
            .strip_prefix("$<BUILD_INTERFACE:")
            .and_then(|r| r.strip_suffix('>'))
        {
            text = inner.to_string();
        }
        if text.contains("$<") {
            return None;
        }
        let mut guard = 0;
        while let Some(start) = text.find("${") {
            guard += 1;
            if guard > 16 {
                return None;
            }
            let end = text[start..].find('}')? + start;
            let name = &text[start + 2..end];
            if name.contains("${") || name.starts_with("ENV{") {
                return None;
            }
            let value = vars.get(name)?.join(";");
            text.replace_range(start..=end, &value);
        }
        if text.contains("$ENV{") {
            return None;
        }
        if quoted {
            return Some(vec![text]);
        }
        Some(
            text.split(';')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        )
    }

    fn expand_all(&self, args: &[String], vars: &Vars) -> Vec<String> {
        args.iter()
            .filter_map(|a| self.expand(a, vars))
            .flatten()
            .collect()
    }

    fn file_in(&self, dir: &str, item: &str) -> Option<String> {
        let path = locate(dir, item)?;
        self.files.contains(path.as_str()).then_some(path)
    }

    fn glob(&self, dir: &str, pattern: &str, recurse: bool) -> Vec<String> {
        let Some(full) = locate(dir, pattern) else {
            return Vec::new();
        };
        let (folder, name) = full.rsplit_once('/').unwrap_or(("", &full));
        self.paths
            .iter()
            .filter(|p| {
                let (pf, pn) = p.rsplit_once('/').unwrap_or(("", p));
                let in_folder = if recurse {
                    folder.is_empty() || pf == folder || pf.starts_with(&format!("{folder}/"))
                } else {
                    pf == folder
                };
                in_folder && wildcard(name, pn)
            })
            .cloned()
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn command(
        &mut self,
        command: &Command,
        dir: &str,
        vars: &mut Vars,
        root: &str,
        depth: usize,
        parent: &mut Vec<(String, Vec<String>)>,
        file: &str,
    ) {
        let args = &command.args;
        let first = args
            .first()
            .and_then(|a| self.expand(a, vars))
            .and_then(|v| v.into_iter().next());
        match command.name.as_str() {
            "project" => {
                if let Some(name) = first {
                    let here = here_var(dir);
                    vars.insert("PROJECT_NAME".into(), vec![name.clone()]);
                    vars.entry("CMAKE_PROJECT_NAME".into())
                        .or_insert_with(|| vec![name.clone()]);
                    vars.insert("PROJECT_SOURCE_DIR".into(), vec![here.clone()]);
                    vars.insert(format!("{name}_SOURCE_DIR"), vec![here.clone()]);
                }
            }
            "set" => {
                let Some(name) = first else { return };
                let rest = &args[1..];
                let stop = rest
                    .iter()
                    .position(|a| matches!(a.as_str(), "CACHE" | "PARENT_SCOPE" | "FORCE"))
                    .unwrap_or(rest.len());
                let value = self.expand_all(&rest[..stop], vars);
                if rest[stop..].iter().any(|a| a == "PARENT_SCOPE") {
                    parent.push((name, value));
                } else if !rest[stop..].iter().any(|a| a == "CACHE") || !vars.contains_key(&name) {
                    vars.insert(name, value);
                }
            }
            "unset" => {
                if let Some(name) = first {
                    vars.remove(&name);
                }
            }
            "list" => {
                let words: Vec<String> = args
                    .iter()
                    .map(|a| a.trim_start_matches('\u{1}').to_string())
                    .collect();
                let (Some(action), Some(name)) = (words.first(), words.get(1)) else {
                    return;
                };
                let items = self.expand_all(&args[2..], vars);
                match action.as_str() {
                    "APPEND" => vars.entry(name.clone()).or_default().extend(items),
                    "PREPEND" => {
                        let list = vars.entry(name.clone()).or_default();
                        let mut new = items;
                        new.append(list);
                        *list = new;
                    }
                    "REMOVE_ITEM" => {
                        if let Some(list) = vars.get_mut(name) {
                            list.retain(|v| !items.contains(v));
                        }
                    }
                    _ => {}
                }
            }
            "file" => {
                let words: Vec<&str> = args.iter().map(|a| a.trim_start_matches('\u{1}')).collect();
                let recurse = match words.first() {
                    Some(&"GLOB") => false,
                    Some(&"GLOB_RECURSE") => true,
                    _ => return,
                };
                let Some(name) = words.get(1) else { return };
                let mut relative: Option<String> = None;
                let mut found: Vec<String> = Vec::new();
                let mut i = 2;
                let mut patterns: Vec<String> = Vec::new();
                while i < args.len() {
                    match words[i] {
                        "RELATIVE" => {
                            relative = args
                                .get(i + 1)
                                .and_then(|a| self.expand(a, vars))
                                .and_then(|v| v.into_iter().next());
                            i += 1;
                        }
                        "LIST_DIRECTORIES" => i += 1,
                        "CONFIGURE_DEPENDS" | "FOLLOW_SYMLINKS" => {}
                        _ => patterns.extend(self.expand(&args[i], vars).unwrap_or_default()),
                    }
                    i += 1;
                }
                for pattern in patterns {
                    found.extend(self.glob(dir, &pattern, recurse));
                }
                found.sort();
                found.dedup();
                // With RELATIVE the items are named from that folder.
                if let Some(base) = relative.and_then(|r| locate(dir, &r)) {
                    found = found
                        .into_iter()
                        .filter_map(|f| {
                            if base.is_empty() {
                                Some(f)
                            } else {
                                f.strip_prefix(&format!("{base}/")).map(str::to_string)
                            }
                        })
                        .collect();
                    // Later uses name them from the current folder, which
                    // is the usual base; keep them as repo paths otherwise.
                    if base != dir {
                        return;
                    }
                } else {
                    // Absolute-looking items are repo paths already; make
                    // them usable from the current folder.
                    found = found.into_iter().map(|f| format!("\u{4}{f}")).collect();
                }
                vars.insert((*name).to_string(), found);
            }
            "add_library" | "add_executable" => {
                let words: Vec<String> = args
                    .iter()
                    .filter_map(|a| self.expand(a, vars))
                    .flatten()
                    .collect();
                let Some(name) = words.first().cloned() else {
                    return;
                };
                if words.iter().any(|w| w == "IMPORTED") {
                    return;
                }
                if words.get(1).is_some_and(|w| w == "ALIAS") {
                    if let Some(real) = words.get(2) {
                        self.aliases.insert(name, real.clone());
                    }
                    return;
                }
                let kind = if command.name == "add_executable" {
                    TargetKind::Executable
                } else {
                    TargetKind::Library
                };
                let mut sources = Vec::new();
                for item in &words[1..] {
                    if matches!(
                        item.as_str(),
                        "STATIC"
                            | "SHARED"
                            | "MODULE"
                            | "OBJECT"
                            | "INTERFACE"
                            | "UNKNOWN"
                            | "EXCLUDE_FROM_ALL"
                            | "WIN32"
                            | "MACOSX_BUNDLE"
                    ) {
                        continue;
                    }
                    sources.extend(self.source(dir, item));
                }
                if let Some(&at) = self.at.get(&name) {
                    self.out.targets[at].sources.extend(sources);
                    return;
                }
                self.at.insert(name.clone(), self.out.targets.len());
                self.out.targets.push(Target {
                    name,
                    kind,
                    dir: dir.to_string(),
                    file: file.to_string(),
                    sources,
                    include_dirs: Vec::new(),
                    links: Vec::new(),
                });
            }
            "target_sources" => {
                let words = self.expand_all(args, vars);
                let Some(name) = words.first() else { return };
                let Some(&at) = self.at.get(name) else { return };
                let mut found = Vec::new();
                for item in &words[1..] {
                    if matches!(
                        item.as_str(),
                        "PRIVATE"
                            | "PUBLIC"
                            | "INTERFACE"
                            | "FILE_SET"
                            | "TYPE"
                            | "BASE_DIRS"
                            | "FILES"
                            | "HEADERS"
                            | "CXX_MODULES"
                    ) {
                        continue;
                    }
                    found.extend(self.source(dir, item));
                }
                self.out.targets[at].sources.extend(found);
            }
            "target_include_directories" => {
                let words = self.expand_all(args, vars);
                let Some(name) = words.first() else { return };
                let Some(&at) = self.at.get(name) else { return };
                let dirs = self.directories(dir, &words[1..]);
                self.out.targets[at].include_dirs.extend(dirs);
            }
            "include_directories" => {
                let words = self.expand_all(args, vars);
                let dirs = self.directories(dir, &words);
                if !dirs.is_empty() {
                    self.out.dir_includes.push((dir.to_string(), dirs));
                }
            }
            "target_link_libraries" | "link_libraries" => {
                let words = self.expand_all(args, vars);
                let (name, rest) = if command.name == "link_libraries" {
                    (None, &words[..])
                } else {
                    match words.split_first() {
                        Some((name, rest)) => (Some(name), rest),
                        None => return,
                    }
                };
                let items: Vec<String> = rest
                    .iter()
                    .filter(|w| {
                        !matches!(
                            w.as_str(),
                            "PUBLIC" | "PRIVATE" | "INTERFACE" | "debug" | "optimized" | "general"
                        )
                    })
                    .cloned()
                    .collect();
                if let Some(&at) = name.and_then(|n| self.at.get(n)) {
                    self.out.targets[at].links.extend(items);
                }
            }
            "add_subdirectory" => {
                let Some(sub) = first.and_then(|f| locate(dir, &f)) else {
                    return;
                };
                if !self.files.contains(join(&sub, "CMakeLists.txt").as_str()) {
                    return;
                }
                let mut inner = vars.clone();
                inner.remove("\u{2}parent");
                self.directory(&sub, &mut inner, root, depth + 1);
                // What the subfolder exported with PARENT_SCOPE.
                for entry in inner.remove("\u{2}parent").unwrap_or_default() {
                    if let Some((name, value)) = entry.split_once('\u{2}') {
                        let value: Vec<String> = if value.is_empty() {
                            Vec::new()
                        } else {
                            value.split('\u{3}').map(str::to_string).collect()
                        };
                        vars.insert(name.to_string(), value);
                    }
                }
            }
            "find_package" => {
                let words = self.expand_all(args, vars);
                if let Some(package) = found(&words, file) {
                    self.out.packages.push(package);
                }
            }
            "fetchcontent_declare" | "cpmaddpackage" | "cpmfindpackage" => {
                let words = self.expand_all(args, vars);
                self.fetch(&command.name, &words, file);
            }
            "add_test" => {
                let words = self.expand_all(args, vars);
                let (name, command_at) = match words.first().map(String::as_str) {
                    Some("NAME") => (
                        words.get(1).cloned(),
                        words.iter().position(|w| w == "COMMAND"),
                    ),
                    Some(_) => (words.first().cloned(), Some(0)),
                    None => (None, None),
                };
                let Some(name) = name else { return };
                let target = command_at
                    .and_then(|at| words.get(at + 1))
                    .filter(|w| self.at.contains_key(w.as_str()))
                    .cloned();
                self.out.tests.push(Test {
                    name,
                    target,
                    discovered: None,
                });
            }
            "gtest_discover_tests"
            | "gtest_add_tests"
            | "catch_discover_tests"
            | "doctest_discover_tests" => {
                let words = self.expand_all(args, vars);
                let target = if command.name == "gtest_add_tests" {
                    words
                        .iter()
                        .position(|w| w == "TARGET")
                        .and_then(|at| words.get(at + 1))
                } else {
                    words.first()
                };
                let Some(target) = target else { return };
                let framework = match command.name.as_str() {
                    "catch_discover_tests" => "catch",
                    "doctest_discover_tests" => "doctest",
                    _ => "gtest",
                };
                self.out.tests.push(Test {
                    name: target.clone(),
                    target: Some(target.clone()),
                    discovered: Some(framework),
                });
            }
            _ => {}
        }
    }

    /// A source named in a target: a repo file, or a glob result that is
    /// already a repo path.
    fn source(&self, dir: &str, item: &str) -> Option<String> {
        if let Some(path) = item.strip_prefix('\u{4}') {
            return self.files.contains(path).then(|| path.to_string());
        }
        self.file_in(dir, item)
    }

    fn directories(&self, dir: &str, words: &[String]) -> Vec<String> {
        words
            .iter()
            .filter(|w| {
                !matches!(
                    w.as_str(),
                    "SYSTEM" | "BEFORE" | "AFTER" | "PUBLIC" | "PRIVATE" | "INTERFACE"
                )
            })
            .filter_map(|w| locate(dir, w))
            .collect()
    }

    fn fetch(&mut self, command: &str, words: &[String], file: &str) {
        if let Some(package) = fetched(command, words, file) {
            self.out.packages.push(package);
        }
    }
}

/// The libraries one `CMakeLists.txt` asks for, read from its text alone:
/// no variable is followed, so a name or version written through one is
/// missed. For the build file of a unit, which has no other files at hand.
pub fn declared_packages(text: &str, file: &str) -> Vec<Package> {
    let mut out = Vec::new();
    for command in commands(text) {
        let words: Vec<String> = command
            .args
            .iter()
            .map(|a| a.trim_start_matches('\u{1}').to_string())
            .filter(|w| !w.contains("${"))
            .collect();
        match command.name.as_str() {
            "find_package" => {
                if let Some(package) = found(&words, file) {
                    out.push(package);
                }
            }
            "fetchcontent_declare" | "cpmaddpackage" | "cpmfindpackage" => {
                if let Some(package) = fetched(&command.name, &words, file) {
                    out.push(package);
                }
            }
            _ => {}
        }
    }
    out
}

fn found(words: &[String], file: &str) -> Option<Package> {
    let name = words.first()?;
    let version = words
        .get(1)
        .filter(|v| v.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .cloned()
        .unwrap_or_default();
    Some(Package {
        name: name.clone(),
        version,
        source: PackageSource::FindPackage,
        file: file.to_string(),
    })
}

fn fetched(command: &str, words: &[String], file: &str) -> Option<Package> {
    {
        let mut name = words.first().cloned().unwrap_or_default();
        let mut version = String::new();
        let mut tag = String::new();
        let mut i = 1;
        while i < words.len() {
            match words[i].as_str() {
                "GIT_TAG" => tag = words.get(i + 1).cloned().unwrap_or_default(),
                "VERSION" => version = words.get(i + 1).cloned().unwrap_or_default(),
                "NAME" => name = words.get(i + 1).cloned().unwrap_or_default(),
                _ => {}
            }
            i += 1;
        }
        // `CPMAddPackage("gh:fmtlib/fmt#10.1.1")`.
        if command != "fetchcontent_declare"
            && let Some(spec) = words
                .first()
                .and_then(|w| w.split_once(':').map(|(_, rest)| rest))
        {
            let (repo, at) = spec.split_once('#').unwrap_or((spec, ""));
            name = repo.rsplit('/').next().unwrap_or(repo).to_string();
            tag = at.to_string();
        }
        if name.is_empty() || name == "NAME" {
            return None;
        }
        if version.is_empty() {
            version = tag;
        }
        Some(Package {
            name,
            version,
            source: PackageSource::FetchContent,
            file: file.to_string(),
        })
    }
}

/// `*` and `?` in a file name.
fn wildcard(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    fn go(p: &[char], n: &[char]) -> bool {
        match p.first() {
            None => n.is_empty(),
            Some('*') => (0..=n.len()).any(|k| go(&p[1..], &n[k..])),
            Some('?') => !n.is_empty() && go(&p[1..], &n[1..]),
            Some(c) => n.first() == Some(c) && go(&p[1..], &n[1..]),
        }
    }
    go(&p, &n)
}

impl Cmake {
    /// True when the repository has a `CMakeLists.txt` that declares a target.
    pub fn has_targets(&self) -> bool {
        !self.targets.is_empty()
    }

    /// The directories a source file may `#include` from, besides its own:
    /// those named for the targets that list it or sit in its folder or
    /// above, for the folders it lies in, and for the targets they link.
    pub fn include_dirs(&self, file: &str) -> Vec<String> {
        let dir = file.rsplit_once('/').map_or("", |(d, _)| d);
        let mut out: Vec<String> = Vec::new();
        for (at, dirs) in &self.dir_includes {
            if at.is_empty() || dir == at || dir.starts_with(&format!("{at}/")) {
                out.extend(dirs.iter().cloned());
            }
        }
        let mut seen: HashSet<&str> = HashSet::new();
        let mut queue: Vec<&Target> = self
            .targets
            .iter()
            .filter(|t| {
                t.sources.iter().any(|s| s == file)
                    || t.dir.is_empty()
                    || dir == t.dir
                    || dir.starts_with(&format!("{}/", t.dir))
            })
            .collect();
        let mut rounds = 0;
        while let Some(target) = queue.pop() {
            if !seen.insert(target.name.as_str()) || rounds > 200 {
                continue;
            }
            rounds += 1;
            out.extend(target.include_dirs.iter().cloned());
            for link in &target.links {
                if let Some(next) = self.targets.iter().find(|t| &t.name == link) {
                    queue.push(next);
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// The target that lists `file`, else the one of the nearest folder
    /// above it, a library before an executable.
    pub fn target_of(&self, file: &str) -> Option<&Target> {
        if let Some(listed) = self
            .targets
            .iter()
            .find(|t| t.sources.iter().any(|s| s == file))
        {
            return Some(listed);
        }
        // A header sits beside the source that includes it.
        let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
        if let Some(pair) = self.targets.iter().find(|t| {
            t.sources
                .iter()
                .any(|s| s.rsplit_once('.').map_or(s.as_str(), |(s, _)| s) == stem)
        }) {
            return Some(pair);
        }
        let dir = file.rsplit_once('/').map_or("", |(d, _)| d);
        let mut best: Option<&Target> = None;
        for target in &self.targets {
            let inside = target.dir.is_empty()
                || dir == target.dir
                || dir.starts_with(&format!("{}/", target.dir));
            if !inside {
                continue;
            }
            let deeper = best.is_none_or(|b| {
                target.dir.len() > b.dir.len()
                    || (target.dir.len() == b.dir.len()
                        && b.kind == TargetKind::Executable
                        && target.kind == TargetKind::Library)
            });
            if deeper {
                best = Some(target);
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(files: &[(&str, &str)]) -> Cmake {
        let mut paths: Vec<String> = files.iter().map(|(p, _)| p.to_string()).collect();
        paths.sort();
        let texts: HashMap<String, String> = files
            .iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect();
        read_cmake(&paths, &|p| texts.get(p).cloned())
    }

    #[test]
    fn splits_commands_and_skips_comments() {
        let found = commands(
            "# a comment\nadd_library(core STATIC # trailing\n  a.c \"b c.c\" [[raw]])\n#[[ block\nadd_library(no) ]]\nIF(X AND (Y OR Z))\nendif()\n",
        );
        let names: Vec<&str> = found.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["add_library", "if", "endif"]);
        assert_eq!(
            found[0].args,
            ["core", "STATIC", "a.c", "\u{1}b c.c", "\u{1}raw"]
        );
    }

    #[test]
    fn reads_targets_sources_through_variables_and_links() {
        let cmake = read(&[
            (
                "CMakeLists.txt",
                "cmake_minimum_required(VERSION 3.20)\nproject(demo VERSION 1.2)\nset(CORE_SRC src/core.c src/util.c)\nlist(APPEND CORE_SRC src/extra.c)\nadd_library(core ${CORE_SRC})\ntarget_include_directories(core PUBLIC include ${CMAKE_CURRENT_SOURCE_DIR}/src)\nadd_executable(demo_app main.c)\ntarget_link_libraries(demo_app PRIVATE core m)\n",
            ),
            ("main.c", ""),
            ("src/core.c", ""),
            ("src/util.c", ""),
            ("src/extra.c", ""),
            ("src/ghost.c", ""),
        ]);
        let core = cmake.targets.iter().find(|t| t.name == "core").unwrap();
        assert_eq!(core.kind, TargetKind::Library);
        assert_eq!(core.sources, ["src/core.c", "src/extra.c", "src/util.c"]);
        assert_eq!(core.include_dirs, ["include", "src"]);
        let app = cmake.targets.iter().find(|t| t.name == "demo_app").unwrap();
        assert_eq!(app.kind, TargetKind::Executable);
        assert_eq!(app.sources, ["main.c"]);
        assert_eq!(app.links, ["core", "m"]);
        // The executable sees the library's include directories.
        assert_eq!(cmake.include_dirs("main.c"), ["include", "src"]);
    }

    #[test]
    fn follows_subdirectories_and_globs() {
        let cmake = read(&[
            (
                "CMakeLists.txt",
                "project(top)\nadd_subdirectory(lib)\nadd_subdirectory(app)\n",
            ),
            (
                "lib/CMakeLists.txt",
                "file(GLOB LIB_SRC CONFIGURE_DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/*.cpp)\nadd_library(net ${LIB_SRC})\ntarget_sources(net PRIVATE extra/more.cpp)\ninclude_directories(${PROJECT_SOURCE_DIR}/lib/inc)\n",
            ),
            ("lib/a.cpp", ""),
            ("lib/b.cpp", ""),
            ("lib/deep/c.cpp", ""),
            ("lib/extra/more.cpp", ""),
            ("lib/a.hpp", ""),
            (
                "app/CMakeLists.txt",
                "add_executable(app main.cpp)\ntarget_link_libraries(app net)\nadd_test(NAME smoke COMMAND app --quick)\nadd_test(plain app)\n",
            ),
            ("app/main.cpp", ""),
        ]);
        let net = cmake.targets.iter().find(|t| t.name == "net").unwrap();
        assert_eq!(
            net.sources,
            ["lib/a.cpp", "lib/b.cpp", "lib/extra/more.cpp"]
        );
        assert_eq!(net.dir, "lib");
        assert_eq!(net.file, "lib/CMakeLists.txt");
        assert_eq!(
            cmake.dir_includes,
            [("lib".to_string(), vec!["lib/inc".to_string()])]
        );
        let names: Vec<(&str, Option<&str>)> = cmake
            .tests
            .iter()
            .map(|t| (t.name.as_str(), t.target.as_deref()))
            .collect();
        assert_eq!(names, [("smoke", Some("app")), ("plain", Some("app"))]);
        // A header with no listing falls to the target of its folder; a
        // header beside a listed source pairs with it.
        assert_eq!(cmake.target_of("lib/a.hpp").unwrap().name, "net");
        assert_eq!(cmake.target_of("lib/deep/x.hpp").unwrap().name, "net");
        assert!(cmake.target_of("tools/x.c").is_none());
    }

    #[test]
    fn exports_with_parent_scope_and_skips_what_it_cannot_read() {
        let cmake = read(&[
            (
                "CMakeLists.txt",
                "add_subdirectory(src)\nadd_executable(tool ${SRC_LIST} $<TARGET_OBJECTS:x> ${UNKNOWN}/y.c)\nfunction(helper)\nadd_library(hidden a.c)\nendfunction()\nif(WIN32)\nadd_library(win win.c)\nelse()\nadd_library(posix posix.c)\nendif()\n",
            ),
            (
                "src/CMakeLists.txt",
                "set(SRC_LIST src/a.c src/b.c PARENT_SCOPE)\n",
            ),
            ("src/a.c", ""),
            ("src/b.c", ""),
            ("win.c", ""),
            ("posix.c", ""),
            ("a.c", ""),
        ]);
        let names: Vec<&str> = cmake.targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["tool", "win", "posix"]);
        assert_eq!(cmake.targets[0].sources, ["src/a.c", "src/b.c"]);
    }

    #[test]
    fn reads_packages_and_registered_tests() {
        let cmake = read(&[
            (
                "CMakeLists.txt",
                "find_package(Boost 1.74 REQUIRED COMPONENTS system)\nfind_package(Threads)\nFetchContent_Declare(googletest GIT_REPOSITORY https://x/googletest.git GIT_TAG v1.14.0)\nCPMAddPackage(\"gh:fmtlib/fmt#10.1.1\")\nadd_executable(unit_tests t.cpp)\ngtest_discover_tests(unit_tests)\nadd_library(alias_real r.cpp)\nadd_library(ns::real ALIAS alias_real)\nadd_executable(user u.cpp)\ntarget_link_libraries(user ns::real)\n",
            ),
            ("t.cpp", ""),
            ("r.cpp", ""),
            ("u.cpp", ""),
        ]);
        let packages: Vec<(&str, &str, PackageSource)> = cmake
            .packages
            .iter()
            .map(|p| (p.name.as_str(), p.version.as_str(), p.source))
            .collect();
        assert_eq!(
            packages,
            [
                ("Boost", "1.74", PackageSource::FindPackage),
                ("Threads", "", PackageSource::FindPackage),
                ("googletest", "v1.14.0", PackageSource::FetchContent),
                ("fmt", "10.1.1", PackageSource::FetchContent),
            ]
        );
        assert_eq!(
            cmake.tests,
            [Test {
                name: "unit_tests".into(),
                target: Some("unit_tests".into()),
                discovered: Some("gtest"),
            }]
        );
        let user = cmake.targets.iter().find(|t| t.name == "user").unwrap();
        assert_eq!(user.links, ["alias_real"]);
    }
}
