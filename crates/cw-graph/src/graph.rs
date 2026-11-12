//! Directed weighted graph over string-named nodes.

use std::collections::{BTreeMap, HashMap};

use cw_core::{Edge, EdgeKind};

#[derive(Debug, Clone, Default)]
pub struct Graph {
    pub nodes: Vec<String>,
    index: HashMap<String, usize>,
    /// Outgoing `(target, weight)` lists, sorted by target.
    pub out: Vec<Vec<(usize, f64)>>,
    /// Incoming `(source, weight)` lists, sorted by source.
    pub inc: Vec<Vec<(usize, f64)>>,
}

impl Graph {
    /// Build from weighted pairs. Parallel edges are merged by summing weights;
    /// self loops are dropped. Nodes are sorted by name for determinism.
    pub fn from_weighted<I, N>(nodes: N, edges: I) -> Self
    where
        N: IntoIterator<Item = String>,
        I: IntoIterator<Item = (String, String, f64)>,
    {
        let edges: Vec<(String, String, f64)> = edges.into_iter().collect();
        let mut names: Vec<String> = nodes.into_iter().collect();
        for (s, d, _) in &edges {
            names.push(s.clone());
            names.push(d.clone());
        }
        names.sort();
        names.dedup();
        let index: HashMap<String, usize> = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i))
            .collect();
        let mut merged: BTreeMap<(usize, usize), f64> = BTreeMap::new();
        for (s, d, w) in edges {
            let (s, d) = (index[&s], index[&d]);
            if s != d {
                *merged.entry((s, d)).or_insert(0.0) += w;
            }
        }
        let mut out = vec![Vec::new(); names.len()];
        let mut inc = vec![Vec::new(); names.len()];
        for ((s, d), w) in merged {
            out[s].push((d, w));
            inc[d].push((s, w));
        }
        for list in &mut inc {
            list.sort_by_key(|(s, _)| *s);
        }
        Self {
            nodes: names,
            index,
            out,
            inc,
        }
    }

    /// Build from resolved edges, keeping only `kinds` and weighting each edge
    /// by its confidence.
    pub fn from_edges(
        nodes: impl IntoIterator<Item = String>,
        edges: &[Edge],
        kinds: &[EdgeKind],
    ) -> Self {
        Self::from_weighted(
            nodes,
            edges
                .iter()
                .filter(|e| kinds.contains(&e.kind))
                .map(|e| (e.src.clone(), e.dst.clone(), e.confidence.weight())),
        )
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn edge_count(&self) -> usize {
        self.out.iter().map(Vec::len).sum()
    }

    pub fn id(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }

    /// Symmetric adjacency: weights of both directions summed per neighbor.
    pub fn undirected(&self) -> Vec<Vec<(usize, f64)>> {
        let mut adj: Vec<BTreeMap<usize, f64>> = vec![BTreeMap::new(); self.len()];
        for (s, list) in self.out.iter().enumerate() {
            for &(d, w) in list {
                *adj[s].entry(d).or_insert(0.0) += w;
                *adj[d].entry(s).or_insert(0.0) += w;
            }
        }
        adj.into_iter().map(|m| m.into_iter().collect()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_parallel_edges_and_drops_self_loops() {
        let g = Graph::from_weighted(
            vec!["z".to_string()],
            vec![
                ("b".to_string(), "a".to_string(), 1.0),
                ("b".to_string(), "a".to_string(), 0.5),
                ("a".to_string(), "a".to_string(), 9.0),
            ],
        );
        assert_eq!(g.nodes, ["a", "b", "z"]);
        assert_eq!(g.out[1], vec![(0, 1.5)]);
        assert_eq!(g.inc[0], vec![(1, 1.5)]);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.undirected()[0], vec![(1, 1.5)]);
    }
}
