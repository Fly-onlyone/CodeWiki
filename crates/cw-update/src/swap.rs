//! Replacing the files of an installation, and undoing it.
//!
//! Every step is a rename inside the installation folder. Before each one the
//! journal `.update\swap.json` records it, so that a swap cut short by a crash
//! or a power cut is undone the next time the app starts.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::UPDATE_DIR;

const JOURNAL: &str = "swap.json";

/// Files of a package that are not installed: the installer itself.
const NOT_INSTALLED: [&str; 2] = ["install.ps1", "install.cmd"];

/// A file that was just written, or a program that just ended, can be held by
/// Windows Defender or the search indexer for a moment. Try this many times,
/// this long apart (as `uninstall.ps1` does).
const ATTEMPTS: u32 = 20;
const PAUSE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Move {
    from: PathBuf,
    to: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Journal {
    moves: Vec<Move>,
}

fn journal_path(install: &Path) -> PathBuf {
    install.join(UPDATE_DIR).join(JOURNAL)
}

fn write_journal(install: &Path, journal: &Journal) -> Result<(), String> {
    let path = journal_path(install);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(journal).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("could not write {}: {e}", path.display()))
}

fn rename_patiently(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut attempt = 1;
    loop {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(_) if attempt < ATTEMPTS => {
                attempt += 1;
                std::thread::sleep(PAUSE);
            }
            Err(e) => return Err(e),
        }
    }
}

/// Mark the swap as finished: delete its journal. While the journal exists,
/// the next start undoes the moves it lists, so one that cannot be deleted (a
/// scanner may hold a new file for a moment) is emptied instead, and tried
/// again for a while before giving up.
fn close_journal(install: &Path) {
    let path = journal_path(install);
    for attempt in 1..=ATTEMPTS {
        if std::fs::remove_file(&path).is_ok() || !path.exists() {
            return;
        }
        if write_journal(install, &Journal::default()).is_ok() {
            return;
        }
        if attempt < ATTEMPTS {
            std::thread::sleep(PAUSE);
        }
    }
}

fn remove(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

/// `<name>.old`, or `.old2`, `.old3`… when an older one is still in use (a
/// program from two updates ago that has not ended).
fn old_name(install: &Path, name: &str) -> Result<PathBuf, String> {
    for n in 1..100 {
        let candidate = if n == 1 {
            install.join(format!("{name}.old"))
        } else {
            install.join(format!("{name}.old{n}"))
        };
        if !candidate.exists() || remove(&candidate).is_ok() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "too many old copies of {name} in {}",
        install.display()
    ))
}

/// Undo `moves` from the last to the first. A move whose target is gone or
/// whose source is back already is skipped, so this can run again after a
/// crash. Returns the moves that could not be undone.
fn undo(moves: &[Move]) -> Vec<String> {
    let mut failed = Vec::new();
    for step in moves.iter().rev() {
        if !step.to.exists() || step.from.exists() {
            continue;
        }
        if let Err(e) = rename_patiently(&step.to, &step.from) {
            failed.push(format!(
                "{} back to {}: {e}",
                step.to.display(),
                step.from.display()
            ));
        }
    }
    failed
}

/// Move every entry of `stage` into `install`, renaming what it replaces to
/// `<name>.old`. On the first failure every step is undone and the
/// installation is as it was.
pub fn swap(install: &Path, stage: &Path) -> Result<(), String> {
    let mut names: Vec<String> = std::fs::read_dir(stage)
        .map_err(|e| format!("could not read {}: {e}", stage.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !NOT_INSTALLED.contains(&name.as_str()))
        .collect();
    names.sort();

    let mut journal = Journal::default();
    let mut result = Ok(());
    'entries: for name in &names {
        let target = install.join(name);
        let mut steps = Vec::new();
        if target.exists() {
            match old_name(install, name) {
                Ok(old) => steps.push(Move {
                    from: target.clone(),
                    to: old,
                }),
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        steps.push(Move {
            from: stage.join(name),
            to: target,
        });
        for step in steps {
            // Recorded before it is made: a step recorded but not made is
            // skipped by `undo`.
            journal.moves.push(step.clone());
            if let Err(e) = write_journal(install, &journal) {
                journal.moves.pop();
                result = Err(e);
                break 'entries;
            }
            if let Err(e) = rename_patiently(&step.from, &step.to) {
                result = Err(format!(
                    "could not move {} to {}: {e}",
                    step.from.display(),
                    step.to.display()
                ));
                break 'entries;
            }
        }
    }

    if let Err(e) = result {
        let failed = undo(&journal.moves);
        if failed.is_empty() {
            let _ = std::fs::remove_file(journal_path(install));
            return Err(format!("{e}; nothing was changed"));
        }
        // The journal stays: the next start tries again.
        return Err(format!(
            "{e}; then could not put back {}; the next start of CodeWiki tries again",
            failed.join("; ")
        ));
    }
    close_journal(install);
    Ok(())
}

/// Run when the app starts. Undo a swap that was cut short, then delete the
/// `.old` files left by a finished one and the `.update` folder. A `.old`
/// file whose replacement is missing is put back instead. Returns what it
/// did, for the log; files still in use are left for the next start.
pub fn recover_and_clean(install: &Path) -> Vec<String> {
    let mut report = Vec::new();
    let journal = journal_path(install);
    if let Ok(text) = std::fs::read_to_string(&journal) {
        match serde_json::from_str::<Journal>(&text) {
            // An emptied journal is a finished swap whose journal could
            // not be deleted at the time.
            Ok(finished) if finished.moves.is_empty() => {
                let _ = std::fs::remove_file(&journal);
            }
            Ok(unfinished) => {
                let failed = undo(&unfinished.moves);
                if failed.is_empty() {
                    report.push("undid an update that did not finish".to_string());
                    let _ = std::fs::remove_file(&journal);
                } else {
                    report.push(format!(
                        "could not undo an update that did not finish: {}",
                        failed.join("; ")
                    ));
                    // Keep the journal and the old files for the next start.
                    return report;
                }
            }
            Err(e) => report.push(format!("ignored an unreadable {}: {e}", journal.display())),
        }
    }

    let Ok(entries) = std::fs::read_dir(install) else {
        return report;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some(name) = replaced_name(&file_name) else {
            continue;
        };
        let old = entry.path();
        let current = install.join(name);
        if current.exists() {
            if remove(&old).is_ok() {
                report.push(format!("deleted {file_name}"));
            }
        } else if rename_patiently(&old, &current).is_ok() {
            report.push(format!("put {file_name} back as {name}"));
        }
    }
    let update = install.join(UPDATE_DIR);
    if update.exists() && std::fs::remove_dir_all(&update).is_ok() {
        report.push(format!("deleted {UPDATE_DIR}"));
    }
    report
}

/// For `codewiki.exe.old` or `codewiki.exe.old2`, the name it replaced.
fn replaced_name(file_name: &str) -> Option<&str> {
    let (name, suffix) = file_name.rsplit_once(".old")?;
    (!name.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())).then_some(name)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Every file under `root` with its content, `.update` left out.
    fn snapshot(root: &Path) -> BTreeMap<String, String> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel == UPDATE_DIR {
                    continue;
                }
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    out.insert(rel, fs::read_to_string(&path).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    /// An installation of version 1 and an unpacked package of version 2,
    /// which adds a file and carries the installer.
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let install = dir.path().join("CodeWiki");
        let stage = install.join(UPDATE_DIR).join("2.0.0").join("stage");
        for root in [&install, &stage] {
            let version = if root == &install { "1" } else { "2" };
            for program in crate::PROGRAMS {
                write(root, program, version);
            }
            write(root, "plugins/claude/plugin.json", version);
            write(root, "uninstall.ps1", version);
        }
        write(&stage, "CHANGES.md", "2");
        write(&stage, "install.ps1", "2");
        write(&stage, "install.cmd", "2");
        (dir, install, stage)
    }

    #[test]
    fn swap_replaces_every_file_and_keeps_the_old_ones_aside() {
        let (_dir, install, stage) = fixture();
        swap(&install, &stage).unwrap();
        let files = snapshot(&install);
        assert_eq!(files["codewiki.exe"], "2");
        assert_eq!(files["codewiki.exe.old"], "1");
        assert_eq!(files["plugins/claude/plugin.json"], "2");
        assert_eq!(files["plugins.old/claude/plugin.json"], "1");
        // A file new in this release is installed too.
        assert_eq!(files["CHANGES.md"], "2");
        // The installer is not: an installation has none.
        assert!(!files.contains_key("install.ps1"));
        assert!(!files.contains_key("install.cmd"));
        assert!(crate::is_installation(&install));
        assert!(!journal_path(&install).exists());

        // The next start deletes what was set aside.
        let report = recover_and_clean(&install);
        assert!(
            report.iter().any(|line| line == "deleted codewiki.exe.old"),
            "{report:?}"
        );
        let files = snapshot(&install);
        assert!(files.keys().all(|name| !name.contains(".old")), "{files:?}");
        assert!(!install.join(UPDATE_DIR).exists());
        assert_eq!(files["codewiki-app.exe"], "2");
    }

    #[test]
    fn an_emptied_journal_does_not_undo_a_finished_swap() {
        let (_dir, install, stage) = fixture();
        swap(&install, &stage).unwrap();
        // What `close_journal` leaves when the journal cannot be deleted.
        write_journal(&install, &Journal::default()).unwrap();
        let report = recover_and_clean(&install);
        assert!(
            report.iter().all(|line| !line.contains("undid")),
            "{report:?}"
        );
        assert_eq!(snapshot(&install)["codewiki.exe"], "2");
        assert!(!journal_path(&install).exists());
    }

    #[test]
    fn a_stale_old_file_is_replaced() {
        let (_dir, install, stage) = fixture();
        write(&install, "codewiki.exe.old", "0");
        swap(&install, &stage).unwrap();
        assert_eq!(
            fs::read_to_string(install.join("codewiki.exe.old")).unwrap(),
            "1"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_file_held_open_undoes_the_whole_swap() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_dir, install, stage) = fixture();
        let before = snapshot(&install);
        // No sharing at all: not even a rename is allowed, as when a program
        // holds a file under `plugins` open.
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(install.join("plugins/claude/plugin.json"))
            .unwrap();
        let error = swap(&install, &stage).unwrap_err();
        drop(held);
        assert!(error.contains("plugins"), "{error}");
        assert!(error.ends_with("nothing was changed"), "{error}");
        assert_eq!(snapshot(&install), before);
        assert!(!journal_path(&install).exists());
        // The package is still there to try again.
        assert!(stage.join("codewiki.exe").is_file());
    }

    #[test]
    fn a_swap_cut_short_is_undone_at_the_next_start() {
        let (_dir, install, stage) = fixture();
        let before = snapshot(&install);
        // As if the app had crashed in a swap: two moves made, a third
        // recorded but not made.
        let moves = vec![
            Move {
                from: install.join("codewiki-app.exe"),
                to: install.join("codewiki-app.exe.old"),
            },
            Move {
                from: stage.join("codewiki-app.exe"),
                to: install.join("codewiki-app.exe"),
            },
            Move {
                from: install.join("codewiki-mcp.exe"),
                to: install.join("codewiki-mcp.exe.old"),
            },
        ];
        fs::rename(&moves[0].from, &moves[0].to).unwrap();
        fs::rename(&moves[1].from, &moves[1].to).unwrap();
        write_journal(&install, &Journal { moves }).unwrap();

        let report = recover_and_clean(&install);
        assert!(report[0].starts_with("undid"), "{report:?}");
        assert_eq!(snapshot(&install), before);
        assert!(!install.join(UPDATE_DIR).exists());
    }

    #[test]
    fn an_old_file_without_its_replacement_is_put_back() {
        let (_dir, install, _stage) = fixture();
        fs::rename(
            install.join("codewiki.exe"),
            install.join("codewiki.exe.old"),
        )
        .unwrap();
        let report = recover_and_clean(&install);
        assert!(report.contains(&"put codewiki.exe.old back as codewiki.exe".to_string()));
        assert_eq!(
            fs::read_to_string(install.join("codewiki.exe")).unwrap(),
            "1"
        );
    }

    #[test]
    fn names_of_old_files() {
        assert_eq!(replaced_name("codewiki.exe.old"), Some("codewiki.exe"));
        assert_eq!(replaced_name("codewiki.exe.old12"), Some("codewiki.exe"));
        assert_eq!(replaced_name("plugins.old"), Some("plugins"));
        assert_eq!(replaced_name("codewiki.exe"), None);
        assert_eq!(replaced_name("notes.older"), None);
        assert_eq!(replaced_name(".old"), None);
    }
}
