use std::path::Path;

use crate::Error;

#[cfg(feature = "libav")]
pub fn is_available() -> bool {
    ffmpeg_next::init().is_ok()
}

#[cfg(not(feature = "libav"))]
pub const fn is_available() -> bool {
    false
}

#[cfg(feature = "libav")]
pub async fn mux_video_audio(
    _video_path: &Path,
    _audio_path: &Path,
    _output_path: &Path,
) -> Result<(), Error> {
    Err(Error::Ffmpeg(
        "libav stream-copy muxing is unavailable; binary fallback required".to_owned(),
    ))
}

#[cfg(not(feature = "libav"))]
pub async fn mux_video_audio(
    _video_path: &Path,
    _audio_path: &Path,
    _output_path: &Path,
) -> Result<(), Error> {
    Err(Error::Ffmpeg("libav feature is disabled".to_owned()))
}
