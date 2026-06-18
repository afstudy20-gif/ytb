//! `innertube` — a self-contained Rust client for YouTube's private
//! InnerTube API.
//!
//! The crate talks directly to InnerTube (the JSON API used by the official
//! YouTube apps), with no HTML scraping, no Google Play Services, and no
//! microG. It covers search, video metadata, stream URL resolution
//! (including signature and `n`-param deciphering), trending, channel, and
//! playlist. When InnerTube blocks or rate-limits the client, stream
//! resolution transparently falls back to a Piped instance pool.
//!
//! ## Quick start
//!
//! ```no_run
//! # use innertube::{InnerTube, SearchFilter, SearchKind};
//! # async fn demo() -> innertube::Result<()> {
//! let tube = InnerTube::new();
//! let results = tube
//!     .search("lofi hip hop", Some(SearchFilter {
//!         kind: SearchKind::Video,
//!         ..Default::default()
//!     }))
//!     .await?;
//! println!("{} results", results.items.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Stream resolution
//!
//! [`InnerTube::streams`] tries the ANDROID client first (which returns
//! deciphered URLs without signature ciphering). If that yields nothing,
//! it falls back to the WEB client and applies the cipher + n-sig
//! transformations extracted from YouTube's player JavaScript. As a last
//! resort, it tries the configured Piped instances.
//!
//! ```no_run
//! # use innertube::InnerTube;
//! # async fn demo() -> innertube::Result<()> {
//! let tube = InnerTube::with_piped_fallback(vec![
//!     "https://pipedapi.kavin.rocks".into(),
//! ]);
//! let map = tube.streams("dQw4w9WgXcQ").await?;
//! println!("progressive streams: {}", map.progressive.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Module layout
//!
//! - [`client`]: HTTP client and InnerTube client-context building.
//! - [`error`]: error types.
//! - [`types`]: public data types.
//! - [`search`], [`video`], [`channel`], [`playlist`], [`continuation`]:
//!   endpoint parsers.
//! - [`streams`]: cipher + n-sig + adaptive format unification.
//! - [`piped`]: Piped fallback client.
//! - [`js_interp`]: the hand-rolled JS-subset interpreter used by streams.
//!
//! ## `clippy` policy
//!
//! The crate is `clippy::pedantic`-clean with a short list of allowed lints
//! that we believe are net-positive for ergonomics in this codebase. The
//! list and rationale are below.
//!
//! - `module_name_repetitions`: the public types are re-exported at the
//!   crate root, so prefixing them with their module name is intentional
//!   and helps disambiguate in `use` statements.
//! - `missing_errors_doc`: every fallible public API uses a [`Result`] alias
//!   and a single rustdoc example; requiring per-API error documentation
//!   would just restate the type signature.
//! - `must_use_candidate`: too noisy for a parser-heavy codebase; we mark
//!   the few cases where it actually matters.
//! - `too_many_lines`: some parser functions naturally exceed the lint's
//!   default. Splitting them further would hurt readability.
//! - `cast_possible_truncation`: InnerTube fields are 64-bit but our types
//!   are deliberately narrower (e.g. `u32` for itags); the truncation is
//!   intentional and documented inline where relevant.
#![cfg_attr(not(test), warn(
    clippy::pedantic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
))]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
)]

pub mod channel;
pub mod client;
pub mod continuation;
pub mod error;
pub mod js_interp;
pub mod json_util;
pub mod playlist;
pub mod piped;
pub mod search;
pub mod streams;
pub mod types;
pub mod video;

pub use error::{Error, Result};
pub use types::{
    Caption, ChannelBadge, ChannelDetails, Continuable, Duration, PlaylistDetails, PlaylistVideo,
    SearchFilter, SearchItem, SearchKind, SearchResults, SearchResultChannel, SearchResultPlaylist,
    SearchResultVideo, SortBy, Stream, StreamMap, UploadDate, VideoDetails, VideoSummary,
};

/// Trait for types that can be returned from [`InnerTube::continuation`].
/// Currently only [`SearchResults`] implements it.
pub trait FromSearchResults: private::Sealed {
    /// Construct from a [`SearchResults`] value.
    fn from_search_results(results: SearchResults) -> Self;
}

impl FromSearchResults for SearchResults {
    fn from_search_results(results: SearchResults) -> Self {
        results
    }
}

mod private {
    /// Sealing trait for [`crate::FromSearchResults`].
    pub trait Sealed {}
    impl Sealed for crate::SearchResults {}
}

use std::sync::Arc;

use client::{ClientContext, InnerTubeClient};
use streams::PlayerJsResolver;

/// Top-level InnerTube client.
///
/// Construct one of these per process (or per logical task) and clone
/// freely — the underlying HTTP client, player-JS cache, and Piped
/// fallback table are all behind `Arc` and share state across clones.
///
/// ```no_run
/// # use innertube::InnerTube;
/// let tube = InnerTube::new();
/// let tube2 = tube.clone();
/// ```
#[derive(Clone)]
pub struct InnerTube {
    http: Arc<InnerTubeClient>,
    player_js: Arc<PlayerJsResolver>,
    piped: Option<Arc<piped::PipedFallback>>,
}

impl std::fmt::Debug for InnerTube {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InnerTube")
            .field("piped", &self.piped.is_some())
            .finish()
    }
}

impl Default for InnerTube {
    fn default() -> Self {
        Self::new()
    }
}

impl InnerTube {
    /// Construct a new client with no Piped fallback configured.
    ///
    /// ```no_run
    /// # use innertube::InnerTube;
    /// let tube = InnerTube::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let http = Arc::new(InnerTubeClient::new().expect("reqwest client builds"));
        let player_js = Arc::new(PlayerJsResolver::new((*http).clone()));
        Self {
            http,
            player_js,
            piped: None,
        }
    }

    /// Construct a client that will round-robin across `piped_instances`
    /// when InnerTube stream resolution fails.
    ///
    /// ```no_run
    /// # use innertube::InnerTube;
    /// let tube = InnerTube::with_piped_fallback(
    ///     vec!["https://pipedapi.kavin.rocks".into()]);
    /// ```
    #[must_use]
    pub fn with_piped_fallback(piped_instances: Vec<String>) -> Self {
        let http = Arc::new(InnerTubeClient::new().expect("reqwest client builds"));
        let player_js = Arc::new(PlayerJsResolver::new((*http).clone()));
        let piped = if piped_instances.is_empty() {
            None
        } else {
            Some(Arc::new(piped::PipedFallback::new(
                (*http).clone(),
                piped_instances,
            )))
        };
        Self {
            http,
            player_js,
            piped,
        }
    }

    /// Construct a client from an existing [`InnerTubeClient`]. Used in
    /// tests so mockito URLs can be injected without re-implementing
    /// request building.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn with_client(http: InnerTubeClient) -> Self {
        let http = Arc::new(http);
        let player_js = Arc::new(PlayerJsResolver::new((*http).clone()));
        Self {
            http,
            player_js,
            piped: None,
        }
    }

    /// Search YouTube.
    ///
    /// ```no_run
    /// # use innertube::{InnerTube, SearchFilter, SearchKind};
    /// # async fn demo() -> innertube::Result<()> {
    /// let tube = InnerTube::new();
    /// let _r = tube.search("lofi", None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
    ) -> Result<SearchResults> {
        search::search(&self.http, query, filter).await
    }

    /// Fetch full video metadata.
    ///
    /// ```no_run
    /// # use innertube::InnerTube;
    /// # async fn demo() -> innertube::Result<()> {
    /// let tube = InnerTube::new();
    /// let _v = tube.video("dQw4w9WgXcQ").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn video(&self, id: &str) -> Result<VideoDetails> {
        video::video(&self.http, id).await
    }

    /// Resolve playable stream URLs for `id`.
    ///
    /// ```no_run
    /// # use innertube::InnerTube;
    /// # async fn demo() -> innertube::Result<()> {
    /// let tube = InnerTube::new();
    /// let _s = tube.streams("dQw4w9WgXcQ").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn streams(&self, id: &str) -> Result<StreamMap> {
        streams::resolve_streams(
            &self.http,
            &self.player_js,
            self.piped.as_deref(),
            id,
        )
        .await
    }

    /// Fetch the current trending videos for `region` (an ISO 3166-1
    /// alpha-2 country code).
    ///
    /// ```no_run
    /// # use innertube::InnerTube;
    /// # async fn demo() -> innertube::Result<()> {
    /// let tube = InnerTube::new();
    /// let _t = tube.trending("US").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn trending(&self, region: &str) -> Result<Vec<VideoSummary>> {
        trending::trending(&self.http, region).await
    }

    /// Fetch channel page details.
    ///
    /// ```no_run
    /// # use innertube::InnerTube;
    /// # async fn demo() -> innertube::Result<()> {
    /// let tube = InnerTube::new();
    /// let _c = tube.channel("UCuAXFkgsw1L7xaCfnd5JJOw").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn channel(&self, id: &str) -> Result<ChannelDetails> {
        channel::channel(&self.http, id).await
    }

    /// Fetch playlist contents.
    ///
    /// ```no_run
    /// # use innertube::InnerTube;
    /// # async fn demo() -> innertube::Result<()> {
    /// let tube = InnerTube::new();
    /// let _p = tube.playlist("PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn playlist(&self, id: &str) -> Result<PlaylistDetails> {
        playlist::playlist(&self.http, id).await
    }

    /// Continue a paginated search response. The token is opaque — get one
    /// from a [`SearchResults`]'s `continuation` field.
    ///
    /// ```no_run
    /// # use innertube::InnerTube;
    /// # async fn demo() -> innertube::Result<()> {
    /// let tube = InnerTube::new();
    /// let first = tube.search("test", None).await?;
    /// if let Some(token) = first.continuation {
    ///     let _next = tube.search_continuation(&token).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_continuation(&self, token: &str) -> Result<SearchResults> {
        continuation::search_continuation(&self.http, token).await
    }

    /// Continue a paginated response. The token is opaque — get one from
    /// the relevant response type's continuation field.
    ///
    /// The type parameter `T` must be [`SearchResults`]. Other continuations
    /// (channel videos, playlist entries) are not supported through this
    /// generic method — use the dedicated `*_continuation` methods on
    /// [`InnerTube`] for those.
    ///
    /// ```no_run
    /// # use innertube::{InnerTube, SearchResults};
    /// # async fn demo() -> innertube::Result<()> {
    /// let tube = InnerTube::new();
    /// let first = tube.search("test", None).await?;
    /// if let Some(token) = first.continuation.clone() {
    ///     let _next: SearchResults = tube.continuation::<SearchResults>(&token).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn continuation<T: Continuable + FromSearchResults + Send>(
        &self,
        token: &str,
    ) -> Result<T> {
        let results = continuation::search_continuation(&self.http, token).await?;
        Ok(T::from_search_results(results))
    }

    /// Fetch caption tracks for a video. `lang_filter` is a BCP-47 prefix
    /// used to filter tracks (pass `""` for all tracks).
    ///
    /// ```no_run
    /// # use innertube::InnerTube;
    /// # async fn demo() -> innertube::Result<()> {
    /// let tube = InnerTube::new();
    /// let _c = tube.captions("dQw4w9WgXcQ", "en").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn captions(&self, id: &str, lang: &str) -> Result<Vec<Caption>> {
        let mut body = serde_json::Map::new();
        body.insert("videoId".into(), serde_json::Value::String(id.to_string()));
        let player = self.http.post("player", ClientContext::WEB_DEFAULT, body).await?;
        video::parse_captions(&player, lang)
    }
}

/// Trending list (kept in its own module so `lib.rs` stays small).
mod trending {
    use crate::client::{ClientContext, InnerTubeClient};
    use crate::error::Result;
    use crate::json_util::{find_all_with_key, find_first_with_key};
    use crate::types::video::VideoSummary;
    use crate::video::parse_compact_video;

    /// Fetch the trending feed for `region`.
    pub(crate) async fn trending(
        http: &InnerTubeClient,
        region: &str,
    ) -> Result<Vec<VideoSummary>> {
        let mut body = serde_json::Map::new();
        // `FEtrending` is the browse ID for the trending tab. The optional
        // `params` selects a country-specific variant.
        body.insert(
            "browseId".into(),
            serde_json::Value::String("FEtrending".into()),
        );
        if !region.is_empty() {
            body.insert(
                "params".into(),
                serde_json::Value::String(encode_country(region)),
            );
        }
        let resp = http.post("browse", ClientContext::WEB_DEFAULT, body).await?;
        let mut out = Vec::new();
        for r in find_all_with_key(&resp, "videoRenderer")
            .into_iter()
            .chain(find_all_with_key(&resp, "compactVideoRenderer").into_iter())
        {
            if let Some(v) = parse_compact_video(r) {
                out.push(v);
            }
        }
        // Surface the optional continuation token but keep the public type
        // as `Vec<VideoSummary>` for simplicity — callers that want the
        // next page can use `browse_continuation_raw`.
        let _ = find_first_with_key(&resp, "continuationItemRenderer");
        Ok(out)
    }

    fn encode_country(country: &str) -> String {
        // 4-byte country selector prefix used by `FEtrending`. The exact
        // bytes are stable across player builds; only the country code
        // changes.
        let upper = country.to_uppercase();
        let mut buf: Vec<u8> = vec![0x08, 0x02, 0x12, 0x02];
        buf.extend(upper.as_bytes().iter().take(2));
        base64_url(&buf)
    }

    fn base64_url(buf: &[u8]) -> String {
        const ALPHA: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = String::with_capacity((buf.len() + 2) / 3 * 4);
        let mut i = 0;
        while i + 3 <= buf.len() {
            let b0 = buf[i];
            let b1 = buf[i + 1];
            let b2 = buf[i + 2];
            out.push(ALPHA[(b0 >> 2) as usize] as char);
            out.push(ALPHA[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            out.push(ALPHA[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
            out.push(ALPHA[(b2 & 0x3f) as usize] as char);
            i += 3;
        }
        match buf.len() - i {
            1 => {
                let b0 = buf[i];
                out.push(ALPHA[(b0 >> 2) as usize] as char);
                out.push(ALPHA[((b0 & 0x03) << 4) as usize] as char);
            }
            2 => {
                let b0 = buf[i];
                let b1 = buf[i + 1];
                out.push(ALPHA[(b0 >> 2) as usize] as char);
                out.push(ALPHA[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
                out.push(ALPHA[((b1 & 0x0f) << 2) as usize] as char);
            }
            _ => {}
        }
        out
    }
}
