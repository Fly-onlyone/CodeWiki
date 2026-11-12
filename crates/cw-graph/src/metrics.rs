//! Centrality and connectivity metrics.

use std::collections::VecDeque;

use crate::graph::Graph;

/// Weighted PageRank by power iteration. Scores sum to 1.
///
/// An edge `a -> b` means "a depends on b", so rank flows to the things most
/// depended upon. Rank held by nodes with no outgoing edges is spread evenly.
pub fn pagerank(g: &Graph, damping: f64, max_iterations: usize, tolerance: f64) -> Vec<f64> {
    let n = g.len();
    if n == 0 {
        return Vec::new();
    }
    let out_weight: Vec<f64> = g
        .out
        .iter()
        .map(|l| l.iter().map(|(_, w)| w).sum())
        .collect();
    let mut rank = vec![1.0 / n as f64; n];
    for _ in 0..max_iterations {
        let dangling: f64 = (0..n)
            .filter(|&i| out_weight[i] == 0.0)
            .map(|i| rank[i])
            .sum();
        let base = (1.0 - damping) / n as f64 + damping * dangling / n as f64;
        let mut next = vec![base; n];
        for (v, incoming) in g.inc.iter().enumerate() {
            let flow: f64 = incoming
                .iter()
                .map(|&(u, w)| rank[u] * w / out_weight[u])
                .sum();
            next[v] += damping * flow;
        }
        let delta: f64 = next.iter().zip(&rank).map(|(a, b)| (a - b).abs()).sum();
        rank = next;
        if delta < tolerance {
            break;
        }
    }
    rank
}

/// Tarjan's algorithm, iterative. Returns a component id per node; ids are
/// numbered in order of each component's lowest node index.
pub fn strongly_connected_components(g: &Graph) -> Vec<usize> {
    const UNVISITED: usize = usize::MAX;
    let n = g.len();
    let mut index = vec![UNVISITED; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut comp = vec![UNVISITED; n];
    let mut next_index = 0usize;
    let mut groups: Vec<Vec<usize>> = Vec::new();

    for start in 0..n {
        if index[start] != UNVISITED {
            continue;
        }
        // (node, position in its adjacency list)
        let mut work: Vec<(usize, usize)> = vec![(start, 0)];
        while let Some(&(v, pos)) = work.last() {
            if pos == 0 {
                index[v] = next_index;
                low[v] = next_index;
                next_index += 1;
                stack.push(v);
                on_stack[v] = true;
            }
            if let Some(&(w, _)) = g.out[v].get(pos) {
                work.last_mut().unwrap().1 += 1;
                if index[w] == UNVISITED {
                    work.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                work.pop();
                if let Some(&(parent, _)) = work.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut group = Vec::new();
                    loop {
                        let w = stack.pop().unwrap();
                        on_stack[w] = false;
                        group.push(w);
                        if w == v {
                            break;
                        }
                    }
                    groups.push(group);
                }
            }
        }
    }

    groups.sort_by_key(|grp| *grp.iter().min().unwrap());
    for (id, group) in groups.iter().enumerate() {
        for &v in group {
            comp[v] = id;
        }
    }
    comp
}

/// Brandes betweenness centrality on the unweighted directed graph.
///
/// Exact when the graph has at most `max_pivots` nodes. Larger graphs use
/// `max_pivots` evenly spaced source nodes and scale the result, which keeps
/// the cost bounded for 50K-node repositories.
pub fn betweenness(g: &Graph, max_pivots: usize) -> Vec<f64> {
    let n = g.len();
    let mut score = vec![0.0f64; n];
    if n == 0 || max_pivots == 0 {
        return score;
    }
    let pivots: Vec<usize> = if n <= max_pivots {
        (0..n).collect()
    } else {
        (0..max_pivots).map(|i| i * n / max_pivots).collect()
    };
    let mut sigma = vec![0.0f64; n];
    let mut dist = vec![-1i64; n];
    let mut delta = vec![0.0f64; n];
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];

    for &s in &pivots {
        sigma.fill(0.0);
        dist.fill(-1);
        delta.fill(0.0);
        preds.iter_mut().for_each(Vec::clear);
        sigma[s] = 1.0;
        dist[s] = 0;
        let mut order = Vec::new();
        let mut queue = VecDeque::from([s]);
        while let Some(v) = queue.pop_front() {
            order.push(v);
            for &(w, _) in &g.out[v] {
                if dist[w] < 0 {
                    dist[w] = dist[v] + 1;
                    queue.push_back(w);
                }
                if dist[w] == dist[v] + 1 {
                    sigma[w] += sigma[v];
                    preds[w].push(v);
                }
            }
        }
        for &w in order.iter().rev() {
            for &v in &preds[w] {
                delta[v] += sigma[v] / sigma[w] * (1.0 + delta[w]);
            }
            if w != s {
                score[w] += delta[w];
            }
        }
    }
    let scale = n as f64 / pivots.len() as f64;
    score.iter_mut().for_each(|x| *x *= scale);
    score
}

/// Shortest directed path from `from` to `to` by edge count, inclusive.
pub fn shortest_path(g: &Graph, from: usize, to: usize) -> Option<Vec<usize>> {
    if from >= g.len() || to >= g.len() {
        return None;
    }
    let mut prev = vec![usize::MAX; g.len()];
    let mut seen = vec![false; g.len()];
    seen[from] = true;
    let mut queue = VecDeque::from([from]);
    while let Some(v) = queue.pop_front() {
        if v == to {
            let mut path = vec![to];
            let mut cur = to;
            while cur != from {
                cur = prev[cur];
                path.push(cur);
            }
            path.reverse();
            return Some(path);
        }
        for &(w, _) in &g.out[v] {
            if !seen[w] {
                seen[w] = true;
                prev[w] = v;
                queue.push_back(w);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(&str, &str)]) -> Graph {
        Graph::from_weighted(
            Vec::new(),
            edges
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string(), 1.0)),
        )
    }

    #[test]
    fn pagerank_favors_the_most_depended_upon() {
        let g = graph(&[
            ("a", "core"),
            ("b", "core"),
            ("c", "core"),
            ("core", "leaf"),
        ]);
        let r = pagerank(&g, 0.85, 100, 1e-10);
        assert!((r.iter().sum::<f64>() - 1.0).abs() < 1e-6);
        let rank = |n: &str| r[g.id(n).unwrap()];
        assert!(rank("core") > rank("a"));
        assert!(rank("leaf") > rank("core"));
        assert!((rank("a") - rank("b")).abs() < 1e-12);
    }

    #[test]
    fn scc_groups_cycles() {
        let g = graph(&[
            ("a", "b"),
            ("b", "c"),
            ("c", "a"),
            ("c", "d"),
            ("d", "e"),
            ("e", "d"),
        ]);
        let c = strongly_connected_components(&g);
        let id = |n: &str| c[g.id(n).unwrap()];
        assert_eq!(id("a"), id("b"));
        assert_eq!(id("b"), id("c"));
        assert_eq!(id("d"), id("e"));
        assert_ne!(id("a"), id("d"));
        assert_eq!(id("a"), 0);
    }

    #[test]
    fn betweenness_peaks_at_the_bridge() {
        let g = graph(&[("a", "m"), ("b", "m"), ("m", "x"), ("m", "y")]);
        let b = betweenness(&g, 1000);
        let m = g.id("m").unwrap();
        assert_eq!(b[m], 4.0);
        assert!(b.iter().enumerate().all(|(i, v)| i == m || *v == 0.0));
    }

    #[test]
    fn finds_shortest_path() {
        let g = graph(&[("a", "b"), ("b", "c"), ("a", "c"), ("c", "d")]);
        let id = |n: &str| g.id(n).unwrap();
        let path = shortest_path(&g, id("a"), id("d")).unwrap();
        let names: Vec<&str> = path.iter().map(|&i| g.nodes[i].as_str()).collect();
        assert_eq!(names, ["a", "c", "d"]);
        assert!(shortest_path(&g, id("d"), id("a")).is_none());
    }
}
