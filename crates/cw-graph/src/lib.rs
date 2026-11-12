//! Graph construction, metrics, community detection and layout for CodeWiki.
//!
//! Every algorithm here is deterministic: the same input always yields the
//! same output, so generated notes and stored layouts do not churn.

pub mod community;
pub mod graph;
pub mod layout;
pub mod metrics;

pub use community::louvain;
pub use graph::Graph;
pub use layout::{LayoutOptions, force_layout};
pub use metrics::{betweenness, pagerank, shortest_path, strongly_connected_components};
