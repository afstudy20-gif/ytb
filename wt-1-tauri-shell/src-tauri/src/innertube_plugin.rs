use innertube::InnerTube;
use innertube::SearchItem;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SearchArgs {
    pub query: String,
    pub filter: Option<String>,
}

#[derive(Deserialize)]
pub struct VideoIdArgs {
    pub id: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub items: Vec<SearchResultItem>,
}

#[derive(Serialize)]
pub struct SearchResultItem {
    pub id: String,
    pub title: String,
    pub thumbnail: String,
    pub author: String,
}

#[derive(Serialize)]
pub struct VideoDetail {
    pub id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub channel_id: String,
    pub view_count: String,
    pub likes: Option<u64>,
    pub duration: Option<u64>,
    pub thumbnail: String,
}

#[derive(Serialize)]
pub struct Stream {
    pub itag: u32,
    pub url: String,
    pub mime_type: String,
    pub bitrate: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub quality_label: Option<String>,
}

#[derive(Serialize)]
pub struct StreamMap {
    pub progressive: Vec<Stream>,
    pub adaptive_video: Vec<Stream>,
    pub adaptive_audio: Vec<Stream>,
    pub hls_manifest_url: Option<String>,
}

#[derive(Serialize)]
pub struct ChannelDetail {
    pub id: String,
    pub author: String,
    pub subscribers: Option<String>,
    pub video_count: Option<String>,
    pub description: String,
}

#[derive(Serialize)]
pub struct PlaylistDetail {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub video_count: Option<u64>,
}

#[derive(Serialize)]
pub struct SponsorSegment {
    pub category: String,
    pub segment: Vec<String>,
    pub locked: bool,
}

#[derive(Serialize)]
pub struct RYDResponse {
    pub likes: u64,
    pub dislikes: u64,
}

#[tauri::command]
pub async fn search(args: SearchArgs) -> Result<SearchResult, String> {
    let tube = InnerTube::new();
    let results = tube.search(&args.query, None).await.map_err(|e| e.to_string())?;
    
    let items: Vec<SearchResultItem> = results
        .items
        .into_iter()
        .take(20)
        .filter_map(|item| {
            match item {
                SearchItem::Video(v) => Some(SearchResultItem {
                    id: v.id,
                    title: v.title,
                    thumbnail: v.thumbnail_url.unwrap_or_default(),
                    author: v.author,
                }),
                _ => None,
            }
        })
        .collect();
    
    Ok(SearchResult { items })
}

#[tauri::command]
pub async fn video(args: VideoIdArgs) -> Result<VideoDetail, String> {
    let tube = InnerTube::new();
    let details = tube.video(&args.id).await.map_err(|e| e.to_string())?;
    
    Ok(VideoDetail {
        id: details.id,
        title: details.title,
        description: details.description,
        author: details.author,
        channel_id: details.channel_id,
        view_count: details.view_count_text.unwrap_or_default(),
        likes: details.likes,
        duration: details.length_seconds,
        thumbnail: details.thumbnail_url.unwrap_or_default(),
    })
}

#[tauri::command]
pub async fn streams(args: VideoIdArgs) -> Result<StreamMap, String> {
    let tube = InnerTube::new();
    let map = tube.streams(&args.id).await.map_err(|e| e.to_string())?;
    
    let to_stream = |s: innertube::Stream| -> Stream {
        Stream {
            itag: s.itag,
            url: s.url,
            mime_type: s.mime_type,
            bitrate: s.bitrate,
            width: s.width,
            height: s.height,
            quality_label: s.quality_label,
        }
    };
    
    Ok(StreamMap {
        progressive: map.progressive.into_iter().map(to_stream).collect(),
        adaptive_video: map.adaptive_video.into_iter().map(to_stream).collect(),
        adaptive_audio: map.adaptive_audio.into_iter().map(to_stream).collect(),
        hls_manifest_url: map.hls_manifest_url,
    })
}

#[tauri::command]
pub async fn channel(args: VideoIdArgs) -> Result<ChannelDetail, String> {
    let tube = InnerTube::new();
    let details = tube.channel(&args.id).await.map_err(|e| e.to_string())?;
    
    Ok(ChannelDetail {
        id: details.id,
        author: details.title,
        subscribers: details.subscriber_count_text,
        video_count: details.video_count_text,
        description: details.description,
    })
}

#[tauri::command]
pub async fn playlist(args: VideoIdArgs) -> Result<PlaylistDetail, String> {
    let tube = InnerTube::new();
    let details = tube.playlist(&args.id).await.map_err(|e| e.to_string())?;
    
    Ok(PlaylistDetail {
        id: details.id,
        title: details.title,
        author: details.author,
        video_count: details.video_count,
    })
}

#[tauri::command]
pub async fn sponsor_block(
    _args: VideoIdArgs,
    _categories: Vec<String>,
) -> Result<Vec<SponsorSegment>, String> {
    Ok(vec![])
}

#[tauri::command]
pub async fn return_youtube_dislike(_args: VideoIdArgs) -> Result<RYDResponse, String> {
    Ok(RYDResponse {
        likes: 0,
        dislikes: 0,
    })
}
