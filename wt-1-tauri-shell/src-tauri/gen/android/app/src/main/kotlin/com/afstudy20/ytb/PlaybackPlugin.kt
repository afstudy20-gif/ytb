package com.afstudy20.ytb

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Build
import android.os.IBinder
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

@TauriPlugin
class PlaybackPlugin(private val activity: android.app.Activity) : Plugin(activity) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var bound = false

    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, service: IBinder) {
            bound = true
        }

        override fun onServiceDisconnected(name: ComponentName) {
            bound = false
        }
    }

    @Command
    fun play(invoke: Invoke) {
        val url = invoke.getString("url") ?: return invoke.reject("url is required")
        val title = invoke.getString("title") ?: ""
        val artist = invoke.getString("artist") ?: ""
        val artwork = invoke.getString("artwork")

        scope.launch {
            sendAction(
                PlaybackService.ACTION_PLAY,
                PlaybackService.EXTRA_URL to url,
                PlaybackService.EXTRA_TITLE to title,
                PlaybackService.EXTRA_ARTIST to artist,
                PlaybackService.EXTRA_ARTWORK to artwork,
            )
            invoke.resolve()
        }
    }

    @Command
    fun pause(invoke: Invoke) {
        scope.launch {
            sendAction(PlaybackService.ACTION_PAUSE)
            invoke.resolve()
        }
    }

    @Command
    fun resume(invoke: Invoke) {
        scope.launch {
            sendAction(PlaybackService.ACTION_RESUME)
            invoke.resolve()
        }
    }

    @Command
    fun seek(invoke: Invoke) {
        val positionMs = invoke.getLong("position_ms") ?: return invoke.reject("position_ms is required")
        scope.launch {
            sendAction(PlaybackService.ACTION_SEEK, PlaybackService.EXTRA_POSITION_MS to positionMs)
            invoke.resolve()
        }
    }

    @Command
    fun stop(invoke: Invoke) {
        scope.launch {
            sendAction(PlaybackService.ACTION_STOP)
            invoke.resolve()
        }
    }

    @Command
    fun setQueue(invoke: Invoke) {
        sendAction(PlaybackService.ACTION_SET_QUEUE)
        invoke.resolve()
    }

    @Command
    fun getPlaybackState(invoke: Invoke) {
        invoke.resolve(
            mapOf(
                "playing" to false,
                "position_ms" to 0,
                "duration_ms" to null,
                "current_id" to null,
            ),
        )
    }

    override fun load(webView: android.webkit.WebView) {
        super.load(webView)
        bind()
    }

    override fun destroy() {
        if (bound) {
            activity.unbindService(connection)
            bound = false
        }
        super.destroy()
    }

    private fun bind() {
        if (bound) return
        val intent = Intent(activity, PlaybackService::class.java)
        activity.bindService(intent, connection, Context.BIND_AUTO_CREATE)
    }

    private fun startService(intent: Intent, foreground: Boolean = false) {
        if (foreground && Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            activity.startForegroundService(intent)
        } else {
            activity.startService(intent)
        }
    }

    private fun sendAction(action: String, vararg extras: Pair<String, Any?>) {
        bind()
        val intent = Intent(activity, PlaybackService::class.java).setAction(action)
        extras.forEach { (key, value) ->
            when (value) {
                is String -> intent.putExtra(key, value)
                is Long -> intent.putExtra(key, value)
                null -> Unit
                else -> error("Unsupported playback extra type for $key")
            }
        }
        startService(intent, foreground = action == PlaybackService.ACTION_PLAY)
    }
}
