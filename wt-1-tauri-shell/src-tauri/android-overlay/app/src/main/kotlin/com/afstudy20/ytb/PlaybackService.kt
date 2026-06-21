package com.afstudy20.ytb

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioManager
import android.media.AudioAttributes as PlatformAudioAttributes
import android.media.AudioFocusRequest
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import android.util.Log

class PlaybackService : MediaSessionService() {
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var player: ExoPlayer? = null
    private var mediaSession: MediaSession? = null
    private var audioManager: AudioManager? = null
    private var audioFocusRequest: AudioFocusRequest? = null

    private val noisyReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (intent.action == AudioManager.ACTION_AUDIO_BECOMING_NOISY) {
                player?.pause()
            }
        }
    }

    private val focusChangeListener = AudioManager.OnAudioFocusChangeListener { change ->
        when (change) {
            AudioManager.AUDIOFOCUS_LOSS,
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> player?.pause()
            AudioManager.AUDIOFOCUS_GAIN -> Unit
        }
    }

    override fun onCreate() {
        super.onCreate()
        try {
            createNotificationChannel()
            audioManager = getSystemService(AudioManager::class.java)

            player = ExoPlayer.Builder(this)
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(C.USAGE_MEDIA)
                        .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
                        .build(),
                    true,
                )
                .setHandleAudioBecomingNoisy(true)
                .setWakeMode(C.WAKE_MODE_LOCAL)
                .build()

            mediaSession = MediaSession.Builder(this, requirePlayer()).build()
            ContextCompat.registerReceiver(
                this,
                noisyReceiver,
                IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY),
                ContextCompat.RECEIVER_NOT_EXPORTED,
            )
        } catch (t: Throwable) {
            Log.e("PlaybackService", "onCreate failed", t)
        }
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? = mediaSession

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        super.onStartCommand(intent, flags, startId)
        try {
            when (intent?.action) {
                ACTION_PLAY -> play(
                    url = intent.getStringExtra(EXTRA_URL).orEmpty(),
                    title = intent.getStringExtra(EXTRA_TITLE).orEmpty(),
                    artist = intent.getStringExtra(EXTRA_ARTIST).orEmpty(),
                    artwork = intent.getStringExtra(EXTRA_ARTWORK),
                )
                ACTION_PAUSE -> pause()
                ACTION_RESUME -> resume()
                ACTION_SEEK -> seek(intent.getLongExtra(EXTRA_POSITION_MS, 0L))
                ACTION_STOP -> stopPlayback()
                ACTION_SET_QUEUE -> Unit
            }
        } catch (t: Throwable) {
            Log.e("PlaybackService", "onStartCommand failed", t)
        }

        return START_STICKY
    }

    override fun onDestroy() {
        try {
            runCatching { unregisterReceiver(noisyReceiver) }
            mediaSession?.run {
                player.release()
                release()
            }
            mediaSession = null
            player = null
            serviceScope.cancel()
            abandonAudioFocus()
        } catch (t: Throwable) {
            Log.e("PlaybackService", "onDestroy failed", t)
        }
        super.onDestroy()
    }

    fun play(url: String, title: String, artist: String, artwork: String?) {
        if (url.isBlank()) return

        serviceScope.launch {
            if (!requestAudioFocus()) return@launch

            val metadata = MediaMetadata.Builder()
                .setTitle(title)
                .setArtist(artist)
                .setArtworkUri(artwork?.let(android.net.Uri::parse))
                .build()
            val item = MediaItem.Builder()
                .setUri(url)
                .setMediaMetadata(metadata)
                .build()

            requirePlayer().apply {
                setMediaItem(item)
                prepare()
                playWhenReady = true
            }
            startForeground(NOTIFICATION_ID, buildNotification(title, artist))
        }
    }

    fun pause() {
        serviceScope.launch {
            requirePlayer().pause()
            stopForeground(STOP_FOREGROUND_DETACH)
        }
    }

    fun resume() {
        serviceScope.launch {
            if (!requestAudioFocus()) return@launch
            requirePlayer().play()
            startForeground(NOTIFICATION_ID, buildNotification())
        }
    }

    fun seek(positionMs: Long) {
        serviceScope.launch {
            requirePlayer().seekTo(positionMs.coerceAtLeast(0L))
        }
    }

    fun stopPlayback() {
        serviceScope.launch {
            requirePlayer().stop()
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        }
    }

    private fun requestAudioFocus(): Boolean {
        val manager = audioManager ?: return false
        val result = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(
                    PlatformAudioAttributes.Builder()
                        .setUsage(PlatformAudioAttributes.USAGE_MEDIA)
                        .setContentType(PlatformAudioAttributes.CONTENT_TYPE_MUSIC)
                        .build(),
                )
                .setOnAudioFocusChangeListener(focusChangeListener)
                .build()
            audioFocusRequest = request
            manager.requestAudioFocus(request)
        } else {
            @Suppress("DEPRECATION")
            manager.requestAudioFocus(
                focusChangeListener,
                AudioManager.STREAM_MUSIC,
                AudioManager.AUDIOFOCUS_GAIN,
            )
        }
        return result == AudioManager.AUDIOFOCUS_REQUEST_GRANTED
    }

    private fun abandonAudioFocus() {
        val manager = audioManager ?: return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            audioFocusRequest?.let(manager::abandonAudioFocusRequest)
            audioFocusRequest = null
        } else {
            @Suppress("DEPRECATION")
            manager.abandonAudioFocus(focusChangeListener)
        }
    }

    private fun requirePlayer(): ExoPlayer =
        checkNotNull(player) { "PlaybackService player is not initialized" }

    private fun buildNotification(
        title: String? = requirePlayer().mediaMetadata.title?.toString(),
        artist: String? = requirePlayer().mediaMetadata.artist?.toString(),
    ): Notification {
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_media_play)
            .setContentTitle(title ?: "YTB")
            .setContentText(artist)
            .setStyle(
                androidx.media.app.NotificationCompat.MediaStyle()
                    .setMediaSession(mediaSession?.sessionCompatToken),
            )
            .setOngoing(requirePlayer().isPlaying)
            .setOnlyAlertOnce(true)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .build()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return

        val channel = NotificationChannel(
            CHANNEL_ID,
            "Playback",
            NotificationManager.IMPORTANCE_LOW,
        )
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    companion object {
        private const val CHANNEL_ID = "playback"
        private const val NOTIFICATION_ID = 1001
        const val ACTION_PLAY = "com.afstudy20.ytb.action.PLAY"
        const val ACTION_PAUSE = "com.afstudy20.ytb.action.PAUSE"
        const val ACTION_RESUME = "com.afstudy20.ytb.action.RESUME"
        const val ACTION_SEEK = "com.afstudy20.ytb.action.SEEK"
        const val ACTION_STOP = "com.afstudy20.ytb.action.STOP"
        const val ACTION_SET_QUEUE = "com.afstudy20.ytb.action.SET_QUEUE"
        const val EXTRA_URL = "url"
        const val EXTRA_TITLE = "title"
        const val EXTRA_ARTIST = "artist"
        const val EXTRA_ARTWORK = "artwork"
        const val EXTRA_POSITION_MS = "position_ms"
    }
}
