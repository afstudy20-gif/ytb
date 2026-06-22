use std::{env, net::SocketAddr, time::Duration};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use innertube::{
    ChannelDetails, InnerTube, PlaylistDetails, PlaylistVideo, SearchFilter, SearchItem,
    SearchKind, SearchResultChannel, SearchResultPlaylist, SearchResultVideo, SearchResults,
    Stream, StreamMap, VideoDetails, VideoSummary,
};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let bind = env::var("WT_BACKEND_BIND").unwrap_or_else(|_| "0.0.0.0:8787".to_string());
    let addr: SocketAddr = bind.parse()?;
    let piped_instances = env::var("WT_PIPED_INSTANCES")
        .ok()
        .map(|value| parse_csv_env(&value))
        .unwrap_or_else(|| vec!["https://api.piped.private.coffee".to_string()]);

    let state = AppState {
        tube: if piped_instances.is_empty() {
            InnerTube::new()
        } else {
            InnerTube::with_piped_fallback(piped_instances)
        },
        http: HttpClient::builder()
            .timeout(Duration::from_secs(12))
            .build()?,
        sponsorblock_base: env::var("SPONSORBLOCK_BASE")
            .unwrap_or_else(|_| "https://sponsor.ajay.app/api".to_string()),
        ryd_base: env::var("RYD_BASE")
            .unwrap_or_else(|_| "https://returnyoutubedislike.com".to_string()),
    };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/search", get(search))
        .route("/trending", get(trending))
        .route("/videos/:id", get(video))
        .route("/streams/:id", get(streams))
        .route("/channels/:id", get(channel))
        .route("/playlists/:id", get(playlist))
        .route("/sponsorblock", get(sponsorblock))
        .route("/ryd/:id", get(ryd))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

#[derive(Clone)]
struct AppState {
    tube: InnerTube,
    http: HttpClient,
    sponsorblock_base: String,
    ryd_base: String,
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({ "ok": true }))
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
    continuation: Option<String>,
    filter: Option<String>,
}

async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResultDto>, ApiError> {
    let results = if let Some(token) = query.continuation.as_deref() {
        state.tube.search_continuation(token).await?
    } else {
        state
            .tube
            .search(
                &query.q,
                Some(SearchFilter {
                    kind: search_kind(query.filter.as_deref()),
                    ..SearchFilter::default()
                }),
            )
            .await?
    };
    Ok(Json(SearchResultDto::from(results)))
}

#[derive(Debug, Deserialize)]
struct TrendingQuery {
    region: Option<String>,
}

async fn trending(
    State(state): State<AppState>,
    Query(query): Query<TrendingQuery>,
) -> Result<Json<Vec<VideoSummaryDto>>, ApiError> {
    let region = query.region.as_deref().unwrap_or("US");
    let videos = state.tube.trending(region).await?;
    Ok(Json(
        videos.into_iter().map(VideoSummaryDto::from).collect(),
    ))
}

async fn video(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<VideoDetailDto>, ApiError> {
    Ok(Json(VideoDetailDto::from(state.tube.video(&id).await?)))
}

async fn streams(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<StreamMapDto>, ApiError> {
    let map = state.tube.streams(&id).await?;
    Ok(Json(StreamMapDto::from_stream_map(id, map)))
}

async fn channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ChannelDetailDto>, ApiError> {
    Ok(Json(ChannelDetailDto::from(state.tube.channel(&id).await?)))
}

async fn playlist(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PlaylistDetailDto>, ApiError> {
    Ok(Json(PlaylistDetailDto::from(
        state.tube.playlist(&id).await?,
    )))
}

#[derive(Debug, Deserialize)]
struct SponsorBlockQuery {
    #[serde(rename = "videoId")]
    video_id: String,
    #[serde(default, rename = "category")]
    categories: Vec<String>,
}

async fn sponsorblock(
    State(state): State<AppState>,
    Query(query): Query<SponsorBlockQuery>,
) -> Result<Json<Vec<SponsorSegmentDto>>, ApiError> {
    let mut req = state
        .http
        .get(format!("{}/skipSegments", state.sponsorblock_base))
        .query(&[("videoID", query.video_id.as_str())]);

    if !query.categories.is_empty() {
        req = req.query(&[(
            "categories",
            serde_json::to_string(&query.categories)
                .map_err(|error| ApiError::internal(error.to_string()))?,
        )]);
    }

    let response = req.send().await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(Json(Vec::new()));
    }
    if !response.status().is_success() {
        return Err(ApiError::upstream(
            response.status(),
            "SponsorBlock request failed",
        ));
    }
    Ok(Json(response.json::<Vec<SponsorSegmentDto>>().await?))
}

async fn ryd(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RydDto>, ApiError> {
    let response = state
        .http
        .get(format!("{}/votes", state.ryd_base))
        .query(&[("videoId", id)])
        .send()
        .await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(Json(RydDto {
            likes: 0,
            dislikes: 0,
        }));
    }
    if !response.status().is_success() {
        return Err(ApiError::upstream(
            response.status(),
            "Return YouTube Dislike request failed",
        ));
    }
    let votes = response.json::<RydApiDto>().await?;
    Ok(Json(RydDto {
        likes: votes.likes.unwrap_or(0),
        dislikes: votes.dislikes.unwrap_or(0),
    }))
}

fn search_kind(filter: Option<&str>) -> SearchKind {
    match filter {
        Some("videos") => SearchKind::Video,
        Some("channels") => SearchKind::Channel,
        Some("playlists") => SearchKind::Playlist,
        _ => SearchKind::All,
    }
}

fn parse_csv_env(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThumbnailDto {
    url: String,
    width: u32,
    height: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorDto {
    id: String,
    name: String,
    avatar_url: String,
    subscriber_count: Option<u64>,
    verified: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoSummaryDto {
    #[serde(rename = "type")]
    item_type: &'static str,
    id: String,
    title: String,
    author: AuthorDto,
    thumbnails: Vec<ThumbnailDto>,
    duration_seconds: u64,
    view_count: u64,
    published_text: String,
}

impl From<VideoSummary> for VideoSummaryDto {
    fn from(video: VideoSummary) -> Self {
        let author = AuthorDto {
            id: video.channel_id,
            name: video.author,
            avatar_url: String::new(),
            subscriber_count: None,
            verified: false,
        };
        Self {
            item_type: "video",
            id: video.id,
            title: video.title,
            author,
            thumbnails: thumbnail_list(video.thumbnail_url, 640, 360),
            duration_seconds: video.length_seconds.unwrap_or(0),
            view_count: video
                .view_count
                .or_else(|| video.view_count_text.as_deref().and_then(parse_count_text))
                .unwrap_or(0),
            published_text: String::new(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChannelSummaryDto {
    #[serde(rename = "type")]
    item_type: &'static str,
    id: String,
    name: String,
    avatar_url: String,
    subscriber_count: u64,
    video_count: u64,
    verified: bool,
    description_short: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistSummaryDto {
    #[serde(rename = "type")]
    item_type: &'static str,
    id: String,
    title: String,
    author: AuthorDto,
    thumbnail_url: String,
    video_count: u64,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum SearchItemDto {
    Video(VideoSummaryDto),
    Channel(ChannelSummaryDto),
    Playlist(PlaylistSummaryDto),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResultDto {
    items: Vec<SearchItemDto>,
    continuation: Option<String>,
    estimated_results: u64,
}

impl From<SearchResults> for SearchResultDto {
    fn from(results: SearchResults) -> Self {
        let items = results
            .items
            .into_iter()
            .filter_map(|item| match item {
                SearchItem::Video(video) => Some(SearchItemDto::Video(video.into())),
                SearchItem::Channel(channel) => Some(SearchItemDto::Channel(channel.into())),
                SearchItem::Playlist(playlist) => Some(SearchItemDto::Playlist(playlist.into())),
                SearchItem::Shelf { .. } => None,
            })
            .collect();

        Self {
            items,
            continuation: results.continuation,
            estimated_results: results.estimated_results.unwrap_or(0),
        }
    }
}

impl From<SearchResultVideo> for VideoSummaryDto {
    fn from(video: SearchResultVideo) -> Self {
        Self {
            item_type: "video",
            id: video.id,
            title: video.title,
            author: AuthorDto {
                id: video.channel_id,
                name: video.author,
                avatar_url: String::new(),
                subscriber_count: None,
                verified: false,
            },
            thumbnails: thumbnail_list(video.thumbnail_url, 640, 360),
            duration_seconds: video
                .length_text
                .as_deref()
                .and_then(length_text_to_seconds)
                .unwrap_or(0),
            view_count: video
                .view_count_text
                .as_deref()
                .and_then(parse_count_text)
                .unwrap_or(0),
            published_text: video.published_text.unwrap_or_default(),
        }
    }
}

impl From<SearchResultChannel> for ChannelSummaryDto {
    fn from(channel: SearchResultChannel) -> Self {
        Self {
            item_type: "channel",
            id: channel.id,
            name: channel.title,
            avatar_url: channel.avatar_url.unwrap_or_default(),
            subscriber_count: channel
                .subscriber_count_text
                .as_deref()
                .and_then(parse_count_text)
                .unwrap_or(0),
            video_count: 0,
            verified: false,
            description_short: channel.description.unwrap_or_default(),
        }
    }
}

impl From<SearchResultPlaylist> for PlaylistSummaryDto {
    fn from(playlist: SearchResultPlaylist) -> Self {
        Self {
            item_type: "playlist",
            id: playlist.id,
            title: playlist.title,
            author: AuthorDto {
                id: String::new(),
                name: playlist.author.unwrap_or_default(),
                avatar_url: String::new(),
                subscriber_count: None,
                verified: false,
            },
            thumbnail_url: playlist.thumbnail_url.unwrap_or_default(),
            video_count: playlist.video_count.unwrap_or(0),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChapterDto {
    title: String,
    start_seconds: u64,
    thumbnail_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoDetailDto {
    id: String,
    title: String,
    author: AuthorDto,
    description: String,
    view_count: u64,
    like_count: u64,
    published_text: String,
    duration_seconds: u64,
    thumbnails: Vec<ThumbnailDto>,
    keywords: Vec<String>,
    chapters: Vec<ChapterDto>,
}

impl From<VideoDetails> for VideoDetailDto {
    fn from(video: VideoDetails) -> Self {
        Self {
            id: video.id,
            title: video.title,
            author: AuthorDto {
                id: video.channel_id,
                name: video.author,
                avatar_url: String::new(),
                subscriber_count: video
                    .subscriber_count_text
                    .as_deref()
                    .and_then(parse_count_text),
                verified: false,
            },
            description: video.description,
            view_count: video
                .view_count_text
                .as_deref()
                .and_then(parse_count_text)
                .unwrap_or(0),
            like_count: video.likes.unwrap_or(0),
            published_text: video.publish_date.unwrap_or_default(),
            duration_seconds: video.length_seconds.unwrap_or(0),
            thumbnails: thumbnail_list(video.thumbnail_url, 640, 360),
            keywords: video.keywords,
            chapters: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FormatDto {
    itag: u32,
    quality_label: String,
    mime_type: String,
    bitrate: u64,
    url: String,
    audio_only: bool,
}

impl From<Stream> for FormatDto {
    fn from(stream: Stream) -> Self {
        let audio_only = stream.has_audio() && !stream.has_video();
        Self {
            itag: stream.itag,
            quality_label: stream
                .quality_label
                .clone()
                .unwrap_or_else(|| quality_label(&stream, audio_only)),
            mime_type: stream.mime_type,
            bitrate: stream.bitrate.unwrap_or(0),
            url: stream.url,
            audio_only,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamMapDto {
    video_id: String,
    formats: Vec<FormatDto>,
    adaptive_formats: Vec<FormatDto>,
    expires_in_seconds: u64,
}

impl StreamMapDto {
    fn from_stream_map(video_id: String, map: StreamMap) -> Self {
        let mut formats = map
            .progressive
            .into_iter()
            .map(FormatDto::from)
            .collect::<Vec<_>>();
        formats.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));

        let mut adaptive_formats = map
            .adaptive_video
            .into_iter()
            .chain(map.adaptive_audio.into_iter())
            .map(FormatDto::from)
            .collect::<Vec<_>>();
        adaptive_formats.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));

        Self {
            video_id,
            formats,
            adaptive_formats,
            expires_in_seconds: 21_600,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChannelDetailDto {
    id: String,
    name: String,
    avatar_url: String,
    banner_url: Option<String>,
    subscriber_count: u64,
    verified: bool,
    description: String,
    video_count: u64,
    videos: Vec<VideoSummaryDto>,
    playlists: Vec<PlaylistSummaryDto>,
}

impl From<ChannelDetails> for ChannelDetailDto {
    fn from(channel: ChannelDetails) -> Self {
        Self {
            id: channel.id,
            name: channel.title,
            avatar_url: channel.avatar_url.unwrap_or_default(),
            banner_url: channel.banner_url,
            subscriber_count: channel
                .subscriber_count_text
                .as_deref()
                .and_then(parse_count_text)
                .unwrap_or(0),
            verified: channel.badges.iter().any(|badge| {
                badge.style.to_ascii_lowercase().contains("verified")
                    || badge.label.to_ascii_lowercase().contains("verified")
            }),
            description: channel.description,
            video_count: channel
                .video_count_text
                .as_deref()
                .and_then(parse_count_text)
                .unwrap_or(0),
            videos: channel
                .videos
                .into_iter()
                .map(VideoSummaryDto::from)
                .collect(),
            playlists: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistDetailDto {
    id: String,
    title: String,
    author: AuthorDto,
    description: String,
    video_count: u64,
    thumbnails: Vec<ThumbnailDto>,
    videos: Vec<VideoSummaryDto>,
}

impl From<PlaylistDetails> for PlaylistDetailDto {
    fn from(playlist: PlaylistDetails) -> Self {
        let thumbnail_url = playlist
            .videos
            .first()
            .and_then(|video| video.thumbnail_url.clone());

        Self {
            id: playlist.id,
            title: playlist.title,
            author: AuthorDto {
                id: playlist.channel_id.unwrap_or_default(),
                name: playlist.author.unwrap_or_default(),
                avatar_url: String::new(),
                subscriber_count: None,
                verified: false,
            },
            description: playlist.view_count_text.unwrap_or_default(),
            video_count: playlist.video_count.unwrap_or(playlist.videos.len() as u64),
            thumbnails: thumbnail_list(thumbnail_url, 640, 360),
            videos: playlist
                .videos
                .into_iter()
                .map(VideoSummaryDto::from)
                .collect(),
        }
    }
}

impl From<PlaylistVideo> for VideoSummaryDto {
    fn from(video: PlaylistVideo) -> Self {
        Self {
            item_type: "video",
            id: video.id,
            title: video.title,
            author: AuthorDto {
                id: video.channel_id,
                name: video.author,
                avatar_url: String::new(),
                subscriber_count: None,
                verified: false,
            },
            thumbnails: thumbnail_list(video.thumbnail_url, 640, 360),
            duration_seconds: video.length_seconds.unwrap_or(0),
            view_count: 0,
            published_text: video.index.map(|i| format!("#{i}")).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SponsorSegmentDto {
    category: String,
    segment: [f64; 2],
    #[serde(rename = "UUID")]
    uuid: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RydApiDto {
    likes: Option<u64>,
    dislikes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct RydDto {
    likes: u64,
    dislikes: u64,
}

fn thumbnail_list(url: Option<String>, width: u32, height: u32) -> Vec<ThumbnailDto> {
    url.filter(|value| !value.is_empty())
        .map(|url| vec![ThumbnailDto { url, width, height }])
        .unwrap_or_default()
}

fn quality_label(stream: &Stream, audio_only: bool) -> String {
    if audio_only {
        return "audio".to_string();
    }
    stream
        .height
        .map(|height| format!("{height}p"))
        .unwrap_or_else(|| "auto".to_string())
}

fn parse_count_text(text: &str) -> Option<u64> {
    let token = text
        .split_whitespace()
        .find(|part| part.chars().any(|ch| ch.is_ascii_digit()))?;
    let cleaned = token.trim_matches(|ch: char| {
        !(ch.is_ascii_digit() || ch == '.' || ch == ',' || ch.is_ascii_alphabetic())
    });
    let multiplier = if cleaned.ends_with(['K', 'k']) {
        1_000.0
    } else if cleaned.ends_with(['M', 'm']) {
        1_000_000.0
    } else if cleaned.ends_with(['B', 'b']) {
        1_000_000_000.0
    } else {
        1.0
    };
    let number = cleaned
        .trim_end_matches(|ch: char| ch.is_ascii_alphabetic())
        .replace(',', "");
    number
        .parse::<f64>()
        .ok()
        .map(|value| (value * multiplier) as u64)
}

fn length_text_to_seconds(text: &str) -> Option<u64> {
    let mut total = 0_u64;
    for part in text.split(':') {
        total = total.checked_mul(60)?;
        total = total.checked_add(part.parse::<u64>().ok()?)?;
    }
    Some(total)
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    fn upstream(status: StatusCode, message: impl Into<String>) -> Self {
        let status = if status.is_client_error() || status.is_server_error() {
            status
        } else {
            StatusCode::BAD_GATEWAY
        };
        Self {
            status,
            message: message.into(),
        }
    }
}

impl From<innertube::Error> for ApiError {
    fn from(error: innertube::Error) -> Self {
        let status = match error {
            innertube::Error::NoStreams(_)
            | innertube::Error::Unavailable(_, _)
            | innertube::Error::AgeRestricted(_)
            | innertube::Error::Region(_) => StatusCode::NOT_FOUND,
            innertube::Error::HttpStatus { status, .. } => {
                StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY)
            }
            _ => StatusCode::BAD_GATEWAY,
        };
        Self {
            status,
            message: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}
