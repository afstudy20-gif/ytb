//! Async client for the [Return YouTube Dislike](https://returnyoutubedislike.com)
//! API, with an in-memory LRU cache (256 entries, 5-minute TTL by default).
//!
//! ```no_run
//! # async fn run() -> ryd::Result<()> {
//! use ryd::Client;
//! let client = Client::new();
//! let votes = client.votes("dQw4w9WgXcQ").await?;
//! println!("{} likes / {} dislikes", votes.likes, votes.dislikes);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::cast_possible_truncation)]

mod cache;
mod client;
mod error;
mod model;

pub use client::Client;
pub use error::Error;
pub use model::Votes;

/// Convenience alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;
