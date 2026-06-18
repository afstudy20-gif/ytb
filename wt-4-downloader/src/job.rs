use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AudioFormat, AudioQuality, VideoQuality};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DownloadKind {
    VideoMuxed {
        quality: VideoQuality,
        audio: AudioFormat,
    },
    AudioOnly {
        format: AudioFormat,
        quality: AudioQuality,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Paused,
    Completed,
    Failed(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Progress {
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub eta_seconds: Option<u64>,
    pub speed_bps: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub id: Uuid,
    pub video_id: String,
    pub title: String,
    pub thumbnail_url: Option<String>,
    pub kind: DownloadKind,
    pub state: JobState,
    pub progress: Progress,
    pub output_path: Option<PathBuf>,
}

impl Job {
    #[must_use]
    pub fn new(
        id: Uuid,
        video_id: String,
        title: String,
        thumbnail_url: Option<String>,
        kind: DownloadKind,
    ) -> Self {
        Self {
            id,
            video_id,
            title,
            thumbnail_url,
            kind,
            state: JobState::Queued,
            progress: Progress::default(),
            output_path: None,
        }
    }
}
