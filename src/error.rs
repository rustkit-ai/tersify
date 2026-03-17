//! Error types for tersify.

use thiserror::Error;

/// All errors that tersify can produce.
#[derive(Debug, Error)]
pub enum TersifyError {
    /// The `--type` flag received a value that isn't recognised.
    #[error("unknown content type '{0}' — expected: code | json | logs | diff | text")]
    UnknownContentType(String),

    /// The input was valid JSON but could not be re-serialised (should never happen).
    #[error("JSON error: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// A file-system operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Stats persistence failed.
    #[error("stats: {0}")]
    Stats(String),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, TersifyError>;
