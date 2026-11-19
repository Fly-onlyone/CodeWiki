//! The embedding models CodeWiki can fetch and run by itself.
//!
//! - [`catalog`]: the models, a list shipped inside the program. Every file
//!   of every model is pinned by revision and SHA-256.
//! - [`hardware`]: what the machine has (memory, cores, an NVIDIA card, free
//!   disk space).
//! - [`recommend`]: which model fits that machine, and why.
//! - [`store`]: the models on disk, under `<data dir>\models`: download with
//!   resume, verification, removal.
//!
//! Nothing here runs a model. The engine that does is a crate of its own,
//! linked only into the programs that run one, so that this crate stays
//! light for everything else that links it.

pub mod catalog;
pub mod hardware;
pub mod recommend;
pub mod store;

pub use catalog::{Entry, Family, Pooling, Tier};
pub use hardware::{Hardware, Nvidia};
pub use recommend::{Device, Recommendation, recommend};
pub use store::{Outcome, Store};

/// Whether this build can run a built-in model. The engine is compiled for
/// Windows only.
pub const ENGINE: bool = cfg!(windows);
