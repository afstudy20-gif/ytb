use serde::{Deserialize, Serialize};
use std::sync::{Mutex, MutexGuard, OnceLock};
use tauri::Manager;

mod innertube_bridge;
mod stream_proxy;
mod video_player;
use innertube_bridge::InnertubeState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub stream_url: String,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaybackState {
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub current_id: Option<String>,
}

#[derive(Debug, Default)]
struct PlayerState {
    queue: Vec<QueueItem>,
    playback: PlaybackState,
}

static PLAYER_STATE: OnceLock<Mutex<PlayerState>> = OnceLock::new();

fn lock_state() -> Result<MutexGuard<'static, PlayerState>, String> {
    PLAYER_STATE
        .get_or_init(|| Mutex::new(PlayerState::default()))
        .lock()
        .map_err(|_| "playback state lock poisoned".to_string())
}

#[tauri::command]
async fn play(
    url: String,
    title: String,
    artist: String,
    artwork: Option<String>,
) -> Result<(), String> {
    let mut state = lock_state()?;
    let id = url.clone();
    state.playback = PlaybackState {
        playing: true,
        position_ms: 0,
        duration_ms: None,
        current_id: Some(id.clone()),
    };

    if !state.queue.iter().any(|item| item.id == id) {
        state.queue.insert(
            0,
            QueueItem {
                id,
                title,
                artist,
                stream_url: url,
                artwork_url: artwork,
            },
        );
    }

    Ok(())
}

#[tauri::command]
async fn pause() -> Result<(), String> {
    lock_state()?.playback.playing = false;
    Ok(())
}

#[tauri::command]
async fn resume() -> Result<(), String> {
    lock_state()?.playback.playing = true;
    Ok(())
}

#[tauri::command]
async fn seek(position_ms: u64) -> Result<(), String> {
    lock_state()?.playback.position_ms = position_ms;
    Ok(())
}

#[tauri::command]
async fn stop() -> Result<(), String> {
    let mut state = lock_state()?;
    state.playback.playing = false;
    state.playback.position_ms = 0;
    Ok(())
}

#[tauri::command]
async fn set_queue(items: Vec<QueueItem>) -> Result<(), String> {
    let mut state = lock_state()?;
    state.queue = items;
    state.playback.current_id = state.queue.first().map(|item| item.id.clone());
    state.playback.position_ms = 0;
    Ok(())
}

#[tauri::command]
async fn get_playback_state() -> Result<PlaybackState, String> {
    Ok(lock_state()?.playback.clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "android")]
            video_player::attach_activity();

            let proxy = tauri::async_runtime::block_on(stream_proxy::StreamProxy::start())
                .expect("failed to start stream proxy");
            app.manage(InnertubeState {
                client: innertube::InnerTube::with_piped_fallback(
                    innertube_bridge::PIPED_INSTANCES.iter().map(|s| (*s).to_string()).collect(),
                ),
                proxy,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            play,
            pause,
            resume,
            seek,
            stop,
            set_queue,
            get_playback_state,
            video_player::open_video_player,
            video_player::close_video_player,
            video_player::set_video_url,
            video_player::set_video_bounds,
            video_player::play_video,
            video_player::pause_video,
            video_player::seek_video,
            video_player::get_video_state,
            innertube_bridge::yt_search,
            innertube_bridge::yt_trending,
            innertube_bridge::yt_video,
            innertube_bridge::yt_streams,
            innertube_bridge::yt_channel,
            innertube_bridge::yt_playlist,
            innertube_bridge::yt_sponsor_block,
            innertube_bridge::yt_return_youtube_dislike
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
