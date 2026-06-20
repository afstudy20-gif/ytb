//! Tauri command bridge over the wt-3-innertube crate.
//!
//! The JSON shapes returned here intentionally mirror the TypeScript types in
//! `wt-2-ui/src/lib/types.ts` so the frontend can consume them directly.

use std::str::FromStr;

use innertube::{InnerTube, SearchFilter, SearchKind, SortBy};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DTOs matching wt-2-ui/src/lib/types.ts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Author {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscriber_count: Option<u64>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SearchItem {
    #[serde(rename = "video")]
    Video(VideoSummary),
    #[serde(rename = "channel")]
    Channel(ChannelSummary),
    #[serde(rename = "playlist")]
    Playlist(PlaylistSummary),
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoSummary {
    pub r#type: String,
    pub id: String,
    pub title: String,
    pub author: Author,
    pub thumbnails: Vec<Thumbnail>,
    pub duration_seconds: u64,
    pub view_count: u64,
    pub published_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelSummary {
    pub r#type: String,
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub subscriber_count: u64,
    pub video_count: u64,
    pub verified: bool,
    pub description_short: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaylistSummary {
    pub r#type: String,
    pub id: String,
    pub title: String,
    pub author: Author,
    pub thumbnail_url: String,
    pub video_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub items: Vec<SearchItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    pub estimated_results: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Chapter {
    pub title: String,
    pub start_seconds: u64,
    pub thumbnail_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoDetail {
    pub id: String,
    pub title: String,
    pub author: Author,
    pub description: String,
    pub view_count: u64,
    pub like_count: u64,
    pub published_text: String,
    pub duration_seconds: u64,
    pub thumbnails: Vec<Thumbnail>,
    pub keywords: Vec<String>,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Format {
    pub itag: u32,
    pub quality_label: String,
    pub mime_type: String,
    pub bitrate: u64,
    pub url: String,
    pub audio_only: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamMap {
    pub video_id: String,
    pub formats: Vec<Format>,
    pub adaptive_formats: Vec<Format>,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelDetail {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner_url: Option<String>,
    pub subscriber_count: u64,
    pub verified: bool,
    pub description: String,
    pub video_count: u64,
    pub videos: Vec<VideoSummary>,
    pub playlists: Vec<PlaylistSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaylistDetail {
    pub id: String,
    pub title: String,
    pub author: Author,
    pub description: String,
    pub video_count: u64,
    pub thumbnails: Vec<Thumbnail>,
    pub videos: Vec<VideoSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SponsorSegment {
    pub category: String,
    pub segment: [f64; 2],
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RydResult {
    pub likes: u64,
    pub dislikes: u64,
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

const DEFAULT_THUMBNAIL_WIDTH: u32 = 640;
const DEFAULT_THUMBNAIL_HEIGHT: u32 = 360;
const DEFAULT_AVATAR: &str = "https://www.gstatic.com/youtube/img/originals/promo/ytr-logo-for-search_96x96.png";

fn thumbnail_from_url(url: Option<&str>) -> Vec<Thumbnail> {
    match url {
        Some(u) if !u.is_empty() => vec![Thumbnail {
            url: u.to_string(),
            width: DEFAULT_THUMBNAIL_WIDTH,
            height: DEFAULT_THUMBNAIL_HEIGHT,
        }],
        _ => vec![],
    }
}

fn thumbnail_url_single(url: Option<&str>) -> String {
    url.unwrap_or(DEFAULT_AVATAR).to_string()
}

/// Parse strings like `1.2M views`, `3,456`, `1.5K` into an integer.
fn parse_human_number(text: Option<&str>) -> Option<u64> {
    let text = text?;
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    let cleaned = cleaned.replace(',', "");
    if cleaned.is_empty() {
        return None;
    }
    let multiplier: u64 = if text.to_lowercase().contains('b') {
        1_000_000_000
    } else if text.to_lowercase().contains('m') {
        1_000_000
    } else if text.to_lowercase().contains('k') {
        1_000
    } else {
        1
    };

    let value = if let Ok(i) = u64::from_str(&cleaned) {
        i
    } else if let Ok(f) = f64::from_str(&cleaned) {
        f as u64
    } else {
        return None;
    };

    Some(value * multiplier)
}

fn parse_length_text(text: Option<&str>) -> Option<u64> {
    let text = text?;
    let parts: Vec<&str> = text.split(':').collect();
    let mut seconds = 0_u64;
    for part in parts {
        let n = u64::from_str(part).ok()?;
        seconds = seconds.checked_mul(60)?.checked_add(n)?;
    }
    Some(seconds)
}

fn parse_subscriber_text(text: Option<&str>) -> u64 {
    parse_human_number(text).unwrap_or(0)
}

fn parse_view_text(text: Option<&str>) -> u64 {
    parse_human_number(text).unwrap_or(0)
}

fn parse_video_count_text(text: Option<&str>) -> u64 {
    parse_human_number(text).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Mapping from wt-3-innertube types to DTOs
// ---------------------------------------------------------------------------

fn map_author(
    name: impl Into<String>,
    channel_id: impl Into<String>,
    subscriber_text: Option<&str>,
    avatar_url: Option<&str>,
) -> Author {
    Author {
        id: channel_id.into(),
        name: name.into(),
        avatar_url: avatar_url.map(String::from).unwrap_or_else(|| DEFAULT_AVATAR.to_string()),
        subscriber_count: parse_human_number(subscriber_text),
        verified: false,
    }
}

fn map_innertube_video(v: &innertube::SearchResultVideo) -> VideoSummary {
    let author = map_author(
        &v.author,
        &v.channel_id,
        v.view_count_text.as_deref(),
        None,
    );
    VideoSummary {
        r#type: "video".to_string(),
        id: v.id.clone(),
        title: v.title.clone(),
        author,
        thumbnails: thumbnail_from_url(v.thumbnail_url.as_deref()),
        duration_seconds: parse_length_text(v.length_text.as_deref()).unwrap_or(0),
        view_count: parse_view_text(v.view_count_text.as_deref()),
        published_text: v.published_text.clone().unwrap_or_default(),
    }
}

fn map_innertube_video_summary(v: &innertube::VideoSummary) -> VideoSummary {
    let author = map_author(
        &v.author,
        &v.channel_id,
        v.view_count_text.as_deref(),
        None,
    );
    VideoSummary {
        r#type: "video".to_string(),
        id: v.id.clone(),
        title: v.title.clone(),
        author,
        thumbnails: thumbnail_from_url(v.thumbnail_url.as_deref()),
        duration_seconds: v.length_seconds.unwrap_or_else(|| parse_length_text(v.length_text.as_deref()).unwrap_or(0)),
        view_count: v.view_count.unwrap_or_else(|| parse_view_text(v.view_count_text.as_deref())),
        published_text: v.view_count_text.clone().unwrap_or_default(),
    }
}

fn map_innertube_channel(c: &innertube::SearchResultChannel) -> ChannelSummary {
    ChannelSummary {
        r#type: "channel".to_string(),
        id: c.id.clone(),
        name: c.title.clone(),
        avatar_url: thumbnail_url_single(c.avatar_url.as_deref()),
        subscriber_count: parse_subscriber_text(c.subscriber_count_text.as_deref()),
        video_count: 0,
        verified: false,
        description_short: c.description.clone().unwrap_or_default(),
    }
}

fn map_innertube_playlist(p: &innertube::SearchResultPlaylist) -> PlaylistSummary {
    let author = map_author(
        p.author.as_deref().unwrap_or(""),
        "",
        None,
        None,
    );
    PlaylistSummary {
        r#type: "playlist".to_string(),
        id: p.id.clone(),
        title: p.title.clone(),
        author,
        thumbnail_url: thumbnail_url_single(p.thumbnail_url.as_deref()),
        video_count: p.video_count.unwrap_or(0),
    }
}

fn map_innertube_search_item(item: &innertube::SearchItem) -> Option<SearchItem> {
    match item {
        innertube::SearchItem::Video(v) => Some(SearchItem::Video(map_innertube_video(v))),
        innertube::SearchItem::Channel(c) => Some(SearchItem::Channel(map_innertube_channel(c))),
        innertube::SearchItem::Playlist(p) => Some(SearchItem::Playlist(map_innertube_playlist(p))),
        innertube::SearchItem::Shelf { items, .. } => {
            // Flatten shelves into the result list for the UI.
            items.iter().find_map(map_innertube_search_item)
        }
    }
}

fn map_video_detail(v: &innertube::VideoDetails) -> VideoDetail {
    let author = map_author(
        &v.author,
        &v.channel_id,
        v.subscriber_count_text.as_deref(),
        None,
    );
    VideoDetail {
        id: v.id.clone(),
        title: v.title.clone(),
        author,
        description: v.description.clone(),
        view_count: parse_view_text(v.view_count_text.as_deref()),
        like_count: v.likes.unwrap_or(0),
        published_text: v.publish_date.clone().unwrap_or_default(),
        duration_seconds: v.length_seconds.unwrap_or(0),
        thumbnails: thumbnail_from_url(v.thumbnail_url.as_deref()),
        keywords: v.keywords.clone(),
        chapters: vec![],
    }
}

fn map_channel_detail(c: &innertube::ChannelDetails) -> ChannelDetail {
    let verified = c.badges.iter().any(|b| {
        b.style
            .to_ascii_uppercase()
            .contains("VERIFIED")
    });
    ChannelDetail {
        id: c.id.clone(),
        name: c.title.clone(),
        avatar_url: thumbnail_url_single(c.avatar_url.as_deref()),
        banner_url: c.banner_url.clone(),
        subscriber_count: parse_subscriber_text(c.subscriber_count_text.as_deref()),
        verified,
        description: c.description.clone(),
        video_count: parse_video_count_text(c.video_count_text.as_deref()),
        videos: c.videos.iter().map(map_innertube_video_summary).collect(),
        playlists: vec![],
    }
}

fn map_playlist_video(v: &innertube::PlaylistVideo) -> VideoSummary {
    let author = map_author(
        &v.author,
        &v.channel_id,
        None,
        None,
    );
    VideoSummary {
        r#type: "video".to_string(),
        id: v.id.clone(),
        title: v.title.clone(),
        author,
        thumbnails: thumbnail_from_url(v.thumbnail_url.as_deref()),
        duration_seconds: v.length_seconds.unwrap_or_else(|| parse_length_text(v.length_text.as_deref()).unwrap_or(0)),
        view_count: 0,
        published_text: String::new(),
    }
}

fn map_playlist_detail(p: &innertube::PlaylistDetails) -> PlaylistDetail {
    let author = map_author(
        p.author.as_deref().unwrap_or(""),
        p.channel_id.as_deref().unwrap_or(""),
        None,
        None,
    );
    PlaylistDetail {
        id: p.id.clone(),
        title: p.title.clone(),
        author,
        description: String::new(),
        video_count: p.video_count.unwrap_or(0),
        thumbnails: thumbnail_from_url(p.videos.first().and_then(|v| v.thumbnail_url.as_deref())),
        videos: p.videos.iter().map(map_playlist_video).collect(),
    }
}

fn map_stream(s: &innertube::Stream) -> Format {
    Format {
        itag: s.itag,
        quality_label: s.quality_label.clone().unwrap_or_else(|| {
            if s.mime_type.starts_with("audio") {
                "audio".to_string()
            } else {
                format!("{}x{}", s.width.unwrap_or(0), s.height.unwrap_or(0))
            }
        }),
        mime_type: s.mime_type.clone(),
        bitrate: s.bitrate.unwrap_or(0),
        url: s.url.clone(),
        audio_only: s.mime_type.starts_with("audio") || s.width.is_none(),
    }
}

fn map_stream_map(id: &str, sm: &innertube::StreamMap) -> StreamMap {
    let mut formats: Vec<Format> = sm.progressive.iter().map(map_stream).collect();
    formats.extend(sm.adaptive_video.iter().map(map_stream));
    let mut adaptive = sm.adaptive_video.iter().map(map_stream).collect::<Vec<_>>();
    adaptive.extend(sm.adaptive_audio.iter().map(map_stream));

    StreamMap {
        video_id: id.to_string(),
        formats,
        adaptive_formats: adaptive,
        expires_in_seconds: 21_600, // 6h fallback; real expiry is encoded in the URLs.
    }
}

// ---------------------------------------------------------------------------
// Search filter mapping
// ---------------------------------------------------------------------------

fn to_innertube_filter(filter: Option<&str>) -> SearchFilter {
    let kind = match filter {
        Some("videos") => SearchKind::Video,
        Some("channels") => SearchKind::Channel,
        Some("playlists") => SearchKind::Playlist,
        _ => SearchKind::All,
    };
    SearchFilter {
        kind,
        upload_date: None,
        duration: None,
        sort_by: SortBy::Relevance,
    }
}

// ---------------------------------------------------------------------------
// Tauri state and commands
// ---------------------------------------------------------------------------

pub struct InnertubeState {
    pub client: InnerTube,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default)]
    pub continuation: Option<String>,
    #[serde(default)]
    pub filter: Option<String>,
}

#[tauri::command]
pub async fn yt_search(
    state: tauri::State<'_, InnertubeState>,
    request: SearchRequest,
) -> Result<SearchResult, String> {
    let results = if let Some(token) = &request.continuation {
        state
            .client
            .search_continuation(token)
            .await
            .map_err(|e| format!("search continuation failed: {e}"))?
    } else {
        let filter = to_innertube_filter(request.filter.as_deref());
        state
            .client
            .search(&request.query, Some(filter))
            .await
            .map_err(|e| format!("search failed: {e}"))?
    };

    Ok(SearchResult {
        items: results.items.iter().filter_map(map_innertube_search_item).collect(),
        continuation: results.continuation,
        estimated_results: results.estimated_results.unwrap_or(0),
    })
}

#[tauri::command]
pub async fn yt_trending(
    state: tauri::State<'_, InnertubeState>,
    region: Option<String>,
) -> Result<Vec<VideoSummary>, String> {
    let region = region.as_deref().unwrap_or("US");
    let videos = state
        .client
        .trending(region)
        .await
        .map_err(|e| format!("trending failed: {e}"))?;
    Ok(videos.iter().map(map_innertube_video_summary).collect())
}

#[tauri::command]
pub async fn yt_video(
    state: tauri::State<'_, InnertubeState>,
    id: String,
) -> Result<VideoDetail, String> {
    let detail = state
        .client
        .video(&id)
        .await
        .map_err(|e| format!("video detail failed: {e}"))?;
    Ok(map_video_detail(&detail))
}

#[tauri::command]
pub async fn yt_streams(
    state: tauri::State<'_, InnertubeState>,
    id: String,
) -> Result<StreamMap, String> {
    let streams = state
        .client
        .streams(&id)
        .await
        .map_err(|e| format!("stream resolution failed: {e}"))?;
    Ok(map_stream_map(&id, &streams))
}

#[tauri::command]
pub async fn yt_channel(
    state: tauri::State<'_, InnertubeState>,
    id: String,
) -> Result<ChannelDetail, String> {
    let detail = state
        .client
        .channel(&id)
        .await
        .map_err(|e| format!("channel detail failed: {e}"))?;
    Ok(map_channel_detail(&detail))
}

#[tauri::command]
pub async fn yt_playlist(
    state: tauri::State<'_, InnertubeState>,
    id: String,
) -> Result<PlaylistDetail, String> {
    let detail = state
        .client
        .playlist(&id)
        .await
        .map_err(|e| format!("playlist detail failed: {e}"))?;
    Ok(map_playlist_detail(&detail))
}

#[tauri::command]
pub async fn yt_sponsor_block(
    id: String,
    categories: Vec<String>,
) -> Result<Vec<SponsorSegment>, String> {
    if categories.is_empty() {
        return Ok(vec![]);
    }
    let cats = serde_json::to_string(&categories).map_err(|e| e.to_string())?;
    let url = format!(
        "https://sponsor.ajay.app/api/skipSegments?videoID={}&categories={}",
        urlencoding::encode(&id),
        urlencoding::encode(&cats)
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("sponsorblock request failed: {e}"))?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let raw: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("sponsorblock decode failed: {e}"))?;
    let mut out = Vec::new();
    for entry in raw {
        let segment = entry
            .get("segment")
            .and_then(|s| s.as_array())
            .and_then(|a| {
                if a.len() == 2 {
                    Some([a[0].as_f64()?, a[1].as_f64()?])
                } else {
                    None
                }
            });
        if let Some(segment) = segment {
            out.push(SponsorSegment {
                category: entry
                    .get("category")
                    .and_then(|c| c.as_str())
                    .unwrap_or("sponsor")
                    .to_string(),
                segment,
                uuid: entry
                    .get("UUID")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn yt_return_youtube_dislike(id: String) -> Result<RydResult, String> {
    let url = format!("https://returnyoutubedislikeapi.com/votes?videoId={}", id);
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("RYD request failed: {e}"))?;
    if !resp.status().is_success() {
        return Ok(RydResult { likes: 0, dislikes: 0 });
    }
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("RYD decode failed: {e}"))?;
    Ok(RydResult {
        likes: json.get("likes").and_then(|v| v.as_u64()).unwrap_or(0),
        dislikes: json.get("dislikes").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}
