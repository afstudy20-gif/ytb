//! Low-level HTTP client for InnerTube and helpers for building client
//! contexts and request envelopes.
//!
//! This module is intentionally minimal: it owns the [`reqwest::Client`],
//! knows how to assemble the JSON envelope InnerTube expects, and exposes a
//! single [`InnerTubeClient::post`] helper. All higher-level shape parsing
//! lives in the sibling modules.

#![allow(dead_code)]

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Client, StatusCode};
use serde_json::{Map, Value};

use crate::error::{Error, Result};

/// The public API key InnerTube accepts for unauthenticated `WEB` traffic.
/// This is embedded in the official YouTube website and rotates rarely.
pub(crate) const INNERTUBE_API_KEY: &str = "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";

/// The Android client's API key (also embedded in the YouTube app). Used by
/// [`crate::streams`] to bypass WEB cipher.
pub(crate) const ANDROID_API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";

/// Base URL for InnerTube.
pub(crate) const INNERTUBE_BASE: &str = "https://www.youtube.com/youtubei/v1";

/// YouTube client identification. Used in the `context.clientName` field of
/// InnerTube requests.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ClientName {
    /// Default browser context. Requires cipher deciphering for streams.
    Web,
    /// Android context. Bypasses cipher for stream URLs.
    Android,
    /// iOS context. Secondary cipher-free fallback.
    Ios,
    /// TV-embedded context. Used to bypass age-gate for restricted videos.
    TvEmbedded,
}

impl ClientName {
    /// Returns the InnerTube `clientName` string InnerTube expects.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ClientName::Web => "WEB",
            ClientName::Android => "ANDROID",
            ClientName::Ios => "IOS",
            ClientName::TvEmbedded => "TVHTML5_SIMPLY_EMBEDDED_PLAYER",
        }
    }

    /// Returns the InnerTube `clientVersion` string for this client.
    pub(crate) fn version(self) -> &'static str {
        match self {
            // Pinned to a known-good recent build. Bumping is safe.
            ClientName::Web => "2.20240726.00.00",
            ClientName::Android => "19.09.37",
            ClientName::Ios => "19.09.3",
            ClientName::TvEmbedded => "2.20240726.00.00",
        }
    }

    /// Returns the InnerTube `clientKey`/API key to use.
    pub(crate) fn api_key(self) -> &'static str {
        match self {
            ClientName::Android => ANDROID_API_KEY,
            _ => INNERTUBE_API_KEY,
        }
    }
}

/// Subset of the request context InnerTube expects under `context.client`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClientContext {
    /// Which client identity to assume.
    pub name: ClientName,
    /// `hl` (UI language) field, BCP-47, e.g. `en`.
    pub hl: &'static str,
    /// `gl` (geolocation) field, ISO 3166-1 alpha-2, e.g. `US`.
    pub gl: &'static str,
}

impl ClientContext {
    /// Default context for the WEB client.
    pub(crate) const WEB_DEFAULT: Self = Self {
        name: ClientName::Web,
        hl: "en",
        gl: "US",
    };

    /// Default context for the ANDROID client.
    pub(crate) const ANDROID_DEFAULT: Self = Self {
        name: ClientName::Android,
        hl: "en",
        gl: "US",
    };

    /// Default context for the IOS client.
    pub(crate) const IOS_DEFAULT: Self = Self {
        name: ClientName::Ios,
        hl: "en",
        gl: "US",
    };

    /// Default context for the TV_EMBEDDED client.
    pub(crate) const TV_EMBEDDED_DEFAULT: Self = Self {
        name: ClientName::TvEmbedded,
        hl: "en",
        gl: "US",
    };

    /// Serialise this context into a `context.client` JSON object.
    pub(crate) fn to_client_json(self) -> Value {
        let mut map = Map::new();
        map.insert("clientName".into(), Value::String(self.name.as_str().into()));
        map.insert("clientVersion".into(), Value::String(self.name.version().into()));
        map.insert("hl".into(), Value::String(self.hl.into()));
        map.insert("gl".into(), Value::String(self.gl.into()));
        // InnerTube's `userInterfaceTheme` is required for some endpoints
        // (notably `search`) when the request comes from the WEB client.
        if matches!(self.name, ClientName::Web) {
            map.insert(
                "userInterfaceTheme".into(),
                Value::String("USER_INTERFACE_THEME_LIGHT".into()),
            );
        }
        if matches!(self.name, ClientName::Android) {
            map.insert("androidSdkVersion".into(), Value::Number(30.into()));
        }
        Value::Object(map)
    }
}

/// Wraps a [`reqwest::Client`] configured with the headers InnerTube
/// expects. Cheap to clone.
#[derive(Clone)]
pub struct InnerTubeClient {
    http: Client,
}

impl std::fmt::Debug for InnerTubeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InnerTubeClient").finish_non_exhaustive()
    }
}

impl InnerTubeClient {
    /// Construct a new client with sensible defaults. The User-Agent mimics
    /// a recent stable Chrome release on Linux, which InnerTube is happy to
    /// serve.
    pub(crate) fn new() -> Result<Self> {
        Self::with_user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
            (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
    }

    /// Construct a client with a custom User-Agent string. Useful in tests
    /// so requests against a mock server can be distinguished.
    pub(crate) fn with_user_agent(ua: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        // `Origin`/`Referer` and `X-YouTube-Client-*` headers make InnerTube
        // treat the request as coming from the web player rather than an
        // API call, which yields the same responses the website gets.
        headers.insert(USER_AGENT, HeaderValue::from_str(ua).map_err(|e| Error::internal(e.to_string()))?);
        headers.insert(
            "Origin",
            HeaderValue::from_static("https://www.youtube.com"),
        );
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://www.youtube.com/"),
        );
        headers.insert(
            "X-YouTube-Client-Name",
            HeaderValue::from_static("1"),
        );
        headers.insert(
            "X-YouTube-Client-Version",
            HeaderValue::from_static("2.20240726.00.00"),
        );

        let http = Client::builder()
            .default_headers(headers)
            .gzip(true)
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(Error::Network)?;

        Ok(Self { http })
    }

    /// Borrow the underlying [`reqwest::Client`]. Used by the Piped fallback
    /// and the player-JS fetcher.
    pub(crate) fn http(&self) -> &Client {
        &self.http
    }

    /// Build the standard InnerTube request envelope and POST it.
    ///
    /// `body_extra` is merged into the envelope after `context`, so callers
    /// can override anything (e.g. `videoId`, `continuation`, `query`).
    pub(crate) async fn post(
        &self,
        endpoint: &str,
        ctx: ClientContext,
        body_extra: Map<String, Value>,
    ) -> Result<Value> {
        let url = format!("{}/{}?key={}", INNERTUBE_BASE, endpoint, ctx.name.api_key());
        let mut envelope = Map::new();
        envelope.insert("context".into(), {
            let mut ctx_obj = Map::new();
            ctx_obj.insert("client".into(), ctx.to_client_json());
            Value::Object(ctx_obj)
        });
        for (k, v) in body_extra {
            envelope.insert(k, v);
        }

        let body = serde_json::to_string(&Value::Object(envelope))
            .map_err(|e| Error::decode(format!("encode request body: {e}")))?;

        tracing::debug!(endpoint = endpoint, body_len = body.len(), "POST innerTube");

        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(Error::Network)?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status_error(status, &url, &text));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| Error::decode(format!("decode response: {e}")))?;
        Ok(json)
    }

    /// Fetch a plain-text resource (used for the player JavaScript).
    pub(crate) async fn get_text(&self, url: &str) -> Result<String> {
        let resp = self.http.get(url).send().await.map_err(Error::Network)?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status_error(status, url, &text));
        }
        resp.text().await.map_err(Error::Network)
    }

    /// Fetch a JSON resource without going through the InnerTube envelope.
    /// Used by the Piped fallback.
    pub(crate) async fn get_json(&self, url: &str) -> Result<Value> {
        let resp = self.http.get(url).send().await.map_err(Error::Network)?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status_error(status, url, &text));
        }
        resp.json().await.map_err(Error::Network)
    }
}

/// Translate an HTTP status code into the most specific [`Error`] variant.
pub(crate) fn map_status_error(status: StatusCode, url: &str, body: &str) -> Error {
    let body_short = if body.len() > 512 { &body[..512] } else { body };
    match status.as_u16() {
        403 => Error::HttpStatus {
            status: 403,
            url: url.to_string(),
            body: body_short.to_string(),
        },
        429 => Error::HttpStatus {
            status: 429,
            url: url.to_string(),
            body: body_short.to_string(),
        },
        _ => Error::HttpStatus {
            status: status.as_u16(),
            url: url.to_string(),
            body: body_short.to_string(),
        },
    }
}

/// Convenience helper to build a `body_extra` map from a single key/value
/// pair, since most InnerTube endpoints want a tiny body.
pub(crate) fn body_kv(key: &str, value: Value) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert(key.into(), value);
    m
}

// Helper: extract a string from a `Value::String` or `Value::Number`, used by
// parsing modules.
#[must_use]
pub(crate) fn value_as_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
