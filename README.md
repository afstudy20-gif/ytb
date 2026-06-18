# YTB

Personal YouTube client for Android (Tauri v2) + web. Background audio playback, video & audio downloads, SponsorBlock, Return YouTube Dislike. No microG, no Google Play Services.

> Personal use only.

## Structure

| Path | Purpose |
|------|---------|
| `wt-1-tauri-shell/` | Tauri v2 app shell + Android Kotlin MediaSession foreground service |
| `wt-2-ui/` | React + Vite + TS mobile-first UI |
| `wt-3-innertube/` | Rust crate — InnerTube API client (search/streams/cipher) + Piped fallback |
| `wt-4-downloader/` | Rust crate — segmented downloader + ffmpeg muxer + persistent queue |
| `wt-5-extras/` | SponsorBlock + RYD clients (Rust + TS) + PWA service worker |

## Build

```bash
# Frontend
cd wt-1-tauri-shell && pnpm install && pnpm tauri dev

# Android
pnpm tauri android init  # first time
pnpm tauri android dev

# Each Rust crate
cd wt-3-innertube && cargo check
```

## Release

Tag push (`v*.*.*`) triggers signed Android APK + AAB + desktop bundles via `.github/workflows/release.yml`.

Required GitHub secrets:
- `ANDROID_KEY_BASE64`
- `ANDROID_KEY_PASSWORD`
- `ANDROID_KEY_ALIAS`
- `ANDROID_KEY_ALIAS_PASSWORD`

## Architecture

```
React UI (wt-2-ui)
   │
   ▼  YtClient interface
InnerTube crate (wt-3) ──► YouTube private API + Piped fallback
   │
   ▼  StreamMap
Tauri shell (wt-1) ──► Android FG service ──► ExoPlayer + MediaSession
   │
   ▼  enqueue
Downloader crate (wt-4) ──► segmented HTTP + ffmpeg mux ──► local files

Extras (wt-5):
  SponsorBlock → segment markers
  Return YouTube Dislike → rating display
  PWA shell → web background audio + offline storage
```
