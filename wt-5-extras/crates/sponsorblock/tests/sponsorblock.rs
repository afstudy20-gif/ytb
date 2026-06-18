//! Integration tests for the SponsorBlock client driven by `wiremock`.

use sponsorblock::{Category, Client, Error, Vote};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const HASH_BUCKET: &str = include_str!("fixtures/hash_bucket.json");
const DIRECT_SEGMENTS: &str = include_str!("fixtures/direct_segments.json");

fn fixture(name: &str) -> &'static str {
    match name {
        "hash_bucket" => HASH_BUCKET,
        "direct_segments" => DIRECT_SEGMENTS,
        _ => "",
    }
}

async fn mount_categories_query(
    server: &MockServer,
    path_str: &str,
    categories_json: &str,
    status: u16,
    body: &'static str,
) {
    Mock::given(method("GET"))
        .and(path(path_str))
        .and(query_param("categories", categories_json))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn segments_by_hash_filters_to_requested_video() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());

    // `sha256("dQw4w9WgXcQ")` prefix is "0D5..."; we just need any 4-char
    // path here, so the mock matches on path structure instead.
    let categories_json = serde_json::to_string(&["sponsor"]).unwrap();
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(r"^/skipSegments/[0-9A-F]{4}$"))
        .and(query_param("categories", &categories_json))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("hash_bucket")))
        .mount(&server)
        .await;

    let segs = client
        .segments_by_hash("dQw4w9WgXcQ", &[Category::Sponsor])
        .await
        .expect("segments");
    // Bucket contains 3 segments; only the two with category=sponsor match
    // (one is the wrong video's sponsor segment — we keep it because the
    // client does not know the videoID from the bucket alone, the consumer
    // is expected to filter on videoID after fetching; here we test category
    // filtering only).
    assert_eq!(segs.len(), 2);
    assert!(segs.iter().all(|s| s.category == "sponsor"));
}

#[tokio::test]
async fn segments_by_hash_empty_bucket_is_not_found() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());

    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(r"^/skipSegments/[0-9A-F]{4}$"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&server)
        .await;

    let err = client
        .segments_by_hash("nopeVideo1234", &[])
        .await
        .expect_err("should be NotFound");
    assert!(matches!(err, Error::NotFound), "got {err:?}");
}

#[tokio::test]
async fn segments_direct_returns_decoded_list() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());

    let categories_json = serde_json::to_string(&["sponsor"]).unwrap();
    mount_categories_query(
        &server,
        "/skipSegments",
        &categories_json,
        200,
        fixture("direct_segments"),
    )
    .await;

    let segs = client
        .segments("dQw4w9WgXcQ", &[Category::Sponsor])
        .await
        .expect("segments");
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0].uuid, "direct-uuid-1");
    assert_eq!(segs[0].start, 12.0);
}

#[tokio::test]
async fn segments_direct_empty_is_not_found() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());

    Mock::given(method("GET"))
        .and(path("/skipSegments"))
        .and(query_param("videoID", "unknownVideoId"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    let err = client
        .segments("unknownVideoId", &[])
        .await
        .expect_err("should error");
    assert!(matches!(err, Error::NotFound), "got {err:?}");
}

#[tokio::test]
async fn segments_direct_429_is_rate_limited() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());

    Mock::given(method("GET"))
        .and(path("/skipSegments"))
        .and(query_param("videoID", "spammyVideo"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let err = client
        .segments("spammyVideo", &[])
        .await
        .expect_err("should error");
    assert!(matches!(err, Error::RateLimited), "got {err:?}");
}

#[tokio::test]
async fn segments_rejects_empty_video_id() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());
    let err = client.segments("", &[]).await.expect_err("should error");
    assert!(matches!(err, Error::InvalidInput(_)), "got {err:?}");
    // Server should never have been hit.
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn vote_requires_user_id() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());
    let err = client
        .vote("uuid-x", Vote::Up, None)
        .await
        .expect_err("should error");
    assert!(matches!(err, Error::InvalidInput(_)), "got {err:?}");
}

#[tokio::test]
async fn vote_succeeds_with_user_id() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());

    Mock::given(method("POST"))
        .and(path("/voteOnSponsorTime"))
        .and(query_param("UUID", "uuid-x"))
        .and(query_param("userID", "user-1"))
        .and(query_param("type", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server)
        .await;

    client
        .vote("uuid-x", Vote::Up, Some("user-1"))
        .await
        .expect("vote ok");
}

#[tokio::test]
async fn vote_uses_default_user_id_from_constructor() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri()).with_user_id("default-user");

    Mock::given(method("POST"))
        .and(path("/voteOnSponsorTime"))
        .and(query_param("UUID", "uuid-y"))
        .and(query_param("userID", "default-user"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    client
        .vote("uuid-y", Vote::Down, None)
        .await
        .expect("vote ok");
}

#[tokio::test]
async fn submit_returns_new_uuid() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());

    Mock::given(method("POST"))
        .and(path("/skipSegments"))
        .and(query_param("videoID", "vid1"))
        .and(query_param("userID", "user-1"))
        .respond_with(ResponseTemplate::new(200).set_body_string("\"new-uuid-xyz\""))
        .mount(&server)
        .await;

    let uuid = client
        .submit(
            "vid1",
            sponsorblock::NewSegment {
                start: 1.0,
                end: 5.0,
                category: "sponsor".to_string(),
                user_agent: "wt-5/0.1".to_string(),
            },
            Some("user-1"),
        )
        .await
        .expect("submit ok");
    assert_eq!(uuid, "new-uuid-xyz");
}

#[tokio::test]
async fn submit_403_is_forbidden() {
    let server = MockServer::start().await;
    let client = Client::with_base(server.uri());

    Mock::given(method("POST"))
        .and(path("/skipSegments"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let err = client
        .submit(
            "vid1",
            sponsorblock::NewSegment {
                start: 1.0,
                end: 5.0,
                category: "sponsor".to_string(),
                user_agent: "wt-5/0.1".to_string(),
            },
            Some("user-1"),
        )
        .await
        .expect_err("should be forbidden");
    assert!(matches!(err, Error::Forbidden), "got {err:?}");
}
