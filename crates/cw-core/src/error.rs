//! One error type shared by the engine crates.

use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid data: {0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{context}: {message}")]
    External { context: String, message: String },
    /// A provider has nothing left to answer with for now, for example every
    /// account behind a proxy is at its limit. Asking again soon is useless.
    #[error("{message}")]
    Exhausted {
        message: String,
        /// Seconds until the provider expects to answer again, when it said.
        retry_after: Option<u64>,
    },
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub fn not_found(what: impl Into<String>) -> Self {
        Self::NotFound(what.into())
    }

    /// Wrap an error from a third-party library with the operation that failed.
    pub fn external(context: impl Into<String>, err: impl std::fmt::Display) -> Self {
        Self::External {
            context: context.into(),
            message: err.to_string(),
        }
    }
}
