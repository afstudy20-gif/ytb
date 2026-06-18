mod binary;
mod libav;

use std::path::Path;

use crate::{AudioFormat, Error};

pub struct Muxer;

impl Muxer {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub async fn mux_video_audio(
        &self,
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> Result<(), Error> {
        if libav::is_available() {
            match libav::mux_video_audio(video_path, audio_path, output_path).await {
                Ok(()) => return Ok(()),
                Err(err) => tracing::debug!(%err, "libav mux failed; trying ffmpeg binary"),
            }
        }
        binary::mux_video_audio(video_path, audio_path, output_path).await
    }

    pub async fn rewrap_audio(&self, input_path: &Path, output_path: &Path) -> Result<(), Error> {
        binary::rewrap_audio(input_path, output_path).await
    }

    pub async fn transcode_audio(
        &self,
        input_path: &Path,
        output_path: &Path,
        format: AudioFormat,
    ) -> Result<(), Error> {
        binary::transcode_audio(input_path, output_path, format).await
    }
}

impl Default for Muxer {
    fn default() -> Self {
        Self::new()
    }
}
