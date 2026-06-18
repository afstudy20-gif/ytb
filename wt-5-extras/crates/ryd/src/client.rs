//! Return YouTube Dislike HTTP client with in-memory LRU cache.

use crate::cache::{Lru, DEFAULT_CAPACITY, DEFAULT_TTL};
use crate::error::{Error, Result};
use crate::model::Votes;
use reqwest::Client as HttpClient;
use std::time::Duration;

const DEFAULT_BASE: &str = "https://returnyoutubedislike.com";

/// Return YouTube Dislike API client.
///
/// Wraps a single `GET /votes?videoId=...` endpoint and caches responses
/// in an in-memory LRU (default 256 entries, 5-minute TTL) to stay friendly
/// to the upstream API.
#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    base: String,
    cache: std::sync::Arc<Lru>,
}

impl Client {
    /// Build a client pointed at the public Return YouTube Dislike instance
    /// with default cache settings (256 entries, 5-min TTL).
    #[must_use]
    pub fn new() -> Self {
        Self {
            http: HttpClient::new(),
            base: DEFAULT_BASE.to_string(),
            cache: std::sync::Arc::new(Lru::new(DEFAULT_CAPACITY, DEFAULT_TTL)),
        }
    }

    /// Point the client at a self-hosted instance. Chainable.
    #[must_use]
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = base.into();
        self
    }

    /// Replace the in-memory cache with one of the given capacity / TTL.
    /// Chainable.
    #[must_use]
    pub fn with_cache(mut self, capacity: usize, ttl: Duration) -> Self {
        self.cache = std::sync::Arc::new(Lru::new(capacity, ttl));
        self
    }

    /// Inject a custom HTTP client (tests / custom TLS stacks).
    #[must_use]
    pub fn with_http(mut self, http: HttpClient) -> Self {
        self.http = http;
        self
    }

    /// Fetch like/dislike snapshot for `video_id`, served from cache when
    /// fresh. Returns [`Error::NotFound`] for unknown videos.
    pub async fn votes(&self, video_id: &str) -> Result<Votes> {
        require_video_id(video_id)?;
        if let Some(hit) = self.cache.get(video_id) {
            return Ok(hit);
        }
        let resp = self
            .http
            .get(format!("{}/votes", self.base))
            .query(&[("videoId", video_id)])
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(map_status(status));
        }
        let body = resp.text().await?;
        let parsed: Votes =
            serde_json::from_str(&body).map_err(|e| Error::Decode(e.to_string()))?;
        // Don't cache deleted records — they may be transient upstream state.
        if !parsed.deleted {
            self.cache.put(video_id.to_string(), parsed.clone());
        }
        Ok(parsed)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

fn require_video_id(video_id: &str) -> Result<()> {
    if video_id.trim().is_empty() {
        return Err(Error::InvalidInput("video_id must not be empty".to_string()));
    }
    Ok(())
}

fn map_status(status: reqwest::StatusCode) -> Error {
    match status.as_u16() {
        400 => Error::InvalidInput("server rejected videoId".to_string()),
        404 => Error::NotFound,
        429 => Error::RateLimited,
        code => Error::Status(code),
    }
}
