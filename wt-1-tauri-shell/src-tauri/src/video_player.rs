//! JNI bridge to the native Android ExoPlayer video surface.
//!
//! On Android the WebView cannot reliably play YouTube/Invidious stream URLs.
//! Instead we overlay a native PlayerView on top of the WebView and control it
//! via these Tauri commands.

use tauri::{command, AppHandle, Runtime};

fn with_video_env<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&mut jni::AttachGuard, jni::objects::JClass) -> Result<R, jni::errors::Error>,
{
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|e| format!("failed to get JavaVM: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("failed to attach JNI thread: {e}"))?;

    let cls = env
        .find_class("com/afstudy20/ytb/VideoPlayerHelper")
        .map_err(|e| format!("failed to find VideoPlayerHelper class: {e}"))?;

    f(&mut env, cls).map_err(|e| format!("JNI call failed: {e}"))
}

pub fn attach_activity() {
    #[cfg(target_os = "android")]
    {
        let _ = with_video_env(|env, cls| {
            let ctx = ndk_context::android_context();
            let ctx_obj = unsafe { jni::objects::JObject::from_raw(ctx.object().cast()) };
            env.call_static_method(
                &cls,
                "attach",
                "(Landroid/content/Context;)V",
                &[jni::objects::JValue::Object(&ctx_obj)],
            )?;
            Ok(())
        });
    }
}

#[command]
pub async fn open_video_player<R: Runtime>(
    _app: AppHandle<R>,
    url: String,
    title: Option<String>,
    artist: Option<String>,
    artwork: Option<String>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        attach_activity();
        let title = title.unwrap_or_default();
        let artist = artist.unwrap_or_default();
        let artwork = artwork.unwrap_or_default();
        with_video_env(|env, cls| {
            let url_j = env.new_string(&url)?;
            let title_j = env.new_string(&title)?;
            let artist_j = env.new_string(&artist)?;
            let artwork_j = env.new_string(&artwork)?;
            env.call_static_method(
                &cls,
                "openPlayer",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;IIII)V",
                &[
                    jni::objects::JValue::Object(&url_j),
                    jni::objects::JValue::Object(&title_j),
                    jni::objects::JValue::Object(&artist_j),
                    jni::objects::JValue::Object(&artwork_j),
                    jni::objects::JValue::Int(x),
                    jni::objects::JValue::Int(y),
                    jni::objects::JValue::Int(width),
                    jni::objects::JValue::Int(height),
                ],
            )?;
            Ok(())
        })?;
    }
    Ok(())
}

#[command]
pub async fn close_video_player<R: Runtime>(_app: AppHandle<R>) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        with_video_env(|env, cls| {
            env.call_static_method(&cls, "closePlayer", "()V", &[])?;
            Ok(())
        })?;
    }
    Ok(())
}

#[command]
pub async fn set_video_url<R: Runtime>(
    _app: AppHandle<R>,
    url: String,
    title: Option<String>,
    artist: Option<String>,
    artwork: Option<String>,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let title = title.unwrap_or_default();
        let artist = artist.unwrap_or_default();
        let artwork = artwork.unwrap_or_default();
        with_video_env(|env, cls| {
            let url_j = env.new_string(&url)?;
            let title_j = env.new_string(&title)?;
            let artist_j = env.new_string(&artist)?;
            let artwork_j = env.new_string(&artwork)?;
            env.call_static_method(
                &cls,
                "setUrl",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                &[
                    jni::objects::JValue::Object(&url_j),
                    jni::objects::JValue::Object(&title_j),
                    jni::objects::JValue::Object(&artist_j),
                    jni::objects::JValue::Object(&artwork_j),
                ],
            )?;
            Ok(())
        })?;
    }
    Ok(())
}

#[command]
pub async fn set_video_bounds<R: Runtime>(
    _app: AppHandle<R>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        with_video_env(|env, cls| {
            env.call_static_method(
                &cls,
                "setBounds",
                "(IIII)V",
                &[
                    jni::objects::JValue::Int(x),
                    jni::objects::JValue::Int(y),
                    jni::objects::JValue::Int(width),
                    jni::objects::JValue::Int(height),
                ],
            )?;
            Ok(())
        })?;
    }
    Ok(())
}

#[command]
pub async fn play_video<R: Runtime>(_app: AppHandle<R>) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        with_video_env(|env, cls| {
            env.call_static_method(&cls, "play", "()V", &[])?;
            Ok(())
        })?;
    }
    Ok(())
}

#[command]
pub async fn pause_video<R: Runtime>(_app: AppHandle<R>) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        with_video_env(|env, cls| {
            env.call_static_method(&cls, "pause", "()V", &[])?;
            Ok(())
        })?;
    }
    Ok(())
}

#[command]
pub async fn seek_video<R: Runtime>(_app: AppHandle<R>, position_ms: i64) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        with_video_env(|env, cls| {
            env.call_static_method(
                &cls,
                "seekTo",
                "(J)V",
                &[jni::objects::JValue::Long(position_ms)],
            )?;
            Ok(())
        })?;
    }
    Ok(())
}

#[command]
pub async fn get_video_state<R: Runtime>(_app: AppHandle<R>) -> Result<VideoPlayerState, String> {
    #[cfg(target_os = "android")]
    {
        with_video_env(|env, cls| {
            let is_playing = env
                .call_static_method(&cls, "isPlaying", "()Z", &[])?
                .z()
                .unwrap_or(false);
            let position = env
                .call_static_method(&cls, "currentPosition", "()J", &[])?
                .j()
                .unwrap_or(0);
            let duration = env
                .call_static_method(&cls, "duration", "()J", &[])?
                .j()
                .unwrap_or(-1);
            Ok(VideoPlayerState {
                is_playing,
                position_ms: position,
                duration_ms: duration,
            })
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        Ok(VideoPlayerState {
            is_playing: false,
            position_ms: 0,
            duration_ms: -1,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPlayerState {
    pub is_playing: bool,
    pub position_ms: i64,
    pub duration_ms: i64,
}
