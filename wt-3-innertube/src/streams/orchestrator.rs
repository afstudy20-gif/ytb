//! Top-level orchestrator: Android → WEB → Piped resolution chain.

use serde_json::{Map, Value};

use crate::client::{ClientContext, InnerTubeClient};
use crate::error::{Error, Result};
use crate::piped::PipedFallback;
use crate::streams::format::build_stream_map;
use crate::streams::PlayerJsResolver;
use crate::types::stream::StreamMap;

/// Top-level orchestrator: try Android, then WEB, then Piped.
pub async fn resolve_streams(
    http: &InnerTubeClient,
    resolver: &PlayerJsResolver,
    piped: Option<&PipedFallback>,
    video_id: &str,
) -> Result<StreamMap> {
    // Android first (no cipher needed).
    match call_player(http, ClientContext::ANDROID_DEFAULT, video_id).await {
        Ok(player) => {
            let map = build_stream_map(&player, resolver).await?;
            if !map.is_empty() {
                return Ok(map);
            }
            tracing::info!("android player returned no usable formats");
        }
        Err(e) => {
            tracing::info!(error = %e, "android player call failed; trying WEB");
        }
    }
    // WEB fallback (cipher deciphering required).
    match call_player(http, ClientContext::WEB_DEFAULT, video_id).await {
        Ok(player) => {
            let map = build_stream_map(&player, resolver).await?;
            if !map.is_empty() {
                return Ok(map);
            }
            tracing::info!("web player returned no usable formats");
        }
        Err(e) => {
            tracing::info!(error = %e, "web player call failed");
        }
    }
    // Piped last-resort.
    if let Some(piped) = piped {
        tracing::info!(video_id, "falling back to Piped for streams");
        return piped.streams(video_id).await;
    }
    Err(Error::NoStreams(video_id.to_string()))
}

/// Call InnerTube's `player` endpoint with a particular client context.
pub(crate) async fn call_player(
    http: &InnerTubeClient,
    ctx: ClientContext,
    video_id: &str,
) -> Result<Value> {
    let mut body = Map::new();
    body.insert("videoId".into(), Value::String(video_id.to_string()));
    http.post("player", ctx, body).await
}
