//! Shared identifiers, hashing, configuration and error types for CodeWiki.

pub mod error;
pub mod hash;
pub mod model;
pub mod paths;

pub use error::{Error, Result};
pub use hash::{normalize_text, sha256_hex};
pub use model::*;

/// Crate version, exposed so binaries report one consistent version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Folder, relative to a repository root, that holds the generated vault.
pub const VAULT_DIR: &str = "obsidian";

/// Folder inside the vault that holds machine metadata. Never rendered as notes.
pub const MANIFEST_DIR: &str = ".codewiki";
