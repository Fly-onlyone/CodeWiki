//! Concept selection and wiki generation pipeline for CodeWiki.

pub mod agentdocs;
pub mod analyze;
pub mod briefing;
pub mod ci;
pub mod contracts;
pub mod coverage;
pub mod decisions;
pub mod generate;
pub mod git;
pub mod gitintel;
pub mod health;
mod level;
pub mod manifests;
pub mod plan;
pub mod query;
pub mod refactor;
pub mod render;
mod rustscan;
pub mod security;
pub mod semantic;
pub mod service;
pub mod testmap;
pub mod units;

pub use analyze::{analyze, dependency_graph};
pub use generate::{
    ProseWriter, Report, Rewrite, Stopped, TEMPLATE_VERSION, WriterContext, WriterUsage, generate,
    generate_with,
};
pub use plan::{Plan, Planned, Repo, Subject};
