# downloader

`downloader` is a Rust 2021 library for queueing YouTube media downloads from a video id plus an Innertube-shaped stream map. It downloads DASH adaptive streams with HTTP `Range` requests, persists queue state in SQLite, and emits progress through a Tokio broadcast channel.

## Queue Model

`Downloader::new(Config)` opens `{output_dir}/.downloader.db`, creates the schema, and changes any interrupted `Running` jobs back to `Queued`. When constructed inside a Tokio runtime it starts a scheduler with up to `Config::max_concurrent_jobs` active jobs. Each job downloads up to `Config::max_concurrent_segments` ranges per stream.

Pause and resume are persisted state transitions. Workers check the database before starting segment work, so pause takes effect between segment downloads while already in-flight requests are allowed to finish.

`wifi_only` is carried in `Config` for platform integrations. This crate does not attempt OS-specific network-interface detection; callers should pause or avoid enqueueing when their platform policy says the network is not Wi-Fi.

## On-Disk Layout

```text
{output_dir}/
  .downloader.db
  .parts/{job_uuid}/
    video.part
    video.part.{idx}.part
    audio.part
    audio.part.{idx}.part
  {sanitized_title}_{video_id}.mp4
  {sanitized_title}_{video_id}.mp4.json
```

Audio-only jobs produce `.m4a` or `.opus` plus the same sidecar JSON. Segment rows live in the `segments` table, and completed segment files are reused after restart.

`rusqlite` is used instead of `sled` because the queue is relational, small, and benefits from simple ad-hoc inspection and migrations. SQLite also makes it straightforward to atomically reset interrupted jobs and query ordered work.

## StreamMap Contract

The integration layer should pass direct, signed adaptive format URLs. This crate expects only the fields in `types::Stream`: `itag`, `url`, `mime_type`, optional `content_length`, `bitrate`, dimensions, and optional audio quality. Cipher/signature resolution remains outside this crate.

## FFmpeg

The portable default uses an `ffmpeg` binary from `DOWNLOADER_FFMPEG` or `PATH`.

Feature flags:

- `bundled-ffmpeg`: looks for an `ffmpeg` binary next to the current executable before falling back to `PATH`.
- `libav`: enables the optional `ffmpeg-next` dependency and probes libav at runtime before falling back to the ffmpeg binary path.

Video muxing uses:

```text
ffmpeg -i video.part -i audio.part -c copy -movflags +faststart out.mp4
```

Audio rewrap/transcode also shells out to ffmpeg unless a future libav implementation handles that path.

## Verification

```bash
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test mux -- --ignored
```

The mux integration test is ignored by default because it requires a working ffmpeg binary with lavfi support.
