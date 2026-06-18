//! Piped fallback client.
//!
//! Piped (<https://github.com/TeamPiped/Piped-Backend>) is an open-source
//! proxy that resolves YouTube stream URLs server-side. We use it as a
//! last-resort fallback when InnerTube returns 403/429 or no usable formats.
//!
//! [`PipedFallback`] holds a list of instance base URLs (e.g.
//! `https://pipedapi.kavin.rocks`) and round-robins between them, demoting
//! any that fail repeatedly.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::client::InnerTubeClient;
use crate::error::{Error, Result};
use crate::types::stream::{Stream, StreamMap};

/// How many consecutive failures before we permanently skip an instance for
/// the rest of the process lifetime.
const MAX_FAILURES: u32 = 3;

/// How long to consider an instance "cooling down" after a failure.
const COOLDOWN: Duration = Duration::from_secs(60);

/// Per-instance health record.
#[derive(Debug)]
struct InstanceState {
    base_url: String,
    failures: u32,
    last_failure: Option<Instant>,
}

impl InstanceState {
    fn is_alive(&self) -> bool {
        if self.failures >= MAX_FAILURES {
            return false;
        }
        if let Some(last) = self.last_failure {
            if last.elapsed() < COOLDOWN {
                return false;
            }
        }
        true
    }

    fn record_failure(&mut self) {
        self.failures = self.failures.saturating_add(1);
        self.last_failure = Some(Instant::now());
    }

    fn record_success(&mut self) {
        self.failures = 0;
        self.last_failure = None;
    }
}

/// The Piped fallback. Cloning shares the health table via `Arc`.
pub struct PipedFallback {
    http: InnerTubeClient,
    instances: Vec<std::sync::Mutex<InstanceState>>,
    cursor: AtomicUsize,
}

impl PipedFallback {
    /// Construct a fallback that will round-robin across `instances`.
    pub fn new(http: InnerTubeClient, instances: Vec<String>) -> Self {
        let instances = instances
            .into_iter()
            .map(|base_url| {
                std::sync::Mutex::new(InstanceState {
                    base_url,
                    failures: 0,
                    last_failure: None,
                })
            })
            .collect();
        Self {
            http,
            instances,
            cursor: AtomicUsize::new(0),
        }
    }

    /// Number of configured instances.
    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether no instances are configured.
    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Try each instance in round-robin order until one returns usable
    /// streams. All resolved streams are marked `via_proxy = true`.
    pub async fn streams(&self, video_id: &str) -> Result<StreamMap> {
        if self.instances.is_empty() {
            return Err(Error::PipedFallbackFailed("no instances configured".into()));
        }
        let start = self.cursor.fetch_add(1, Ordering::Relaxed);
        let mut errors = Vec::new();
        for offset in 0..self.instances.len() {
            let idx = (start + offset) % self.instances.len();
            let alive = {
                let state = self.instances[idx]
                    .lock()
                    .expect("piped state lock poisoned");
                state.is_alive()
            };
            if !alive {
                continue;
            }
            let base_url = {
                let state = self.instances[idx]
                    .lock()
                    .expect("piped state lock poisoned");
                state.base_url.clone()
            };
            let url = format!("{}/streams/{}", base_url.trim_end_matches('/'), video_id);
            tracing::info!(%url, "trying piped instance");
            match self.http.get_json(&url).await {
                Ok(value) => {
                    let map = parse_piped_response(&value)?;
                    if map.is_empty() {
                        let msg = format!("{base_url}: empty response");
                        errors.push(msg.clone());
                        self.record_failure(idx);
                        continue;
                    }
                    self.record_success(idx);
                    return Ok(map);
                }
                Err(e) => {
                    let msg = format!("{base_url}: {e}");
                    tracing::info!(%msg, "piped instance failed");
                    errors.push(msg);
                    self.record_failure(idx);
                }
            }
        }
        Err(Error::PipedFallbackFailed(errors.join("; ")))
    }

    fn record_failure(&self, idx: usize) {
        if let Ok(mut state) = self.instances[idx].lock() {
            state.record_failure();
        }
    }

    fn record_success(&self, idx: usize) {
        if let Ok(mut state) = self.instances[idx].lock() {
            state.record_success();
        }
    }
}

/// Map a Piped `/streams/{videoId}` JSON response into our [`StreamMap`].
///
/// Piped's response shape (abridged):
/// ```json
/// {
///   "videoId": "...",
///   "videoStreams": [{"url": "...", "mimeType": "...", "itag": 137, ...}],
///   "audioStreams": [{"url": "...", "mimeType": "...", "itag": 251, ...}],
///   "hls": "..."
/// }
/// ```
fn parse_piped_response(value: &serde_json::Value) -> Result<StreamMap> {
    use serde_json::Value;

    fn parse_one(v: &Value) -> Option<Stream> {
        let url = v.get("url").and_then(|x| x.as_str())?.to_string();
        let mime_type = v
            .get("mimeType")
            .and_then(|x| x.as_str())
            .unwrap_or("application/octet-stream")
            .to_string();
        let itag = v.get("itag").and_then(|x| x.as_u64())? as u32;
        let bitrate = v.get("bitrate").and_then(|x| x.as_u64());
        let width = v.get("width").and_then(|x| x.as_u64()).map(|x| x as u32);
        let height = v.get("height").and_then(|x| x.as_u64()).map(|x| x as u32);
        let fps = v.get("fps").and_then(|x| x.as_u64()).map(|x| x as u32);
        let audio_sample_rate = v
            .get("audioSampleRate")
            .and_then(|x| x.as_u64())
            .map(|x| x as u32);
        let content_length = v
            .get("contentLength")
            .and_then(|x| x.as_u64())
            .or_else(|| {
                v.get("contentLength")
                    .and_then(|x| x.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
            });
        let quality_label = v
            .get("quality")
            .and_then(|x| x.as_str())
            .map(String::from);
        Some(Stream {
            itag,
            url,
            mime_type,
            bitrate,
            width,
            height,
            fps,
            audio_sample_rate,
            audio_channels: None,
            content_length,
            duration_ms: None,
            via_proxy: true,
            quality_label,
        })
    }

    let mut progressive = Vec::new();
    let mut adaptive_video = Vec::new();
    let mut adaptive_audio = Vec::new();

    if let Some(arr) = value.get("videoStreams").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = parse_one(v) {
                if s.has_audio() {
                    progressive.push(s);
                } else {
                    adaptive_video.push(s);
                }
            }
        }
    }
    if let Some(arr) = value.get("audioStreams").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = parse_one(v) {
                adaptive_audio.push(s);
            }
        }
    }
    let hls = value
        .get("hls")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    Ok(StreamMap {
        progressive,
        adaptive_video,
        adaptive_audio,
        hls_manifest_url: hls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_piped_response_maps_video_and_audio_streams() {
        let resp = json!({
            "videoStreams": [
                {"url": "https://cdn/v", "mimeType": "video/mp4", "itag": 137, "height": 1080, "bitrate": 5000000},
                {"url": "https://cdn/p", "mimeType": "video/mp4", "itag": 18, "width": 640, "height": 360, "audioSampleRate": 44100}
            ],
            "audioStreams": [
                {"url": "https://cdn/a", "mimeType": "audio/webm", "itag": 251, "bitrate": 160000, "audioSampleRate": 44100}
            ],
            "hls": "https://hls/m.m3u8"
        });
        let map = parse_piped_response(&resp).expect("parsed");
        assert_eq!(map.progressive.len(), 1);
        assert_eq!(map.adaptive_video.len(), 1);
        assert_eq!(map.adaptive_audio.len(), 1);
        assert_eq!(map.hls_manifest_url.as_deref(), Some("https://hls/m.m3u8"));
        assert!(map.progressive[0].via_proxy);
        assert!(map.adaptive_audio[0].via_proxy);
    }

    #[test]
    fn parse_piped_response_handles_empty() {
        let resp = json!({});
        let map = parse_piped_response(&resp).expect("parsed");
        assert!(map.is_empty());
    }

    #[test]
    fn instance_state_alive_logic() {
        let mut s = InstanceState {
            base_url: "x".into(),
            failures: 0,
            last_failure: None,
        };
        assert!(s.is_alive());
        for _ in 0..MAX_FAILURES {
            s.record_failure();
        }
        assert!(!s.is_alive());
        s.record_success();
        assert!(s.is_alive());
    }
}
