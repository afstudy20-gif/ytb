//! Channel endpoint.

use serde_json::{Map, Value};

use crate::client::{ClientContext, InnerTubeClient};
use crate::error::Result;
use crate::json_util::{
    collect_text, find_all_with_key, first_thumbnail, length_text_to_seconds, parse_count,
};
use crate::types::channel::{ChannelBadge, ChannelDetails};
use crate::types::video::VideoSummary;

/// Fetch channel metadata via InnerTube's `browse` endpoint.
pub(crate) async fn channel(http: &InnerTubeClient, id: &str) -> Result<ChannelDetails> {
    let mut body = Map::new();
    body.insert("browseId".into(), Value::String(id.to_string()));
    let resp = http.post("browse", ClientContext::WEB_DEFAULT, body).await?;
    parse_channel_response(&resp, id)
}

/// Parse a channel `browse` response.
pub(crate) fn parse_channel_response(resp: &Value, id: &str) -> Result<ChannelDetails> {
    let header = resp.get("header").unwrap_or(&Value::Null);
    let metadata = resp
        .get("metadata")
        .and_then(|m| m.get("channelMetadataRenderer"))
        .cloned()
        .unwrap_or(Value::Null);

    let title = metadata
        .get("title")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default();
    let description = metadata
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default();
    let avatar_url = metadata
        .get("avatar")
        .and_then(|a| a.get("thumbnails"))
        .and_then(|t| t.as_array())
        .and_then(|a| a.last())
        .and_then(|f| f.get("url"))
        .and_then(|u| u.as_str())
        .map(String::from);
    let keywords = metadata
        .get("keywords")
        .and_then(|v| v.as_str())
        .map(String::from);

    let _ = keywords;

    // Header-level fields (subscriber count etc.).
    let subscriber_count_text = find_in_header(header, "subscriberCountText");
    let video_count_text = find_in_header(header, "videosCountText")
        .or_else(|| find_in_header(header, "videoCountText"));
    let banner_url = header
        .get("c4TabbedHeaderRenderer")
        .and_then(|h| h.get("banner"))
        .and_then(|b| b.get("thumbnails"))
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|f| f.get("url"))
        .and_then(|u| u.as_str())
        .map(String::from);
    let country = find_in_header(header, "country");
    let badges = parse_badges(header);

    // The "videos" tab content.
    let (videos, videos_continuation) = collect_channel_videos(resp);

    Ok(ChannelDetails {
        id: id.to_string(),
        title,
        description,
        subscriber_count_text,
        video_count_text,
        country,
        avatar_url,
        banner_url,
        badges,
        videos,
        videos_continuation,
    })
}

fn find_in_header(header: &Value, key: &str) -> Option<String> {
    // Header shape varies (c4TabbedHeaderRenderer, pageHeaderRenderer).
    for renderer_key in [
        "c4TabbedHeaderRenderer",
        "pageHeaderRenderer",
        "carouselHeaderRenderer",
    ] {
        if let Some(h) = header.get(renderer_key) {
            if let Some(v) = h.get(key).and_then(collect_text) {
                return Some(v);
            }
        }
    }
    None
}

fn parse_badges(header: &Value) -> Vec<ChannelBadge> {
    let Some(arr) = header
        .get("c4TabbedHeaderRenderer")
        .and_then(|h| h.get("badges"))
        .and_then(|b| b.as_array())
    else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|b| {
            let r = b.get("metadataBadgeRenderer")?;
            Some(ChannelBadge {
                style: r
                    .get("style")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                label: r
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect()
}

fn collect_channel_videos(resp: &Value) -> (Vec<VideoSummary>, Option<String>) {
    let mut videos = Vec::new();
    for r in find_all_with_key(resp, "gridVideoRenderer")
        .into_iter()
        .chain(find_all_with_key(resp, "compactVideoRenderer").into_iter())
    {
        if let Some(v) = parse_grid_video(r) {
            videos.push(v);
        }
    }
    let continuation = crate::json_util::find_first_with_key(resp, "continuationItemRenderer")
        .and_then(|c| {
            c.get("continuationEndpoint")
                .and_then(|e| e.get("continuationCommand"))
                .and_then(|c| c.get("token"))
                .and_then(|t| t.as_str())
                .map(String::from)
        });
    (videos, continuation)
}

fn parse_grid_video(r: &Value) -> Option<VideoSummary> {
    let id = r.get("videoId").and_then(|v| v.as_str())?.to_string();
    let title = collect_text(r.get("title")?)?;
    let author = String::new();
    let channel_id = String::new();
    let view_count_text = r
        .get("viewCountText")
        .and_then(|v| collect_text(v));
    let view_count = view_count_text.as_deref().and_then(parse_count);
    let length_text = r.get("lengthText").and_then(|v| collect_text(v));
    let length_seconds = length_text.as_deref().and_then(length_text_to_seconds);
    let thumbnail_url = first_thumbnail(r.get("thumbnail").unwrap_or(&Value::Null));
    Some(VideoSummary {
        id,
        title,
        author,
        channel_id,
        view_count,
        view_count_text,
        length_text,
        length_seconds,
        thumbnail_url,
    })
}
