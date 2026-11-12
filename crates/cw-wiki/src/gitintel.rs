//! Git intelligence: what history says about each file.
//!
//! Churn, ownership, bus factor and co-change are mined from `git log`. They
//! are combined with the dependency graph into a risk score that an agent or a
//! reviewer can read before changing code.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;
use std::process::Command;

use cw_core::{EdgeKind, Result};
use cw_index::Index;
use serde::{Deserialize, Serialize};

/// Commits examined; older history says little about today's risk.
const MAX_COMMITS: usize = 3000;
/// Commits touching more files than this are bulk edits (formatting, renames)
/// and would link unrelated files.
const MAX_FILES_FOR_COUPLING: usize = 30;
/// Co-changes below this count are coincidence.
const MIN_CO_CHANGES: u32 = 3;
const MAX_PARTNERS: usize = 8;
/// Meta key under which the mined history is cached in the index.
pub const META_KEY: &str = "git_intel";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Owner {
    pub name: String,
    pub commits: u32,
    /// Share of the file's commits, 0.0 to 1.0.
    pub share: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Partner {
    pub path: String,
    pub together: u32,
    /// Share of the rarer file's commits that included both, 0.0 to 1.0.
    pub strength: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileHistory {
    pub path: String,
    pub commits: u32,
    pub lines_changed: u64,
    /// Unix time of the most recent commit touching the file.
    pub last_changed: i64,
    /// Authors by commit count, largest first.
    pub owners: Vec<Owner>,
    /// Smallest number of authors who together made over half the commits.
    pub bus_factor: u32,
    /// Files that usually change in the same commit.
    pub partners: Vec<Partner>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GitIntel {
    pub commits_scanned: usize,
    /// Keyed by repo-relative path.
    pub files: BTreeMap<String, FileHistory>,
}

struct Commit {
    author: String,
    time: i64,
    /// `(path, lines added + deleted)`.
    files: Vec<(String, u64)>,
}

/// Parse `git log --numstat` output produced with the format in [`mine`].
fn parse_log(text: &str) -> Vec<Commit> {
    let mut commits = Vec::new();
    for block in text.split('\u{1}').skip(1) {
        let mut lines = block.lines();
        let Some(header) = lines.next() else {
            continue;
        };
        let mut parts = header.split('\u{2}');
        let (Some(author), Some(time)) = (parts.next(), parts.next()) else {
            continue;
        };
        let mut files = Vec::new();
        for line in lines {
            let mut cols = line.splitn(3, '\t');
            let (Some(added), Some(deleted), Some(path)) = (cols.next(), cols.next(), cols.next())
            else {
                continue;
            };
            // Binary files report `-`; count them as one changed line.
            let count = |s: &str| s.parse::<u64>().unwrap_or(1);
            // Renames appear as `old => new` or `dir/{old => new}/file`.
            files.push((rename_target(path), count(added) + count(deleted)));
        }
        commits.push(Commit {
            author: author.trim().to_string(),
            time: time.trim().parse().unwrap_or(0),
            files,
        });
    }
    commits
}

/// Path after a rename, from git's `a/{b => c}/d` or `a => b` notation.
fn rename_target(path: &str) -> String {
    let Some(arrow) = path.find(" => ") else {
        return path.to_string();
    };
    match (path[..arrow].rfind('{'), path[arrow..].find('}')) {
        (Some(open), Some(close)) => {
            let close = arrow + close;
            let joined = format!(
                "{}{}{}",
                &path[..open],
                &path[arrow + 4..close],
                &path[close + 1..]
            );
            joined.replace("//", "/")
        }
        _ => path[arrow + 4..].to_string(),
    }
}

/// Mine history for the repository at `root`. Returns empty data outside a
/// git repository.
pub fn mine(root: &Path) -> GitIntel {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "log",
            "--no-merges",
            "--numstat",
            "-M",
            &format!("-n{MAX_COMMITS}"),
            "--format=%x01%an%x02%at",
        ])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            analyze(&parse_log(&String::from_utf8_lossy(&out.stdout)))
        }
        _ => GitIntel::default(),
    }
}

fn analyze(commits: &[Commit]) -> GitIntel {
    struct Acc {
        commits: u32,
        lines: u64,
        last: i64,
        authors: BTreeMap<String, u32>,
    }
    let mut files: BTreeMap<&str, Acc> = BTreeMap::new();
    let mut pairs: HashMap<(&str, &str), u32> = HashMap::new();
    for commit in commits {
        for (path, lines) in &commit.files {
            let acc = files.entry(path).or_insert(Acc {
                commits: 0,
                lines: 0,
                last: 0,
                authors: BTreeMap::new(),
            });
            acc.commits += 1;
            acc.lines += lines;
            acc.last = acc.last.max(commit.time);
            *acc.authors.entry(commit.author.clone()).or_insert(0) += 1;
        }
        if commit.files.len() >= 2 && commit.files.len() <= MAX_FILES_FOR_COUPLING {
            let mut names: Vec<&str> = commit.files.iter().map(|(p, _)| p.as_str()).collect();
            names.sort_unstable();
            names.dedup();
            for (i, a) in names.iter().enumerate() {
                for b in &names[i + 1..] {
                    *pairs.entry((a, b)).or_insert(0) += 1;
                }
            }
        }
    }

    let mut partners: HashMap<&str, Vec<Partner>> = HashMap::new();
    for ((a, b), together) in &pairs {
        if *together < MIN_CO_CHANGES {
            continue;
        }
        let rarer = files[a].commits.min(files[b].commits).max(1);
        let strength = *together as f64 / rarer as f64;
        for (from, to) in [(a, b), (b, a)] {
            partners.entry(from).or_default().push(Partner {
                path: to.to_string(),
                together: *together,
                strength,
            });
        }
    }

    let files = files
        .into_iter()
        .map(|(path, acc)| {
            let mut owners: Vec<Owner> = acc
                .authors
                .iter()
                .map(|(name, commits)| Owner {
                    name: name.clone(),
                    commits: *commits,
                    share: *commits as f64 / acc.commits as f64,
                })
                .collect();
            owners.sort_by(|a, b| b.commits.cmp(&a.commits).then(a.name.cmp(&b.name)));
            let mut covered = 0u32;
            let mut bus_factor = 0u32;
            for owner in &owners {
                covered += owner.commits;
                bus_factor += 1;
                if covered * 2 > acc.commits {
                    break;
                }
            }
            let mut linked = partners.remove(path).unwrap_or_default();
            linked.sort_by(|a, b| {
                b.together
                    .cmp(&a.together)
                    .then(b.strength.total_cmp(&a.strength))
                    .then(a.path.cmp(&b.path))
            });
            linked.truncate(MAX_PARTNERS);
            (
                path.to_string(),
                FileHistory {
                    path: path.to_string(),
                    commits: acc.commits,
                    lines_changed: acc.lines,
                    last_changed: acc.last,
                    owners,
                    bus_factor,
                    partners: linked,
                },
            )
        })
        .collect();
    GitIntel {
        commits_scanned: commits.len(),
        files,
    }
}

/// Store mined history in the index.
pub fn save(index: &Index, intel: &GitIntel) -> Result<()> {
    let json = serde_json::to_string(intel)
        .map_err(|e| cw_core::Error::external("encoding git history", e))?;
    index.set_meta(META_KEY, &json)
}

/// Load mined history from the index; empty when none was stored.
pub fn load(index: &Index) -> Result<GitIntel> {
    Ok(index
        .meta(META_KEY)?
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hotspot {
    pub path: String,
    pub commits: u32,
    pub lines: u32,
    /// Churn times size, scaled so the top file is 100.
    pub score: f64,
    pub top_owner: Option<String>,
    pub bus_factor: u32,
}

/// Files that are both large and frequently changed, highest first. These are
/// where defects concentrate and where refactoring pays off.
pub fn hotspots(index: &Index, intel: &GitIntel, limit: usize) -> Result<Vec<Hotspot>> {
    let mut rows: Vec<Hotspot> = index
        .files()?
        .into_iter()
        .filter_map(|file| {
            let history = intel.files.get(&file.path)?;
            Some(Hotspot {
                score: history.commits as f64 * (file.lines.max(1) as f64).ln_1p(),
                path: file.path,
                commits: history.commits,
                lines: file.lines,
                top_owner: history.owners.first().map(|o| o.name.clone()),
                bus_factor: history.bus_factor,
            })
        })
        .collect();
    let top = rows.iter().map(|r| r.score).fold(0.0f64, f64::max);
    if top > 0.0 {
        for row in &mut rows {
            row.score = (row.score / top * 1000.0).round() / 10.0;
        }
    }
    rows.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.path.cmp(&b.path)));
    rows.truncate(limit);
    Ok(rows)
}

/// File-level reverse dependencies: file -> files that depend on it.
pub(crate) fn dependents_by_file(index: &Index) -> Result<HashMap<String, BTreeSet<String>>> {
    let file_of: HashMap<String, String> = index
        .symbols()?
        .into_iter()
        .map(|s| (s.qualified, s.file))
        .collect();
    let mut reverse: HashMap<String, BTreeSet<String>> = HashMap::new();
    for edge in index.edges()? {
        let (src, dst) = match edge.kind {
            EdgeKind::Imports => (
                edge.src.strip_prefix("file:").map(str::to_string),
                edge.dst.strip_prefix("file:").map(str::to_string),
            ),
            EdgeKind::Calls | EdgeKind::References | EdgeKind::Implements => (
                file_of.get(&edge.src).cloned(),
                file_of.get(&edge.dst).cloned(),
            ),
            EdgeKind::Declares => (None, None),
        };
        if let (Some(src), Some(dst)) = (src, dst)
            && src != dst
        {
            reverse.entry(dst).or_default().insert(src);
        }
    }
    Ok(reverse)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlastRadius {
    pub path: String,
    /// Files that depend on `path` directly.
    pub direct: Vec<String>,
    /// Files reached through further hops, nearest first.
    pub transitive: Vec<String>,
}

/// Files affected if `path` changes its behavior.
pub fn blast_radius(index: &Index, path: &str) -> Result<BlastRadius> {
    let reverse = dependents_by_file(index)?;
    let direct: Vec<String> = reverse
        .get(path)
        .map(|s| s.iter().cloned().collect())
        .unwrap_or_default();
    let mut seen: HashSet<&str> = HashSet::from([path]);
    seen.extend(direct.iter().map(String::as_str));
    let mut queue: VecDeque<&str> = direct.iter().map(String::as_str).collect();
    let mut transitive = Vec::new();
    while let Some(current) = queue.pop_front() {
        for next in reverse.get(current).into_iter().flatten() {
            if seen.insert(next) {
                transitive.push(next.clone());
                queue.push_back(next);
            }
        }
    }
    Ok(BlastRadius {
        path: path.to_string(),
        direct,
        transitive,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileRisk {
    pub path: String,
    /// 0 to 100.
    pub score: u32,
    pub level: RiskLevel,
    /// Why the score is what it is, most important first.
    pub reasons: Vec<String>,
    pub history: Option<FileHistory>,
    pub dependents: usize,
}

fn level(score: u32) -> RiskLevel {
    match score {
        0..=34 => RiskLevel::Low,
        35..=64 => RiskLevel::Medium,
        _ => RiskLevel::High,
    }
}

/// Risk of changing one file: how often it changes, how many people know it,
/// and how much depends on it.
pub fn file_risk(index: &Index, intel: &GitIntel, path: &str) -> Result<FileRisk> {
    let radius = blast_radius(index, path)?;
    let dependents = radius.direct.len() + radius.transitive.len();
    let history = intel.files.get(path).cloned();
    let busiest = intel
        .files
        .values()
        .map(|f| f.commits)
        .max()
        .unwrap_or(1)
        .max(1);
    let mut score = 0.0f64;
    let mut reasons = Vec::new();

    // Up to 40 points for reach.
    let reach = (dependents as f64).ln_1p() / 5.0f64.ln_1p().max(1.0);
    score += (reach * 20.0).min(40.0);
    if dependents > 0 {
        reasons.push(format!(
            "{} files depend on it ({} directly)",
            dependents,
            radius.direct.len()
        ));
    }
    if let Some(h) = &history {
        // Up to 35 points for churn relative to the busiest file.
        let churn = h.commits as f64 / busiest as f64;
        score += churn * 35.0;
        if churn >= 0.5 {
            reasons.push(format!(
                "changes often: {} commits, among the busiest files",
                h.commits
            ));
        }
        // 15 points when one person holds the knowledge of a file with history.
        if h.bus_factor == 1 && h.commits >= 5 {
            score += 15.0;
            if let Some(owner) = h.owners.first() {
                reasons.push(format!(
                    "one main author ({} wrote {:.0}% of commits)",
                    owner.name,
                    owner.share * 100.0
                ));
            }
        }
        // 10 points for strong hidden coupling.
        if let Some(partner) = h.partners.iter().find(|p| p.strength >= 0.6) {
            score += 10.0;
            reasons.push(format!(
                "usually changes together with `{}` ({} times)",
                partner.path, partner.together
            ));
        }
    } else {
        reasons.push("no commit history yet".to_string());
    }
    let score = score.round().clamp(0.0, 100.0) as u32;
    Ok(FileRisk {
        path: path.to_string(),
        score,
        level: level(score),
        reasons,
        history,
        dependents,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeRisk {
    pub level: RiskLevel,
    pub score: u32,
    pub files: Vec<FileRisk>,
    /// Files that usually change with the changed files but are not in the change.
    pub missing_partners: Vec<Partner>,
    /// Files outside the change that depend on it.
    pub affected: Vec<String>,
}

/// Paths changed in the working tree relative to `HEAD`, including untracked files.
pub fn working_tree_changes(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output();
    let Ok(out) = output else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.len() > 3)
        // Lines read `XY path`, or `XY old -> new` for a rename.
        .map(|l| {
            let path = l[3..].rsplit(" -> ").next().unwrap_or("");
            path.trim().trim_matches('"').to_string()
        })
        .filter(|p| !p.is_empty())
        .collect()
}

/// Risk of a set of changed files taken together.
pub fn change_risk(index: &Index, intel: &GitIntel, changed: &[String]) -> Result<ChangeRisk> {
    let in_change: HashSet<&str> = changed.iter().map(String::as_str).collect();
    let mut files = Vec::new();
    let mut missing: BTreeMap<String, Partner> = BTreeMap::new();
    let mut affected: BTreeSet<String> = BTreeSet::new();
    for path in changed {
        let risk = file_risk(index, intel, path)?;
        if let Some(history) = &risk.history {
            for partner in history.partners.iter().filter(|p| p.strength >= 0.5) {
                if !in_change.contains(partner.path.as_str()) {
                    let entry = missing
                        .entry(partner.path.clone())
                        .or_insert_with(|| partner.clone());
                    if partner.together > entry.together {
                        *entry = partner.clone();
                    }
                }
            }
        }
        let radius = blast_radius(index, path)?;
        affected.extend(
            radius
                .direct
                .into_iter()
                .chain(radius.transitive)
                .filter(|p| !in_change.contains(p.as_str())),
        );
        files.push(risk);
    }
    files.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));
    // The riskiest file dominates; breadth and forgotten partners add to it.
    let top = files.first().map(|f| f.score).unwrap_or(0) as f64;
    let breadth = (files.len() as f64).ln_1p() * 4.0;
    let forgotten = (missing.len() as f64 * 5.0).min(15.0);
    let score = (top + breadth + forgotten).round().clamp(0.0, 100.0) as u32;
    let mut missing_partners: Vec<Partner> = missing.into_values().collect();
    missing_partners.sort_by(|a, b| b.together.cmp(&a.together).then(a.path.cmp(&b.path)));
    Ok(ChangeRisk {
        level: level(score),
        score,
        files,
        missing_partners,
        affected: affected.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(author: &str, time: i64, files: &[&str]) -> Commit {
        Commit {
            author: author.into(),
            time,
            files: files.iter().map(|f| (f.to_string(), 10)).collect(),
        }
    }

    #[test]
    fn parses_numstat_log() {
        let log = "\u{1}Ann\u{2}1700000000\n\n3\t1\tsrc/a.rs\n-\t-\tlogo.png\n\u{1}Bo\u{2}1700000100\n\n5\t0\tsrc/{old => new}/b.rs\n2\t2\told.rs => new.rs\n";
        let commits = parse_log(log);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].author, "Ann");
        assert_eq!(
            commits[0].files,
            [("src/a.rs".to_string(), 4), ("logo.png".to_string(), 2)]
        );
        assert_eq!(commits[1].time, 1700000100);
        assert_eq!(commits[1].files[0].0, "src/new/b.rs");
        assert_eq!(commits[1].files[1].0, "new.rs");
    }

    #[test]
    fn resolves_rename_notation() {
        assert_eq!(rename_target("a/{b => c}/d.rs"), "a/c/d.rs");
        assert_eq!(rename_target("a/{ => sub}/d.rs"), "a/sub/d.rs");
        assert_eq!(rename_target("x.rs => y.rs"), "y.rs");
        assert_eq!(rename_target("plain.rs"), "plain.rs");
    }

    #[test]
    fn computes_ownership_bus_factor_and_coupling() {
        let commits = vec![
            commit("Ann", 10, &["core.rs", "api.rs"]),
            commit("Ann", 20, &["core.rs", "api.rs"]),
            commit("Ann", 30, &["core.rs", "api.rs"]),
            commit("Bo", 40, &["core.rs"]),
            commit("Bo", 50, &["docs.md"]),
            commit("Cy", 60, &["docs.md"]),
        ];
        let intel = analyze(&commits);
        assert_eq!(intel.commits_scanned, 6);
        let core = &intel.files["core.rs"];
        assert_eq!(
            (core.commits, core.last_changed, core.bus_factor),
            (4, 40, 1)
        );
        assert_eq!(core.owners[0].name, "Ann");
        assert_eq!(core.owners[0].share, 0.75);
        assert_eq!(core.partners.len(), 1);
        assert_eq!(
            (core.partners[0].path.as_str(), core.partners[0].together),
            ("api.rs", 3)
        );
        assert_eq!(core.partners[0].strength, 1.0);
        // Two authors with one commit each: both are needed to pass half.
        assert_eq!(intel.files["docs.md"].bus_factor, 2);
        assert!(intel.files["docs.md"].partners.is_empty());
    }

    #[test]
    fn bulk_commits_do_not_create_coupling() {
        let many: Vec<String> = (0..40).map(|i| format!("f{i}.rs")).collect();
        let refs: Vec<&str> = many.iter().map(String::as_str).collect();
        let commits = vec![
            commit("Ann", 1, &refs),
            commit("Ann", 2, &refs),
            commit("Ann", 3, &refs),
        ];
        let intel = analyze(&commits);
        assert!(intel.files.values().all(|f| f.partners.is_empty()));
    }

    #[test]
    fn risk_levels_follow_the_score() {
        assert_eq!(level(0), RiskLevel::Low);
        assert_eq!(level(35), RiskLevel::Medium);
        assert_eq!(level(65), RiskLevel::High);
        assert!(RiskLevel::High > RiskLevel::Low);
    }
}
