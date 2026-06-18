//! `SponsorBlock` HTTP client.
//!
//! Implements the public API documented at
//! <https://wiki.sponsor.ajay.app/w/API_Docs>. Both the direct
//! `/skipSegments?videoID=...` lookup and the privacy-preserving
//! `/skipSegments/{hashPrefix}` variant are supported; the hash variant is
//! preferred because it hides the exact video being queried from the server.

use crate::error::{Error, Result};
use crate::http::{
    decode_or_not_found, is_success, join_categories, map_status, sha256_prefix_4,
};
use crate::model::{Category, NewSegment, Segment, Vote};
use reqwest::Client as HttpClient;

const DEFAULT_BASE: &str = "https://sponsor.ajay.app/api";

/// `SponsorBlock` API client.
///
/// Cheap to clone: internally wraps a [`reqwest::Client`] which is itself an
/// `Arc`-backed connection pool. The optional `user_id_hash` is captured at
/// construction time so callers do not need to thread it through every call.
#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    base: String,
    user_id_hash: Option<String>,
}

impl Client {
    /// Build a client pointed at the public `SponsorBlock` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::with_base(DEFAULT_BASE)
    }

    /// Build a client pointed at a self-hosted `SponsorBlock` mirror.
    #[must_use]
    pub fn with_base(base: impl Into<String>) -> Self {
        Self {
            http: HttpClient::new(),
            base: base.into(),
            user_id_hash: None,
        }
    }

    /// Attach a default user-id hash used for votes/submits when the caller
    /// does not pass one explicitly.
    #[must_use]
    pub fn with_user_id(mut self, user_id_hash: impl Into<String>) -> Self {
        self.user_id_hash = Some(user_id_hash.into());
        self
    }

    /// Replace the inner HTTP client (mainly for tests / custom TLS stacks).
    #[must_use]
    pub fn with_http(mut self, http: HttpClient) -> Self {
        self.http = http;
        self
    }

    fn resolve_user<'a>(&'a self, explicit: Option<&'a str>) -> Result<&'a str> {
        explicit.or(self.user_id_hash.as_deref()).ok_or_else(|| {
            Error::InvalidInput("a user id is required for this call".to_string())
        })
    }

    /// Fetch segments by exact `videoId`. Uses `/skipSegments?videoID=...`.
    ///
    /// Returns [`Error::NotFound`] when the server has no segments for the
    /// given video in the requested categories.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidInput`] if `video_id` is empty.
    /// - [`Error::Network`] on transport failure.
    /// - [`Error::NotFound`] / [`Error::RateLimited`] / [`Error::Forbidden`] /
    ///   [`Error::Status`] mapped from the upstream status code.
    /// - [`Error::Decode`] if the JSON body is malformed.
    pub async fn segments(
        &self,
        video_id: &str,
        categories: &[Category],
    ) -> Result<Vec<Segment>> {
        require_video_id(video_id)?;
        let mut req = self
            .http
            .get(format!("{}/skipSegments", self.base))
            .query(&[("videoID", video_id)]);
        if !categories.is_empty() {
            let joined = join_categories(categories);
            req = req.query(&[("categories", serde_json::to_string(&joined).map_err(
                |e| Error::Decode(e.to_string()),
            )?)]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !is_success(status) {
            return Err(map_status(status));
        }
        let body = resp.text().await?;
        decode_or_not_found(&body)
    }

    /// Privacy-preserving variant: requests the `/skipSegments/{hashPrefix}`
    /// endpoint (first 4 hex chars of `SHA256(videoId)`), then filters the
    /// returned bucket client-side so the server never learns the exact video.
    ///
    /// This is the recommended default lookup path.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidInput`] if `video_id` is empty.
    /// - [`Error::Network`] on transport failure.
    /// - [`Error::NotFound`] when the bucket is empty or absent.
    /// - [`Error::RateLimited`] / [`Error::Forbidden`] / [`Error::Status`]
    ///   mapped from the upstream status code.
    /// - [`Error::Decode`] if the JSON body is malformed.
    pub async fn segments_by_hash(
        &self,
        video_id: &str,
        categories: &[Category],
    ) -> Result<Vec<Segment>> {
        require_video_id(video_id)?;
        let prefix = sha256_prefix_4(video_id);
        let mut req = self
            .http
            .get(format!("{}/skipSegments/{}", self.base, prefix));
        if !categories.is_empty() {
            let joined = join_categories(categories);
            req = req.query(&[(
                "categories",
                serde_json::to_string(&joined).map_err(|e| Error::Decode(e.to_string()))?,
            )]);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !is_success(status) {
            return Err(map_status(status));
        }
        let body = resp.text().await?;
        let bucket: Vec<Segment> = if body.trim().is_empty() {
            return Err(Error::NotFound);
        } else {
            serde_json::from_str(&body).map_err(|e| Error::Decode(e.to_string()))?
        };
        let wanted_cat: Option<std::collections::HashSet<&'static str>> =
            (!categories.is_empty()).then(|| categories.iter().map(|c| c.as_str()).collect());
        let filtered: Vec<Segment> = match wanted_cat {
            Some(set) => bucket
                .into_iter()
                .filter(|s| set.contains(s.category.as_str()))
                .collect(),
            None => bucket,
        };
        Ok(filtered)
    }

    /// Cast a vote on an existing segment.
    ///
    /// `user_id` should be a private, stable hash identifying the local user
    /// (see `SponsorBlock` docs). If [`Client::with_user_id`] was used at
    /// construction time, `explicit` may be `None`.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidInput`] if `segment_uuid` is empty or no user id is
    ///   available.
    /// - [`Error::Network`] on transport failure.
    /// - [`Error::Forbidden`] / [`Error::RateLimited`] / [`Error::Status`]
    ///   mapped from the upstream status code.
    pub async fn vote(
        &self,
        segment_uuid: &str,
        vote: Vote,
        user_id: Option<&str>,
    ) -> Result<()> {
        require_nonempty(segment_uuid, "segment_uuid")?;
        let user = self.resolve_user(user_id)?;
        let resp = self
            .http
            .post(format!("{}/voteOnSponsorTime", self.base))
            .query(&[
                ("UUID", segment_uuid),
                ("userID", user),
                ("type", &vote.as_i64().to_string()),
            ])
            .send()
            .await?;
        let status = resp.status();
        if is_success(status) {
            Ok(())
        } else {
            Err(map_status(status))
        }
    }

    /// Submit a new segment. Returns the new segment's UUID on success.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidInput`] if `video_id` is empty or no user id is
    ///   available.
    /// - [`Error::Network`] on transport failure.
    /// - [`Error::Forbidden`] / [`Error::RateLimited`] / [`Error::Status`]
    ///   mapped from the upstream status code.
    /// - [`Error::Decode`] if the server did not return a usable UUID.
    pub async fn submit(
        &self,
        video_id: &str,
        segment: NewSegment,
        user_id: Option<&str>,
    ) -> Result<String> {
        require_video_id(video_id)?;
        let user = self.resolve_user(user_id)?;
        let resp = self
            .http
            .post(format!("{}/skipSegments", self.base))
            .query(&[("videoID", video_id), ("userID", user)])
            .json(&segment)
            .send()
            .await?;
        let status = resp.status();
        if !is_success(status) {
            return Err(map_status(status));
        }
        // Server returns the new UUID as a quoted/empty string body.
        let text = resp.text().await?;
        let trimmed = text.trim().trim_matches('"');
        if trimmed.is_empty() {
            return Err(Error::Decode("server returned empty uuid".to_string()));
        }
        Ok(trimmed.to_string())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

fn require_video_id(video_id: &str) -> Result<()> {
    require_nonempty(video_id, "video_id")
}

fn require_nonempty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(Error::InvalidInput(format!("{field} must not be empty")));
    }
    Ok(())
}
