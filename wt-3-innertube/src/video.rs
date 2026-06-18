//! Video metadata: combine InnerTube `player` and `next` to assemble
//! [`VideoDetails`].

use serde_json::{Map, Value};

use crate::client::{ClientContext, InnerTubeClient};
use crate::error::{Error, Result};
use crate::json_util::{
    collect_text, find_all_with_key, find_first_with_key, first_thumbnail, parse_count,
};
use crate::types::video::{Caption, VideoDetails, VideoSummary};

/// Fetch and assemble full video metadata for `id`.
pub(crate) async fn video(http: &InnerTubeClient, id: &str) -> Result<VideoDetails> {
    let mut body = Map::new();
    body.insert("videoId".into(), Value::String(id.to_string()));
    let player = http.post("player", ClientContext::WEB_DEFAULT, body).await?;

    let mut next_body = Map::new();
    next_body.insert("videoId".into(), Value::String(id.to_string()));
    let next = http.post("next", ClientContext::WEB_DEFAULT, next_body).await?;

    parse_video_details(&player, &next, id)
}

/// Parse the merged `player` + `next` responses into a [`VideoDetails`].
pub fn parse_video_details(player: &Value, next: &Value, id: &str) -> Result<VideoDetails> {
    let details = player
        .get("videoDetails")
        .ok_or_else(|| Error::Decode("player response missing videoDetails".into()))?;

    let title = details
        .get("title")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default();
    let description = details
        .get("shortDescription")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default();
    let author = details
        .get("author")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default();
    let channel_id = details
        .get("channelId")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default();
    let keywords = details
        .get("keywords")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let length_seconds = details
        .get("lengthSeconds")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| details.get("lengthSeconds").and_then(|v| v.as_u64()));
    let view_count_text = details
        .get("viewCount")
        .and_then(|v| v.as_str())
        .map(|c| format!("{c} views"));
    let thumbnail_url = details
        .get("thumbnail")
        .and_then(|t| t.get("thumbnails"))
        .and_then(|t| t.as_array())
        .and_then(|a| a.last())
        .and_then(|f| f.get("url"))
        .and_then(|u| u.as_str())
        .map(String::from);
    let is_live = details
        .get("isLive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_upcoming = details
        .get("isUpcoming")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // The `next` response carries richer engagement data (likes,
    // subscriber counts, related videos).
    let likes = find_first_with_key(next, "sentimentBarRenderer")
        .and_then(|s| {
            s.get("sentimentBarStatusTooltip")
                .or_else(|| s.get("tooltip"))
                .and_then(|t| t.as_str())
        })
        .and_then(|s| s.split('/').next())
        .and_then(|n| {
            n.chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .ok()
        });
    let publish_date = find_first_with_key(next, "dateTextRenderer")
        .and_then(|d| d.get("simpleText").or_else(|| d.get("content")))
        .and_then(|v| collect_text(v));
    let subscriber_count_text = find_first_with_key(next, "subscriberCountText")
        .and_then(|s| collect_text(s));

    let related = collect_related_videos(next);

    Ok(VideoDetails {
        id: id.to_string(),
        title,
        description,
        author,
        channel_id,
        subscriber_count_text,
        view_count_text,
        likes,
        publish_date,
        length_seconds,
        keywords,
        is_live,
        is_upcoming,
        thumbnail_url,
        related,
    })
}

/// Walk the `next` response looking for `compactVideoRenderer`s (the
/// "related/Up next" rail).
fn collect_related_videos(next: &Value) -> Vec<VideoSummary> {
    let mut out = Vec::new();
    for r in find_all_with_key(next, "compactVideoRenderer") {
        if let Some(summary) = parse_compact_video(r) {
            out.push(summary);
        }
    }
    out
}

pub(crate) fn parse_compact_video(r: &Value) -> Option<VideoSummary> {
    let id = r.get("videoId").and_then(|v| v.as_str())?.to_string();
    let title = collect_text(r.get("title")?)?;
    let author = collect_text(
        r.get("longBylineText")
            .or_else(|| r.get("shortBylineText"))?,
    )
    .unwrap_or_default();
    let channel_id = r
        .get("longBylineText")
        .or_else(|| r.get("shortBylineText"))
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
    let view_count_text = r
        .get("viewCountText")
        .and_then(|v| collect_text(v));
    let view_count = view_count_text
        .as_deref()
        .and_then(parse_count);
    let length_text = r
        .get("lengthText")
        .and_then(|v| collect_text(v));
    let length_seconds = length_text
        .as_deref()
        .and_then(crate::json_util::length_text_to_seconds);
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

/// Parse the `captions` field of a `player` response into [`Caption`]s.
pub fn parse_captions(player: &Value, lang_filter: &str) -> Result<Vec<Caption>> {
    let tracks = player
        .get("captions")
        .and_then(|c| c.get("playerCaptionsTracklistRenderer"))
        .and_then(|t| t.get("captionTracks"))
        .and_then(|c| c.as_array());
    let Some(tracks) = tracks else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for track in tracks {
        let lang = track
            .get("languageCode")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !lang_filter.is_empty()
            && !lang.starts_with(lang_filter)
            && !lang_filter.starts_with(&lang)
        {
            continue;
        }
        let name = collect_text(track.get("name").unwrap_or(&Value::Null)).unwrap_or_default();
        let base_url = track
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // Force the VTT-compatible timedtext format.
        let vtt_url = if base_url.contains("fmt=") {
            base_url.replace("fmt=", "fmt=vtt&")
        } else if base_url.contains('?') {
            format!("{base_url}&fmt=vtt")
        } else {
            format!("{base_url}?fmt=vtt")
        };
        let is_auto_generated = track
            .get("kind")
            .and_then(|v| v.as_str())
            .is_some_and(|k| k == "asr");
        out.push(Caption {
            lang,
            name,
            vtt_url,
            is_auto_generated,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_captions_filters_and_rewrites_url() {
        let player = json!({
            "captions": {
                "playerCaptionsTracklistRenderer": {
                    "captionTracks": [
                        {"languageCode": "en", "name": {"simpleText": "English"}, "baseUrl": "https://x/timedtext?lang=en&v=1"},
                        {"languageCode": "es", "name": {"simpleText": "Spanish"}, "baseUrl": "https://x/timedtext?lang=es"}
                    ]
                }
            }
        });
        let caps = parse_captions(&player, "en").expect("parsed");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].lang, "en");
        assert!(caps[0].vtt_url.contains("fmt=vtt"));
    }

    #[test]
    fn parse_compact_video_basic() {
        let v = json!({
            "videoId": "xyz",
            "title": {"simpleText": "Rel"},
            "shortBylineText": {"runs": [{"text": "Auth", "navigationEndpoint": {"browseEndpoint": {"browseId": "UCabc"}}}]},
            "viewCountText": {"simpleText": "100 views"},
            "lengthText": {"simpleText": "2:00"},
            "thumbnail": {"thumbnails": [{"url": "https://t/r.jpg"}]}
        });
        let s = parse_compact_video(&v).expect("parsed");
        assert_eq!(s.id, "xyz");
        assert_eq!(s.title, "Rel");
        assert_eq!(s.author, "Auth");
        assert_eq!(s.view_count, Some(100));
        assert_eq!(s.length_seconds, Some(120));
    }
}
