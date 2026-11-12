//! SQLite index for CodeWiki.
//!
//! The index is a cache: everything in it can be rebuilt from the repository
//! and the vault. It is safe to delete at any time.

mod schema;
mod search;
mod store;
mod vectors;

pub use search::{Document, Hit};
pub use store::{Index, NodeMetrics};
pub use vectors::VectorRow;
