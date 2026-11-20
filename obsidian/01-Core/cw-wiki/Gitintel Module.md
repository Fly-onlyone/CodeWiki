---
tags: [core, cw-wiki, module]
---

# Gitintel Module

> Git intelligence: what history says about each file.

## What it does

Churn, ownership, bus factor and co-change are mined from `git log`. They are combined with the dependency graph into a risk score that an agent or a reviewer can read before changing code.

## Source

- `crates/cw-wiki/src/gitintel.rs` — module `cw_wiki::gitintel`

## How it works

1. `run` in [[codewiki]] and `change_risk` in [[Api Module]], and 8 more, call `load`.
2. `run` in [[codewiki]] and `hotspots` in [[Api Module]], and 4 more, call `hotspots`.
3. `Facts::load` in [[Refactor Module]] and `gather` in [[Testmap Module]] call `dependents_by_file`.
4. `run` in [[codewiki]] and `file_risk` in [[Api Module]] call `file_risk`. `file_risk` calls `blast_radius`, then `level`.
5. `run` in [[codewiki]] and `change_risk` in [[Api Module]], and 1 more, call `working_tree_changes`.

4 more entry points: `change_risk`, `mine`, `save`, `blast_radius`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const META_KEY: &str` — meta key under which the mined history is cached in the index.
- `pub struct Owner` — public struct; used by `FileHistory`, `analyze`.
- `pub struct Partner` — public struct; used by `ChangeRisk`, `FileHistory`, `analyze`, `change_risk`.
- `pub struct FileHistory` — public struct; used by `FileRisk`, `GitIntel`, `analyze`.
- `pub struct GitIntel` — public struct; used by `analyze`, `build`, `change_risk`, `file_risk` and 5 more.
- `pub fn mine(root: &Path) -> GitIntel` — mine history for the repository at `root`. Returns empty data outside a git repository.
- `pub fn save(index: &Index, intel: &GitIntel) -> Result<()>` — store mined history in the index.
- `pub fn load(index: &Index) -> Result<GitIntel>` — load mined history from the index; empty when none was stored.
- `pub struct Hotspot` — public struct; used by `WorkspaceRepo`, `hotspots`.
- `pub fn hotspots(index: &Index, intel: &GitIntel, limit: usize) -> Result<Vec<Hotspot>>` — files that are both large and frequently changed, highest first. These are where defects concentrate and where refactoring pays off.
- `pub(crate) fn dependents_by_file(index: &Index) -> Result<HashMap<String, BTreeSet<String>>>` — File-level reverse dependencies: file -> files that depend on it.
- `pub struct BlastRadius` — public struct; used by `blast_radius`.
- `pub fn blast_radius(index: &Index, path: &str) -> Result<BlastRadius>` — files affected if `path` changes its behavior.
- `pub enum RiskLevel` — public enum; used by `ChangeRisk`, `FileRisk`, `level`.
- `pub struct FileRisk` — public struct; used by `ChangeRisk`, `file_risk`.
- `fn level(score: u32) -> RiskLevel` — private function, returns `RiskLevel`; called by `change_risk`, `file_risk`.
- `pub fn file_risk(index: &Index, intel: &GitIntel, path: &str) -> Result<FileRisk>` — risk of changing one file: how often it changes, how many people know it, and how much depends on it.
- `pub struct ChangeRisk` — public struct; used by `change_risk`.
- `pub fn working_tree_changes(root: &Path) -> Vec<String>` — paths changed in the working tree relative to `HEAD`, including untracked files.
- `pub fn change_risk(index: &Index, intel: &GitIntel, changed: &[String]) -> Result<ChangeRisk>` — risk of a set of changed files taken together.

And 8 more items.

## Depends on

- [[Index]] — uses `Index`, `edges`, `files`, `meta`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `external`

## Used by

- [[Api Module]] — from `blast_radius`, `change_risk`, `file_risk`, `history`
- [[cw-wiki]] — from `blast_radius_follows_dependents`, `change_risk_flags_forgotten_partners_and_reads_the_working_tree`, `file_risk_explains_itself`, `mines_history_and_ranks_hotspots`
- [[Workspace Module (cw-server)]] — from `WorkspaceRepo`, `summarize`
- [[Decisions Module]] — from `build`, `list`
- [[codewiki]] — from `run`
- [[Agentdocs Module]] — from `render`
- [[Coverage Module]] — from `report`
- [[Refactor Module]] — from `load`

## See also

- [[_index]]
- [[cw-wiki]]
