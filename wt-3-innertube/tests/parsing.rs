//! InnerTube JSON parsing tests against captured response fixtures.
//! These exercise the search, player+next, and captions parsers against
//! realistic (synthetic) InnerTube shapes.

#![cfg(test)]

use std::fs;

use innertube::search::parse_search_response;
use innertube::types::search::SearchItem;
use innertube::video::{parse_captions, parse_video_details};

/// Load a fixture as a string.
fn fixture(name: &str) -> String {
    let path = format!("tests/fixtures/{name}");
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read fixture {path}: {e}"))
}

#[test]
fn parses_search_lofi_fixture() {
    let src = fixture("search_lofi.json");
    let value: serde_json::Value =
        serde_json::from_str(&src).expect("fixture is valid JSON");
    // Diagnostic: count raw videoRenderer nodes found by the helper.
    let raw = innertube::json_util::find_all_with_key(&value, "videoRenderer");
    eprintln!("raw videoRenderer count: {}", raw.len());
    for (i, r) in raw.iter().enumerate() {
        eprintln!(
            "  {i}: id={:?}",
            r.get("videoId").and_then(|v| v.as_str())
        );
    }
    let results = parse_search_response(&value).expect("parses");
    eprintln!("items: {:?}", results.items);
    // Two videos, one channel, one playlist.
    let videos = results
        .items
        .iter()
        .filter(|i| matches!(i, SearchItem::Video(_)))
        .count();
    let channels = results
        .items
        .iter()
        .filter(|i| matches!(i, SearchItem::Channel(_)))
        .count();
    let playlists = results
        .items
        .iter()
        .filter(|i| matches!(i, SearchItem::Playlist(_)))
        .count();
    assert_eq!(videos, 2);
    assert_eq!(channels, 1);
    assert_eq!(playlists, 1);
    // Estimated results carried over.
    assert_eq!(results.estimated_results, Some(1_234_567));
    // Continuation token surfaced.
    assert_eq!(results.continuation.as_deref(), Some("XYZ_NEXT_PAGE_TOKEN"));
}

#[test]
fn parses_search_lofi_live_video_badge() {
    let src = fixture("search_lofi.json");
    let value: serde_json::Value = serde_json::from_str(&src).expect("valid JSON");
    let results = parse_search_response(&value).expect("parses");
    let live = results
        .items
        .iter()
        .filter_map(|i| match i {
            SearchItem::Video(v) => Some(v),
            _ => None,
        })
        .find(|v| v.id == "jfKfPfyJRdk")
        .expect("live video present");
    assert!(live.is_live);
    assert_eq!(live.author, "Lofi Girl");
    assert_eq!(live.channel_id, "UCLkYhRoVJpobI7RgsiOoZ8w");
}

#[test]
fn parses_player_and_next_into_video_details() {
    let player_src = fixture("player_dQw4w9WgXcQ.json");
    let next_src = fixture("next_dQw4w9WgXcQ.json");
    let player: serde_json::Value =
        serde_json::from_str(&player_src).expect("player JSON");
    let next: serde_json::Value =
        serde_json::from_str(&next_src).expect("next JSON");
    let details = parse_video_details(&player, &next, "dQw4w9WgXcQ").expect("parses");
    assert_eq!(details.id, "dQw4w9WgXcQ");
    assert_eq!(
        details.title,
        "Rick Astley - Never Gonna Give You Up (Official Video)"
    );
    assert_eq!(details.author, "Rick Astley");
    assert_eq!(details.channel_id, "UCuAXFkgsw1L7xaCfnd5JJOw");
    assert_eq!(details.length_seconds, Some(213));
    assert!(!details.is_live);
    assert!(!details.is_upcoming);
    assert!(details.thumbnail_url.is_some());
    assert!(details.thumbnail_url.as_deref().unwrap().contains("maxresdefault"));
    assert_eq!(details.keywords, vec!["rick astley", "never gonna give you up"]);
    // Related rail surfaced one video.
    assert_eq!(details.related.len(), 1);
    assert_eq!(details.related[0].id, "oHg5SJYRHA0");
    // Engagement / metadata from next.
    assert!(details.publish_date.as_deref().is_some());
}

#[test]
fn parses_player_captions_filtered_by_lang() {
    let player_src = fixture("player_dQw4w9WgXcQ.json");
    let player: serde_json::Value =
        serde_json::from_str(&player_src).expect("player JSON");
    let en = parse_captions(&player, "en").expect("parses");
    assert_eq!(en.len(), 1);
    assert_eq!(en[0].lang, "en");
    assert!(en[0].is_auto_generated);
    assert!(en[0].vtt_url.contains("fmt=vtt"));

    let all = parse_captions(&player, "").expect("parses");
    assert_eq!(all.len(), 2);
}

#[test]
fn parses_search_estimated_results_missing() {
    // When `estimatedResults` is absent, the parser should not panic and
    // should leave the field as `None`.
    let v = serde_json::json!({
        "contents": {
            "twoColumnSearchResultsRenderer": {
                "primaryContents": {
                    "sectionListRenderer": {
                        "contents": [
                            {"itemSectionRenderer": {"contents": []}}
                        ]
                    }
                }
            }
        }
    });
    let results = parse_search_response(&v).expect("parses");
    assert!(results.estimated_results.is_none());
    assert!(results.items.is_empty());
}
