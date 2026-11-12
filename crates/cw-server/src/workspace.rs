//! Views across every repository the backend manages.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use cw_core::{Error, Result};
use cw_wiki::contracts::{self, RepoContracts, WorkspaceContracts};
use cw_wiki::gitintel::{self, Hotspot};
use cw_wiki::{query, service};
use serde::{Deserialize, Serialize};

use crate::repos::{self, RepoEntry};

/// Hotspots listed per repository.
const TOP_HOTSPOTS: usize = 3;

/// One repository in the workspace view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRepo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub files: usize,
    pub symbols: usize,
    pub notes: usize,
    pub pending_proposals: usize,
    pub stale: bool,
    /// The files that are both largest and most often changed.
    pub hotspots: Vec<Hotspot>,
    /// Why the repository could not be read; the counts are then zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// An external package more than one repository uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedDependency {
    pub name: String,
    /// Names of the repositories that use it.
    pub repos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub repos: Vec<WorkspaceRepo>,
    pub shared_dependencies: Vec<SharedDependency>,
}

/// A repository left out of a workspace result, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoError {
    pub repo: String,
    pub error: String,
}

/// Contracts matched across the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContractsView {
    #[serde(flatten)]
    pub contracts: WorkspaceContracts,
    /// Repositories that could not be read.
    pub errors: Vec<RepoError>,
}

/// The index of a registered repository, or a clear error when its folder is
/// gone.
fn open(data_dir: &Path, entry: &RepoEntry) -> Result<cw_index::Index> {
    let root = Path::new(&entry.path);
    if !root.is_dir() {
        return Err(Error::not_found(format!("folder '{}'", entry.path)));
    }
    service::open_index_in(data_dir, root)
}

/// Counts, hotspots and external packages of one repository.
fn summarize(data_dir: &Path, entry: &RepoEntry) -> Result<(WorkspaceRepo, BTreeSet<String>)> {
    let root = Path::new(&entry.path);
    let index = open(data_dir, entry)?;
    let status = query::status(root, &index)?;
    let hotspots = gitintel::hotspots(&index, &gitintel::load(&index)?, TOP_HOTSPOTS)?;
    let crates = index.crates()?;
    let own: BTreeSet<&str> = crates.iter().map(|c| c.name.as_str()).collect();
    // Declared dependencies, plus crates the source imports without declaring
    // them in a manifest the scan understood.
    let external: BTreeSet<String> = crates
        .iter()
        .flat_map(|c| c.deps.iter().cloned())
        .chain(index.external_uses()?.into_iter().map(|u| u.krate))
        .filter(|name| !own.contains(name.as_str()))
        .collect();
    let repo = WorkspaceRepo {
        id: entry.id.clone(),
        name: entry.name.clone(),
        path: entry.path.clone(),
        files: status.files,
        symbols: status.symbols,
        notes: status.notes,
        pending_proposals: status.pending_proposals,
        stale: status.stale,
        hotspots,
        error: None,
    };
    Ok((repo, external))
}

/// One entry per registered repository, and the external packages two or
/// more of them use. A repository that cannot be read is listed with its
/// error instead of failing the call.
pub fn workspace(data_dir: &Path) -> Result<Workspace> {
    let mut repos = Vec::new();
    let mut users: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for entry in repos::load(data_dir)? {
        match summarize(data_dir, &entry) {
            Ok((repo, external)) => {
                for name in external {
                    users.entry(name).or_default().insert(entry.id.clone());
                }
                repos.push(repo);
            }
            Err(e) => repos.push(WorkspaceRepo {
                id: entry.id,
                name: entry.name,
                path: entry.path,
                files: 0,
                symbols: 0,
                notes: 0,
                pending_proposals: 0,
                stale: false,
                hotspots: Vec::new(),
                error: Some(e.to_string()),
            }),
        }
    }
    let name_of = |id: &String| {
        repos
            .iter()
            .find(|r| r.id == *id)
            .map_or_else(|| id.clone(), |r| r.name.clone())
    };
    let shared_dependencies = users
        .iter()
        .filter(|(_, ids)| ids.len() >= 2)
        .map(|(name, ids)| SharedDependency {
            name: name.clone(),
            repos: ids.iter().map(name_of).collect(),
        })
        .collect();
    Ok(Workspace {
        repos,
        shared_dependencies,
    })
}

/// HTTP contracts of one registered repository.
pub fn repo_contracts(data_dir: &Path, entry: &RepoEntry) -> Result<RepoContracts> {
    contracts::scan(Path::new(&entry.path), &open(data_dir, entry)?)
}

/// Contracts of every registered repository, matched against each other.
pub fn workspace_contracts(data_dir: &Path) -> Result<WorkspaceContractsView> {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    for entry in repos::load(data_dir)? {
        match repo_contracts(data_dir, &entry) {
            Ok(contracts) => found.push((entry.name, contracts)),
            Err(e) => errors.push(RepoError {
                repo: entry.name,
                error: e.to_string(),
            }),
        }
    }
    Ok(WorkspaceContractsView {
        contracts: contracts::match_workspace(&found),
        errors,
    })
}
