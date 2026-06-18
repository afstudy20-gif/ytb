//! Stream-related types: a single [`Stream`] and the [`StreamMap`] that groups
//! them per video.

/// A single playable stream URL returned by InnerTube.
///
/// `Stream` is intentionally a flat struct rather than a tagged enum because
/// callers (downloaders, transcoders) almost always want a uniform view across
/// muxed and adaptive formats. Audio-only fields (`audio_sample_rate`,
/// `audio_channels`) and video-only fields (`width`, `height`, `fps`) are
/// `Option` so callers can branch on `mime_type.contains("audio")` etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    /// YouTube's "itag" format identifier. See
    /// <https://gist.github.com/sidneys/7095afe4da4ae58694d128f103216e18>.
    pub itag: u32,
    /// The fully resolved (deciphered, n-param-rewritten) URL. Querying it
    /// returns bytes directly from Google's CDN.
    pub url: String,
    /// The MIME type plus codecs, e.g. `video/mp4; codecs="avc1.4d401f"`.
    pub mime_type: String,
    /// Best-effort split of the codec string from `mime_type` (the first
    /// codec if multiple).
    pub bitrate: Option<u64>,
    /// Pixel width, present for video-containing formats only.
    pub width: Option<u32>,
    /// Pixel height, present for video-containing formats only.
    pub height: Option<u32>,
    /// Frames per second, present for video-containing formats only.
    pub fps: Option<u32>,
    /// Audio sample rate in Hz, present for audio-containing formats only.
    pub audio_sample_rate: Option<u32>,
    /// Number of audio channels, present for audio-containing formats only.
    pub audio_channels: Option<u32>,
    /// Total content length in bytes if InnerTube supplied it; populated from
    /// `contentLength` for muxed formats and clonable to adaptive formats
    /// once the HEAD response is observed (this crate does not HEAD).
    pub content_length: Option<u64>,
    /// Stream duration in milliseconds, when known.
    pub duration_ms: Option<u64>,
    /// `true` if this stream was resolved via a Piped instance rather than
    /// directly from InnerTube.
    pub via_proxy: bool,
    /// Approximate average quality label, e.g. `1080p` or `Medium`. Populated
    /// from InnerTube's `qualityLabel` when present.
    pub quality_label: Option<String>,
}

impl Stream {
    /// `true` if the stream contains a video track.
    #[must_use]
    pub fn has_video(&self) -> bool {
        self.mime_type.starts_with("video") || self.width.is_some()
    }

    /// `true` if the stream contains an audio track.
    #[must_use]
    pub fn has_audio(&self) -> bool {
        self.mime_type.starts_with("audio") || self.audio_sample_rate.is_some()
    }

    /// `true` if the stream contains both audio and video (a "muxed"
    /// / progressive format, typically mp4 with itag 18/22).
    #[must_use]
    pub fn is_progressive(&self) -> bool {
        self.has_video() && self.has_audio()
    }
}

/// All playable streams resolved for a single video.
///
/// `progressive` holds the classic muxed formats (audio+video in one URL),
/// while `adaptive_video` and `adaptive_audio` hold the higher-quality split
/// tracks used for DASH-style playback or downloading. `hls_manifest_url` is
/// populated for live and DVR content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamMap {
    /// Muxed (audio + video) formats, ordered highest quality first.
    pub progressive: Vec<Stream>,
    /// Adaptive, video-only formats, ordered by height descending.
    pub adaptive_video: Vec<Stream>,
    /// Adaptive, audio-only formats, ordered by bitrate descending.
    pub adaptive_audio: Vec<Stream>,
    /// Master HLS playlist URL for live/DVR content, when InnerTube
    /// advertises one.
    pub hls_manifest_url: Option<String>,
}

impl StreamMap {
    /// Returns `true` if no streams of any kind are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.progressive.is_empty()
            && self.adaptive_video.is_empty()
            && self.adaptive_audio.is_empty()
            && self.hls_manifest_url.is_none()
    }

    /// Total number of streams across all categories (excluding the HLS URL).
    #[must_use]
    pub fn len(&self) -> usize {
        self.progressive.len() + self.adaptive_video.len() + self.adaptive_audio.len()
    }

    /// Best (highest-resolution) progressive stream, if any.
    #[must_use]
    pub fn best_progressive(&self) -> Option<&Stream> {
        self.progressive
            .iter()
            .max_by_key(|s| s.height.unwrap_or(0))
    }

    /// Best (highest-resolution) adaptive video stream, if any.
    #[must_use]
    pub fn best_video(&self) -> Option<&Stream> {
        self.adaptive_video
            .iter()
            .max_by_key(|s| (s.height.unwrap_or(0), s.fps.unwrap_or(0)))
    }

    /// Best (highest-bitrate) adaptive audio stream, if any.
    #[must_use]
    pub fn best_audio(&self) -> Option<&Stream> {
        self.adaptive_audio
            .iter()
            .max_by_key(|s| s.bitrate.unwrap_or(0))
    }
}
