//! Error types for the `innertube` crate.

use thiserror::Error;

/// All errors returned by this crate.
///
/// Each variant maps to a distinct failure mode encountered when talking to the
/// InnerTube API or the Piped fallback. `Error` is cheap to construct, `Send +
/// Sync`, and implements `std::error::Error` via [`thiserror`].
#[derive(Debug, Error)]
pub enum Error {
    /// A network-level failure: connection refused, DNS error, TLS handshake,
    /// timeout, etc. Wraps the underlying [`reqwest::Error`].
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// The HTTP layer succeeded but the response body could not be parsed,
    /// either because it was not valid JSON or because the InnerTube shape
    /// diverged from what this crate expects. Includes the failing context.
    #[error("decode error: {0}")]
    Decode(String),

    /// Could not extract or evaluate the YouTube player JavaScript that
    /// decipheres stream signatures or the `n` parameter. This is usually a
    /// sign that YouTube has rotated the player build and the regexes in
    /// [`crate::streams`] need updating.
    #[error("cipher error: {0}")]
    Cipher(String),

    /// The video exists but no playable streams could be resolved by either
    /// InnerTube or Piped. Surfaces after all fallbacks have been tried.
    #[error("no usable streams found for video {0}")]
    NoStreams(String),

    /// The video is age-restricted and cannot be played without an
    /// authenticated account. InnerTube's `TV_EMBEDDED` client bypasses this
    /// for many videos but not for genuinely restricted ones.
    #[error("video {0} is age restricted")]
    AgeRestricted(String),

    /// The video is geo-blocked or otherwise not available in the requested
    /// region.
    #[error("video {0} is not available in this region")]
    Region(String),

    /// The video has been removed, made private, or otherwise rendered
    /// unavailable. The optional message echoes InnerTube's reason text when
    /// present.
    #[error("video {0} is unavailable{1}")]
    Unavailable(String, String),

    /// All Piped fallback instances were tried and none returned usable
    /// streams. The wrapped string contains a summary of which instances
    /// failed and why.
    #[error("piped fallback failed: {0}")]
    PipedFallbackFailed(String),

    /// An internal HTTP status that InnerTube returned (4xx/5xx) which the
    /// crate did not recognise as one of the more specific cases above.
    #[error("http status {status} from {url}: {body}")]
    HttpStatus {
        /// The numeric HTTP status code returned by the server.
        status: u16,
        /// The URL that produced this status.
        url: String,
        /// The (truncated) response body, for diagnostics.
        body: String,
    },

    /// A miscellaneous error that does not fit any of the other variants.
    /// Used internally for assertion-style failures.
    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Helper for building a [`Error::Decode`] from anything that implements
    /// [`std::fmt::Display`].
    pub(crate) fn decode(context: impl Into<String>) -> Self {
        Self::Decode(context.into())
    }

    /// Helper for building a [`Error::Cipher`] with a context string.
    pub(crate) fn cipher(context: impl Into<String>) -> Self {
        Self::Cipher(context.into())
    }

    /// Helper for building a [`Error::Internal`] from anything that implements
    /// [`std::fmt::Display`].
    #[allow(dead_code)]
    pub(crate) fn internal(context: impl Into<String>) -> Self {
        Self::Internal(context.into())
    }
}

/// Alias used throughout the crate for ergonomic `?` propagation.
pub type Result<T, E = Error> = std::result::Result<T, E>;
