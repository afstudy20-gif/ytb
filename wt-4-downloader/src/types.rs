use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoQuality {
    P144,
    P240,
    P360,
    P480,
    P720,
    P1080,
    P1440,
    P2160,
    Best,
}

impl VideoQuality {
    #[must_use]
    pub const fn max_height(self) -> Option<u32> {
        match self {
            Self::P144 => Some(144),
            Self::P240 => Some(240),
            Self::P360 => Some(360),
            Self::P480 => Some(480),
            Self::P720 => Some(720),
            Self::P1080 => Some(1080),
            Self::P1440 => Some(1440),
            Self::P2160 => Some(2160),
            Self::Best => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioFormat {
    M4a,
    Opus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AudioQuality {
    Low,
    Med,
    High,
    Best,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamKind {
    Video,
    Audio,
}

impl StreamKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Audio => "audio",
        }
    }
}

/// Minimal Innertube-shaped metadata consumed by this crate.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoMeta {
    pub title: String,
    pub author: Option<String>,
    pub duration_seconds: Option<u64>,
    pub thumbnail_url: Option<String>,
}

/// Minimal Innertube-shaped stream map.
///
/// `streams` should contain adaptive formats with direct, signed URLs. The
/// integration layer is expected to resolve signatures/ciphers first.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamMap {
    pub streams: Vec<Stream>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Stream {
    pub itag: u32,
    pub url: String,
    pub mime_type: String,
    pub content_length: Option<u64>,
    pub bitrate: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub quality_label: Option<String>,
    pub audio_quality: Option<AudioQuality>,
}

impl Stream {
    #[must_use]
    pub fn is_video(&self) -> bool {
        self.mime_type.starts_with("video/")
    }

    #[must_use]
    pub fn is_audio(&self) -> bool {
        self.mime_type.starts_with("audio/")
    }

    #[must_use]
    pub fn audio_format(&self) -> Option<AudioFormat> {
        if self.mime_type.contains("mp4") || self.mime_type.contains("m4a") {
            Some(AudioFormat::M4a)
        } else if self.mime_type.contains("webm") || self.mime_type.contains("opus") {
            Some(AudioFormat::Opus)
        } else {
            None
        }
    }
}
