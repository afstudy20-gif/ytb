//! Error type for the Return YouTube Dislike client.

use thiserror::Error;

/// Errors returned by [`crate::Client`] operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Network failure (DNS, connection reset, TLS, etc.).
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Response body could not be decoded into [`crate::Votes`].
    #[error("decode error: {0}")]
    Decode(String),

    /// Server replied 404 — video has no dislike record yet.
    #[error("not found")]
    NotFound,

    /// Server replied 429 — rate limited.
    #[error("rate limited")]
    RateLimited,

    /// Any other non-2xx status with the raw code.
    #[error("unexpected status {0}")]
    Status(u16),

    /// Input was rejected before sending (empty video id, …).
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;
