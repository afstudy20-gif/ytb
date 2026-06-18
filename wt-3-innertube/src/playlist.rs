//! Playlist endpoint.

use serde_json::{Map, Value};

use crate::client::{ClientContext, InnerTubeClient};
use crate::error::Result;
use crate::json_util::{
    collect_text, find_all_with_key, find_first_with_key, first_thumbnail,
    length_text_to_seconds,
};
use crate::types::playlist::{PlaylistDetails, PlaylistVideo};

/// Fetch playlist contents via InnerTube's `browse` endpoint.
pub(crate) async fn playlist(http: &InnerTubeClient, id: &str) -> Result<PlaylistDetails> {
    let mut body = Map::new();
    body.insert("browseId".into(), Value::String(format!("VL{id}")));
    let resp = http.post("browse", ClientContext::WEB_DEFAULT, body).await?;
    parse_playlist_response(&resp, id)
}

/// Parse a playlist `browse` response.
pub(crate) fn parse_playlist_response(resp: &Value, id: &str) -> Result<PlaylistDetails> {
    let header = resp
        .get("header")
        .and_then(|h| {
            h.get("playlistHeaderRenderer")
                .or_else(|| h.get("playlistSidebarPrimaryInfoRenderer"))
        })
        .cloned()
        .unwrap_or(Value::Null);

    let title = collect_text(header.get("title").unwrap_or(&Value::Null))
        .or_else(|| {
            resp.get("metadata")
                .and_then(|m| m.get("playlistMetadataRenderer"))
                .and_then(|m| m.get("title"))
                .and_then(collect_text)
        })
        .unwrap_or_default();
    let author = header
        .get("ownerText")
        .or_else(|| header.get("shortBylineText"))
        .and_then(collect_text);
    let channel_id = header
        .get("ownerText")
        .or_else(|| header.get("shortBylineText"))
        .and_then(|t| t.get("runs"))
        .and_then(|runs| runs.as_array())
        .and_then(|runs| runs.first())
        .and_then(|first| {
            first
                .get("navigationEndpoint")
                .and_then(|ne| ne.get("browseEndpoint"))
                .and_then(|be| be.get("browseId"))
                .and_then(|b| b.as_str())
                .map(String::from)
        });
    let video_count = header
        .get("numVideosText")
        .and_then(|t| collect_text(t))
        .and_then(|s| {
            // "1,234 videos" -> 1234
            s.chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .ok()
        });
    let view_count_text = header
        .get("viewCountText")
        .and_then(|t| collect_text(t));

    let mut videos = Vec::new();
    for r in find_all_with_key(resp, "playlistVideoRenderer") {
        if let Some(v) = parse_playlist_video(r) {
            videos.push(v);
        }
    }
    // Some playlists use `compactVideoRenderer` for entries; also pick those up.
    for r in find_all_with_key(resp, "compactVideoRenderer") {
        if let Some(v) = parse_compact_playlist_video(r) {
            videos.push(v);
        }
    }

    let continuation = find_first_with_key(resp, "continuationItemRenderer")
        .and_then(|c| {
            c.get("continuationEndpoint")
                .or_else(|| c.get("token"))
                .and_then(|e| {
                    e.get("continuationCommand")
                        .and_then(|c| c.get("token"))
                        .or_else(|| e.get("token"))
                        .and_then(|t| t.as_str())
                        .map(String::from)
                })
        });

    Ok(PlaylistDetails {
        id: id.to_string(),
        title,
        author,
        channel_id,
        video_count,
        view_count_text,
        videos,
        continuation,
    })
}

fn parse_playlist_video(r: &Value) -> Option<PlaylistVideo> {
    let id = r.get("videoId").and_then(|v| v.as_str())?.to_string();
    let title = collect_text(r.get("title")?)?;
    let author = r
        .get("shortBylineText")
        .and_then(collect_text)
        .unwrap_or_default();
    let channel_id = r
        .get("shortBylineText")
        .and_then(|t| t.get("runs"))
        .and_then(|runs| runs.as_array())
        .and_then(|runs| runs.first())
        .and_then(|first| {
            first
                .get("navigationEndpoint")
                .and_then(|ne| ne.get("browseEndpoint"))
                .and_then(|be| be.get("browseId"))
                .and_then(|b| b.as_str())
                .map(String::from)
        })
        .unwrap_or_default();
    let length_text = r
        .get("lengthText")
        .and_then(|v| collect_text(v));
    let length_seconds = length_text.as_deref().and_then(length_text_to_seconds);
    let thumbnail_url = first_thumbnail(r.get("thumbnail").unwrap_or(&Value::Null));
    let index = r
        .get("index")
        .and_then(|i| i.get("simpleText"))
        .and_then(|s| s.as_str())
        .and_then(|s| s.parse::<u64>().ok());
    Some(PlaylistVideo {
        id,
        title,
        author,
        channel_id,
        length_text,
        length_seconds,
        thumbnail_url,
        index,
    })
}

fn parse_compact_playlist_video(r: &Value) -> Option<PlaylistVideo> {
    let id = r.get("videoId").and_then(|v| v.as_str())?.to_string();
    let title = collect_text(r.get("title")?)?;
    let author = r
        .get("shortBylineText")
        .or_else(|| r.get("longBylineText"))
        .and_then(collect_text)
        .unwrap_or_default();
    let channel_id = r
        .get("shortBylineText")
        .or_else(|| r.get("longBylineText"))
        .and_then(|t| t.get("runs"))
        .and_then(|runs| runs.as_array())
        .and_then(|runs| runs.first())
        .and_then(|first| {
            first
                .get("navigationEndpoint")
                .and_then(|ne| ne.get("browseEndpoint"))
                .and_then(|be| be.get("browseId"))
                .and_then(|b| b.as_str())
                .map(String::from)
        })
        .unwrap_or_default();
    let length_text = r
        .get("lengthText")
        .and_then(|v| collect_text(v));
    let length_seconds = length_text.as_deref().and_then(length_text_to_seconds);
    let thumbnail_url = first_thumbnail(r.get("thumbnail").unwrap_or(&Value::Null));
    Some(PlaylistVideo {
        id,
        title,
        author,
        channel_id,
        length_text,
        length_seconds,
        thumbnail_url,
        index: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_util::parse_count;
    use serde_json::json;

    #[test]
    fn parse_playlist_video_basic() {
        let v = json!({
            "videoId": "vid",
            "title": {"simpleText": "Vid"},
            "shortBylineText": {"runs": [{"text": "Auth", "navigationEndpoint": {"browseEndpoint": {"browseId": "UC1"}}}]},
            "lengthText": {"simpleText": "3:45"},
            "thumbnail": {"thumbnails": [{"url": "https://t/p.jpg"}]},
            "index": {"simpleText": "1"}
        });
        let pv = parse_playlist_video(&v).expect("parsed");
        assert_eq!(pv.id, "vid");
        assert_eq!(pv.title, "Vid");
        assert_eq!(pv.author, "Auth");
        assert_eq!(pv.channel_id, "UC1");
        assert_eq!(pv.length_seconds, Some(225));
        assert_eq!(pv.index, Some(1));
    }

    #[test]
    fn parse_count_handles_video_count() {
        assert_eq!(parse_count("1,234 videos"), Some(1234));
    }
}
