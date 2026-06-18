use std::path::Path;

use downloader::mux::Muxer;
use tempfile::tempdir;
use tokio::process::Command;

#[tokio::test]
#[ignore = "requires ffmpeg with lavfi support"]
async fn muxes_tiny_video_and_audio_with_binary_fallback() {
    require_ffmpeg().await;
    let dir = tempdir().expect("tempdir");
    let video = dir.path().join("video.mp4");
    let audio = dir.path().join("audio.m4a");
    let out = dir.path().join("out.mp4");

    ffmpeg([
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=16x16:duration=1:rate=1",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        video.to_str().expect("video path"),
    ])
    .await;
    ffmpeg([
        "-y",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=1000:duration=1",
        "-c:a",
        "aac",
        audio.to_str().expect("audio path"),
    ])
    .await;

    Muxer::new()
        .mux_video_audio(&video, &audio, &out)
        .await
        .expect("mux");
    assert!(Path::new(&out).exists());
}

async fn require_ffmpeg() {
    let status = Command::new("ffmpeg")
        .arg("-version")
        .status()
        .await
        .expect("ffmpeg missing");
    assert!(status.success());
}

async fn ffmpeg<const N: usize>(args: [&str; N]) {
    let status = Command::new("ffmpeg")
        .args(args)
        .status()
        .await
        .expect("ffmpeg command");
    assert!(status.success());
}
