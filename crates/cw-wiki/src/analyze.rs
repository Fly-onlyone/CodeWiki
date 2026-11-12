//! Graph metrics for a scanned repository.

use std::collections::HashMap;

use cw_core::EdgeKind;
use cw_graph::{
    Graph, LayoutOptions, betweenness, force_layout, louvain, pagerank,
    strongly_connected_components,
};
use cw_index::NodeMetrics;
use cw_parse::RepoFacts;

/// Edge kinds that express "depends on" between symbols.
pub const DEPENDENCY_KINDS: [EdgeKind; 3] =
    [EdgeKind::Calls, EdgeKind::References, EdgeKind::Implements];

/// Sources sampled for betweenness on large graphs.
const BETWEENNESS_PIVOTS: usize = 256;

/// Build the symbol dependency graph.
pub fn dependency_graph(facts: &RepoFacts) -> Graph {
    Graph::from_edges(
        facts.symbols.iter().map(|s| s.qualified.clone()),
        &facts.edges,
        &DEPENDENCY_KINDS,
    )
}

/// Compute every per-node metric and a layout, sorted by node name.
pub fn analyze(facts: &RepoFacts) -> Vec<(String, NodeMetrics)> {
    let graph = dependency_graph(facts);
    let rank = pagerank(&graph, 0.85, 100, 1e-9);
    let between = betweenness(&graph, BETWEENNESS_PIVOTS);
    let scc = strongly_connected_components(&graph);
    let undirected = graph.undirected();
    let community = louvain(&undirected);
    let positions = force_layout(&undirected, LayoutOptions::default());
    graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, name)| {
            (
                name.clone(),
                NodeMetrics {
                    pagerank: rank[i],
                    betweenness: between[i],
                    scc: scc[i],
                    community: community[i],
                    x: positions[i][0],
                    y: positions[i][1],
                },
            )
        })
        .collect()
}

pub fn metrics_map(metrics: &[(String, NodeMetrics)]) -> HashMap<&str, &NodeMetrics> {
    metrics.iter().map(|(k, v)| (k.as_str(), v)).collect()
}
