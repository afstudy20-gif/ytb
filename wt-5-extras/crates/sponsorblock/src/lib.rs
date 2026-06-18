//! Async client for the [SponsorBlock](https://sponsor.ajay.app) API.
//!
//! The crate exposes a single [`Client`] which speaks both the direct
//! `/skipSegments?videoID=...` and the privacy-preserving
//! `/skipSegments/{hashPrefix}` lookup styles, plus voting and submission.
//!
//! ```no_run
//! # async fn run() -> sponsorblock::Result<()> {
//! use sponsorblock::{Client, Category};
//!
//! let client = Client::new();
//! let segs = client
//!     .segments_by_hash("dQw4w9WgXcQ", &[Category::Sponsor])
//!     .await?;
//! println!("{} segments", segs.len());
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::cast_possible_truncation)]

mod client;
mod error;
mod http;
mod model;

pub use client::Client;
pub use error::Error;
pub use model::{ActionType, Category, NewSegment, Segment, Vote};

/// Convenience alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
