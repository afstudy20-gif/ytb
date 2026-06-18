use downloader::{
    AudioFormat, AudioQuality, Config, DownloadKind, Downloader, JobState, Stream, StreamMap,
    VideoMeta,
};
use tempfile::tempdir;

#[test]
fn queued_jobs_survive_reopen() {
    let output = tempdir().expect("tempdir");
    let cfg = Config {
        output_dir: output.path().to_path_buf(),
        ..Config::default()
    };

    let id = {
        let downloader = Downloader::new(cfg.clone()).expect("downloader");
        futures::executor::block_on(downloader.enqueue(
            "persisted",
            DownloadKind::AudioOnly {
                format: AudioFormat::M4a,
                quality: AudioQuality::Best,
            },
            StreamMap {
                streams: vec![Stream {
                    itag: 140,
                    url: "http://127.0.0.1/audio".to_owned(),
                    mime_type: "audio/mp4".to_owned(),
                    content_length: Some(10),
                    bitrate: Some(128_000),
                    width: None,
                    height: None,
                    quality_label: None,
                    audio_quality: Some(AudioQuality::High),
                }],
            },
            VideoMeta {
                title: "Persisted".to_owned(),
                author: None,
                duration_seconds: None,
                thumbnail_url: None,
            },
        ))
        .expect("enqueue")
    };

    let reopened = Downloader::new(cfg).expect("reopen");
    let jobs = futures::executor::block_on(reopened.list()).expect("list");
    let job = jobs
        .into_iter()
        .find(|job| job.id == id)
        .expect("persisted job");
    assert_eq!(job.state, JobState::Queued);
    assert_eq!(job.video_id, "persisted");
}
