//! The file-level map: units and folders for source files of a language
//! CodeWiki has no parser for.
//!
//! A file counts when [`Language::file_level_name`] names its language. Such a
//! file has a folder, a language and a size in lines, and nothing else is
//! known about it: no symbols, no imports, no calls. What is said about it
//! is therefore always correct, whatever the language. A language that gets
//! a parser leaves this map, because [`Language::file_level_name`] stops
//! naming it; no list of languages lives here.
//!
//! # Units
//!
//! A unit is the counterpart of a crate or a package, and has a note of its
//! own under `01-Core`. The rule, from the paths alone:
//!
//! - A file at `<container>/<project>/…`, where `<container>` is one of
//!   [`CONTAINERS`] (`src`, `tests`, `apps`, `packages`, …), belongs to the
//!   unit `<container>/<project>`. A layout of `src/<Project>/` and
//!   `tests/<Project>/` therefore gives one unit per project.
//! - Every other file belongs to the root unit, the repository itself. A
//!   flat repository is one unit.
//! - A unit of fewer than [`MIN_UNIT_FILES`] files is folded into the root
//!   unit, and beside mapped code (a repository with parsed units) the root
//!   unit must hold that many files too, so that a few scripts or style
//!   sheets beside a Rust workspace get no notes. A unit an agent explained
//!   is never dropped while it has a file.
//!
//! # Folders
//!
//! A folder gets a note when the files it holds, its own and those of the
//! folders below it that have no note, number at least
//! [`MIN_FOLDER_FILES`]. A smaller folder rolls up: its files are listed in
//! the nearest folder above it that has a note, or in the unit's note. At
//! most [`MAX_FOLDER_NOTES`] folders get a note, the largest in lines first;
//! the others roll up the same way. The folder of a note an agent explained
//! is never cut while it has a file.
//!
//! Ids are `unit:<name>` and `dir:<repo-relative folder>`. A folder's id
//! names the place, never its neighbours, so no other file changes it.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use cw_core::Language;
use cw_parse::RepoFacts;

use crate::plan::{ZONE_CORE, cut, title_case};
use crate::render::{count, join_and};
use crate::units::slug;

/// Fewer files than this and a unit folds into the repository's own.
pub const MIN_UNIT_FILES: usize = 3;
/// Fewer files than this and a folder rolls up into the one above it.
pub const MIN_FOLDER_FILES: usize = 3;
/// The most folders that get a note, as many as a module ranking allows.
pub const MAX_FOLDER_NOTES: usize = 300;
/// Source files a note cites.
const MAX_SOURCES: usize = 10;
/// Source files a unit that holds no file itself cites.
const MAX_UNIT_SOURCES: usize = 5;
/// Files named in a note's list; the rest are counted.
const MAX_FILES_LISTED: usize = 40;
/// Languages named in a sentence before "and N other languages".
const MAX_LANGUAGES_NAMED: usize = 3;
/// Languages named in the home note's sentence about parsers.
const MAX_LANGUAGES_ALL: usize = 6;
/// Languages counted one by one in a note's totals.
const MAX_LANGUAGES_LISTED: usize = 5;
/// Folders that hold the projects of a repository, which are one unit each.
const CONTAINERS: [&str; 16] = [
    "src", "source", "sources", "lib", "libs", "app", "apps", "packages", "projects", "modules",
    "services", "plugins", "tests", "test", "samples", "examples",
];
const ROOT_KEY: &str = "root-files";

/// A source file of a language without a parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Repo-relative path.
    pub path: String,
    /// Display name of the language, from the extension.
    pub language: &'static str,
    pub lines: u32,
    pub bytes: u64,
}

impl SourceFile {
    /// `120 lines`, `1 line`, `empty`, or the size in bytes when the file
    /// was not read as text.
    pub fn size(&self) -> String {
        if self.lines > 0 {
            count(self.lines as usize, "line")
        } else if self.bytes == 0 {
            "empty".to_string()
        } else {
            format!("{} bytes", self.bytes)
        }
    }
}

/// Files, lines, folders and languages of a set of source files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tally {
    pub files: usize,
    pub lines: u64,
    folders: BTreeSet<String>,
    languages: BTreeMap<&'static str, usize>,
}

impl Tally {
    fn add(&mut self, file: &SourceFile) {
        self.files += 1;
        self.lines += u64::from(file.lines);
        self.folders.insert(parent_dir(&file.path).to_string());
        *self.languages.entry(file.language).or_insert(0) += 1;
    }

    /// Folders that hold a file.
    pub fn folders(&self) -> usize {
        self.folders.len()
    }

    /// Languages with their file counts, the largest first.
    pub fn languages(&self) -> Vec<(&'static str, usize)> {
        let mut ranked: Vec<(&'static str, usize)> =
            self.languages.iter().map(|(l, n)| (*l, *n)).collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        ranked
    }

    /// `C#`, `C# and AXAML`, `Go, SQL, Shell, and 2 other languages`.
    pub fn language_names(&self) -> String {
        self.names_up_to(MAX_LANGUAGES_NAMED)
    }

    /// The names of up to `limit` languages, the rest counted.
    fn names_up_to(&self, limit: usize) -> String {
        let ranked = self.languages();
        let mut names: Vec<String> = ranked
            .iter()
            .take(limit)
            .map(|(l, _)| l.to_string())
            .collect();
        if ranked.len() > limit {
            names.push(count(ranked.len() - limit, "other language"));
        }
        join_and(&names)
    }
}

/// The unit of a repository that is not mapped by a parser.
#[derive(Debug, Clone)]
pub struct FileUnit {
    /// Stable key; the note's id is `unit:<name>`.
    pub name: String,
    /// Name shown to readers: the project folder, or the repository.
    pub package: String,
    /// Folder under `01-Core`.
    pub slug: String,
    /// Repo-relative folder, empty for the repository root.
    pub dir: String,
    /// Files no folder note holds.
    pub files: Vec<SourceFile>,
    /// Folders with a note directly in the unit.
    pub folders: Vec<usize>,
    pub total: Tally,
    /// The files the note cites.
    pub sources: Vec<SourceFile>,
}

/// A folder with a note.
#[derive(Debug, Clone)]
pub struct Folder {
    /// Repo-relative path; the note's id is `dir:<dir>`.
    pub dir: String,
    /// Title of the note, before the vault makes it unique.
    pub title: String,
    /// Vault folder of the note.
    pub vault_dir: String,
    /// Index into [`FileMap::units`].
    pub unit: usize,
    /// The nearest folder above with a note.
    pub parent: Option<usize>,
    /// Folders below whose nearest note above is this one.
    pub children: Vec<usize>,
    /// Files of the folder and of the folders below it that have no note.
    pub files: Vec<SourceFile>,
    /// Everything below, the folders with a note included.
    pub total: Tally,
    /// The files the note cites.
    pub sources: Vec<SourceFile>,
}

/// Every unit and folder with a note, and the totals of the repository.
#[derive(Debug, Clone, Default)]
pub struct FileMap {
    pub units: Vec<FileUnit>,
    pub folders: Vec<Folder>,
    /// Every file of a unit.
    pub total: Tally,
}

impl FileMap {
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    pub fn unit(&self, name: &str) -> Option<&FileUnit> {
        self.units.iter().find(|u| u.name == name)
    }

    pub fn folder(&self, dir: &str) -> Option<&Folder> {
        self.folders.iter().find(|f| f.dir == dir)
    }

    /// The sentence the home note opens with, for a repository whose only
    /// code is mapped at file level.
    pub fn home_summary(&self, repo_name: &str, tail: &str) -> String {
        // The headline names the two main languages, the parser sentence all
        // the languages there are, up to a limit.
        let headline = join_and(
            &self
                .total
                .languages()
                .iter()
                .take(2)
                .map(|(l, _)| l.to_string())
                .collect::<Vec<_>>(),
        );
        let names = self.total.names_up_to(MAX_LANGUAGES_ALL);
        let first = self
            .total
            .languages()
            .first()
            .map_or("", |(language, _)| *language);
        let article = if first.starts_with(['A', 'E', 'I', 'O', 'U'])
            || matches!(first, "F#" | "R" | "SQL" | "HTML")
        {
            "an"
        } else {
            "a"
        };
        format!(
            "**{repo_name}** is {article} {headline} codebase of {} and {}, mapped at file level: CodeWiki has no parser for {names} yet. {tail}",
            count(self.total.folders(), "folder"),
            count(self.total.files, "source file"),
        )
    }

    /// The sentence a mixed repository's home note gains.
    pub fn mixed_sentence(&self) -> String {
        let files = self.total.files;
        format!(
            "{} in {} {} mapped at file level: CodeWiki has no parser for them yet.",
            count(files, "source file"),
            self.total.names_up_to(MAX_LANGUAGES_ALL),
            if files == 1 { "is" } else { "are" },
        )
    }
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// The folder above `dir`; `None` for the repository root.
fn parent_of(dir: &str) -> Option<&str> {
    match dir.rsplit_once('/') {
        Some((parent, _)) => Some(parent),
        None if dir.is_empty() => None,
        None => Some(""),
    }
}

fn depth(dir: &str) -> usize {
    if dir.is_empty() {
        0
    } else {
        dir.matches('/').count() + 1
    }
}

/// The unit folder of a file: `src/App` for `src/App/Views/Main.axaml`.
fn unit_dir(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() >= 3 && CONTAINERS.contains(&segments[0].to_ascii_lowercase().as_str()) {
        format!("{}/{}", segments[0], segments[1])
    } else {
        String::new()
    }
}

fn unique(base: String, used: &mut HashSet<String>) -> String {
    let mut candidate = base.clone();
    let mut n = 2;
    while !used.insert(candidate.to_lowercase()) {
        candidate = format!("{base}-{n}");
        n += 1;
    }
    candidate
}

/// A folder that wants a note, before the limit.
struct Candidate {
    unit: usize,
    dir: String,
    lines: u64,
}

/// The folders of one unit that want a note: those that hold enough files,
/// counting what rolls up into them, and the pinned ones that hold any.
fn candidates(
    unit: usize,
    root: &str,
    files: &[SourceFile],
    pinned: &HashSet<&str>,
) -> Vec<Candidate> {
    let mut own: BTreeMap<&str, Vec<&SourceFile>> = BTreeMap::new();
    for file in files {
        own.entry(parent_dir(&file.path)).or_default().push(file);
    }
    let mut dirs: BTreeSet<&str> = BTreeSet::new();
    for dir in own.keys() {
        let mut current = *dir;
        loop {
            dirs.insert(current);
            if current == root {
                break;
            }
            match parent_of(current) {
                Some(parent) => current = parent,
                None => break,
            }
        }
    }
    // Deepest first, so that what rolls up has arrived when its folder is
    // judged.
    let mut order: Vec<&str> = dirs.into_iter().collect();
    order.sort_by(|a, b| depth(b).cmp(&depth(a)).then(a.cmp(b)));
    let mut below: HashMap<&str, usize> = HashMap::new();
    let mut carry: HashMap<&str, Vec<&SourceFile>> = HashMap::new();
    let mut found = Vec::new();
    for dir in order {
        let mut here: Vec<&SourceFile> = own.get(dir).cloned().unwrap_or_default();
        here.extend(carry.remove(dir).unwrap_or_default());
        let subtree = below.get(dir).copied().unwrap_or(0) + own.get(dir).map_or(0, Vec::len);
        if dir == root {
            continue;
        }
        let parent = parent_of(dir).unwrap_or("");
        *below.entry(parent).or_insert(0) += subtree;
        if here.len() >= MIN_FOLDER_FILES || (subtree > 0 && pinned.contains(dir)) {
            found.push(Candidate {
                unit,
                dir: dir.to_string(),
                lines: here.iter().map(|f| u64::from(f.lines)).sum(),
            });
        } else {
            carry.entry(parent).or_default().extend(here);
        }
    }
    found
}

/// The folder with a note that holds a file of `file_dir`, nearest first;
/// `None` when the unit's own note does.
fn holder<'a>(noted: &'a BTreeMap<String, usize>, root: &str, file_dir: &str) -> Option<&'a str> {
    let mut current = file_dir;
    loop {
        if current == root {
            return None;
        }
        if let Some((dir, _)) = noted.get_key_value(current) {
            return Some(dir);
        }
        current = parent_of(current)?;
    }
}

/// The largest of `files`, by lines then path.
fn largest(files: &[SourceFile], limit: usize) -> Vec<SourceFile> {
    let mut ranked: Vec<SourceFile> = files.to_vec();
    ranked.sort_by(|a, b| b.lines.cmp(&a.lines).then(a.path.cmp(&b.path)));
    ranked.truncate(limit);
    ranked
}

/// Build the file-level map of a repository.
///
/// `taken_slugs` are the lowercase folders under `01-Core` that parsed units
/// already use; `has_mapped` says whether the repository has such units;
/// `pinned` holds the ids of the notes an agent explained.
pub fn build(
    facts: &RepoFacts,
    repo_name: &str,
    taken_slugs: &HashSet<String>,
    has_mapped: bool,
    pinned: &HashSet<String>,
) -> FileMap {
    let mut files: Vec<SourceFile> = facts
        .files
        .iter()
        .filter_map(|f| {
            Some(SourceFile {
                path: f.path.clone(),
                language: Language::file_level_name(&f.path)?,
                lines: f.lines,
                bytes: f.bytes,
            })
        })
        .collect();
    if files.is_empty() {
        return FileMap::default();
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let mut groups: BTreeMap<String, Vec<SourceFile>> = BTreeMap::new();
    for file in files {
        groups.entry(unit_dir(&file.path)).or_default().push(file);
    }
    let key_of = |dir: &str| {
        if dir.is_empty() {
            ROOT_KEY.to_string()
        } else {
            slug(dir)
        }
    };
    let is_pinned = |dir: &str| pinned.contains(&format!("unit:{}", key_of(dir)));
    let small: Vec<String> = groups
        .iter()
        .filter(|(dir, files)| {
            !dir.is_empty() && files.len() < MIN_UNIT_FILES && !is_pinned(dir.as_str())
        })
        .map(|(dir, _)| dir.clone())
        .collect();
    for dir in small {
        if let Some(files) = groups.remove(&dir) {
            groups.entry(String::new()).or_default().extend(files);
        }
    }
    if let Some(root) = groups.get_mut("") {
        root.sort_by(|a, b| a.path.cmp(&b.path));
        if has_mapped && root.len() < MIN_UNIT_FILES && !is_pinned("") {
            groups.remove("");
        }
    }

    let pinned_dirs: HashSet<&str> = pinned
        .iter()
        .filter_map(|id| id.strip_prefix("dir:"))
        .collect();
    let mut names: HashSet<String> = HashSet::new();
    let mut slugs: HashSet<String> = taken_slugs.clone();
    let mut units: Vec<FileUnit> = Vec::new();
    let mut unit_files: Vec<Vec<SourceFile>> = Vec::new();
    for (dir, files) in groups {
        let package = if dir.is_empty() {
            repo_name.to_string()
        } else {
            dir.rsplit('/').next().unwrap_or(&dir).to_string()
        };
        let name = unique(key_of(&dir), &mut names);
        let choices = [
            slug(&package),
            if dir.is_empty() {
                slug(repo_name)
            } else {
                slug(&dir)
            },
        ];
        let folder = match choices.iter().find(|s| !slugs.contains(*s)) {
            Some(free) => {
                slugs.insert(free.clone());
                free.clone()
            }
            None => unique(format!("{}-files", choices[0]), &mut slugs),
        };
        units.push(FileUnit {
            name,
            package,
            slug: folder,
            dir,
            files: Vec::new(),
            folders: Vec::new(),
            total: Tally::default(),
            sources: Vec::new(),
        });
        unit_files.push(files);
    }

    // Which folders want a note, then which of them the limit keeps.
    let mut wanted: Vec<Candidate> = Vec::new();
    for (index, (unit, files)) in units.iter().zip(&unit_files).enumerate() {
        wanted.extend(candidates(index, &unit.dir, files, &pinned_dirs));
    }
    wanted.sort_by(|a, b| b.lines.cmp(&a.lines).then(a.dir.cmp(&b.dir)));
    cut(&mut wanted, MAX_FOLDER_NOTES, |c| {
        pinned_dirs.contains(c.dir.as_str())
    });
    let noted: BTreeMap<String, usize> = wanted.into_iter().map(|c| (c.dir, c.unit)).collect();

    let mut folders: Vec<Folder> = Vec::new();
    let mut index_of: HashMap<&str, usize> = HashMap::new();
    for (dir, unit) in &noted {
        let unit_dir = units[*unit].dir.as_str();
        let relative = dir
            .strip_prefix(unit_dir)
            .map_or(dir.as_str(), |rest| rest.trim_start_matches('/'));
        let mut vault_dir = format!("{ZONE_CORE}/{}", units[*unit].slug);
        for segment in relative.split('/') {
            vault_dir.push('/');
            vault_dir.push_str(&slug(segment));
        }
        let name = dir.rsplit('/').next().unwrap_or(dir);
        index_of.insert(dir.as_str(), folders.len());
        folders.push(Folder {
            dir: dir.clone(),
            title: format!("{} Folder", title_case(name)),
            vault_dir,
            unit: *unit,
            parent: None,
            children: Vec::new(),
            files: Vec::new(),
            total: Tally::default(),
            sources: Vec::new(),
        });
    }
    // Folders of one name in several places are told apart by their unit,
    // which the vault would otherwise do by the folder's own name.
    let mut named: HashMap<String, usize> = HashMap::new();
    for folder in &folders {
        *named.entry(folder.title.clone()).or_insert(0) += 1;
    }
    for folder in &mut folders {
        if named[&folder.title] > 1 {
            let unit = &units[folder.unit];
            // The folder it sits in, or the unit when it sits in the unit's.
            let above = parent_of(&folder.dir)
                .filter(|parent| *parent != unit.dir)
                .map(|parent| parent.rsplit('/').next().unwrap_or(parent));
            folder.title = format!("{} ({})", folder.title, above.unwrap_or(&unit.package));
        }
    }
    for index in 0..folders.len() {
        let unit_dir = units[folders[index].unit].dir.clone();
        let above = parent_of(&folders[index].dir).and_then(|p| holder(&noted, &unit_dir, p));
        if let Some(parent) = above.and_then(|d| index_of.get(d).copied()) {
            folders[index].parent = Some(parent);
            folders[parent].children.push(index);
        } else {
            units[folders[index].unit].folders.push(index);
        }
    }

    let mut total = Tally::default();
    for (unit_index, files) in unit_files.iter().enumerate() {
        for file in files {
            total.add(file);
            units[unit_index].total.add(file);
            let unit_dir = units[unit_index].dir.clone();
            let mut at = holder(&noted, &unit_dir, parent_dir(&file.path))
                .and_then(|d| index_of.get(d).copied());
            match at {
                Some(index) => folders[index].files.push(file.clone()),
                None => units[unit_index].files.push(file.clone()),
            }
            while let Some(index) = at {
                folders[index].total.add(file);
                at = folders[index].parent;
            }
        }
        let own = units[unit_index].files.clone();
        units[unit_index].sources = if own.is_empty() {
            largest(files, MAX_UNIT_SOURCES)
        } else {
            largest(&own, MAX_SOURCES)
        };
    }
    for folder in &mut folders {
        folder.sources = largest(&folder.files, MAX_SOURCES);
    }
    FileMap {
        units,
        folders,
        total,
    }
}

/// `- `path` — C#, 120 lines`, the paths relative to `base`.
fn file_lines(files: &[SourceFile], base: &str) -> String {
    let mut listed: Vec<&SourceFile> = files.iter().collect();
    let more = listed.len().saturating_sub(MAX_FILES_LISTED);
    if more > 0 {
        listed.sort_by(|a, b| b.lines.cmp(&a.lines).then(a.path.cmp(&b.path)));
        listed.truncate(MAX_FILES_LISTED);
    }
    listed.sort_by(|a, b| a.path.cmp(&b.path));
    let mut out: Vec<String> = listed
        .iter()
        .map(|f| {
            let shown = if base.is_empty() {
                f.path.as_str()
            } else {
                f.path
                    .strip_prefix(base)
                    .map_or(f.path.as_str(), |rest| rest.trim_start_matches('/'))
            };
            format!("- `{shown}` — {}, {}", f.language, f.size())
        })
        .collect();
    if more > 0 {
        let rest: Vec<&SourceFile> = {
            let kept: HashSet<&str> = listed.iter().map(|f| f.path.as_str()).collect();
            files
                .iter()
                .filter(|f| !kept.contains(f.path.as_str()))
                .collect()
        };
        let lines: u64 = rest.iter().map(|f| u64::from(f.lines)).sum();
        out.push(String::new());
        out.push(format!(
            "And {} more, {} in all.",
            count(more, "smaller file"),
            count(lines as usize, "line")
        ));
    }
    out.join("\n")
}

/// What a set of files holds: `Holds 3 source files with 60 lines in 1
/// folder: 2 in AXAML, 1 in C#.`
fn holds(total: &Tally) -> String {
    let ranked = total.languages();
    let mut split: Vec<String> = ranked
        .iter()
        .take(MAX_LANGUAGES_LISTED)
        .map(|(language, n)| format!("{n} in {language}"))
        .collect();
    if ranked.len() > MAX_LANGUAGES_LISTED {
        split.push(count(ranked.len() - MAX_LANGUAGES_LISTED, "other language"));
    }
    format!(
        "Holds {} with {} in {}: {}.",
        count(total.files, "source file"),
        count(total.lines as usize, "line"),
        count(total.folders(), "folder"),
        join_and(&split)
    )
}

/// The `Source` section: the files a note cites.
fn source_lines(files: &[SourceFile]) -> String {
    files
        .iter()
        .map(|f| format!("- `{}` — {}, {}", f.path, f.language, f.size()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Links by note id; `None` when the note is not in the vault.
pub type Link<'a> = &'a dyn Fn(&str) -> Option<String>;

/// One-sentence summary of a unit's note.
pub fn unit_summary(unit: &FileUnit) -> String {
    let names = unit.total.language_names();
    let files = count(unit.total.files, "source file");
    if unit.dir.is_empty() {
        format!("{names} source of the repository with {files}, mapped at file level.")
    } else {
        format!(
            "{names} source folder `{}` with {files}, mapped at file level.",
            unit.dir
        )
    }
}

/// One-sentence summary of a folder's note.
pub fn folder_summary(map: &FileMap, folder: &Folder) -> String {
    format!(
        "{} source folder `{}` of `{}` with {}, mapped at file level.",
        folder.total.language_names(),
        folder.dir,
        map.units[folder.unit].package,
        count(folder.total.files, "source file")
    )
}

fn folder_bullet(folder: &Folder, link: Link) -> Option<String> {
    Some(format!(
        "- {} — {}, {}",
        link(&format!("dir:{}", folder.dir))?,
        count(folder.total.files, "source file"),
        count(folder.total.lines as usize, "line")
    ))
}

/// The sections rules write for a unit's note, in order.
pub fn unit_sections(map: &FileMap, unit: &FileUnit, link: Link) -> Vec<(&'static str, String)> {
    let mut items = vec![holds(&unit.total)];
    let folders: Vec<String> = unit
        .folders
        .iter()
        .filter_map(|&i| folder_bullet(&map.folders[i], link))
        .collect();
    if !folders.is_empty() {
        items.push(format!("Folders:\n\n{}", folders.join("\n")));
    }
    if !unit.files.is_empty() {
        let place = if folders.is_empty() {
            "Files"
        } else {
            "Files outside those folders"
        };
        items.push(format!(
            "{place}:\n\n{}",
            file_lines(&unit.files, &unit.dir)
        ));
    }
    vec![
        ("Source", source_lines(&unit.sources)),
        ("Items", items.join("\n\n")),
        ("See also", "- [[_index]]".to_string()),
    ]
}

/// The sections rules write for a folder's note, in order.
pub fn folder_sections(map: &FileMap, folder: &Folder, link: Link) -> Vec<(&'static str, String)> {
    let unit = &map.units[folder.unit];
    let unit_link = link(&format!("unit:{}", unit.name));
    let parent_link = folder
        .parent
        .and_then(|p| link(&format!("dir:{}", map.folders[p].dir)));
    let place = match (&parent_link, &unit_link) {
        (Some(parent), Some(unit)) => format!(", inside {parent} of {unit}"),
        (None, Some(unit)) => format!(", in {unit}"),
        (Some(parent), None) => format!(", inside {parent}"),
        (None, None) => String::new(),
    };
    let mut items = vec![format!("Folder `{}`{place}.", folder.dir)];
    items.push(holds(&folder.total));
    let below: Vec<String> = folder
        .children
        .iter()
        .filter_map(|&i| folder_bullet(&map.folders[i], link))
        .collect();
    if !below.is_empty() {
        items.push(format!("Folders below:\n\n{}", below.join("\n")));
    }
    if !folder.files.is_empty() {
        items.push(format!(
            "Files:\n\n{}",
            file_lines(&folder.files, &folder.dir)
        ));
    }
    let mut see = vec!["- [[_index]]".to_string()];
    see.extend(unit_link.map(|l| format!("- {l}")));
    see.extend(parent_link.map(|l| format!("- {l}")));
    vec![
        ("Source", source_lines(&folder.sources)),
        ("Items", items.join("\n\n")),
        ("See also", see.join("\n")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_core::FileRecord;

    fn facts(paths: &[(&str, u32)]) -> RepoFacts {
        RepoFacts {
            files: paths
                .iter()
                .map(|(path, lines)| FileRecord {
                    path: path.to_string(),
                    language: Language::from_path(path),
                    hash: "h".into(),
                    bytes: u64::from(*lines) * 10,
                    lines: *lines,
                })
                .collect(),
            ..RepoFacts::default()
        }
    }

    fn map(paths: &[(&str, u32)], mapped: bool) -> FileMap {
        build(
            &facts(paths),
            "repo",
            &HashSet::new(),
            mapped,
            &HashSet::new(),
        )
    }

    #[test]
    fn projects_under_a_container_are_units() {
        let map = map(
            &[
                ("src/A/a1.cs", 1),
                ("src/A/a2.cs", 1),
                ("src/A/a3.cs", 1),
                ("src/B/b1.cs", 1),
                ("src/B/b2.cs", 1),
                ("src/B/b3.cs", 1),
                ("tests/A/t1.cs", 1),
                ("tests/A/t2.cs", 1),
                ("tests/A/t3.cs", 1),
                ("build.cs", 1),
            ],
            false,
        );
        let names: Vec<&str> = map.units.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(names, ["root-files", "src-a", "src-b", "tests-a"]);
        // The unit folders are the project names; a clash falls back to the
        // path.
        let slugs: Vec<&str> = map.units.iter().map(|u| u.slug.as_str()).collect();
        assert_eq!(slugs, ["repo", "a", "b", "tests-a"]);
    }

    #[test]
    fn a_small_unit_folds_into_the_root_and_a_mapped_repository_needs_three() {
        let paths = [("src/A/a1.cs", 1), ("src/A/a2.cs", 1), ("x.cs", 1)];
        let map_of = |mapped| map(&paths, mapped);
        let alone = map_of(false);
        assert_eq!(alone.units.len(), 1);
        assert_eq!(alone.units[0].total.files, 3);
        // The same three files beside parsed code: exactly enough.
        assert_eq!(map_of(true).units.len(), 1);
        let few = map(&paths[1..], true);
        assert!(few.is_empty(), "two stray files beside mapped code");
    }

    #[test]
    fn folders_roll_up_until_they_hold_enough() {
        let map = map(
            &[
                ("a/x1.go", 1),
                ("a/x2.go", 1),
                ("a/b/y.go", 1),
                ("a/b/c/z.go", 1),
                ("p/q/1.go", 1),
                ("p/q/2.go", 1),
                ("p/q/3.go", 1),
                ("p/q/r/4.go", 1),
            ],
            false,
        );
        let dirs: Vec<&str> = map.folders.iter().map(|f| f.dir.as_str()).collect();
        assert_eq!(dirs, ["a", "p/q"]);
        assert_eq!(map.folder("a").unwrap().files.len(), 4);
        // `p/q/r` has one file, which `p/q` holds, and `p` holds none.
        assert_eq!(map.folder("p/q").unwrap().files.len(), 4);
        assert!(map.units[0].files.is_empty());
        assert_eq!(map.units[0].folders, [0, 1]);
    }

    #[test]
    fn a_folder_with_a_note_below_another_is_its_child() {
        let mut paths: Vec<(String, u32)> = Vec::new();
        for dir in ["a", "a/b"] {
            for n in 0..3 {
                paths.push((format!("{dir}/f{n}.go"), 1));
            }
        }
        let refs: Vec<(&str, u32)> = paths.iter().map(|(p, l)| (p.as_str(), *l)).collect();
        let map = map(&refs, false);
        assert_eq!(map.folders.len(), 2);
        let (a, b) = (&map.folders[0], &map.folders[1]);
        assert_eq!((a.dir.as_str(), b.dir.as_str()), ("a", "a/b"));
        assert_eq!(a.children, [1]);
        assert_eq!(b.parent, Some(0));
        assert_eq!(a.total.files, 6);
        assert_eq!(a.files.len(), 3);
        assert_eq!(b.vault_dir, "01-Core/repo/a/b");
    }

    #[test]
    fn the_limit_cuts_the_smallest_but_never_a_pinned_folder() {
        let mut paths: Vec<(String, u32)> = Vec::new();
        for n in 1..=(MAX_FOLDER_NOTES + 2) {
            for f in 0..3 {
                paths.push((format!("d{n:03}/f{f}.go"), n as u32));
            }
        }
        let refs: Vec<(&str, u32)> = paths.iter().map(|(p, l)| (p.as_str(), *l)).collect();
        let facts = facts(&refs);
        let none = HashSet::new();
        let plain = build(&facts, "repo", &none, false, &none);
        assert_eq!(plain.folders.len(), MAX_FOLDER_NOTES);
        assert!(plain.folder("d001").is_none() && plain.folder("d002").is_none());
        // What the cut folders held is held by the unit.
        assert_eq!(plain.units[0].files.len(), 6);

        let pinned = HashSet::from(["dir:d001".to_string()]);
        let kept = build(&facts, "repo", &none, false, &pinned);
        assert!(kept.folder("d001").is_some() && kept.folder("d002").is_none());
        assert_eq!(kept.folders.len(), MAX_FOLDER_NOTES + 1);
    }

    #[test]
    fn folders_of_one_name_are_told_apart_by_their_place() {
        let mut paths: Vec<(String, u32)> = Vec::new();
        for dir in [
            "src/A/models",
            "src/B/models",
            "src/B/views",
            "src/B/views/models",
        ] {
            for n in 0..3 {
                paths.push((format!("{dir}/f{n}.go"), 1));
            }
        }
        for n in 0..3 {
            paths.push((format!("src/A/x{n}.go"), 1));
            paths.push((format!("src/B/x{n}.go"), 1));
        }
        let refs: Vec<(&str, u32)> = paths.iter().map(|(p, l)| (p.as_str(), *l)).collect();
        let map = map(&refs, false);
        let titles: Vec<&str> = map.folders.iter().map(|f| f.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Models Folder (A)",
                "Models Folder (B)",
                "Views Folder",
                "Models Folder (views)"
            ]
        );
    }

    #[test]
    fn language_sentences() {
        let two = map(
            &[("a.cs", 1), ("b.cs", 1), ("c.axaml", 1), ("d/e.cs", 1)],
            false,
        );
        assert_eq!(two.total.language_names(), "C# and AXAML");
        assert_eq!(two.total.folders(), 2);
        let home = two.home_summary("ta", "Tail.");
        assert_eq!(
            home,
            "**ta** is a C# and AXAML codebase of 2 folders and 4 source files, mapped at file level: CodeWiki has no parser for C# and AXAML yet. Tail."
        );
        let many = map(
            &[
                ("a.go", 1),
                ("b.sql", 1),
                ("c.sh", 1),
                ("d.lua", 1),
                ("e.zig", 1),
            ],
            false,
        );
        assert_eq!(
            many.total.language_names(),
            "Go, Lua, SQL, and 2 other languages"
        );
        // The headline names two, the parser sentence all.
        assert_eq!(
            many.home_summary("x", "T."),
            "**x** is a Go and Lua codebase of 1 folder and 5 source files, mapped at file level: CodeWiki has no parser for Go, Lua, SQL, Shell, and Zig yet. T."
        );
    }
}
