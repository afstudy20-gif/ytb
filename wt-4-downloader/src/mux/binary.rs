use std::ffi::OsString;
use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::{AudioFormat, Error};

pub async fn mux_video_audio(
    video_path: &Path,
    audio_path: &Path,
    output_path: &Path,
) -> Result<(), Error> {
    run_ffmpeg([
        "-y".into(),
        "-i".into(),
        video_path.as_os_str().to_owned(),
        "-i".into(),
        audio_path.as_os_str().to_owned(),
        "-c".into(),
        "copy".into(),
        "-movflags".into(),
        "+faststart".into(),
        output_path.as_os_str().to_owned(),
    ])
    .await
}

pub async fn rewrap_audio(input_path: &Path, output_path: &Path) -> Result<(), Error> {
    run_ffmpeg([
        "-y".into(),
        "-i".into(),
        input_path.as_os_str().to_owned(),
        "-c:a".into(),
        "copy".into(),
        output_path.as_os_str().to_owned(),
    ])
    .await
}

pub async fn transcode_audio(
    input_path: &Path,
    output_path: &Path,
    format: AudioFormat,
) -> Result<(), Error> {
    let codec = match format {
        AudioFormat::M4a => "aac",
        AudioFormat::Opus => "libopus",
    };
    run_ffmpeg([
        "-y".into(),
        "-i".into(),
        input_path.as_os_str().to_owned(),
        "-c:a".into(),
        codec.into(),
        output_path.as_os_str().to_owned(),
    ])
    .await
}

async fn run_ffmpeg<const N: usize>(args: [OsString; N]) -> Result<(), Error> {
    let binary = ffmpeg_binary();
    let output = Command::new(&binary)
        .args(args)
        .output()
        .await
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                Error::FfmpegNotFound(binary.clone())
            } else {
                Error::Io(err)
            }
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Ffmpeg(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}

fn ffmpeg_binary() -> PathBuf {
    if let Ok(path) = std::env::var("DOWNLOADER_FFMPEG") {
        return PathBuf::from(path);
    }
    #[cfg(feature = "bundled-ffmpeg")]
    {
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(dir) = current_exe.parent() {
                return dir.join("ffmpeg");
            }
        }
    }
    PathBuf::from("ffmpeg")
}
