use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, Semaphore};
use tracing::{debug, error};
use uuid::Uuid;

use crate::job::{DownloadKind, Job, JobState, Progress};
use crate::mux::Muxer;
use crate::segment::{download_stream, DownloadRequest};
use crate::types::{AudioFormat, AudioQuality, Stream, StreamKind, StreamMap, VideoMeta};
use crate::{Error, Event, Inner, VideoQuality};

const SCHEDULER_TICK: Duration = Duration::from_millis(250);

pub(crate) fn start_scheduler(inner: Arc<Inner>) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        debug!("Downloader created outside a Tokio runtime; scheduler not started");
        return;
    };
    let semaphore = Arc::new(Semaphore::new(inner.cfg.max_concurrent_jobs));
    handle.spawn(async move {
        loop {
            if let Err(err) = schedule_once(&inner, &semaphore).await {
                error!(%err, "scheduler tick failed");
            }
            tokio::select! {
                () = inner.notify.notified() => {}
                () = tokio::time::sleep(SCHEDULER_TICK) => {}
            }
        }
    });
}

async fn schedule_once(inner: &Arc<Inner>, semaphore: &Arc<Semaphore>) -> Result<(), Error> {
    let ids = {
        let storage = inner.storage.lock().await;
        storage.queued_job_ids()?
    };
    for id in ids {
        let Ok(permit) = Arc::clone(semaphore).try_acquire_owned() else {
            break;
        };
        let started = {
            let mut storage = inner.storage.lock().await;
            storage.try_start(id)?
        };
        if !started {
            drop(permit);
            continue;
        }
        let job_inner = Arc::clone(inner);
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = run_and_record(job_inner, id).await {
                tracing::error!(%id, %err, "job failed");
            }
        });
    }
    Ok(())
}

async fn run_and_record(inner: Arc<Inner>, id: Uuid) -> Result<(), Error> {
    let result = run_job(Arc::clone(&inner), id).await;
    match result {
        Ok(JobOutcome::Completed(path)) => {
            let mut storage = inner.storage.lock().await;
            storage.set_completed(id, &path)?;
            let _ = inner.events.send(Event::Completed(id, path));
        }
        Err(Error::Paused) => {
            let mut storage = inner.storage.lock().await;
            storage.update_state(id, &JobState::Paused)?;
            let _ = inner.events.send(Event::Paused(id));
        }
        Err(Error::Cancelled | Error::JobNotFound(_)) => {}
        Err(err) => {
            let reason = err.to_string();
            let mut storage = inner.storage.lock().await;
            storage.set_failed(id, &reason)?;
            let _ = inner.events.send(Event::Failed(id, reason));
        }
    }
    Ok(())
}

enum JobOutcome {
    Completed(PathBuf),
}

async fn run_job(inner: Arc<Inner>, id: Uuid) -> Result<JobOutcome, Error> {
    let (job, streams, meta) = {
        let storage = inner.storage.lock().await;
        let job = storage.get_job(id)?;
        let (streams, meta) = storage.get_payload(id)?;
        (job, streams, meta)
    };
    let _ = inner.events.send(Event::Started(id));
    let work_dir = inner.cfg.output_dir.join(".parts").join(id.to_string());
    tokio::fs::create_dir_all(&work_dir).await?;

    let outcome = match job.kind.clone() {
        DownloadKind::VideoMuxed { quality, audio } => {
            run_video_job(&inner, &job, &streams, &meta, quality, audio, &work_dir).await?
        }
        DownloadKind::AudioOnly { format, quality } => {
            run_audio_job(&inner, &job, &streams, &meta, format, quality, &work_dir).await?
        }
    };
    Ok(JobOutcome::Completed(outcome))
}

async fn run_video_job(
    inner: &Arc<Inner>,
    job: &Job,
    streams: &StreamMap,
    meta: &VideoMeta,
    quality: VideoQuality,
    audio: AudioFormat,
    work_dir: &Path,
) -> Result<PathBuf, Error> {
    let video = pick_video(streams, quality)?;
    let audio_stream = pick_audio(streams, audio, AudioQuality::Best)?;
    let video_path = work_dir.join("video.part");
    let audio_path = work_dir.join("audio.part");
    let (progress_tx, progress_task) = progress_task(inner, job, total_of([video, audio_stream]));

    let video_req = stream_request(
        inner,
        job.id,
        StreamKind::Video,
        video,
        &video_path,
        &progress_tx,
    );
    let audio_req = stream_request(
        inner,
        job.id,
        StreamKind::Audio,
        audio_stream,
        &audio_path,
        &progress_tx,
    );
    let (video_result, audio_result) =
        tokio::join!(download_stream(video_req), download_stream(audio_req));
    drop(progress_tx);
    progress_task
        .await
        .map_err(|err| Error::InvalidResponse(err.to_string()))??;
    video_result?;
    audio_result?;

    let output = output_path(&inner.cfg.output_dir, &job.title, &job.video_id, "mp4");
    Muxer::new()
        .mux_video_audio(&video_path, &audio_path, &output)
        .await?;
    write_sidecar(&output, meta, streams).await?;
    Ok(output)
}

async fn run_audio_job(
    inner: &Arc<Inner>,
    job: &Job,
    streams: &StreamMap,
    meta: &VideoMeta,
    format: AudioFormat,
    quality: AudioQuality,
    work_dir: &Path,
) -> Result<PathBuf, Error> {
    let audio = pick_audio(streams, format, quality)?;
    let source_path = work_dir.join("audio.part");
    let (progress_tx, progress_task) = progress_task(inner, job, total_of([audio]));
    let req = stream_request(
        inner,
        job.id,
        StreamKind::Audio,
        audio,
        &source_path,
        &progress_tx,
    );
    download_stream(req).await?;
    drop(progress_tx);
    progress_task
        .await
        .map_err(|err| Error::InvalidResponse(err.to_string()))??;

    let ext = match format {
        AudioFormat::M4a => "m4a",
        AudioFormat::Opus => "opus",
    };
    let output = output_path(&inner.cfg.output_dir, &job.title, &job.video_id, ext);
    match (audio.audio_format(), format) {
        (Some(AudioFormat::M4a), AudioFormat::M4a) => {
            tokio::fs::rename(&source_path, &output).await?;
            tag_m4a(&output, meta)?;
        }
        (Some(AudioFormat::Opus), AudioFormat::Opus) => {
            Muxer::new().rewrap_audio(&source_path, &output).await?;
        }
        _ => {
            Muxer::new()
                .transcode_audio(&source_path, &output, format)
                .await?;
        }
    }
    write_sidecar(&output, meta, streams).await?;
    Ok(output)
}

fn stream_request(
    inner: &Arc<Inner>,
    job_id: Uuid,
    stream_kind: StreamKind,
    stream: &Stream,
    dest_path: &Path,
    progress_tx: &mpsc::UnboundedSender<u64>,
) -> DownloadRequest {
    DownloadRequest {
        job_id,
        stream_kind,
        url: stream.url.clone(),
        content_length: stream.content_length,
        dest_path: dest_path.to_path_buf(),
        max_concurrent_segments: inner.cfg.max_concurrent_segments,
        http: inner.http.clone(),
        storage: Arc::clone(&inner.storage),
        progress_tx: progress_tx.clone(),
    }
}

fn progress_task(
    inner: &Arc<Inner>,
    job: &Job,
    total: Option<u64>,
) -> (
    mpsc::UnboundedSender<u64>,
    tokio::task::JoinHandle<Result<(), Error>>,
) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let inner = Arc::clone(inner);
    let id = job.id;
    let mut progress = job.progress.clone();
    progress.bytes_total = total;
    let task = tokio::spawn(async move {
        let mut window = SpeedWindow::default();
        while let Some(delta) = rx.recv().await {
            progress.bytes_downloaded = progress.bytes_downloaded.saturating_add(delta);
            let speed = window.record(delta);
            progress.speed_bps = speed;
            progress.eta_seconds = eta(progress.bytes_downloaded, progress.bytes_total, speed);
            {
                let mut storage = inner.storage.lock().await;
                if storage.get_job(id)?.state == JobState::Paused {
                    return Ok(());
                }
                storage.update_progress(id, &progress)?;
            }
            let _ = inner.events.send(Event::Progress(id, progress.clone()));
        }
        Ok(())
    });
    (tx, task)
}

fn pick_video(streams: &StreamMap, quality: VideoQuality) -> Result<&Stream, Error> {
    streams
        .streams
        .iter()
        .filter(|stream| stream.is_video())
        .filter(|stream| match quality.max_height() {
            Some(max_height) => stream.height.is_some_and(|height| height <= max_height),
            None => true,
        })
        .max_by_key(|stream| {
            (
                stream.height.unwrap_or_default(),
                stream.bitrate.unwrap_or_default(),
            )
        })
        .ok_or_else(|| Error::MissingStream(format!("video <= {quality:?}")))
}

fn pick_audio(
    streams: &StreamMap,
    format: AudioFormat,
    quality: AudioQuality,
) -> Result<&Stream, Error> {
    streams
        .streams
        .iter()
        .filter(|stream| stream.is_audio())
        .filter(|stream| stream.audio_format() == Some(format))
        .filter(|stream| {
            quality == AudioQuality::Best || stream.audio_quality.unwrap_or(quality) <= quality
        })
        .max_by_key(|stream| {
            (
                stream.audio_quality.unwrap_or(AudioQuality::Low),
                stream.bitrate.unwrap_or_default(),
            )
        })
        .ok_or_else(|| Error::MissingStream(format!("{format:?} audio <= {quality:?}")))
}

fn total_of<'a>(streams: impl IntoIterator<Item = &'a Stream>) -> Option<u64> {
    streams.into_iter().try_fold(0_u64, |acc, stream| {
        stream.content_length.map(|len| acc + len)
    })
}

fn output_path(output_dir: &Path, title: &str, video_id: &str, ext: &str) -> PathBuf {
    output_dir.join(format!(
        "{}_{}.{}",
        sanitize_filename(title),
        sanitize_filename(video_id),
        ext
    ))
}

fn sanitize_filename(input: &str) -> String {
    let sanitized = input
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches([' ', '.']);
    if trimmed.is_empty() {
        "video".to_owned()
    } else {
        trimmed.to_owned()
    }
}

async fn write_sidecar(path: &Path, meta: &VideoMeta, streams: &StreamMap) -> Result<(), Error> {
    let sidecar = path.with_extension(format!(
        "{}.json",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("media")
    ));
    let payload = serde_json::json!({
        "meta": meta,
        "streams": streams,
    });
    tokio::fs::write(sidecar, serde_json::to_vec_pretty(&payload)?).await?;
    Ok(())
}

fn tag_m4a(path: &Path, meta: &VideoMeta) -> Result<(), Error> {
    let mut tag = mp4ameta::Tag::default();
    tag.set_title(meta.title.clone());
    if let Some(author) = &meta.author {
        tag.set_artist(author.clone());
    }
    if let Err(err) = tag.write_to_path(path) {
        tracing::warn!(%err, "mp4 tag write failed");
    }
    Ok(())
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn eta(done: u64, total: Option<u64>, speed_bps: f64) -> Option<u64> {
    let total = total?;
    if done >= total || speed_bps <= f64::EPSILON {
        return None;
    }
    Some(((total - done) as f64 / speed_bps).ceil() as u64)
}

#[derive(Default)]
struct SpeedWindow {
    samples: VecDeque<(Instant, u64)>,
}

impl SpeedWindow {
    fn record(&mut self, delta: u64) -> f64 {
        let now = Instant::now();
        self.samples.push_back((now, delta));
        while let Some((instant, _)) = self.samples.front() {
            if now.duration_since(*instant) <= Duration::from_secs(1) {
                break;
            }
            let _ = self.samples.pop_front();
        }
        let bytes = self.samples.iter().map(|(_, sample)| *sample).sum::<u64>();
        #[allow(clippy::cast_precision_loss)]
        {
            bytes as f64
        }
    }
}
