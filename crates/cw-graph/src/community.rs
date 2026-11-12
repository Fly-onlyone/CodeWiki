//! Community detection.
//!
//! Implements the Louvain method (Blondel et al., "Fast unfolding of
//! communities in large networks", 2008) from the paper. Nodes are visited in
//! index order and ties go to the lowest community id, so results are stable.

use std::collections::BTreeMap;

/// Upper bound on local-moving sweeps per level; real graphs settle far sooner.
const MAX_SWEEPS: usize = 100;

/// Partition a symmetric weighted adjacency list. Returns a community id per
/// node, numbered in order of each community's lowest node index.
pub fn louvain(adjacency: &[Vec<(usize, f64)>]) -> Vec<usize> {
    let n = adjacency.len();
    let mut membership: Vec<usize> = (0..n).collect();
    let mut adj: Vec<Vec<(usize, f64)>> = adjacency.to_vec();

    loop {
        let (assignment, moved) = local_moving(&adj);
        if !moved {
            break;
        }
        let (renumbered, count) = renumber(&assignment);
        for m in &mut membership {
            *m = renumbered[*m];
        }
        // Aggregate: one node per community. Internal weight becomes a self
        // loop so node strengths are preserved at the next level.
        let mut next: Vec<BTreeMap<usize, f64>> = vec![BTreeMap::new(); count];
        for (i, list) in adj.iter().enumerate() {
            for &(j, w) in list {
                *next[renumbered[i]].entry(renumbered[j]).or_insert(0.0) += w;
            }
        }
        adj = next.into_iter().map(|m| m.into_iter().collect()).collect();
        if count == 1 {
            break;
        }
    }
    renumber(&membership).0
}

/// One level of greedy modularity optimization.
fn local_moving(adj: &[Vec<(usize, f64)>]) -> (Vec<usize>, bool) {
    let n = adj.len();
    let strength: Vec<f64> = adj.iter().map(|l| l.iter().map(|(_, w)| w).sum()).collect();
    let total: f64 = strength.iter().sum();
    let mut community: Vec<usize> = (0..n).collect();
    if total == 0.0 {
        return (community, false);
    }
    let mut community_total = strength.clone();
    let mut any_move = false;

    for _ in 0..MAX_SWEEPS {
        let mut moved = false;
        for i in 0..n {
            let own = community[i];
            let mut links: BTreeMap<usize, f64> = BTreeMap::new();
            for &(j, w) in &adj[i] {
                if j != i {
                    *links.entry(community[j]).or_insert(0.0) += w;
                }
            }
            community_total[own] -= strength[i];
            let gain = |c: usize, w: f64| w - community_total[c] * strength[i] / total;
            let mut best = own;
            let mut best_gain = gain(own, links.get(&own).copied().unwrap_or(0.0));
            for (&c, &w) in &links {
                let g = gain(c, w);
                if g > best_gain + 1e-12 {
                    best = c;
                    best_gain = g;
                }
            }
            community_total[best] += strength[i];
            if best != own {
                community[i] = best;
                moved = true;
                any_move = true;
            }
        }
        if !moved {
            break;
        }
    }
    (community, any_move)
}

/// Map arbitrary ids to `0..count` in order of first appearance.
fn renumber(ids: &[usize]) -> (Vec<usize>, usize) {
    let mut map: BTreeMap<usize, usize> = BTreeMap::new();
    let mut out = Vec::with_capacity(ids.len());
    for &id in ids {
        let next = map.len();
        out.push(*map.entry(id).or_insert(next));
    }
    (out, map.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symmetric(n: usize, edges: &[(usize, usize)]) -> Vec<Vec<(usize, f64)>> {
        let mut adj = vec![Vec::new(); n];
        for &(a, b) in edges {
            adj[a].push((b, 1.0));
            adj[b].push((a, 1.0));
        }
        adj
    }

    #[test]
    fn separates_two_cliques_joined_by_a_bridge() {
        let mut edges = Vec::new();
        for group in [[0, 1, 2, 3], [4, 5, 6, 7]] {
            for a in 0..4 {
                for b in a + 1..4 {
                    edges.push((group[a], group[b]));
                }
            }
        }
        edges.push((3, 4));
        let c = louvain(&symmetric(8, &edges));
        assert_eq!(&c[..4], [0, 0, 0, 0]);
        assert_eq!(&c[4..], [1, 1, 1, 1]);
    }

    #[test]
    fn isolated_nodes_stay_alone() {
        let c = louvain(&symmetric(3, &[(0, 1)]));
        assert_eq!(c, [0, 0, 1]);
        assert!(louvain(&[]).is_empty());
    }
}
