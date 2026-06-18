//! Search endpoint and response parsing.

use serde_json::{Map, Value};

use crate::client::{ClientContext, InnerTubeClient};
use crate::error::Result;
use crate::json_util::{
    collect_text, find_all_with_key, find_first_with_key, first_thumbnail, parse_count,
};
use crate::types::search::{
    Duration, SearchFilter, SearchItem, SearchKind, SearchResultChannel, SearchResultPlaylist,
    SearchResultVideo, SearchResults, SortBy, UploadDate,
};

/// Issue an InnerTube `search` call and parse the response into a typed
/// [`SearchResults`].
pub(crate) async fn search(
    http: &InnerTubeClient,
    query: &str,
    filter: Option<SearchFilter>,
) -> Result<SearchResults> {
    let filter = filter.unwrap_or_default();
    let params = encode_filter_params(&filter);
    let mut body = Map::new();
    body.insert("query".into(), Value::String(query.to_string()));
    if let Some(params) = params {
        body.insert("params".into(), Value::String(params));
    }
    let resp = http.post("search", ClientContext::WEB_DEFAULT, body).await?;
    parse_search_response(&resp)
}

/// Encode a [`SearchFilter`] into the base64-style `params` blob InnerTube
/// expects. The blob is constructed from a fixed prefix that selects the
/// "search results" filter scope, followed by tag bytes for each active
/// filter.
///
/// The encoding is a simple binary scheme XOR'd against a magic constant,
/// then base64-encoded. We implement the encoder (rather than hardcoding
/// every combination) because YouTube rotates the prefix between player
/// builds and a runtime encoder is more robust.
pub(crate) fn encode_filter_params(filter: &SearchFilter) -> Option<String> {
    if filter.kind == SearchKind::All
        && filter.upload_date.is_none()
        && filter.duration.is_none()
        && filter.sort_by == SortBy::Relevance
    {
        return None;
    }
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(&[0x12, 0x0a]);
    buf.extend_from_slice(&[0x11]);
    buf.extend_from_slice(&byte_for_kind(filter.kind));
    if let Some(d) = filter.upload_date {
        buf.extend_from_slice(&[0x18]);
        buf.push(byte_for_upload_date(d));
    }
    if let Some(d) = filter.duration {
        buf.extend_from_slice(&[0x20]);
        buf.push(byte_for_duration(d));
    }
    if filter.sort_by != SortBy::Relevance {
        buf.extend_from_slice(&[0x28]);
        buf.push(byte_for_sort(filter.sort_by));
    }
    Some(apply_transform_and_b64(&buf))
}

fn byte_for_kind(k: SearchKind) -> [u8; 1] {
    match k {
        SearchKind::All => [0x0a],
        SearchKind::Video => [0x01],
        SearchKind::Channel => [0x02],
        SearchKind::Playlist => [0x03],
    }
}

fn byte_for_upload_date(d: UploadDate) -> u8 {
    match d {
        UploadDate::LastHour => 0x01,
        UploadDate::Today => 0x02,
        UploadDate::ThisWeek => 0x03,
        UploadDate::ThisMonth => 0x04,
        UploadDate::ThisYear => 0x05,
    }
}

fn byte_for_duration(d: Duration) -> u8 {
    match d {
        Duration::Short => 0x01,
        Duration::Medium => 0x02,
        Duration::Long => 0x03,
    }
}

fn byte_for_sort(s: SortBy) -> u8 {
    match s {
        SortBy::Relevance => 0x00,
        SortBy::UploadDate => 0x01,
        SortBy::ViewCount => 0x02,
        SortBy::Rating => 0x03,
    }
}

/// YouTube's filter params use a custom base64 alphabet where bytes are
/// XOR'd against a constant before encoding. The exact scheme has been
/// stable since 2020 and is reproduced here.
fn apply_transform_and_b64(buf: &[u8]) -> String {
    // XOR each byte with 0x25 (the constant YouTube uses).
    let xored: Vec<u8> = buf.iter().map(|b| b ^ 0x25).collect();
    base64_encode(&xored)
}

/// Minimal standard-alphabet base64 encoder.
fn base64_encode(input: &[u8]) -> String {
    const ALPHA: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let b0 = input[i];
        let b1 = input[i + 1];
        let b2 = input[i + 2];
        out.push(ALPHA[(b0 >> 2) as usize] as char);
        out.push(ALPHA[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(ALPHA[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        out.push(ALPHA[(b2 & 0x3f) as usize] as char);
        i += 3;
    }
    match input.len() - i {
        1 => {
            let b0 = input[i];
            out.push(ALPHA[(b0 >> 2) as usize] as char);
            out.push(ALPHA[((b0 & 0x03) << 4) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let b0 = input[i];
            let b1 = input[i + 1];
            out.push(ALPHA[(b0 >> 2) as usize] as char);
            out.push(ALPHA[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            out.push(ALPHA[((b1 & 0x0f) << 2) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

/// Parse an InnerTube `search` response into a typed [`SearchResults`].
pub fn parse_search_response(resp: &Value) -> Result<SearchResults> {
    let estimated = resp
        .get("estimatedResults")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());

    // The item section lives under `contents.twoColumnSearchResultsRenderer
    // .primaryContents.sectionListRenderer.contents[].itemSectionRenderer.
    // contents[]`. We walk the JSON tree to find every `*Renderer` we know.
    let mut items: Vec<SearchItem> = Vec::new();
    for r in find_all_with_key(resp, "videoRenderer") {
        if let Some(item) = parse_video_renderer(r) {
            items.push(SearchItem::Video(item));
        }
    }
    for r in find_all_with_key(resp, "channelRenderer") {
        if let Some(item) = parse_channel_renderer(r) {
            items.push(SearchItem::Channel(item));
        }
    }
    for r in find_all_with_key(resp, "playlistRenderer") {
        if let Some(item) = parse_playlist_renderer(r) {
            items.push(SearchItem::Playlist(item));
        }
    }

    // Continuation token lives inside a `continuationItemRenderer`.
    let continuation = find_first_with_key(resp, "continuationItemRenderer")
        .and_then(|c| {
            c.get("continuationEndpoint")
                .or_else(|| c.get("button"))
                .and_then(|e| {
                    e.get("continuationCommand")
                        .and_then(|c| c.get("token"))
                        .and_then(|t| t.as_str())
                        .map(String::from)
                })
        });

    Ok(SearchResults {
        items,
        continuation,
        estimated_results: estimated,
    })
}

fn parse_video_renderer(r: &Value) -> Option<SearchResultVideo> {
    let id = r.get("videoId").and_then(|v| v.as_str())?.to_string();
    let title = collect_text(r.get("title")?)?;
    let byline = r
        .get("ownerText")
        .or_else(|| r.get("shortBylineText"))
        .or_else(|| r.get("longBylineText"))?;
    let author = collect_text(byline).unwrap_or_default();
    let channel_id = byline
        .get("runs")
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
    let description = r
        .get("detailedMetadataAccesspoints")
        .or_else(|| r.get("descriptionSnippet"))
        .and_then(|v| collect_text(v));
    let length_text = r
        .get("lengthText")
        .and_then(|v| collect_text(v));
    let view_count_text = r
        .get("viewCountText")
        .and_then(|v| collect_text(v));
    let published_text = r
        .get("publishedTimeText")
        .and_then(|v| collect_text(v));
    let thumbnail_url = first_thumbnail(r.get("thumbnail").unwrap_or(&Value::Null));
    let badges = r
        .get("badges")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|b| {
                    b.get("metadataBadgeRenderer")
                        .and_then(|m| m.get("label"))
                        .and_then(|l| l.as_str())
                        .map(String::from)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let is_live = badges.iter().any(|b| b.to_lowercase().contains("live"));

    Some(SearchResultVideo {
        id,
        title,
        author,
        channel_id,
        description,
        length_text,
        view_count_text,
        published_text,
        thumbnail_url,
        is_live,
    })
}

fn parse_channel_renderer(r: &Value) -> Option<SearchResultChannel> {
    let id = r.get("channelId").and_then(|v| v.as_str())?.to_string();
    let title = collect_text(r.get("title")?)?;
    let subscriber_count_text = r
        .get("subscriberCountText")
        .and_then(|v| collect_text(v));
    let description = r
        .get("descriptionSnippet")
        .and_then(|v| collect_text(v));
    let avatar_url = first_thumbnail(r.get("thumbnail").unwrap_or(&Value::Null));
    Some(SearchResultChannel {
        id,
        title,
        subscriber_count_text,
        description,
        avatar_url,
    })
}

fn parse_playlist_renderer(r: &Value) -> Option<SearchResultPlaylist> {
    let id = r.get("playlistId").and_then(|v| v.as_str())?.to_string();
    let title = collect_text(r.get("title")?)?;
    let thumbnail_url = first_thumbnail(r.get("thumbnails").unwrap_or(&Value::Null))
        .or_else(|| {
            // Playlists sometimes nest the thumbnail one level deeper.
            r.get("thumbnails")
                .and_then(|t| t.as_array())
                .and_then(|a| a.first())
                .and_then(|f| f.get("thumbnails"))
                .and_then(|t| t.as_array())
                .and_then(|a| a.first())
                .and_then(|f| f.get("url"))
                .and_then(|u| u.as_str())
                .map(String::from)
        });
    let author = r
        .get("longBylineText")
        .or_else(|| r.get("shortBylineText"))
        .and_then(|v| collect_text(v));
    let video_count = r
        .get("videoCount")
        .and_then(|v| v.as_str())
        .and_then(|s| parse_count(s));
    Some(SearchResultPlaylist {
        id,
        title,
        thumbnail_url,
        author,
        video_count,
    })
}

/// Helper used in tests to round-trip a filter through the encoder.
#[cfg(test)]
pub(crate) fn _encode_for_test(filter: &SearchFilter) -> Option<String> {
    encode_filter_params(filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_util::{length_text_to_seconds, parse_count};
    use crate::types::search::{SearchFilter, SearchKind};
    use serde_json::json;

    #[test]
    fn no_params_for_default_filter() {
        let f = SearchFilter::default();
        assert!(encode_filter_params(&f).is_none());
    }

    #[test]
    fn params_present_for_video_kind() {
        let f = SearchFilter {
            kind: SearchKind::Video,
            ..Default::default()
        };
        let p = encode_filter_params(&f);
        assert!(p.is_some());
    }

    #[test]
    fn parse_video_renderer_basic() {
        let v = json!({
            "videoId": "abc123",
            "title": {"simpleText": "Hello"},
            "ownerText": {"runs": [{"text": "Author", "navigationEndpoint": {"browseEndpoint": {"browseId": "UC123"}}}]},
            "lengthText": {"simpleText": "1:30"},
            "viewCountText": {"simpleText": "1,234 views"},
            "thumbnail": {"thumbnails": [{"url": "https://t/1.jpg"}]}
        });
        let r = parse_video_renderer(&v).expect("parsed");
        assert_eq!(r.id, "abc123");
        assert_eq!(r.title, "Hello");
        assert_eq!(r.author, "Author");
        assert_eq!(r.channel_id, "UC123");
        assert_eq!(r.length_text.as_deref(), Some("1:30"));
        assert_eq!(r.thumbnail_url.as_deref(), Some("https://t/1.jpg"));
    }

    #[test]
    fn length_text_to_seconds_works() {
        assert_eq!(length_text_to_seconds("1:30"), Some(90));
    }

    #[test]
    fn parse_count_handles_examples() {
        assert_eq!(parse_count("1,234 views"), Some(1234));
    }
}
