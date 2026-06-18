//! Converting InnerTube `format` objects into [`Stream`] values, with
//! signature deciphering and n-param rewriting.

use serde_json::Value;

use crate::error::{Error, Result};
use crate::streams::url_util::{
    parse_signature_cipher, replace_query_param, rewrite_n_param,
};
use crate::streams::PlayerJsResolver;
use crate::types::stream::{Stream, StreamMap};

/// Parse a single InnerTube format object into a [`Stream`]. Returns
/// `Ok(None)` when the format carries neither a direct `url` nor a
/// `signatureCipher`.
pub(crate) fn format_to_stream(fmt: &Value, via_proxy: bool) -> Result<Option<Stream>> {
    let url = fmt.get("url").and_then(|v| v.as_str()).map(String::from);
    let sc = fmt.get("signatureCipher").and_then(|v| v.as_str());
    let url = match (url, sc) {
        (Some(u), _) => u,
        (None, Some(s)) => match parse_signature_cipher(s) {
            Some((_s_val, _sp, url_val)) => url_val,
            None => return Ok(None),
        },
        (None, None) => return Ok(None),
    };

    let mime_type = fmt
        .get("mimeType")
        .and_then(|v| v.as_str())
        .unwrap_or("application/octet-stream")
        .to_string();
    let itag = fmt
        .get("itag")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Error::decode("format missing itag"))? as u32;

    let bitrate = fmt.get("bitrate").and_then(|v| v.as_u64());
    let width = fmt.get("width").and_then(|v| v.as_u64()).map(|x| x as u32);
    let height = fmt.get("height").and_then(|v| v.as_u64()).map(|x| x as u32);
    let fps = fmt.get("fps").and_then(|v| v.as_u64()).map(|x| x as u32);
    let audio_sample_rate = fmt
        .get("audioSampleRate")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u32>().ok());
    let audio_channels = fmt
        .get("audioChannels")
        .and_then(|v| v.as_u64())
        .map(|x| x as u32);
    let content_length = fmt
        .get("contentLength")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());
    let approx_duration = fmt
        .get("approxDurationMs")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| fmt.get("approxDurationMs").and_then(|v| v.as_u64()));
    let quality_label = fmt
        .get("qualityLabel")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(Some(Stream {
        itag,
        url,
        mime_type,
        bitrate,
        width,
        height,
        fps,
        audio_sample_rate,
        audio_channels,
        content_length,
        duration_ms: approx_duration,
        via_proxy,
        quality_label,
    }))
}

/// Convert an InnerTube format object into a fully resolved [`Stream`],
/// deciphering the signature if necessary and rewriting the n-param.
pub(crate) async fn finalize_stream(
    fmt: &Value,
    resolver: &PlayerJsResolver,
    via_proxy: bool,
) -> Result<Option<Stream>> {
    let Some(mut stream) = format_to_stream(fmt, via_proxy)? else {
        return Ok(None);
    };

    if let Some(sc) = fmt.get("signatureCipher").and_then(|v| v.as_str()) {
        if let Some((s_val, sp, _)) = parse_signature_cipher(sc) {
            let deciphered = resolver.decipher_s(&s_val).await?;
            stream.url = replace_query_param(&stream.url, &sp, &deciphered);
        }
    }

    stream.url = rewrite_n_param(&stream.url, resolver).await?;
    Ok(Some(stream))
}

/// Sort helper: progressive by height desc, video by (height, fps) desc,
/// audio by bitrate desc.
pub(crate) fn sort_streams(streams: &mut [Stream]) {
    streams.sort_by(|a, b| {
        let av = (a.height.unwrap_or(0), a.fps.unwrap_or(0), a.bitrate.unwrap_or(0));
        let bv = (b.height.unwrap_or(0), b.fps.unwrap_or(0), b.bitrate.unwrap_or(0));
        bv.cmp(&av)
    });
}

/// Build a [`StreamMap`] from a `player` response's `streamingData`. Returns
/// `Ok(map)` even if the map is empty so the caller can decide whether to
/// fall back to Piped.
pub(crate) async fn build_stream_map(
    player: &Value,
    resolver: &PlayerJsResolver,
) -> Result<StreamMap> {
    let streaming = match player.get("streamingData") {
        Some(s) => s,
        None => return Ok(StreamMap::default()),
    };
    let formats = streaming.get("formats").and_then(|f| f.as_array());
    let adaptive = streaming
        .get("adaptiveFormats")
        .and_then(|f| f.as_array());
    let hls = streaming
        .get("hlsManifestUrl")
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut progressive = Vec::new();
    let mut adaptive_video = Vec::new();
    let mut adaptive_audio = Vec::new();

    if let Some(formats) = formats {
        for fmt in formats {
            if let Some(stream) = finalize_stream(fmt, resolver, false).await? {
                progressive.push(stream);
            }
        }
    }
    if let Some(adaptive) = adaptive {
        for fmt in adaptive {
            if let Some(stream) = finalize_stream(fmt, resolver, false).await? {
                if stream.has_video() {
                    adaptive_video.push(stream);
                } else if stream.has_audio() {
                    adaptive_audio.push(stream);
                }
            }
        }
    }

    sort_streams(&mut progressive);
    sort_streams(&mut adaptive_video);
    sort_streams(&mut adaptive_audio);

    Ok(StreamMap {
        progressive,
        adaptive_video,
        adaptive_audio,
        hls_manifest_url: hls,
    })
}
