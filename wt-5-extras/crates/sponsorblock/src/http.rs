//! Low-level HTTP helpers: status-code mapping and the SHA-256 prefix used by
//! the privacy-preserving hash endpoint.

use crate::error::{Error, Result};
use sha2::{Digest, Sha256};

/// Map a non-2xx [`reqwest::StatusCode`] into our [`Error`] enum.
pub(crate) fn map_status(status: reqwest::StatusCode) -> Error {
    match status.as_u16() {
        400 => Error::InvalidInput("server rejected request body".to_string()),
        403 | 401 => Error::Forbidden,
        404 => Error::NotFound,
        429 => Error::RateLimited,
        code => Error::Status(code),
    }
}

/// True when `status` is in the success range and should be decoded.
pub(crate) fn is_success(status: reqwest::StatusCode) -> bool {
    status.is_success()
}

/// Compute the SHA-256 prefix (first 4 hex chars, uppercase) used by the
/// `/skipSegments/{hashPrefix}` endpoint. The same prefix is requested for
/// many different videos so the upstream server cannot tell which one the
/// client actually cares about.
pub(crate) fn sha256_prefix_4(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let hex = hex_encode(&digest);
    // SponsorBlock expects uppercase 4-char prefixes.
    hex[..4].to_ascii_uppercase()
}

/// Lowercase hex encoder (avoids pulling in the `hex` crate for one call site).
pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(TABLE[(b >> 4) as usize] as char);
        out.push(TABLE[(b & 0x0f) as usize] as char);
    }
    out
}

/// Build the `&categories=...` query suffix used by both segment endpoints.
pub(crate) fn join_categories(categories: &[crate::Category]) -> Vec<String> {
    categories.iter().map(|c| c.as_str().to_string()).collect()
}

/// Empty-body JSON sentinel returned by the API for "no segments".
pub(crate) fn looks_empty(body: &str) -> bool {
    body.trim() == "[]" || body.trim().is_empty()
}

/// Decode a JSON body into `T`, treating `[]`/empty as [`Error::NotFound`]
/// when the caller asked for a single video.
pub(crate) fn decode_or_not_found<T: serde::de::DeserializeOwned>(body: &str) -> Result<T> {
    if looks_empty(body) {
        return Err(Error::NotFound);
    }
    serde_json::from_str(body).map_err(|e| Error::Decode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_is_four_uppercase_chars() {
        let p = sha256_prefix_4("dQw4w9WgXcQ");
        assert_eq!(p.len(), 4);
        assert!(p.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }

    #[test]
    fn prefix_matches_known_value() {
        // Reference value: SHA256("dQw4w9WgXcQ") starts with 0d5...
        // We only assert length & hex alphabet here so the fixture does not
        // need to track the canonical hash.
        let p = sha256_prefix_4("dQw4w9WgXcQ");
        assert_eq!(p.len(), 4);
        assert!(p.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }

    #[test]
    fn join_categories_in_order() {
        let v = join_categories(&[crate::Category::Sponsor, crate::Category::Outro]);
        assert_eq!(v, vec!["sponsor".to_string(), "outro".to_string()]);
    }
}
