use std::sync::Arc;
use std::time::Duration;

use downloader::{
    AudioFormat, AudioQuality, Config, DownloadKind, Downloader, Event, Stream, StreamMap,
    VideoMeta,
};
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

#[tokio::test]
async fn downloads_audio_with_range_segments() {
    let bytes = Arc::new(
        (0..1_200_000)
            .map(|idx| (idx % 251) as u8)
            .collect::<Vec<_>>(),
    );
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/audio"))
        .respond_with(RangeResponder {
            bytes: Arc::clone(&bytes),
        })
        .mount(&server)
        .await;

    let output = tempdir().expect("tempdir");
    let downloader = Downloader::new(Config {
        output_dir: output.path().to_path_buf(),
        max_concurrent_jobs: 1,
        max_concurrent_segments: 3,
        ..Config::default()
    })
    .expect("downloader");

    let mut events = downloader.events();
    let id = downloader
        .enqueue(
            "abc123",
            DownloadKind::AudioOnly {
                format: AudioFormat::M4a,
                quality: AudioQuality::Best,
            },
            StreamMap {
                streams: vec![Stream {
                    itag: 140,
                    url: format!("{}/audio", server.uri()),
                    mime_type: "audio/mp4".to_owned(),
                    content_length: Some(u64::try_from(bytes.len()).expect("length fits u64")),
                    bitrate: Some(128_000),
                    width: None,
                    height: None,
                    quality_label: None,
                    audio_quality: Some(AudioQuality::High),
                }],
            },
            VideoMeta {
                title: "Segment Test".to_owned(),
                author: Some("tester".to_owned()),
                duration_seconds: Some(1),
                thumbnail_url: None,
            },
        )
        .await
        .expect("enqueue");

    let completed = wait_completed(&mut events, id).await;
    let downloaded = tokio::fs::read(completed).await.expect("read output");
    assert_eq!(downloaded.as_slice(), bytes.as_slice());
}

async fn wait_completed(
    events: &mut tokio::sync::broadcast::Receiver<Event>,
    id: uuid::Uuid,
) -> std::path::PathBuf {
    let deadline = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            event = events.recv() => match event.expect("event") {
                Event::Completed(event_id, path) if event_id == id => return path,
                Event::Failed(event_id, reason) if event_id == id => panic!("{reason}"),
                _ => {}
            },
            () = &mut deadline => panic!("timed out waiting for completion"),
        }
    }
}

struct RangeResponder {
    bytes: Arc<Vec<u8>>,
}

impl Respond for RangeResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let Some(range) = request
            .headers
            .get("range")
            .and_then(|value| value.to_str().ok())
        else {
            return ResponseTemplate::new(416);
        };
        let Some(range) = range.strip_prefix("bytes=") else {
            return ResponseTemplate::new(416);
        };
        let Some((start, end)) = range.split_once('-') else {
            return ResponseTemplate::new(416);
        };
        let Ok(start) = start.parse::<usize>() else {
            return ResponseTemplate::new(416);
        };
        let Ok(end) = end.parse::<usize>() else {
            return ResponseTemplate::new(416);
        };
        let end = end.min(self.bytes.len().saturating_sub(1));
        if start > end || start >= self.bytes.len() {
            return ResponseTemplate::new(416);
        }
        let body = self.bytes[start..=end].to_vec();
        ResponseTemplate::new(206)
            .insert_header("content-length", body.len().to_string())
            .insert_header(
                "content-range",
                format!("bytes {start}-{end}/{}", self.bytes.len()),
            )
            .set_body_bytes(body)
    }
}
