//! Integration tests for the Return YouTube Dislike client driven by `wiremock`.

use ryd::{Client, Error};
use std::time::Duration;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE: &str = include_str!("fixtures/votes.json");

#[tokio::test]
async fn votes_decodes_fixture() {
    let server = MockServer::start().await;
    let client = Client::new().with_base(server.uri());

    Mock::given(method("GET"))
        .and(path("/votes"))
        .and(query_param("videoId", "dQw4w9WgXcQ"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .mount(&server)
        .await;

    let v = client.votes("dQw4w9WgXcQ").await.expect("votes");
    assert_eq!(v.id, "dQw4w9WgXcQ");
    assert_eq!(v.likes, 17_000_000);
    assert_eq!(v.dislikes, 1_700_000);
    assert!(!v.deleted);
}

#[tokio::test]
async fn votes_served_from_cache_on_second_call() {
    let server = MockServer::start().await;
    let client = Client::new().with_base(server.uri());

    Mock::given(method("GET"))
        .and(path("/votes"))
        .and(query_param("videoId", "vid-cached"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .expect(1)
        .mount(&server)
        .await;

    let v1 = client.votes("vid-cached").await.expect("first call");
    let v2 = client.votes("vid-cached").await.expect("second call (cache)");
    assert_eq!(v1, v2);
}

#[tokio::test]
async fn votes_404_is_not_found() {
    let server = MockServer::start().await;
    let client = Client::new().with_base(server.uri());

    Mock::given(method("GET"))
        .and(path("/votes"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let err = client.votes("missingVideo").await.expect_err("should error");
    assert!(matches!(err, Error::NotFound), "got {err:?}");
}

#[tokio::test]
async fn votes_429_is_rate_limited() {
    let server = MockServer::start().await;
    let client = Client::new().with_base(server.uri());

    Mock::given(method("GET"))
        .and(path("/votes"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let err = client.votes("spammy").await.expect_err("should error");
    assert!(matches!(err, Error::RateLimited), "got {err:?}");
}

#[tokio::test]
async fn empty_video_id_is_invalid_input() {
    let server = MockServer::start().await;
    let client = Client::new().with_base(server.uri());
    let err = client.votes("").await.expect_err("should error");
    assert!(matches!(err, Error::InvalidInput(_)), "got {err:?}");
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn deleted_records_are_not_cached() {
    let server = MockServer::start().await;
    let client = Client::new().with_base(server.uri());

    let deleted_body = r#"{
        "id":"vid-del","dateCreated":1.0,"likes":0,"dislikes":0,"rating":0.0,"viewCount":0,"deleted":true
    }"#;

    Mock::given(method("GET"))
        .and(path("/votes"))
        .and(query_param("videoId", "vid-del"))
        .respond_with(ResponseTemplate::new(200).set_body_string(deleted_body))
        .expect(2)
        .mount(&server)
        .await;

    // Two calls should both hit the network since deleted records are not cached.
    let _ = client.votes("vid-del").await.expect("first");
    let _ = client.votes("vid-del").await.expect("second");
}

#[tokio::test]
async fn short_ttl_re_fetches_after_expiry() {
    let server = MockServer::start().await;
    let client = Client::new()
        .with_base(server.uri())
        .with_cache(8, Duration::from_millis(20));

    Mock::given(method("GET"))
        .and(path("/votes"))
        .and(query_param("videoId", "vid-ttl"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .expect(2)
        .mount(&server)
        .await;

    let _ = client.votes("vid-ttl").await.expect("first");
    std::thread::sleep(Duration::from_millis(40));
    let _ = client.votes("vid-ttl").await.expect("second after expiry");
}
