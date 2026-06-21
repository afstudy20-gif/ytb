package com.afstudy20.ytb

import android.app.Activity
import android.content.Context
import android.net.Uri
import android.util.Log
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.ui.PlayerView

/**
 * Static helper used from Rust via JNI to show a native ExoPlayer video surface
 * on top of the WebView. The WebView-based player cannot play YouTube/Invidious
 * stream URLs because they require specific headers; ExoPlayer handles them directly.
 *
 * Bounds received from the web layer are in CSS pixels; they are converted to
 * physical pixels using the activity's display density before laying out the view.
 */
object VideoPlayerHelper {
    private const val TAG = "VideoPlayerHelper"

    private var activity: Activity? = null
    private var player: ExoPlayer? = null
    private var playerView: PlayerView? = null
    private var container: FrameLayout? = null

    @JvmStatic
    fun attach(context: Context) {
        when (context) {
            is Activity -> activity = context
            else -> Log.w(TAG, "attach: context is not an Activity (${context.javaClass.name})")
        }
    }

    @JvmStatic
    fun detach() {
        closePlayer()
        activity = null
    }

    @JvmStatic
    fun openPlayer(
        url: String,
        title: String?,
        artist: String?,
        artwork: String?,
        x: Int,
        y: Int,
        width: Int,
        height: Int,
    ) {
        try {
            val act = activity ?: run {
                Log.w(TAG, "openPlayer: no activity attached")
                return
            }
            if (act.isFinishing) {
                Log.w(TAG, "openPlayer: activity is finishing")
                return
            }
            closePlayer()

            val density = act.resources.displayMetrics.density
            val px = boundsToPx(x, y, width, height, density)
            Log.d(TAG, "openPlayer: bounds=$px, url=${url.take(120)}")

            // PlayerView and view hierarchy operations must run on the Android UI thread.
            act.runOnUiThread {
                try {
                    if (activity !== act || act.isFinishing) return@runOnUiThread

                    val surface = PlayerView(act).apply {
                        // The web controls are hidden behind this overlay, so use ExoPlayer's
                        // native controller for play/pause/seek/fullscreen.
                        useController = true
                        setShowBuffering(PlayerView.SHOW_BUFFERING_ALWAYS)
                    }
                    val dataSourceFactory = DefaultHttpDataSource.Factory()
                        .setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                    val exo = ExoPlayer.Builder(act)
                        .setMediaSourceFactory(DefaultMediaSourceFactory(dataSourceFactory))
                        .build()
                    surface.player = exo
                    exo.addListener(object : Player.Listener {
                        override fun onPlaybackStateChanged(state: Int) {
                            val stateName = when (state) {
                                Player.STATE_IDLE -> "IDLE"
                                Player.STATE_BUFFERING -> "BUFFERING"
                                Player.STATE_READY -> "READY"
                                Player.STATE_ENDED -> "ENDED"
                                else -> state.toString()
                            }
                            Log.d(TAG, "ExoPlayer state: $stateName")
                        }
                        override fun onPlayerError(error: PlaybackException) {
                            Log.e(TAG, "ExoPlayer error: ${error.errorCodeName} / ${error.message}", error)
                        }
                    })

                    val frame = FrameLayout(act).apply {
                        addView(surface, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT))
                    }

                    val root = act.findViewById<ViewGroup>(android.R.id.content)
                    // Add on top of the WebView so the native video surface is visible.
                    root?.addView(frame, FrameLayout.LayoutParams(px.width, px.height).apply {
                        setMargins(px.x, px.y, 0, 0)
                    })

                    player = exo
                    playerView = surface
                    container = frame

                    loadUrl(url, title, artist, artwork)
                } catch (t: Throwable) {
                    Log.e(TAG, "openPlayer UI setup failed", t)
                }
            }
        } catch (t: Throwable) {
            Log.e(TAG, "openPlayer failed", t)
        }
    }

    @JvmStatic
    fun closePlayer() {
        val p = player
        val pv = playerView
        val c = container
        player = null
        playerView = null
        container = null

        // View cleanup must happen on the UI thread. Use the view's own handler
        // so we don't depend on the Activity still being attached.
        try {
            (pv ?: c)?.post {
                try {
                    pv?.player = null
                    p?.release()
                    c?.let { (it.parent as? ViewGroup)?.removeView(it) }
                } catch (t: Throwable) {
                    Log.e(TAG, "closePlayer UI cleanup failed", t)
                }
            }
        } catch (t: Throwable) {
            Log.e(TAG, "closePlayer failed", t)
        }
    }

    @JvmStatic
    fun setUrl(url: String, title: String?, artist: String?, artwork: String?) {
        loadUrl(url, title, artist, artwork)
    }

    @JvmStatic
    fun setBounds(x: Int, y: Int, width: Int, height: Int) {
        try {
            val act = activity ?: return
            val density = act.resources.displayMetrics.density
            val px = boundsToPx(x, y, width, height, density)
            val c = container
            c?.post {
                try {
                    val lp = c.layoutParams as? FrameLayout.LayoutParams ?: return@post
                    lp.width = px.width
                    lp.height = px.height
                    lp.setMargins(px.x, px.y, 0, 0)
                    c.layoutParams = lp
                } catch (t: Throwable) {
                    Log.e(TAG, "setBounds UI update failed", t)
                }
            }
        } catch (t: Throwable) {
            Log.e(TAG, "setBounds failed", t)
        }
    }

    @JvmStatic
    fun play() {
        player?.play()
    }

    @JvmStatic
    fun pause() {
        player?.pause()
    }

    @JvmStatic
    fun seekTo(positionMs: Long) {
        player?.seekTo(positionMs.coerceAtLeast(0L))
    }

    @JvmStatic
    fun setPlayWhenReady(playWhenReady: Boolean) {
        player?.playWhenReady = playWhenReady
    }

    @JvmStatic
    fun isPlaying(): Boolean {
        return player?.isPlaying == true
    }

    @JvmStatic
    fun currentPosition(): Long {
        return player?.currentPosition ?: 0L
    }

    @JvmStatic
    fun duration(): Long {
        return player?.duration ?: -1L
    }

    @JvmStatic
    fun setSurfaceVisible(visible: Boolean) {
        val c = container
        c?.post {
            try {
                c.visibility = if (visible) View.VISIBLE else View.GONE
            } catch (t: Throwable) {
                Log.e(TAG, "setSurfaceVisible UI update failed", t)
            }
        }
    }

    private fun boundsToPx(x: Int, y: Int, width: Int, height: Int, density: Float): Bounds {
        return Bounds(
            x = (x * density).toInt(),
            y = (y * density).toInt(),
            width = (width * density).toInt(),
            height = (height * density).toInt(),
        )
    }

    private data class Bounds(val x: Int, val y: Int, val width: Int, val height: Int)

    private fun loadUrl(url: String, title: String?, artist: String?, artwork: String?) {
        val exo = player ?: run {
            Log.w(TAG, "loadUrl: player not initialized")
            return
        }
        if (url.isBlank()) {
            Log.w(TAG, "loadUrl: empty url")
            return
        }
        Log.d(TAG, "loadUrl: ${url.take(120)}")
        val metadata = MediaMetadata.Builder()
            .setTitle(title ?: "")
            .setArtist(artist ?: "")
            .setArtworkUri(artwork?.let(Uri::parse))
            .build()
        val item = MediaItem.Builder()
            .setUri(url)
            .setMediaMetadata(metadata)
            .build()
        exo.setMediaItem(item)
        exo.prepare()
        exo.playWhenReady = true
    }
}
