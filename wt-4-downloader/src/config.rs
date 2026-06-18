use std::path::PathBuf;

use crate::{AudioFormat, AudioQuality, Error, VideoQuality};

#[derive(Clone, Debug)]
pub struct Config {
    pub output_dir: PathBuf,
    pub max_concurrent_jobs: usize,
    pub max_concurrent_segments: usize,
    pub wifi_only: bool,
    pub default_video_quality: VideoQuality,
    pub default_audio_format: AudioFormat,
    pub default_audio_quality: AudioQuality,
}

impl Config {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.max_concurrent_jobs == 0 {
            return Err(Error::InvalidConfig(
                "max_concurrent_jobs must be greater than zero".to_owned(),
            ));
        }
        if self.max_concurrent_segments == 0 {
            return Err(Error::InvalidConfig(
                "max_concurrent_segments must be greater than zero".to_owned(),
            ));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("downloads"),
            max_concurrent_jobs: 2,
            max_concurrent_segments: 6,
            wifi_only: false,
            default_video_quality: VideoQuality::Best,
            default_audio_format: AudioFormat::M4a,
            default_audio_quality: AudioQuality::Best,
        }
    }
}
