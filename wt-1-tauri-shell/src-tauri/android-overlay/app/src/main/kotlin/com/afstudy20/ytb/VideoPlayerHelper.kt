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
        val act = activity ?: run {
            Log.w(TAG, "openPlayer: no activity attached")
            return
        }
        closePlayer()

        val density = act.resources.displayMetrics.density
        val px = boundsToPx(x, y, width, height, density)

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
    }

    @JvmStatic
    fun closePlayer() {
        playerView?.player = null
        player?.release()
        container?.let { c ->
            (c.parent as? ViewGroup)?.removeView(c)
        }
        player = null
        playerView = null
        container = null
    }

    @JvmStatic
    fun setUrl(url: String, title: String?, artist: String?, artwork: String?) {
        loadUrl(url, title, artist, artwork)
    }

    @JvmStatic
    fun setBounds(x: Int, y: Int, width: Int, height: Int) {
        val act = activity ?: return
        val density = act.resources.displayMetrics.density
        val px = boundsToPx(x, y, width, height, density)
        val lp = container?.layoutParams as? FrameLayout.LayoutParams ?: return
        lp.width = px.width
        lp.height = px.height
        lp.setMargins(px.x, px.y, 0, 0)
        container?.layoutParams = lp
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
        container?.visibility = if (visible) View.VISIBLE else View.GONE
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
