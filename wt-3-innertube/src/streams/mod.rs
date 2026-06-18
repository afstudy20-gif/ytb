//! Stream URL resolution: deciphering, n-param transformation, and adaptive
//! format unification.
//!
//! The flow for resolving playable stream URLs from InnerTube:
//!
//! 1. Call `player` with the [`crate::client::ClientName::Android`] client
//!    first. The Android player usually returns already-deciphered URLs
//!    (no `signatureCipher` field, just `url`).
//! 2. If Android yields nothing usable, fall back to the [`crate::client::ClientName::Web`]
//!    client. WEB responses wrap the real URL in a `signatureCipher` string
//!    that contains `s=<ciphered>&url=<unciphered>&sp=signature`. We
//!    decipher `s` with the cipher program extracted from the player JS,
//!    then patch it into the URL under the `sp` name (usually `signature`).
//! 3. The WEB URLs also carry an `n=<...>` parameter. YouTube throttles
//!    requests whose `n` hasn't been transformed by the n-sig function.
//!    We extract the n-sig function from the player JS, evaluate it on
//!    each `n`, and patch the result back into the URL.
//! 4. If even WEB yields nothing usable (HTTP 403/429 or empty formats),
//!    [`crate::InnerTube::streams`] falls back to the configured Piped
//!    instances (see [`crate::piped`]).
//!
//! This module is split across:
//!
//! - [`cache`]: caches and refreshes the player JS, exposing cipher + n-sig.
//! - [`extractor`]: regex-based extraction of the cipher program and n-sig
//!   function from a `base.js` source string.
//! - [`url_util`]: query-string manipulation and `signatureCipher` parsing.
//! - [`format`]: turning InnerTube `format` objects into [`Stream`] values.
//! - [`orchestrator`]: top-level Android → WEB → Piped resolution chain.

pub mod cache;
pub mod extractor;
pub mod format;
pub mod orchestrator;
pub mod url_util;

pub use cache::PlayerJsResolver;
pub use orchestrator::resolve_streams;
