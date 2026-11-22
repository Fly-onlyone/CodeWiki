//! Which tool calls coding agents may make: folders closed to them in
//! CodeWiki's settings get no tools, and cross-project tools do not show
//! listed projects. Plain functions, no backend, so they test without one.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// Whether a tool call may go ahead.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Open,
    Refused(String),
}

/// What the agent is told in a closed folder.
pub fn refusal(entry: &Path) -> String {
    format!(
        "CodeWiki is turned off for this project ({} is kept from coding agents in CodeWiki's settings). Do not call CodeWiki tools here; continue without them.",
        entry.display()
    )
}

/// What the agent is told when the list cannot be read.
pub fn unreadable(file: &Path) -> String {
    format!(
        "CodeWiki's list of folders kept from coding agents ({}) cannot be read; fix or delete it. Until then CodeWiki's tools are off for coding agents.",
        file.display()
    )
}

/// The folders the session works in: the working directory, plus
/// `CLAUDE_PROJECT_DIR` when set, made absolute against the working directory
/// so that a relative value cannot slip past the list.
pub fn session_folders(cwd: &Path, project_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut folders = vec![cwd.to_path_buf()];
    if let Some(dir) = project_dir {
        folders.push(cw_core::agents_off::normalize(dir, cwd));
    }
    folders
}

/// A tool's `repo` argument: a folder path or a repository id or name.
#[derive(Debug, PartialEq, Eq)]
pub enum RepoArg {
    Path(PathBuf),
    IdOrName(String),
}

/// A path when it is absolute, starts with `.`, or contains a separator;
/// otherwise an id or name. A path is made absolute against `cwd`.
pub fn repo_arg(value: &str, cwd: &Path) -> RepoArg {
    let path = Path::new(value);
    if path.is_absolute() || value.starts_with('.') || value.contains(['/', '\\']) {
        RepoArg::Path(cw_core::agents_off::normalize(path, cwd))
    } else {
        RepoArg::IdOrName(value.to_string())
    }
}

/// The id or name to look up in `/v1/repos` for a `repo` argument, if any:
/// an id or name as given, or the last component of a path that is not an
/// existing folder. The backend falls back to that name when it cannot add
/// the path (`cw-client`'s `repo_id`), so a missing path can still reach a
/// listed repository. An existing folder needs no lookup.
pub fn lookup_name(arg: &RepoArg, exists: impl Fn(&Path) -> bool) -> Option<String> {
    match arg {
        RepoArg::IdOrName(name) => Some(name.clone()),
        RepoArg::Path(path) if exists(path) => None,
        RepoArg::Path(path) => {
            let written = path.to_string_lossy().replace('\\', "/");
            let name = written.trim_end_matches('/').rsplit('/').next()?;
            (!name.is_empty()).then(|| name.to_string())
        }
    }
}

/// The `/v1/repos` entry whose id equals `id_or_name` or whose name equals it
/// case-insensitively (the rule of `cw-client`'s `repo_id`).
fn find_repo<'a>(listing: &'a Value, id_or_name: &str) -> Option<&'a Value> {
    listing.as_array()?.iter().find(|r| {
        r["id"] == id_or_name
            || r["name"]
                .as_str()
                .is_some_and(|n| n.eq_ignore_ascii_case(id_or_name))
    })
}

/// The `path` of the entry `id_or_name` names.
pub fn resolve_id(listing: &Value, id_or_name: &str) -> Option<PathBuf> {
    find_repo(listing, id_or_name)?["path"]
        .as_str()
        .map(PathBuf::from)
}

/// The folder a `repo` argument means when no registered repository answers
/// to it: a path as it is, a bare word as a path under `cwd`. The backend
/// would read a bare word against its own folder, so it is never sent one.
pub fn direct_path(arg: &RepoArg, cwd: &Path) -> PathBuf {
    match arg {
        RepoArg::Path(path) => path.clone(),
        RepoArg::IdOrName(word) => cw_core::agents_off::normalize(Path::new(word), cwd),
    }
}

/// What the backend is given for a `repo` argument.
#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    /// A registered repository's id.
    Id(String),
    /// An absolute folder path.
    Path(PathBuf),
}

/// An id or name that a registered repository answers to is that repository's
/// id; any other word is a folder under `cwd`; a path stays a path.
pub fn target(arg: &RepoArg, listing: Option<&Value>, cwd: &Path) -> Target {
    if let RepoArg::IdOrName(word) = arg
        && let Some(id) = listing
            .and_then(|l| find_repo(l, word))
            .and_then(|r| r["id"].as_str())
    {
        return Target::Id(id.to_string());
    }
    Target::Path(direct_path(arg, cwd))
}

/// Refused when the list cannot be read or covers any session folder.
pub fn verdict(
    list: &cw_core::Result<Vec<PathBuf>>,
    list_file: &Path,
    folders: &[PathBuf],
) -> Verdict {
    match list {
        Err(_) => Verdict::Refused(unreadable(list_file)),
        Ok(entries) => folders
            .iter()
            .find_map(|folder| cw_core::agents_off::covering(entries, folder))
            .map_or(Verdict::Open, |entry| Verdict::Refused(refusal(&entry))),
    }
}

/// Remove covered entries from a `/v1/repos` array; returns their names.
pub fn hide_repos(listing: &mut Value, entries: &[PathBuf]) -> HashSet<String> {
    let mut hidden = HashSet::new();
    if let Some(rows) = listing.as_array_mut() {
        rows.retain(|row| {
            let covered = row["path"]
                .as_str()
                .is_some_and(|p| cw_core::agents_off::covering(entries, Path::new(p)).is_some());
            if covered && let Some(name) = row["name"].as_str() {
                hidden.insert(name.to_string());
            }
            !covered
        });
    }
    hidden
}

fn is_hidden(row: &Value, key: &str, hidden: &HashSet<String>) -> bool {
    row[key].as_str().is_some_and(|n| hidden.contains(n))
}

fn retain_rows(value: &mut Value, key: &str, keep: impl FnMut(&Value) -> bool) {
    if let Some(rows) = value[key].as_array_mut() {
        rows.retain(keep);
    }
}

/// Drop hidden repositories from a `/v1/workspace` view and from its shared
/// dependencies (a dependency left with fewer than two repositories goes).
pub fn hide_in_workspace(workspace: &mut Value, hidden: &HashSet<String>) {
    retain_rows(workspace, "repos", |r| !is_hidden(r, "name", hidden));
    if let Some(deps) = workspace["shared_dependencies"].as_array_mut() {
        for dep in deps.iter_mut() {
            if let Some(names) = dep["repos"].as_array_mut() {
                names.retain(|n| n.as_str().is_none_or(|n| !hidden.contains(n)));
            }
        }
        deps.retain(|d| d["repos"].as_array().is_some_and(|n| n.len() >= 2));
    }
}

/// Drop hidden repositories from a `/v1/workspace/contracts` view: rows of
/// every list keyed by a hidden `repo`, and hidden providers of a matched
/// call. A visible call left with no visible provider moves to `unprovided`,
/// so the lists keep adding up to each repository's counts.
///
/// Known limitation: a visible route called only by a hidden repository leaves
/// `matched` and does not appear in `unconsumed`, because a provider reference
/// has no file or line to rebuild the row from. Fixing it needs the server to
/// compute over visible repositories only.
pub fn hide_in_contracts(contracts: &mut Value, hidden: &HashSet<String>) {
    for key in ["repos", "errors", "unprovided", "unconsumed", "matched"] {
        retain_rows(contracts, key, |r| !is_hidden(r, "repo", hidden));
    }
    let Some(rows) = contracts["matched"].as_array_mut() else {
        return;
    };
    let mut orphaned = Vec::new();
    rows.retain_mut(|row| {
        if let Some(providers) = row["providers"].as_array_mut() {
            providers.retain(|p| !is_hidden(p, "repo", hidden));
            if providers.is_empty() {
                orphaned.push(json!({
                    "repo": row["repo"], "path": row["path"],
                    "file": row["file"], "line": row["line"],
                }));
                return false;
            }
        }
        true
    });
    if !orphaned.is_empty() {
        match contracts["unprovided"].as_array_mut() {
            Some(unprovided) => unprovided.extend(orphaned),
            None => contracts["unprovided"] = Value::Array(orphaned),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn the_refusal_text_is_exact() {
        assert_eq!(
            refusal(&p("D:\\secret")),
            "CodeWiki is turned off for this project (D:\\secret is kept from coding agents in CodeWiki's settings). Do not call CodeWiki tools here; continue without them."
        );
    }

    #[test]
    fn a_covered_session_folder_is_refused() {
        let list = Ok(vec![p("D:\\secret")]);
        let folders = session_folders(&p("D:\\secret\\sub"), None);
        assert_eq!(
            verdict(&list, &p("list.json"), &folders),
            Verdict::Refused(refusal(&p("D:\\secret")))
        );
    }

    #[test]
    fn a_covered_claude_project_dir_is_refused() {
        let list = Ok(vec![p("D:\\secret")]);
        let folders = session_folders(&p("D:\\other"), Some(&p("D:\\secret")));
        assert!(matches!(
            verdict(&list, &p("list.json"), &folders),
            Verdict::Refused(_)
        ));
    }

    #[test]
    fn a_relative_claude_project_dir_is_made_absolute() {
        let folders = session_folders(&p("D:\\work\\app"), Some(&p("..\\secret")));
        assert_eq!(folders, vec![p("D:\\work\\app"), p("D:\\work\\secret")]);
    }

    #[test]
    fn an_uncovered_session_is_open() {
        let list = Ok(vec![p("D:\\secret")]);
        let folders = session_folders(&p("D:\\other"), Some(&p("D:\\fine")));
        assert_eq!(verdict(&list, &p("list.json"), &folders), Verdict::Open);
    }

    #[test]
    fn a_broken_list_refuses_with_the_file_name() {
        let list = Err(cw_core::Error::invalid("bad"));
        let folders = session_folders(&p("D:\\other"), None);
        assert_eq!(
            verdict(&list, &p("D:\\data\\agents-off.json"), &folders),
            Verdict::Refused("CodeWiki's list of folders kept from coding agents (D:\\data\\agents-off.json) cannot be read; fix or delete it. Until then CodeWiki's tools are off for coding agents.".to_string())
        );
    }

    #[test]
    fn repo_args_are_classified() {
        let cwd = p("D:\\work");
        for path in ["D:\\x", "..", "src/x"] {
            assert!(matches!(repo_arg(path, &cwd), RepoArg::Path(_)), "{path}");
        }
        for name in ["cw-core-9bb4c92c", "clients"] {
            assert_eq!(repo_arg(name, &cwd), RepoArg::IdOrName(name.to_string()));
        }
    }

    fn listing() -> Value {
        json!([
            { "id": "cw-core-9bb4c92c", "name": "Clients", "path": "D:\\work\\clients" },
            { "id": "pub-1", "name": "public", "path": "D:\\work\\public" },
        ])
    }

    #[test]
    fn a_missing_path_is_looked_up_by_its_last_component() {
        let cwd = p("D:\\work");
        let missing = |_: &Path| false;
        let covered = vec![p("D:\\work\\clients")];
        for written in [
            "D:\\elsewhere\\clients",
            "./clients",
            "clients/",
            "x/Clients",
        ] {
            let arg = repo_arg(written, &cwd);
            let name = lookup_name(&arg, missing).expect(written);
            let path = resolve_id(&listing(), &name).expect(written);
            assert_eq!(
                cw_core::agents_off::covering(&covered, &path),
                Some(p("D:\\work\\clients")),
                "{written}"
            );
        }
        // An existing folder needs no lookup; an id or name always does.
        let here = repo_arg("D:\\work\\public", &cwd);
        assert_eq!(lookup_name(&here, |_| true), None);
        let bare = repo_arg("pub-1", &cwd);
        assert_eq!(lookup_name(&bare, |_| true), Some("pub-1".to_string()));
    }

    #[test]
    fn the_backend_is_never_given_a_bare_word() {
        let cwd = p("D:\\work");
        let l = listing();
        let word = |w: &str| repo_arg(w, &cwd);
        assert_eq!(
            target(&word("clients"), Some(&l), &cwd),
            Target::Id("cw-core-9bb4c92c".to_string())
        );
        assert_eq!(
            target(&word("pub-1"), Some(&l), &cwd),
            Target::Id("pub-1".to_string())
        );
        // No repository answers: a folder under cwd, with or without a listing.
        assert_eq!(
            target(&word("secret"), Some(&l), &cwd),
            Target::Path(p("D:\\work\\secret"))
        );
        assert_eq!(
            target(&word("clients"), None, &cwd),
            Target::Path(p("D:\\work\\clients"))
        );
        // A relative path becomes absolute too.
        assert_eq!(
            target(&word("./x"), None, &cwd),
            Target::Path(p("D:\\work\\x"))
        );
    }

    #[test]
    fn an_unmatched_word_is_refused_when_its_folder_under_cwd_is_covered() {
        let cwd = p("D:\\work");
        let covered = vec![p("D:\\work\\secret")];
        let arg = repo_arg("secret", &cwd);
        assert_eq!(
            cw_core::agents_off::covering(&covered, &direct_path(&arg, &cwd)),
            Some(p("D:\\work\\secret"))
        );
        let fine = repo_arg("public", &cwd);
        assert_eq!(
            cw_core::agents_off::covering(&covered, &direct_path(&fine, &cwd)),
            None
        );
    }

    #[test]
    fn an_id_or_name_resolves_to_its_path() {
        assert_eq!(resolve_id(&listing(), "pub-1"), Some(p("D:\\work\\public")));
        assert_eq!(
            resolve_id(&listing(), "clients"),
            Some(p("D:\\work\\clients"))
        );
        assert_eq!(resolve_id(&listing(), "nope"), None);
    }

    #[test]
    fn hidden_repos_leave_the_listing() {
        let mut l = listing();
        let hidden = hide_repos(&mut l, &[p("D:\\work\\clients")]);
        assert_eq!(hidden, HashSet::from(["Clients".to_string()]));
        assert_eq!(l.as_array().unwrap().len(), 1);
        assert_eq!(l[0]["name"], "public");
    }

    fn hidden_clients() -> HashSet<String> {
        HashSet::from(["Clients".to_string()])
    }

    #[test]
    fn hidden_repos_leave_the_workspace_and_shared_dependencies() {
        let mut w = json!({
            "repos": [{ "name": "Clients" }, { "name": "public" }, { "name": "tools" }],
            "shared_dependencies": [
                { "name": "serde", "repos": ["Clients", "public", "tools"] },
                { "name": "rand", "repos": ["Clients", "public"] },
            ],
        });
        hide_in_workspace(&mut w, &hidden_clients());
        assert_eq!(w["repos"].as_array().unwrap().len(), 2);
        assert_eq!(w["shared_dependencies"].as_array().unwrap().len(), 1);
        assert_eq!(
            w["shared_dependencies"][0]["repos"],
            json!(["public", "tools"])
        );
    }

    #[test]
    fn hidden_repos_leave_contract_matches() {
        let mut c = json!({
            "repos": [{ "repo": "Clients" }, { "repo": "public" }],
            "errors": [{ "repo": "Clients", "error": "x" }],
            "matched": [
                { "path": "/a", "repo": "Clients", "providers": [{ "repo": "public" }] },
                { "path": "/b", "repo": "public", "file": "b.rs", "line": 7, "providers": [{ "repo": "Clients" }] },
                { "path": "/c", "repo": "public", "providers": [{ "repo": "Clients" }, { "repo": "tools" }] },
            ],
            "unprovided": [{ "repo": "Clients", "path": "/d" }, { "repo": "public", "path": "/e" }],
            "unconsumed": [{ "repo": "Clients", "path": "/f" }],
        });
        hide_in_contracts(&mut c, &hidden_clients());
        assert_eq!(c["repos"].as_array().unwrap().len(), 1);
        assert!(c["errors"].as_array().unwrap().is_empty());
        assert_eq!(c["matched"].as_array().unwrap().len(), 1);
        assert_eq!(c["matched"][0]["providers"], json!([{ "repo": "tools" }]));
        assert_eq!(c["unprovided"].as_array().unwrap().len(), 2);
        assert_eq!(
            c["unprovided"][1],
            json!({ "repo": "public", "path": "/b", "file": "b.rs", "line": 7 })
        );
        assert!(c["unconsumed"].as_array().unwrap().is_empty());
    }
}
