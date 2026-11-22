//! The list of folders closed to coding agents.
//!
//! One JSON file in the data directory holds the list. The MCP bridge, the
//! session-start hook, the command line and the desktop app all read and
//! change it through this module.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Name of the list file inside the data directory.
pub const FILE: &str = "agents-off.json";

/// The only layout of the file this version understands.
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Document {
    version: u32,
    folders: Vec<PathBuf>,
}

/// What [`add`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Added {
    /// The folder is now on the list, in this normalized form.
    Added(PathBuf),
    /// This listed folder already covers it. Nothing was written.
    CoveredBy(PathBuf),
}

/// What [`remove`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Removed {
    /// The folder was on the list and is gone from it.
    Removed(PathBuf),
    /// Only this listed parent covers the folder. Nothing was written.
    CoveredBy(PathBuf),
    /// Neither the folder nor a parent is listed.
    NotListed,
}

/// Path of the list file.
pub fn file(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}

/// The listed folders. A missing file is an empty list; a file that cannot be
/// read or understood is an error, and so is an entry that is empty or not
/// absolute: `""` would cover every folder and `"secret"` would mean a
/// different folder to each reader.
pub fn load(data_dir: &Path) -> Result<Vec<PathBuf>> {
    let path = file(data_dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(Error::invalid(format!(
                "cannot read {}: {e}",
                path.display()
            )));
        }
    };
    let document: Document = serde_json::from_str(&text)
        .map_err(|e| Error::invalid(format!("cannot parse {}: {e}", path.display())))?;
    if document.version != VERSION {
        return Err(Error::invalid(format!(
            "{} has version {}, expected {VERSION}",
            path.display(),
            document.version
        )));
    }
    if let Some(entry) = document.folders.iter().find(|entry| !entry.is_absolute()) {
        return Err(Error::invalid(format!(
            "{} lists {:?}, which is not an absolute folder",
            path.display(),
            entry.to_string_lossy()
        )));
    }
    Ok(document.folders)
}

/// Make `path` absolute (relative paths are joined to `base`), resolve `.` and
/// `..` without touching the disk, and drop any trailing separator. A path
/// that joining leaves relative (on Windows a drive without a root,
/// `D:secret`) is resolved by the system first, against the process's folder
/// on that drive. Only when the system cannot do that, for a path with a NUL
/// in it or one too long for it, is the result still relative; the backend
/// cannot open such a path either, and [`load`] and [`add`] refuse it.
pub fn normalize(path: &Path, base: &Path) -> PathBuf {
    let mut joined = base.join(path);
    if !joined.is_absolute() {
        joined = std::path::absolute(&joined).unwrap_or(joined);
    }
    let mut parts: Vec<Component> = Vec::new();
    for part in joined.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => match parts.last() {
                Some(Component::Normal(_)) => {
                    parts.pop();
                }
                Some(Component::Prefix(_) | Component::RootDir) => {}
                _ => parts.push(part),
            },
            _ => parts.push(part),
        }
    }
    parts.iter().map(|part| part.as_os_str()).collect()
}

/// The listed entry that is `path` or a parent of it, if any. Both sides are
/// compared as they are on disk (links followed) when they exist, and as
/// written otherwise. Windows ignores case. `path` must be absolute.
pub fn covering(entries: &[PathBuf], path: &Path) -> Option<PathBuf> {
    let wanted = forms(path);
    entries
        .iter()
        .find(|entry| {
            forms(entry)
                .iter()
                .any(|a| wanted.iter().any(|b| b.starts_with(a)))
        })
        .cloned()
}

/// What an agent is told when a folder of its session is kept from coding
/// agents: `entry` is the listed folder that keeps it out.
pub fn refusal(entry: &Path) -> String {
    format!(
        "CodeWiki is turned off for this project ({} is kept from coding agents in CodeWiki's settings). Do not call CodeWiki tools here; continue without them.",
        entry.display()
    )
}

/// What an agent is told when only the repository a tool names is kept from
/// coding agents. It names no path, so it tells nothing about the folder.
pub fn repo_refusal() -> &'static str {
    "CodeWiki is turned off for that repository; it is kept from coding agents in CodeWiki's settings. Do not call CodeWiki tools for it."
}

/// What the MCP bridge, the command line and the app say when the list
/// (`file`) cannot be read.
pub fn unreadable(file: &Path) -> String {
    format!(
        "CodeWiki's list of folders kept from coding agents ({}) cannot be read; fix or delete it. Until then CodeWiki's tools are off for coding agents.",
        file.display()
    )
}

/// Add `folder` to the list unless a listed folder already covers it.
pub fn add(data_dir: &Path, folder: &Path) -> Result<Added> {
    let mut entries = load(data_dir)?;
    let folder = absolute(folder)?;
    if let Some(parent) = covering(&entries, &folder) {
        return Ok(Added::CoveredBy(parent));
    }
    entries.push(folder.clone());
    save(data_dir, &entries)?;
    Ok(Added::Added(folder))
}

/// Take `folder` off the list. A folder that only a listed parent covers is
/// left alone.
pub fn remove(data_dir: &Path, folder: &Path) -> Result<Removed> {
    let mut entries = load(data_dir)?;
    let folder = absolute(folder)?;
    let wanted = forms(&folder);
    let exact = entries
        .iter()
        .position(|entry| forms(entry).iter().any(|a| wanted.contains(a)));
    if let Some(index) = exact {
        let gone = entries.remove(index);
        save(data_dir, &entries)?;
        return Ok(Removed::Removed(gone));
    }
    Ok(match covering(&entries, &folder) {
        Some(parent) => Removed::CoveredBy(parent),
        None => Removed::NotListed,
    })
}

/// Write an empty list over whatever the file holds, even a broken file.
pub fn reset(data_dir: &Path) -> Result<()> {
    save(data_dir, &[])
}

fn absolute(folder: &Path) -> Result<PathBuf> {
    let cwd = std::env::current_dir()
        .map_err(|e| Error::invalid(format!("cannot find the current folder: {e}")))?;
    let made = normalize(folder, &cwd);
    if !made.is_absolute() {
        // Written to the list, it would make the list unreadable.
        return Err(Error::invalid(format!(
            "{:?} cannot be made an absolute folder",
            folder.to_string_lossy()
        )));
    }
    Ok(made)
}

fn save(data_dir: &Path, folders: &[PathBuf]) -> Result<()> {
    let document = Document {
        version: VERSION,
        folders: folders.to_vec(),
    };
    let text = serde_json::to_string_pretty(&document)
        .map_err(|e| Error::external("writing the agent blacklist", e))?;
    let path = file(data_dir);
    std::fs::create_dir_all(data_dir).map_err(|e| Error::io(data_dir, e))?;
    let draft = data_dir.join(format!("{FILE}.tmp"));
    std::fs::write(&draft, text).map_err(|e| Error::io(&draft, e))?;
    std::fs::rename(&draft, &path).map_err(|e| Error::io(&path, e))?;
    Ok(())
}

/// The component lists a path can be compared by: as written (normalized), and
/// as the disk has it when the path exists and that differs.
fn forms(path: &Path) -> Vec<Vec<String>> {
    let mut forms = vec![components(path)];
    if let Ok(real) = path.canonicalize() {
        let real = components(&strip_verbatim(&real));
        if !forms.contains(&real) {
            forms.push(real);
        }
    }
    forms
}

fn components(path: &Path) -> Vec<String> {
    normalize(path, Path::new(""))
        .components()
        .map(|part| fold(&part.as_os_str().to_string_lossy()))
        .collect()
}

#[cfg(windows)]
fn fold(text: &str) -> String {
    text.to_lowercase()
}

#[cfg(not(windows))]
fn fold(text: &str) -> String {
    text.to_string()
}

/// Drop the `\\?\` prefix `canonicalize` adds on Windows.
fn strip_verbatim(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str) -> PathBuf {
        PathBuf::from(text)
    }

    #[cfg(windows)]
    const ROOT: &str = r"D:\work\clients";
    #[cfg(not(windows))]
    const ROOT: &str = "/work/clients";

    #[test]
    fn the_texts_are_exact() {
        assert_eq!(
            refusal(&p("D:\\secret")),
            "CodeWiki is turned off for this project (D:\\secret is kept from coding agents in CodeWiki's settings). Do not call CodeWiki tools here; continue without them."
        );
        assert_eq!(
            repo_refusal(),
            "CodeWiki is turned off for that repository; it is kept from coding agents in CodeWiki's settings. Do not call CodeWiki tools for it."
        );
        assert_eq!(
            unreadable(&p("D:\\data\\agents-off.json")),
            "CodeWiki's list of folders kept from coding agents (D:\\data\\agents-off.json) cannot be read; fix or delete it. Until then CodeWiki's tools are off for coding agents."
        );
    }

    #[test]
    fn missing_file_is_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), Vec::<PathBuf>::new());
    }

    #[test]
    fn an_entry_covers_itself_and_below() {
        let entries = vec![p(ROOT)];
        assert_eq!(covering(&entries, &p(ROOT)), Some(p(ROOT)));
        let below = p(ROOT).join("api").join("src");
        assert_eq!(covering(&entries, &below), Some(p(ROOT)));
    }

    #[cfg(windows)]
    #[test]
    fn case_separators_and_trailing_slash_do_not_matter() {
        let entries = vec![p(ROOT)];
        assert!(covering(&entries, &p("d:/Work/Clients/")).is_some());
        assert!(covering(&entries, &p(r"D:\work\clients\.\api")).is_some());
    }

    #[test]
    fn a_sibling_with_a_common_prefix_is_not_covered() {
        let entries = vec![p(ROOT)];
        let short = p(&ROOT[..ROOT.len() - 1]);
        let long = p(&format!("{ROOT}X"));
        assert_eq!(covering(&entries, &short), None);
        assert_eq!(covering(&entries, &long), None);
    }

    #[cfg(windows)]
    #[test]
    fn a_drive_root_covers_the_drive_only() {
        let entries = vec![p(r"D:\")];
        assert!(covering(&entries, &p(r"D:\x")).is_some());
        assert!(covering(&entries, &p(r"C:\x")).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn a_drive_relative_path_is_made_absolute() {
        for written in [r"D:secret", r"D:x\y"] {
            let made = normalize(Path::new(written), Path::new(r"C:\work"));
            assert!(made.is_absolute(), "{written} gave {}", made.display());
        }
    }

    #[test]
    fn relative_paths_are_made_absolute() {
        #[cfg(windows)]
        assert_eq!(
            normalize(Path::new(r"..\b"), Path::new(r"C:\a\c")),
            p(r"C:\a\b")
        );
        #[cfg(not(windows))]
        assert_eq!(normalize(Path::new("../b"), Path::new("/a/c")), p("/a/b"));
    }

    #[test]
    fn add_then_load_then_remove() {
        let dir = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let folder = work.path().join("clients");
        let normalized = normalize(&folder, work.path());

        assert_eq!(
            add(dir.path(), &folder).unwrap(),
            Added::Added(normalized.clone())
        );
        assert_eq!(load(dir.path()).unwrap(), vec![normalized.clone()]);

        let child = folder.join("api");
        assert_eq!(
            add(dir.path(), &child).unwrap(),
            Added::CoveredBy(normalized.clone())
        );
        assert_eq!(load(dir.path()).unwrap(), vec![normalized.clone()]);

        assert_eq!(
            remove(dir.path(), &child).unwrap(),
            Removed::CoveredBy(normalized.clone())
        );
        assert_eq!(
            remove(dir.path(), &work.path().join("other")).unwrap(),
            Removed::NotListed
        );
        assert_eq!(
            remove(dir.path(), &folder).unwrap(),
            Removed::Removed(normalized)
        );
        assert!(load(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn a_broken_file_is_an_error_and_is_not_overwritten() {
        for broken in [
            "",
            "[]",
            "{\"version\":2,\"folders\":[]}",
            "{\"version\":1,",
            // An entry that is not an absolute folder would cover every
            // folder, or one that depends on where it is read.
            "{\"version\":1,\"folders\":[\"\"]}",
            "{\"version\":1,\"folders\":[\".\"]}",
            "{\"version\":1,\"folders\":[\"x\"]}",
        ] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(file(dir.path()), broken).unwrap();
            assert!(load(dir.path()).is_err(), "{broken:?}");
            assert!(add(dir.path(), &p(ROOT)).is_err(), "{broken:?}");
            assert!(remove(dir.path(), &p(ROOT)).is_err(), "{broken:?}");
            assert_eq!(std::fs::read(file(dir.path())).unwrap(), broken.as_bytes());
            reset(dir.path()).unwrap();
            assert_eq!(load(dir.path()).unwrap(), Vec::<PathBuf>::new());
        }
    }

    #[test]
    fn a_relative_entry_is_named_in_the_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(file(dir.path()), "{\"version\":1,\"folders\":[\"secret\"]}").unwrap();
        let error = load(dir.path()).unwrap_err().to_string();
        assert!(error.contains("\"secret\""), "{error}");
    }

    #[cfg(windows)]
    #[test]
    fn a_linked_folder_matches_its_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(target.join("inner")).unwrap();
        let link = dir.path().join("link");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output();
        if !made.is_ok_and(|out| out.status.success()) {
            return;
        }
        let entries = vec![target];
        assert!(covering(&entries, &link).is_some());
        assert!(covering(&entries, &link.join("inner")).is_some());
    }
}
