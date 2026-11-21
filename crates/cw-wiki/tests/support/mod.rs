//! What the test programs that build an index share.
//!
//! Building an index embeds its documents with the process's embedder. Until
//! that slot is configured it holds what the machine's environment names, so
//! a test could call a model server set up on the machine that runs it.

use cw_core::settings::Resolved;

/// Have the process embed with the word-form embedder, which empty settings
/// and an empty environment name. Call it before anything builds an index.
pub fn no_model_server() {
    cw_wiki::semantic::configure(&Resolved::from_parts(&Default::default(), |_| None));
}
