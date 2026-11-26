---
tags: [core, cw-wiki, module]
---

# Gitintel Module

> Git intelligence: what history says about each file.

## What it does

History tells which files are risky to touch. This module mines `git log` for churn, authors, bus factor (how few people know a file) and files that usually change together. It combines them with the dependency graph into a risk score for one file or a whole change. Agents and reviewers read the score before editing.

## Source

- `crates/cw-wiki/src/gitintel.rs` — module `cw_wiki::gitintel`

## How it works

`mine` runs `git log --no-merges --numstat -M` over the last 3000 commits and parses it. Per file it counts commits, lines changed, last change time and commits per author. Rename notation is resolved so history follows the new path. Binary files count as one changed line.

`analyze` then derives:

- Owners, sorted by commit count. The bus factor is the smallest number of top authors whose commits exceed half.
- Partners: pairs of files in the same commit. Commits touching fewer than 2 or more than 30 files are ignored (bulk edits). A pair needs at least 3 co-changes. Strength is together count divided by the commits of the rarer file. At most 8 partners are kept per file.

`save` stores the result as JSON in the index, and `load` reads it back (empty if absent or unreadable).

`hotspots` scores each file as commits times the log of its line count, scaled so the top file is 100.

`dependents_by_file` builds reverse dependencies between files from import edges and call, reference and implement edges. `blast_radius` walks them breadth first: direct dependents, then those reached through more hops.

`file_risk` adds points to a 0 to 100 score: up to 40 for reach, up to 35 for churn against the busiest file, 15 for a single author with at least 5 commits, 10 for a partner with strength 0.6 or more. Levels: low to 34, medium to 64, high above. `change_risk` takes the riskiest file, adds a breadth term and up to 15 points for partners missing from the change.

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

## Gotchas

- Only the last 3000 commits are read, so old history does not count.
- Outside a git repository `mine` returns empty data, and every file then reads as having no history.
- `working_tree_changes` includes untracked files and reads the new name of a rename.
- Missing partners need strength 0.5 or more, a stricter test than the 0.6 used for the score.

## See also

- [[_index]]
- [[cw-wiki]]
