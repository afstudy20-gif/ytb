# YTB

Standalone Tauri v2 shell for a React frontend and Android-first media playback service.

## Development

```sh
pnpm install
pnpm tauri dev
```

## Android

```sh
pnpm tauri android dev
```

The checked-in Android tree lives under `src-tauri/gen/android`. It targets API 26+ and uses AndroidX Media3 ExoPlayer only; there is no microG or Google Play Services dependency.

## Commands

- `play(url, title, artist, artwork)` starts playback for a direct HTTP(S) stream URL.
- `pause()` pauses current playback.
- `resume()` resumes current playback.
- `seek(position_ms)` seeks current media.
- `stop()` stops playback and resets the foreground service.
- `set_queue(items)` replaces the queue with `QueueItem` records.
- `get_playback_state()` returns `PlaybackState`.

## Service Architecture

Rust exposes stable Tauri command signatures and currently stores desktop/dev playback state in memory.
On Android, `PlaybackPlugin` starts and binds to `PlaybackService`.
`PlaybackService` extends Media3 `MediaSessionService`, owns an `ExoPlayer`, and publishes a media notification while foregrounded.
Audio focus loss and becoming-noisy events pause playback.
The service receives direct stream URLs; YouTube extraction, downloads, and API calls are intentionally out of scope.
