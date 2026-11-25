//! Shared identifiers, hashing, configuration and error types for CodeWiki.

pub mod agents_off;
pub mod auto_explain;
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

/// Whether this is a development build (debug assertions on). A development
/// build keeps its own folders ([`paths::APP_FOLDER`]) and its own name, so
/// that one run from the sources never shares a backend, an index or settings
/// with an installed CodeWiki on the same machine.
pub const DEV_BUILD: bool = cfg!(debug_assertions);

/// The name the app shows in its title, its tray icon and its notifications.
pub const APP_NAME: &str = if DEV_BUILD {
    "CodeWiki Dev"
} else {
    "CodeWiki"
};

/// Folder, relative to a repository root, that holds the generated vault.
pub const VAULT_DIR: &str = "obsidian";

/// Folder inside the vault that holds machine metadata. Never rendered as notes.
pub const MANIFEST_DIR: &str = ".codewiki";

/// How many minutes a note listed for a coding agent stays reserved for it.
/// Other agents are not offered the note until this time has passed.
pub const CLAIM_MINUTES: u64 = 30;

/// Request header in which a client of the backend says what it is. Only the
/// MCP bridge sends it; the backend reports when a bridge last called.
pub const CLIENT_HEADER: &str = "x-codewiki-client";

/// What the MCP bridge sends in [`CLIENT_HEADER`].
pub const MCP_CLIENT: &str = "mcp";
