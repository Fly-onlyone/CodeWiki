//! Obsidian vault model for CodeWiki: notes, manifest, name registry, linter,
//! lifecycle and proposals.

pub mod lifecycle;
pub mod lint;
pub mod manifest;
pub mod note;
pub mod proposal;
pub mod registry;

pub use lifecycle::{Action, NoteState};
pub use lint::{Issue, Mode, Severity, link_resolution_rate, lint};
pub use manifest::{Manifest, NoteEntry, Origin, SourceRef, VaultMeta, body_hash};
pub use note::{Note, NoteKind, Section, WikiLink};
pub use proposal::{Proposal, ProposalStatus};
pub use registry::{Registry, RegistryEntry, sanitize_name};
