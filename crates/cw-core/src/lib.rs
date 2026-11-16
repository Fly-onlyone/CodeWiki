//! Shared identifiers, hashing, configuration and error types for CodeWiki.

pub mod error;
pub mod hash;
pub mod logs;
pub mod model;
pub mod paths;
pub mod secret;
pub mod settings;

pub use error::{Error, Result};
pub use hash::{normalize_text, sha256_hex};
pub use model::*;

/// Crate version, exposed so binaries report one consistent version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Folder, relative to a repository root, that holds the generated vault.
pub const VAULT_DIR: &str = "obsidian";

/// Folder inside the vault that holds machine metadata. Never rendered as notes.
pub const MANIFEST_DIR: &str = ".codewiki";

/// Request header in which a client of the backend says what it is. Only the
/// MCP bridge sends it; the backend reports when a bridge last called.
pub const CLIENT_HEADER: &str = "x-codewiki-client";

/// What the MCP bridge sends in [`CLIENT_HEADER`].
pub const MCP_CLIENT: &str = "mcp";
