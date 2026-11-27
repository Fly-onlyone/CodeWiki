//! What the build files of a JavaScript or TypeScript repository say, read
//! as data and never run: `tsconfig.json` and `jsconfig.json` (`baseUrl`,
//! `paths`, `extends`), `package.json` (name, entry points, workspaces, the
//! package manager), `pnpm-workspace.yaml`, and the lock files of npm, pnpm,
//! Yarn and Bun.
//!
//! tsconfig is JSONC: comments and trailing commas are allowed. A bundler's
//! own aliases (`vite.config.ts`, webpack) are code and are not evaluated.
//! The YAML readers are line based and know only the shapes their tests pin;
//! a file in another shape reads as empty, and a lock file of a format this
//! module does not know is "not read", never a guess.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::Value;

/// Source extensions a `package.json` entry may carry.
const SOURCE_EXTENSIONS: [&str; 8] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];
/// Directories a bundler writes to; `main` often points into one while the
/// source sits under `src`.
const OUTPUT_DIRS: [&str; 4] = ["dist", "build", "lib", "out"];
/// How far an `extends` chain is followed.
const MAX_EXTENDS: usize = 8;

/// Lock files, in the order that decides when a folder holds several.
const LOCK_FILES: [(&str, Manager); 6] = [
    ("pnpm-lock.yaml", Manager::Pnpm),
    ("yarn.lock", Manager::Yarn),
    ("bun.lock", Manager::Bun),
    ("bun.lockb", Manager::Bun),
    ("package-lock.json", Manager::Npm),
    ("npm-shrinkwrap.json", Manager::Npm),
];

pub fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

pub fn join(dir: &str, file: &str) -> String {
    if dir.is_empty() {
        file.to_string()
    } else {
        format!("{dir}/{file}")
    }
}

/// `dir` and each of its ancestors, nearest first, ending at the root (`""`).
pub fn ancestors(dir: &str) -> Vec<&str> {
    let mut out = vec![dir];
    let mut current = dir;
    while let Some((parent, _)) = current.rsplit_once('/') {
        out.push(parent);
        current = parent;
    }
    if !dir.is_empty() {
        out.push("");
    }
    out
}

/// `target` taken from `dir`, with `.` and `..` folded; `None` when it
/// climbs out of the repository.
pub fn join_norm(dir: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for segment in target.split(['/', '\\']) {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

// ---------------------------------------------------------------- JSONC

/// `text` without comments and trailing commas, so that a JSON parser reads
/// it. Strings are left alone.
pub fn strip_jsonc(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '"' => {
                out.push(c);
                i += 1;
                while i < chars.len() {
                    out.push(chars[i]);
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        out.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    i += 1;
                    if chars[i - 1] == '"' {
                        break;
                    }
                }
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if chars.get(i + 1) == Some(&'*') => {
                i += 2;
                while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                    i += 1;
                }
                i += 2;
            }
            ',' => {
                // A comma before a closing bracket is a trailing one.
                let mut j = i + 1;
                loop {
                    while j < chars.len() && chars[j].is_whitespace() {
                        j += 1;
                    }
                    if chars.get(j) == Some(&'/') && chars.get(j + 1) == Some(&'/') {
                        while j < chars.len() && chars[j] != '\n' {
                            j += 1;
                        }
                    } else if chars.get(j) == Some(&'/') && chars.get(j + 1) == Some(&'*') {
                        j += 2;
                        while j < chars.len()
                            && !(chars[j] == '*' && chars.get(j + 1) == Some(&'/'))
                        {
                            j += 1;
                        }
                        j += 2;
                    } else {
                        break;
                    }
                }
                if !matches!(chars.get(j), Some('}' | ']')) {
                    out.push(c);
                }
                i += 1;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// A JSON document that may hold comments and trailing commas.
pub fn parse_jsonc(text: &str) -> Option<Value> {
    serde_json::from_str(strip_jsonc(text.trim_start_matches('\u{feff}')).as_str()).ok()
}

// ------------------------------------------------------------- tsconfig

/// One entry of `compilerOptions.paths`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRule {
    pub pattern: String,
    /// Targets as written, tried in order.
    pub targets: Vec<String>,
}

/// What a `tsconfig.json` or `jsconfig.json` says about how imports resolve.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TsConfig {
    /// Folder of the config file, repo-relative.
    pub dir: String,
    /// `baseUrl`, as a repo-relative folder, from the config that set it.
    pub base_url: Option<String>,
    pub paths: Vec<PathRule>,
    /// Folder the targets of `paths` are relative to when `baseUrl` is not
    /// set: the folder of the config that wrote `paths`.
    paths_dir: String,
}

/// What an import specifier names, as far as a config can tell.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct AliasHit {
    /// True when a `paths` pattern matches the specifier: the import is the
    /// repository's, found or not.
    pub alias: bool,
    /// Repo-relative paths to try, without an extension, in order.
    pub candidates: Vec<String>,
}

impl TsConfig {
    /// The paths `spec` may name: the targets of the best matching `paths`
    /// pattern, then `baseUrl` itself.
    pub fn lookup(&self, spec: &str) -> AliasHit {
        let mut hit = AliasHit::default();
        let base = self.base_url.as_deref().unwrap_or(&self.paths_dir);
        // The pattern with the longest prefix before its `*` wins; one with
        // no `*` must match whole.
        let best = self
            .paths
            .iter()
            .filter_map(|rule| {
                let captured = match_pattern(&rule.pattern, spec)?;
                Some((rule, captured))
            })
            .max_by_key(|(rule, _)| rule.pattern.find('*').unwrap_or(usize::MAX));
        if let Some((rule, captured)) = best {
            hit.alias = true;
            for target in &rule.targets {
                if let Some(path) = join_norm(base, &target.replace('*', &captured)) {
                    hit.candidates.push(path);
                }
            }
        }
        if let Some(base_url) = &self.base_url
            && !spec.starts_with(['.', '/'])
            && let Some(path) = join_norm(base_url, spec)
        {
            hit.candidates.push(path);
        }
        hit
    }
}

/// What a pattern with at most one `*` captures from `spec`.
fn match_pattern(pattern: &str, spec: &str) -> Option<String> {
    match pattern.split_once('*') {
        None => (pattern == spec).then(String::new),
        Some((prefix, suffix)) => {
            let rest = spec.strip_prefix(prefix)?;
            let captured = rest.strip_suffix(suffix)?;
            (rest.len() >= suffix.len()).then(|| captured.to_string())
        }
    }
}

/// The file an `extends` names, relative or in `node_modules` up the tree.
fn extends_file(
    from_dir: &str,
    spec: &str,
    read: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let variants = |base: &str| -> Vec<String> {
        let mut out = vec![base.to_string()];
        if !base.ends_with(".json") {
            out.push(format!("{base}.json"));
            out.push(format!("{base}/tsconfig.json"));
        }
        out
    };
    if spec.starts_with(['.', '/']) {
        let base = join_norm(from_dir, spec.trim_start_matches('/'))?;
        return variants(&base)
            .into_iter()
            .find(|candidate| read(candidate).is_some());
    }
    for dir in ancestors(from_dir) {
        let base = join(dir, &format!("node_modules/{spec}"));
        if let Some(found) = variants(&base)
            .into_iter()
            .find(|candidate| read(candidate).is_some())
        {
            return Some(found);
        }
    }
    None
}

/// Read the config at `path` and what it extends. A file that is missing or
/// not JSON reads as nothing; an `extends` that cannot be found is skipped.
pub fn read_tsconfig(path: &str, read: &dyn Fn(&str) -> Option<String>) -> Option<TsConfig> {
    load_tsconfig(path, read, 0)
}

fn load_tsconfig(
    path: &str,
    read: &dyn Fn(&str) -> Option<String>,
    depth: usize,
) -> Option<TsConfig> {
    let value = parse_jsonc(&read(path)?)?;
    let dir = parent_dir(path).to_string();
    let mut config = TsConfig {
        dir: dir.clone(),
        base_url: None,
        paths: Vec::new(),
        paths_dir: dir.clone(),
    };
    if depth < MAX_EXTENDS {
        let parents: Vec<&str> = match value.get("extends") {
            Some(Value::String(one)) => vec![one.as_str()],
            Some(Value::Array(many)) => many.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        // Later parents win over earlier ones, and the file over them all.
        for parent in parents {
            let loaded = extends_file(&dir, parent, read)
                .and_then(|file| load_tsconfig(&file, read, depth + 1));
            if let Some(loaded) = loaded {
                if loaded.base_url.is_some() {
                    config.base_url = loaded.base_url;
                }
                if !loaded.paths.is_empty() {
                    config.paths = loaded.paths;
                    config.paths_dir = loaded.paths_dir;
                }
            }
        }
    }
    let options = value.get("compilerOptions");
    if let Some(base) = options
        .and_then(|o| o.get("baseUrl"))
        .and_then(Value::as_str)
    {
        config.base_url = join_norm(&dir, base);
    }
    if let Some(paths) = options
        .and_then(|o| o.get("paths"))
        .and_then(Value::as_object)
    {
        config.paths = paths
            .iter()
            .map(|(pattern, targets)| PathRule {
                pattern: pattern.clone(),
                targets: targets
                    .as_array()
                    .map(|list| {
                        list.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect();
        config.paths_dir = dir;
    }
    Some(config)
}

// --------------------------------------------------------- file lookups

/// Source file `base` names: itself, with an extension, or an `index` file in
/// the folder of that name. `./text.js` is how ESM TypeScript names `./text.ts`.
pub fn resolve_source(files: &HashSet<&str>, base: &str) -> Option<String> {
    const TS: [&str; 5] = ["ts", "tsx", "d.ts", "mts", "cts"];
    const JS: [&str; 4] = ["js", "jsx", "mjs", "cjs"];
    let is_source = |p: &str| {
        let ext = p.rsplit_once('.').map_or("", |(_, e)| e);
        SOURCE_EXTENSIONS.contains(&ext)
    };
    if is_source(base) && files.contains(base) {
        return Some(base.to_string());
    }
    let stem = [".js", ".jsx", ".mjs", ".cjs"]
        .iter()
        .find_map(|ext| base.strip_suffix(ext))
        .unwrap_or(base);
    let index = if base.is_empty() {
        "index".to_string()
    } else {
        format!("{base}/index")
    };
    [stem, index.as_str()]
        .iter()
        .filter(|candidate| !candidate.is_empty())
        .flat_map(|candidate| {
            TS.iter()
                .chain(JS.iter())
                .map(move |ext| format!("{candidate}.{ext}"))
        })
        .find(|candidate| files.contains(candidate.as_str()))
}

/// Source file a `package.json` path names. Published packages point at
/// build output, so the matching file under `src` is tried as well.
pub fn resolve_js_entry(files: &HashSet<&str>, dir: &str, spec: &str) -> Option<String> {
    let spec = spec.trim().trim_start_matches("./");
    if spec.is_empty() || spec.starts_with("..") {
        return None;
    }
    let stem = SOURCE_EXTENSIONS
        .iter()
        .find_map(|ext| spec.strip_suffix(&format!(".{ext}")))
        .unwrap_or(spec);
    let mut stems = vec![stem.to_string()];
    if let Some((first, rest)) = stem.split_once('/')
        && OUTPUT_DIRS.contains(&first)
    {
        stems.push(format!("src/{rest}"));
    }
    stems
        .iter()
        .flat_map(|s| {
            SOURCE_EXTENSIONS
                .iter()
                .map(move |ext| join(dir, &format!("{s}.{ext}")))
        })
        .find(|candidate| files.contains(candidate.as_str()))
}

// -------------------------------------------------------------- packages

/// A package of the repository: a `package.json` with a name.
#[derive(Debug, Clone)]
pub struct JsPackage {
    pub dir: String,
    pub name: String,
    json: Value,
}

impl JsPackage {
    /// Every table of `package.json` that names the packages it needs.
    pub fn dependencies(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for table in [
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ] {
            if let Some(map) = self.json.get(table).and_then(Value::as_object) {
                for (name, spec) in map {
                    out.push((name.clone(), spec.as_str().unwrap_or("").to_string()));
                }
            }
        }
        out
    }

    pub fn json(&self) -> &Value {
        &self.json
    }

    /// Files a bare import of the package may mean, best first: the targets
    /// of `exports` for `.`, then `source`, `module`, `main`, `types`.
    fn entry_specs(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(exports) = self.json.get("exports") {
            let root = match exports {
                Value::Object(map) if map.keys().any(|k| k.starts_with('.')) => map.get("."),
                other => Some(other),
            };
            if let Some(root) = root {
                export_targets(root, &mut out);
            }
        }
        for key in ["source", "module", "main", "types", "typings"] {
            if let Some(spec) = self.json.get(key).and_then(Value::as_str) {
                out.push(spec.to_string());
            }
        }
        out
    }

    /// Targets of `exports` for the subpath `./sub`, patterns with `*`
    /// included.
    fn subpath_specs(&self, sub: &str) -> Vec<String> {
        let mut out = Vec::new();
        let Some(Value::Object(map)) = self.json.get("exports") else {
            return out;
        };
        let wanted = format!("./{sub}");
        for (key, value) in map.iter().filter(|(k, _)| k.starts_with('.')) {
            let mut found = Vec::new();
            export_targets(value, &mut found);
            match match_pattern(key, &wanted) {
                Some(captured) if key.contains('*') => {
                    out.extend(found.iter().map(|t| t.replace('*', &captured)));
                }
                Some(_) => out.extend(found),
                None => {}
            }
        }
        out
    }
}

/// The paths an `exports` value points at, the conditions a bundler or a
/// runtime picks first coming first.
fn export_targets(value: &Value, out: &mut Vec<String>) {
    const CONDITIONS: [&str; 8] = [
        "source", "import", "module", "default", "require", "node", "types", "browser",
    ];
    match value {
        Value::String(spec) => out.push(spec.clone()),
        Value::Array(list) => {
            for item in list {
                export_targets(item, out);
            }
        }
        Value::Object(map) => {
            for key in CONDITIONS {
                if let Some(inner) = map.get(key) {
                    export_targets(inner, out);
                }
            }
            for (key, inner) in map {
                if !CONDITIONS.contains(&key.as_str()) && !key.starts_with('.') {
                    export_targets(inner, out);
                }
            }
        }
        _ => {}
    }
}

/// The patterns of `pnpm-workspace.yaml`'s `packages:` list. Only the list
/// forms pnpm documents are read: block (`- 'packages/*'`) and flow
/// (`packages: ['a', 'b']`).
pub fn parse_pnpm_workspace(text: &str) -> Vec<String> {
    let unquote = |s: &str| {
        let s = s.split(" #").next().unwrap_or(s).trim();
        s.trim_matches(['\'', '"']).to_string()
    };
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }
        let top_level = !trimmed.starts_with([' ', '\t', '-']);
        if top_level {
            inside = false;
            if let Some(rest) = trimmed.strip_prefix("packages:") {
                let rest = rest.trim();
                if let Some(flow) = rest.strip_prefix('[') {
                    let flow = flow.trim_end_matches(']');
                    out.extend(flow.split(',').map(unquote).filter(|p| !p.is_empty()));
                } else if rest.is_empty() || rest.starts_with('#') {
                    inside = true;
                }
            }
            continue;
        }
        if inside && let Some(item) = trimmed.trim_start().strip_prefix('-') {
            let item = unquote(item);
            if !item.is_empty() {
                out.push(item);
            }
        }
    }
    out
}

/// True when the folder `dir` (relative to the workspace root) is named by
/// one glob of a workspace: `*` stands for one folder name, `**` for any
/// number of folders.
pub fn glob_matches(pattern: &str, dir: &str) -> bool {
    fn go(pattern: &[&str], dir: &[&str]) -> bool {
        match pattern.split_first() {
            None => dir.is_empty(),
            Some((&"**", rest)) => (0..=dir.len()).any(|skip| go(rest, &dir[skip..])),
            Some((segment, rest)) => dir
                .split_first()
                .is_some_and(|(first, more)| segment_matches(segment, first) && go(rest, more)),
        }
    }
    fn segment_matches(pattern: &str, name: &str) -> bool {
        match pattern.split_once('*') {
            None => pattern == name,
            Some((prefix, suffix)) => {
                name.len() >= prefix.len() + suffix.len()
                    && name.starts_with(prefix)
                    && name.ends_with(suffix)
            }
        }
    }
    let pattern = pattern.trim_start_matches("./").trim_end_matches('/');
    let p: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let d: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    go(&p, &d)
}

// ------------------------------------------------------ package manager

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Manager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl Manager {
    pub fn name(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
        }
    }
}

/// The package manager of one package and where it works from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmInfo {
    pub manager: Manager,
    /// Yarn 2 and later (Berry), as against Yarn 1.
    pub berry: bool,
    /// Folder an install runs in: the one that holds the lock file, else the
    /// workspace root, else the package itself.
    pub root: String,
    /// The lock file, repo-relative, when there is one the manager writes.
    pub lock: Option<String>,
    /// Name of the package when it is a member of a workspace, other than
    /// the workspace's own root package.
    pub member: Option<String>,
}

impl PmInfo {
    /// The npm of a repository that says nothing else.
    pub fn plain(root: &str) -> Self {
        Self {
            manager: Manager::Npm,
            berry: false,
            root: root.to_string(),
            lock: None,
            member: None,
        }
    }

    /// The command that installs from the lock file when there is one:
    /// exactly what it pins, failing when `package.json` disagrees.
    pub fn install(&self) -> &'static str {
        match (self.manager, self.lock.is_some(), self.berry) {
            (Manager::Npm, true, _) => "npm ci",
            (Manager::Npm, false, _) => "npm install",
            (Manager::Pnpm, true, _) => "pnpm install --frozen-lockfile",
            (Manager::Pnpm, false, _) => "pnpm install",
            (Manager::Yarn, true, true) => "yarn install --immutable",
            (Manager::Yarn, true, false) => "yarn install --frozen-lockfile",
            (Manager::Yarn, false, _) => "yarn install",
            (Manager::Bun, true, _) => "bun install --frozen-lockfile",
            (Manager::Bun, false, _) => "bun install",
        }
    }
}

/// `pnpm@9.1.0+sha512...` as (manager, major version).
fn parse_package_manager(field: &str) -> Option<(Manager, Option<u32>)> {
    let (name, version) = field.split_once('@').unwrap_or((field, ""));
    let manager = match name {
        "npm" => Manager::Npm,
        "pnpm" => Manager::Pnpm,
        "yarn" => Manager::Yarn,
        "bun" => Manager::Bun,
        _ => return None,
    };
    let major = version
        .split(['.', '+'])
        .next()
        .and_then(|major| major.parse().ok());
    Some((manager, major))
}

/// Everything a repository's JavaScript build files say, read once.
#[derive(Debug, Default)]
pub struct JsRepo {
    packages: Vec<JsPackage>,
    /// Workspace root folder -> its globs (a `!` leads an exclusion).
    workspaces: BTreeMap<String, Vec<String>>,
    configs: BTreeMap<String, TsConfig>,
    /// Every file of the repository, for lock files and `.yarnrc.yml`.
    files: HashSet<String>,
    /// Folders that hold a `.yarnrc.yml`.
    berry_roots: HashSet<String>,
    /// Whether each `yarn.lock` is Yarn Berry's, by its path.
    lock_headers: HashMap<String, bool>,
    /// The packages some workspace lists: index into `packages`, and the
    /// folder of the workspace.
    members: Vec<(usize, String)>,
}

impl JsRepo {
    /// Read the build files among `paths` through `read`.
    pub fn read(paths: &[String], read: &dyn Fn(&str) -> Option<String>) -> Self {
        let mut repo = Self {
            files: paths.iter().cloned().collect(),
            ..Self::default()
        };
        for path in paths {
            let name = path.rsplit('/').next().unwrap_or(path);
            let dir = parent_dir(path);
            match name {
                "package.json" => {
                    let Some(json) =
                        read(path).and_then(|t| serde_json::from_str::<Value>(&t).ok())
                    else {
                        continue;
                    };
                    let globs = match json.get("workspaces") {
                        Some(Value::Array(list)) => list.clone(),
                        Some(Value::Object(map)) => map
                            .get("packages")
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    let globs: Vec<String> = globs
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect();
                    if !globs.is_empty() {
                        repo.workspaces
                            .entry(dir.to_string())
                            .or_default()
                            .extend(globs);
                    }
                    let name = json
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    repo.packages.push(JsPackage {
                        dir: dir.to_string(),
                        name,
                        json,
                    });
                }
                "pnpm-workspace.yaml" => {
                    if let Some(text) = read(path) {
                        let globs = parse_pnpm_workspace(&text);
                        if !globs.is_empty() {
                            repo.workspaces
                                .entry(dir.to_string())
                                .or_default()
                                .extend(globs);
                        }
                    }
                }
                "tsconfig.json" => {
                    if let Some(config) = read_tsconfig(path, read) {
                        repo.configs.insert(dir.to_string(), config);
                    }
                }
                _ => {}
            }
        }
        // `tsconfig.json` wins over `jsconfig.json` in one folder.
        for path in paths.iter().filter(|p| p.ends_with("jsconfig.json")) {
            let dir = parent_dir(path);
            if path.rsplit('/').next() == Some("jsconfig.json")
                && !repo.configs.contains_key(dir)
                && let Some(config) = read_tsconfig(path, read)
            {
                repo.configs.insert(dir.to_string(), config);
            }
        }
        // Yarn 2 and later keeps its settings in `.yarnrc.yml`; Yarn 1 reads
        // `.yarnrc`. A lock file's header says too.
        for path in paths {
            if path.rsplit('/').next() == Some("yarn.lock") {
                let berry = read(path).is_some_and(|text| text.contains("__metadata:"));
                repo.lock_headers.insert(path.clone(), berry);
            }
            if path.rsplit('/').next() == Some(".yarnrc.yml") {
                repo.berry_roots.insert(parent_dir(path).to_string());
            }
        }
        repo.find_members();
        repo
    }

    pub fn package_at(&self, dir: &str) -> Option<&JsPackage> {
        self.packages.iter().find(|p| p.dir == dir)
    }

    pub fn has_workspaces(&self) -> bool {
        !self.workspaces.is_empty()
    }

    /// The packages that a workspace lists, with the root folder of the
    /// workspace that lists each. A package of the workspace root itself is
    /// not among them.
    pub fn members(&self) -> Vec<(&JsPackage, &str)> {
        self.members
            .iter()
            .map(|(at, root)| (&self.packages[*at], root.as_str()))
            .collect()
    }

    fn find_members(&mut self) {
        let mut out: Vec<(usize, String)> = Vec::new();
        for (root, globs) in &self.workspaces {
            for (at, package) in self.packages.iter().enumerate() {
                if package.dir == *root || out.iter().any(|(i, _)| *i == at) {
                    continue;
                }
                let Some(below) = (if root.is_empty() {
                    Some(package.dir.as_str())
                } else {
                    package
                        .dir
                        .strip_prefix(root.as_str())
                        .and_then(|rest| rest.strip_prefix('/'))
                }) else {
                    continue;
                };
                let mut listed = false;
                for glob in globs {
                    match glob.strip_prefix('!') {
                        Some(excluded) => {
                            if glob_matches(excluded, below) {
                                listed = false;
                            }
                        }
                        None => {
                            if glob_matches(glob, below) {
                                listed = true;
                            }
                        }
                    }
                }
                if listed {
                    out.push((at, root.clone()));
                }
            }
        }
        self.members = out;
    }

    /// The workspace member in `dir`, with the root of its workspace.
    fn member_at(&self, dir: &str) -> Option<(&JsPackage, String)> {
        self.members
            .iter()
            .find(|(at, _)| self.packages[*at].dir == dir)
            .map(|(at, root)| (&self.packages[*at], root.clone()))
    }

    /// The package manager of the package in `dir`: what the nearest
    /// `package.json` that says so names in `packageManager`, else the lock
    /// file nearest above, else a workspace file, else npm.
    pub fn manager_of(&self, dir: &str) -> PmInfo {
        let mut from_field: Option<(Manager, Option<u32>)> = None;
        for d in ancestors(dir) {
            if let Some(found) = self
                .package_at(d)
                .and_then(|p| p.json.get("packageManager"))
                .and_then(Value::as_str)
                .and_then(parse_package_manager)
            {
                from_field = Some(found);
                break;
            }
        }
        let lock_in = |d: &str, wanted: Option<Manager>| -> Option<(String, Manager)> {
            LOCK_FILES
                .iter()
                .filter(|(_, m)| wanted.is_none_or(|w| w == *m))
                .map(|(file, m)| (join(d, file), *m))
                .find(|(path, _)| self.files.contains(path))
        };
        let wanted = from_field.map(|(m, _)| m);
        let lock = ancestors(dir)
            .into_iter()
            .find_map(|d| lock_in(d, wanted).map(|(path, m)| (d.to_string(), path, m)));
        let workspace_root = ancestors(dir)
            .into_iter()
            .find(|d| self.workspaces.contains_key(*d))
            .map(str::to_string);
        let pnpm_file = ancestors(dir)
            .into_iter()
            .find(|d| self.files.contains(&join(d, "pnpm-workspace.yaml")))
            .map(str::to_string);
        let manager = match (&from_field, &lock) {
            (Some((m, _)), _) => *m,
            (None, Some((_, _, m))) => *m,
            (None, None) if pnpm_file.is_some() => Manager::Pnpm,
            (None, None) => Manager::Npm,
        };
        // A lock file the manager cannot read (`bun.lockb`) is no lock.
        let lock = lock.filter(|(_, path, _)| !path.ends_with("bun.lockb"));
        let root = match &lock {
            Some((d, _, _)) => d.clone(),
            None => workspace_root.clone().unwrap_or_else(|| dir.to_string()),
        };
        let berry = manager == Manager::Yarn
            && match from_field {
                Some((Manager::Yarn, Some(major))) => major >= 2,
                _ => {
                    self.berry_roots
                        .iter()
                        .any(|r| r.is_empty() || root == *r || root.starts_with(&format!("{r}/")))
                        || lock
                            .as_ref()
                            .is_some_and(|(_, path, _)| self.lock_headers.get(path) == Some(&true))
                }
            };
        PmInfo {
            manager,
            berry,
            root,
            lock: lock.map(|(_, path, _)| path),
            member: self
                .member_at(dir)
                .map(|(p, _)| p.name.clone())
                .filter(|name| !name.is_empty()),
        }
    }

    // ------------------------------------------------- import resolution

    /// The config that applies to a file: the nearest one up the tree.
    pub fn config_for(&self, file: &str) -> Option<&TsConfig> {
        ancestors(parent_dir(file))
            .into_iter()
            .find_map(|d| self.configs.get(d))
    }

    /// The file a workspace package's name or subpath names.
    pub fn resolve_member(&self, files: &HashSet<&str>, spec: &str) -> Option<String> {
        let members = self.members();
        let (package, sub) = members
            .iter()
            .filter(|(p, _)| !p.name.is_empty())
            .filter_map(|(p, _)| {
                let rest = spec.strip_prefix(p.name.as_str())?;
                (rest.is_empty() || rest.starts_with('/'))
                    .then_some((*p, rest.trim_start_matches('/')))
            })
            .max_by_key(|(p, _)| p.name.len())?;
        let dir = package.dir.as_str();
        if sub.is_empty() {
            for entry in package.entry_specs() {
                if let Some(found) = resolve_js_entry(files, dir, &entry) {
                    return Some(found);
                }
            }
            return resolve_source(files, &join(dir, "src/index"))
                .or_else(|| resolve_source(files, &join(dir, "index")));
        }
        for entry in package.subpath_specs(sub) {
            if let Some(found) = resolve_js_entry(files, dir, &entry) {
                return Some(found);
            }
        }
        resolve_source(files, &join(dir, sub))
            .or_else(|| resolve_source(files, &join(dir, &format!("src/{sub}"))))
    }

    /// The repository file an import of `spec` in `from` names, and whether
    /// the specifier is an alias of the repository whether or not a file was
    /// found. The order: `paths`, `baseUrl`, a workspace package by name.
    pub fn resolve_import(
        &self,
        files: &HashSet<&str>,
        from: &str,
        spec: &str,
    ) -> (Option<String>, bool) {
        if spec.starts_with(['.', '/']) {
            return (None, false);
        }
        let mut alias = false;
        if let Some(config) = self.config_for(from) {
            let hit = config.lookup(spec);
            alias = hit.alias;
            if let Some(found) = hit
                .candidates
                .iter()
                .find_map(|candidate| resolve_source(files, candidate))
            {
                return (Some(found), true);
            }
        }
        if let Some(found) = self.resolve_member(files, spec) {
            return (Some(found), true);
        }
        (None, alias)
    }
}

// ------------------------------------------------------------ lock files

/// The versions a lock file pins. A format the reader does not know is
/// `None` from [`JsLock::read`], and so no version is claimed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsLock {
    /// `package-lock.json` and `npm-shrinkwrap.json`, lockfileVersion 1 to 3:
    /// install path -> version.
    Npm(BTreeMap<String, String>),
    /// `pnpm-lock.yaml`, lockfileVersion 6 and 9: importer folder -> package
    /// -> version.
    Pnpm(BTreeMap<String, BTreeMap<String, String>>),
    /// `yarn.lock` of Yarn 1 and Yarn Berry: `name@range` -> version.
    Yarn(BTreeMap<String, String>),
    /// `bun.lock`, lockfileVersion 0 and 1: key -> version.
    Bun(BTreeMap<String, String>),
}

impl JsLock {
    /// Read the lock file at `path` (by its name) with the text `text`.
    pub fn read(path: &str, text: &str) -> Option<Self> {
        match path.rsplit('/').next()? {
            "package-lock.json" | "npm-shrinkwrap.json" => npm_lock(text).map(Self::Npm),
            "pnpm-lock.yaml" => pnpm_lock(text).map(Self::Pnpm),
            "yarn.lock" => yarn_lock(text).map(Self::Yarn),
            "bun.lock" => bun_lock(text).map(Self::Bun),
            _ => None,
        }
    }

    /// The version pinned for `name`, which the package in `dir` asks for as
    /// `requirement`. `root` is the folder of the lock file.
    pub fn version(&self, root: &str, dir: &str, name: &str, requirement: &str) -> Option<String> {
        let below = dir
            .strip_prefix(root)
            .map(|d| d.trim_start_matches('/'))
            .unwrap_or(dir);
        match self {
            Self::Npm(map) => {
                let own = if below.is_empty() {
                    format!("node_modules/{name}")
                } else {
                    format!("{below}/node_modules/{name}")
                };
                map.get(&own)
                    .or_else(|| map.get(&format!("node_modules/{name}")))
                    .or_else(|| map.get(name))
                    .cloned()
            }
            Self::Pnpm(importers) => {
                let key = if below.is_empty() { "." } else { below };
                importers.get(key)?.get(name).cloned()
            }
            Self::Yarn(map) => {
                let bare = requirement.trim();
                [format!("{name}@{bare}"), format!("{name}@npm:{bare}")]
                    .iter()
                    .find_map(|key| map.get(key))
                    .cloned()
                    .or_else(|| {
                        // No entry for that range: the only version there is.
                        let prefix = format!("{name}@");
                        let mut found: Vec<&String> = map
                            .iter()
                            .filter(|(k, _)| k.starts_with(&prefix))
                            .map(|(_, v)| v)
                            .collect();
                        found.sort();
                        found.dedup();
                        (found.len() == 1).then(|| found[0].clone())
                    })
            }
            Self::Bun(map) => {
                let own = if below.is_empty() {
                    name.to_string()
                } else {
                    format!("{below}/{name}")
                };
                map.get(&own).or_else(|| map.get(name)).cloned()
            }
        }
    }
}

fn npm_lock(text: &str) -> Option<BTreeMap<String, String>> {
    let value: Value = serde_json::from_str(text).ok()?;
    let version = value.get("lockfileVersion")?.as_u64()?;
    if !(1..=3).contains(&version) {
        return None;
    }
    let mut out = BTreeMap::new();
    if let Some(packages) = value.get("packages").and_then(Value::as_object) {
        for (path, entry) in packages {
            if let Some(v) = entry.get("version").and_then(Value::as_str)
                && path.contains("node_modules/")
            {
                out.insert(path.clone(), v.to_string());
            }
        }
    }
    // Version 1 names its packages under `dependencies`.
    if let Some(dependencies) = value.get("dependencies").and_then(Value::as_object) {
        for (name, entry) in dependencies {
            if let Some(v) = entry.get("version").and_then(Value::as_str) {
                out.entry(name.clone()).or_insert_with(|| v.to_string());
            }
        }
    }
    Some(out)
}

/// The importers of `pnpm-lock.yaml` (lockfileVersion '6.0' or '9.0'): the
/// part above `packages:` and `snapshots:`, read by indentation.
fn pnpm_lock(text: &str) -> Option<BTreeMap<String, BTreeMap<String, String>>> {
    let mut lines = text.lines();
    let version = lines.find_map(|l| l.strip_prefix("lockfileVersion:"))?;
    let version = version.trim().trim_matches(['\'', '"']);
    if !matches!(version, "6.0" | "9.0") {
        return None;
    }
    let key = |s: &str| s.trim().trim_matches(['\'', '"']).to_string();
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    // Where the reader is: the importer, the table, the package.
    let mut importer: Option<String> = None;
    let mut in_importers = false;
    let mut table = false;
    let mut package: Option<String> = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let body = line.trim();
        if body.is_empty() || body.starts_with('#') {
            continue;
        }
        if indent == 0 {
            in_importers = body == "importers:";
            table = false;
            package = None;
            importer = None;
            if matches!(
                body,
                "dependencies:" | "devDependencies:" | "optionalDependencies:"
            ) {
                // A lock file of one project has no `importers`.
                importer = Some(".".to_string());
                table = true;
            }
            if body.starts_with("packages:") || body.starts_with("snapshots:") {
                break;
            }
            continue;
        }
        let (importer_indent, table_indent, package_indent) = if in_importers {
            (2, 4, 6)
        } else {
            (usize::MAX, 0, 2)
        };
        if in_importers && indent == importer_indent {
            importer = Some(key(body.trim_end_matches(':')));
            table = false;
            package = None;
        } else if indent == table_indent && importer.is_some() {
            table = matches!(
                body,
                "dependencies:" | "devDependencies:" | "optionalDependencies:"
            );
            package = None;
        } else if table && indent == package_indent {
            // `react:` with its `specifier` and `version` below, or the
            // short form `react: 18.2.0` of an older file.
            match body.split_once(':') {
                Some((name, rest)) if rest.trim().is_empty() => package = Some(key(name)),
                Some((name, rest)) => {
                    package = None;
                    if let Some(importer) = &importer {
                        out.entry(importer.clone())
                            .or_default()
                            .insert(key(name), pnpm_version(rest));
                    }
                }
                None => {}
            }
        } else if table
            && indent == package_indent + 2
            && let Some(value) = body.strip_prefix("version:")
            && let (Some(importer), Some(package)) = (&importer, &package)
        {
            out.entry(importer.clone())
                .or_default()
                .insert(package.clone(), pnpm_version(value));
        }
    }
    Some(out)
}

/// `18.2.0(react@18.2.0)` and `/react@18.2.0` are the version `18.2.0`.
fn pnpm_version(raw: &str) -> String {
    let raw = raw.trim().trim_matches(['\'', '"']);
    let raw = raw.split('(').next().unwrap_or(raw);
    raw.rsplit_once('@')
        .map_or(raw, |(_, version)| version)
        .to_string()
}

/// Yarn 1 (`# yarn lockfile v1`) and Berry (`__metadata:` with a version of
/// 4 to 8): each entry lists `name@range` keys and holds a `version`.
fn yarn_lock(text: &str) -> Option<BTreeMap<String, String>> {
    let berry = text.contains("__metadata:");
    if !berry && !text.contains("# yarn lockfile v1") {
        return None;
    }
    if berry {
        let version = text
            .lines()
            .skip_while(|l| l.trim() != "__metadata:")
            .nth(1)
            .and_then(|l| l.trim().strip_prefix("version:"))
            .and_then(|v| v.trim().parse::<u32>().ok())?;
        if !(4..=9).contains(&version) {
            return None;
        }
    }
    let mut out = BTreeMap::new();
    let mut keys: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(' ') && line.trim_end().ends_with(':') {
            keys = line
                .trim_end()
                .trim_end_matches(':')
                .split(',')
                .map(|k| k.trim().trim_matches('"').to_string())
                .collect();
            continue;
        }
        let body = line.trim();
        let version = if berry {
            body.strip_prefix("version:")
        } else {
            body.strip_prefix("version ")
        };
        if line.starts_with("  ")
            && !line.starts_with("   ")
            && let Some(version) = version
        {
            let version = version.trim().trim_matches('"').to_string();
            for key in keys.drain(..) {
                if key != "__metadata" {
                    out.insert(key, version.clone());
                }
            }
        }
    }
    Some(out)
}

/// `bun.lock`: JSONC with `lockfileVersion` and `packages`, each package an
/// array whose first item is `name@version`.
fn bun_lock(text: &str) -> Option<BTreeMap<String, String>> {
    let value = parse_jsonc(text)?;
    let version = value.get("lockfileVersion")?.as_u64()?;
    if version > 1 {
        return None;
    }
    let mut out = BTreeMap::new();
    for (key, entry) in value.get("packages")?.as_object()? {
        let Some(first) = entry
            .as_array()
            .and_then(|a| a.first())
            .and_then(Value::as_str)
        else {
            continue;
        };
        // A workspace package is `name@workspace:folder`.
        if let Some((_, version)) = first[first.char_indices().nth(1).map_or(0, |(i, _)| i)..]
            .rsplit_once('@')
            .map(|(a, b)| (a, b.to_string()))
            && !version.starts_with("workspace:")
        {
            out.insert(key.clone(), version);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reader<'a>(files: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |path| {
            files
                .iter()
                .find(|(p, _)| *p == path)
                .map(|(_, t)| t.to_string())
        }
    }

    #[test]
    fn jsonc_loses_comments_and_trailing_commas_but_not_strings() {
        let text =
            "{\n // line\n \"a\": \"x // not a comment\", /* block */\n \"b\": [1, 2,],\n}\n";
        let value = parse_jsonc(text).unwrap();
        assert_eq!(value["a"], "x // not a comment");
        assert_eq!(value["b"], serde_json::json!([1, 2]));
        assert!(parse_jsonc("\u{feff}{\"a\": 1}").is_some());
        assert!(parse_jsonc("{\"a\": ").is_none());
    }

    #[test]
    fn paths_pick_the_longest_prefix_and_try_every_target() {
        let files = [(
            "web/tsconfig.json",
            r#"{ "compilerOptions": { "baseUrl": ".", "paths": {
                "@/*": ["src/*"], "@/ui/*": ["lib/ui/*", "src/ui/*"], "~config": ["config/index.ts"] } } }"#,
        )];
        let config = read_tsconfig("web/tsconfig.json", &reader(&files)).unwrap();
        let hit = config.lookup("@/store/cart");
        assert!(hit.alias);
        assert_eq!(hit.candidates[0], "web/src/store/cart");
        let ui = config.lookup("@/ui/Button");
        assert_eq!(
            ui.candidates[..2],
            ["web/lib/ui/Button", "web/src/ui/Button"]
        );
        assert_eq!(
            config.lookup("~config").candidates[0],
            "web/config/index.ts"
        );
        // `baseUrl` makes a bare specifier a path of the folder.
        let bare = config.lookup("utils/text");
        assert!(!bare.alias);
        assert_eq!(bare.candidates, ["web/utils/text"]);
        assert!(config.lookup("./local").candidates.is_empty());
    }

    #[test]
    fn extends_follows_files_and_packages_and_skips_what_is_missing() {
        let files = [
            (
                "app/tsconfig.json",
                r#"{ "extends": ["@tsconfig/node20/tsconfig.json", "../tsconfig.base", "@missing/preset"],
                     "compilerOptions": { "paths": { "@app/*": ["./src/*"] } } }"#,
            ),
            (
                "tsconfig.base.json",
                r#"{ "compilerOptions": { "baseUrl": "." } }"#,
            ),
            (
                "node_modules/@tsconfig/node20/tsconfig.json",
                r#"{ "compilerOptions": { "paths": { "x/*": ["y/*"] } } }"#,
            ),
        ];
        let config = read_tsconfig("app/tsconfig.json", &reader(&files)).unwrap();
        // The file's own `paths` replace the inherited ones, as TypeScript does.
        assert_eq!(config.paths.len(), 1);
        assert_eq!(config.base_url.as_deref(), Some(""));
        // `baseUrl` from the parent (the repository root) is what targets follow.
        assert_eq!(config.lookup("@app/a").candidates[0], "src/a");
        // Without a `baseUrl`, targets are relative to the config that wrote them.
        let own = [
            ("pkg/tsconfig.json", r#"{ "extends": "./base.json" }"#),
            (
                "pkg/base.json",
                r#"{ "compilerOptions": { "paths": { "@/*": ["src/*"] } } }"#,
            ),
        ];
        let config = read_tsconfig("pkg/tsconfig.json", &reader(&own)).unwrap();
        assert_eq!(config.lookup("@/a").candidates, ["pkg/src/a"]);
    }

    #[test]
    fn the_nearest_config_applies_and_references_change_nothing() {
        let tsconfigs = [
            (
                "tsconfig.json",
                r#"{ "references": [{ "path": "./packages/a" }], "compilerOptions": { "paths": { "@/*": ["root/*"] } } }"#,
            ),
            (
                "packages/a/tsconfig.json",
                r#"{ "compilerOptions": { "paths": { "@/*": ["src/*"] } } }"#,
            ),
        ];
        let paths: Vec<String> = tsconfigs.iter().map(|(p, _)| p.to_string()).collect();
        let repo = JsRepo::read(&paths, &reader(&tsconfigs));
        assert_eq!(
            repo.config_for("packages/a/src/x.ts").unwrap().dir,
            "packages/a"
        );
        assert_eq!(repo.config_for("scripts/y.ts").unwrap().dir, "");
        assert!(
            repo.config_for("packages/a/src/x.ts")
                .unwrap()
                .lookup("@/q")
                .candidates[0]
                .starts_with("packages/a/src")
        );
    }

    #[test]
    fn a_workspace_lists_its_members_and_names_their_entries() {
        let files = [
            (
                "package.json",
                r#"{ "name": "root", "workspaces": ["packages/*", "!packages/old"] }"#,
            ),
            (
                "packages/ui/package.json",
                r#"{ "name": "@acme/ui", "exports": { ".": { "types": "./dist/index.d.ts", "import": "./dist/index.js" }, "./button": "./dist/button.js" } }"#,
            ),
            ("packages/ui/src/index.ts", ""),
            ("packages/ui/src/button.ts", ""),
            ("packages/old/package.json", r#"{ "name": "old" }"#),
            (
                "packages/web/package.json",
                r#"{ "name": "web", "main": "src/main.ts" }"#,
            ),
            ("packages/web/src/main.ts", ""),
        ];
        let paths: Vec<String> = files.iter().map(|(p, _)| p.to_string()).collect();
        let repo = JsRepo::read(&paths, &reader(&files));
        let names: Vec<&str> = repo
            .members()
            .iter()
            .map(|(p, _)| p.name.as_str())
            .collect();
        assert_eq!(names, ["@acme/ui", "web"]);
        let set: HashSet<&str> = paths.iter().map(String::as_str).collect();
        assert_eq!(
            repo.resolve_member(&set, "@acme/ui").as_deref(),
            Some("packages/ui/src/index.ts")
        );
        assert_eq!(
            repo.resolve_member(&set, "@acme/ui/button").as_deref(),
            Some("packages/ui/src/button.ts")
        );
        assert_eq!(
            repo.resolve_member(&set, "web").as_deref(),
            Some("packages/web/src/main.ts")
        );
        assert_eq!(repo.resolve_member(&set, "old"), None);
        assert_eq!(repo.resolve_member(&set, "@acme/uix"), None);
        // `workspaces` as an object, and no workspace without a list.
        let object = [
            (
                "package.json",
                r#"{ "workspaces": { "packages": ["apps/*"] } }"#,
            ),
            ("apps/a/package.json", r#"{ "name": "a" }"#),
        ];
        let paths: Vec<String> = object.iter().map(|(p, _)| p.to_string()).collect();
        let repo = JsRepo::read(&paths, &reader(&object));
        assert_eq!(repo.members().len(), 1);
    }

    #[test]
    fn pnpm_workspace_yaml_lists_globs_in_block_and_flow_form() {
        let block = "# comment\npackages:\n  - 'packages/*'\n  - \"apps/**\"  # all apps\n  - '!**/test/**'\n\ncatalog:\n  react: ^18\n";
        assert_eq!(
            parse_pnpm_workspace(block),
            ["packages/*", "apps/**", "!**/test/**"]
        );
        assert_eq!(
            parse_pnpm_workspace("packages: ['a', \"b/*\"]\n"),
            ["a", "b/*"]
        );
        assert!(parse_pnpm_workspace("catalog:\n  - x\n").is_empty());
        assert!(glob_matches("packages/*", "packages/ui"));
        assert!(!glob_matches("packages/*", "packages/ui/deep"));
        assert!(glob_matches("apps/**", "apps/a/b"));
        assert!(glob_matches("**/test/**", "x/test/y"));
    }

    #[test]
    fn the_package_manager_comes_from_the_field_then_the_lock_then_npm() {
        let files = [
            (
                "package.json",
                r#"{ "name": "root", "packageManager": "pnpm@9.1.0", "workspaces": ["packages/*"] }"#,
            ),
            ("pnpm-lock.yaml", "lockfileVersion: '9.0'\n"),
            ("packages/a/package.json", r#"{ "name": "a" }"#),
            ("other/package.json", r#"{ "name": "o" }"#),
            ("other/yarn.lock", "# yarn lockfile v1\n"),
            ("berry/package.json", r#"{ "name": "b" }"#),
            ("berry/yarn.lock", "__metadata:\n  version: 8\n"),
            (
                "bun/package.json",
                r#"{ "name": "u", "packageManager": "bun@1.1.0" }"#,
            ),
            ("bun/bun.lockb", ""),
            ("plain/package.json", r#"{ "name": "p" }"#),
        ];
        let paths: Vec<String> = files.iter().map(|(p, _)| p.to_string()).collect();
        let repo = JsRepo::read(&paths, &reader(&files));
        let a = repo.manager_of("packages/a");
        assert_eq!(
            (a.manager, a.root.as_str(), a.member.as_deref()),
            (Manager::Pnpm, "", Some("a"))
        );
        assert_eq!(a.lock.as_deref(), Some("pnpm-lock.yaml"));
        assert_eq!(a.install(), "pnpm install --frozen-lockfile");
        // The field names the manager of every folder below it, even where a
        // lock file of another says otherwise.
        let other = repo.manager_of("other");
        assert_eq!(other.manager, Manager::Pnpm);
        let classic_files = [
            ("package.json", "{}"),
            ("yarn.lock", "# yarn lockfile v1\n"),
        ];
        let classic_paths: Vec<String> = classic_files.iter().map(|(p, _)| p.to_string()).collect();
        let classic = JsRepo::read(&classic_paths, &reader(&classic_files)).manager_of("");
        assert_eq!((classic.manager, classic.berry), (Manager::Yarn, false));
        assert_eq!(classic.install(), "yarn install --frozen-lockfile");
        let berry_files = [
            ("package.json", "{}"),
            ("yarn.lock", "__metadata:\n  version: 8\n"),
        ];
        let berry_paths: Vec<String> = berry_files.iter().map(|(p, _)| p.to_string()).collect();
        let berry = JsRepo::read(&berry_paths, &reader(&berry_files)).manager_of("");
        assert!(berry.berry);
        assert_eq!(berry.install(), "yarn install --immutable");
        // `bun.lockb` is binary: it names bun and is not read.
        let bun = repo.manager_of("bun");
        assert_eq!((bun.manager, bun.lock.clone()), (Manager::Bun, None));
        assert_eq!(bun.install(), "bun install");
        let plain = repo.manager_of("plain");
        assert_eq!(plain.manager, Manager::Pnpm); // the root field reaches it
        let none = JsRepo::read(
            &["package.json".to_string()],
            &reader(&[("package.json", "{}")]),
        );
        let m = none.manager_of("");
        assert_eq!((m.manager, m.lock.clone()), (Manager::Npm, None));
        assert_eq!(m.install(), "npm install");
    }

    #[test]
    fn lock_files_give_the_version_pinned() {
        let npm = r#"{ "lockfileVersion": 3, "packages": {
            "": { "name": "root" },
            "node_modules/react": { "version": "18.2.0" },
            "packages/ui/node_modules/react": { "version": "17.0.2" } } }"#;
        let lock = JsLock::read("package-lock.json", npm).unwrap();
        assert_eq!(
            lock.version("", "", "react", "^18").as_deref(),
            Some("18.2.0")
        );
        assert_eq!(
            lock.version("", "packages/ui", "react", "^17").as_deref(),
            Some("17.0.2")
        );
        assert!(JsLock::read("package-lock.json", r#"{ "lockfileVersion": 9 }"#).is_none());

        let pnpm = "lockfileVersion: '9.0'\n\nimporters:\n\n  .:\n    dependencies:\n      react:\n        specifier: ^18.2.0\n        version: 18.2.0\n    devDependencies:\n      vitest:\n        specifier: ^1.6.0\n        version: 1.6.0(@types/node@20.1.0)\n\n  packages/ui:\n    dependencies:\n      '@acme/core':\n        specifier: workspace:*\n        version: link:../core\n      lodash:\n        specifier: ^4\n        version: 4.17.21\n\npackages:\n\n  react@18.2.0:\n    resolution: {integrity: x}\n";
        let lock = JsLock::read("pnpm-lock.yaml", pnpm).unwrap();
        assert_eq!(lock.version("", "", "react", "").as_deref(), Some("18.2.0"));
        assert_eq!(lock.version("", "", "vitest", "").as_deref(), Some("1.6.0"));
        assert_eq!(
            lock.version("", "packages/ui", "lodash", "").as_deref(),
            Some("4.17.21")
        );
        assert_eq!(lock.version("", "", "lodash", ""), None);
        assert!(JsLock::read("pnpm-lock.yaml", "lockfileVersion: 5.4\n").is_none());
        let single = "lockfileVersion: '6.0'\n\ndependencies:\n  react:\n    specifier: ^18\n    version: 18.2.0\n\npackages:\n";
        let lock = JsLock::read("pnpm-lock.yaml", single).unwrap();
        assert_eq!(lock.version("", "", "react", "").as_deref(), Some("18.2.0"));

        let v1 = "# yarn lockfile v1\n\n\"@scope/pkg@^1.0.0\", \"@scope/pkg@^1.1.0\":\n  version \"1.2.3\"\n  resolved \"x\"\n\nreact@^18.0.0:\n  version \"18.2.0\"\n";
        let lock = JsLock::read("yarn.lock", v1).unwrap();
        assert_eq!(
            lock.version("", "", "@scope/pkg", "^1.1.0").as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            lock.version("", "", "react", "^18.0.0").as_deref(),
            Some("18.2.0")
        );
        let berry = "# This file is generated by running \"yarn install\" inside your project.\n\n__metadata:\n  version: 8\n  cacheKey: 10c0\n\n\"react@npm:^18.2.0\":\n  version: 18.2.0\n  resolution: \"react@npm:18.2.0\"\n";
        let lock = JsLock::read("yarn.lock", berry).unwrap();
        assert_eq!(
            lock.version("", "", "react", "^18.2.0").as_deref(),
            Some("18.2.0")
        );
        assert!(JsLock::read("yarn.lock", "__metadata:\n  version: 1\n").is_none());

        let bun = "{\n  \"lockfileVersion\": 1,\n  \"workspaces\": { \"\": { \"name\": \"root\" }, },\n  \"packages\": {\n    \"react\": [\"react@18.2.0\", \"\", {}, \"sha512-x\"],\n    \"@types/node\": [\"@types/node@20.1.0\", \"\", {}, \"sha\"],\n    \"ui\": [\"ui@workspace:packages/ui\"],\n  }\n}\n";
        let lock = JsLock::read("bun.lock", bun).unwrap();
        assert_eq!(lock.version("", "", "react", "").as_deref(), Some("18.2.0"));
        assert_eq!(
            lock.version("", "", "@types/node", "").as_deref(),
            Some("20.1.0")
        );
        assert_eq!(lock.version("", "", "ui", ""), None);
        assert!(JsLock::read("bun.lockb", "").is_none());
        assert!(JsLock::read("bun.lock", "{ \"lockfileVersion\": 7 }").is_none());
    }
}
