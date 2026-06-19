package com.afstudy20.ytb

import android.app.Activity
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Build
import android.os.IBinder
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

@InvokeArg
class PlayArgs {
    lateinit var url: String
    var title: String? = null
    var artist: String? = null
    var artwork: String? = null
}

@InvokeArg
class SeekArgs {
    var position_ms: Long = 0L
}

@TauriPlugin
class PlaybackPlugin(private val activity: Activity) : Plugin(activity) {
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
        val args = invoke.parseArgs(PlayArgs::class.java)
        if (args.url.isBlank()) return invoke.reject("url is required")
        scope.launch {
            sendAction(
                PlaybackService.ACTION_PLAY,
                PlaybackService.EXTRA_URL to args.url,
                PlaybackService.EXTRA_TITLE to (args.title ?: ""),
                PlaybackService.EXTRA_ARTIST to (args.artist ?: ""),
                PlaybackService.EXTRA_ARTWORK to args.artwork,
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
        val args = invoke.parseArgs(SeekArgs::class.java)
        scope.launch {
            sendAction(PlaybackService.ACTION_SEEK, PlaybackService.EXTRA_POSITION_MS to args.position_ms)
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
        val result = JSObject()
        result.put("playing", false)
        result.put("position_ms", 0)
        result.put("duration_ms", null as Any?)
        result.put("current_id", null as Any?)
        invoke.resolve(result)
    }

    override fun load(webView: android.webkit.WebView) {
        super.load(webView)
        bind()
    }

    fun teardown() {
        if (bound) {
            runCatching { activity.unbindService(connection) }
            bound = false
        }
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
