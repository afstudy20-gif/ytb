use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures::{stream, StreamExt};
use reqwest::header::{CONTENT_LENGTH, RANGE};
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::storage::Storage;
use crate::{Error, JobState, StreamKind};

const MIN_SEGMENT_SIZE: u64 = 512 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct SegmentRecord {
    pub idx: u32,
    pub range_start: u64,
    pub range_end: u64,
    pub completed: bool,
}

pub(crate) struct DownloadRequest {
    pub job_id: Uuid,
    pub stream_kind: StreamKind,
    pub url: String,
    pub content_length: Option<u64>,
    pub dest_path: PathBuf,
    pub max_concurrent_segments: usize,
    pub http: Client,
    pub storage: Arc<tokio::sync::Mutex<Storage>>,
    pub progress_tx: mpsc::UnboundedSender<u64>,
}

pub(crate) async fn download_stream(req: DownloadRequest) -> Result<u64, Error> {
    let content_length = match req.content_length {
        Some(length) => length,
        None => probe_content_length(&req.http, &req.url).await?,
    };
    if existing_complete(&req.dest_path, content_length).await? {
        return Ok(content_length);
    }

    let segment_size = segment_size(content_length, req.max_concurrent_segments);
    let segments = {
        let mut storage = req.storage.lock().await;
        storage.ensure_segments(req.job_id, req.stream_kind, content_length, segment_size)?
    };

    let missing = reconcile_local_segments(&req, &segments).await?;
    let result = stream::iter(missing)
        .map(|segment| {
            let req = req.clone_light();
            async move { download_segment_with_retry(&req, &segment).await }
        })
        .buffer_unordered(req.max_concurrent_segments)
        .collect::<Vec<_>>()
        .await;

    for item in result {
        item?;
    }
    assemble_segments(&req.dest_path, &segments).await?;
    Ok(content_length)
}

async fn probe_content_length(http: &Client, url: &str) -> Result<u64, Error> {
    let response = http.head(url).send().await?.error_for_status()?;
    let value = response
        .headers()
        .get(CONTENT_LENGTH)
        .ok_or_else(|| Error::InvalidResponse("missing Content-Length".to_owned()))?;
    value
        .to_str()
        .map_err(|err| Error::InvalidResponse(err.to_string()))?
        .parse::<u64>()
        .map_err(|err| Error::InvalidResponse(err.to_string()))
}

async fn reconcile_local_segments(
    req: &DownloadRequest,
    segments: &[SegmentRecord],
) -> Result<Vec<SegmentRecord>, Error> {
    let mut missing = Vec::new();
    for segment in segments {
        if segment.completed {
            continue;
        }
        let path = segment_path(&req.dest_path, segment.idx);
        let expected = segment.range_end.saturating_sub(segment.range_start) + 1;
        if existing_complete(&path, expected).await? {
            mark_complete(req, segment).await?;
            let _ = req.progress_tx.send(expected);
        } else {
            missing.push(segment.clone());
        }
    }
    Ok(missing)
}

async fn download_segment_with_retry(
    req: &LightDownloadRequest,
    segment: &SegmentRecord,
) -> Result<(), Error> {
    let mut delay = Duration::from_millis(200);
    let mut last_error = None;
    for _ in 0..3 {
        match download_segment(req, segment).await {
            Ok(()) => return Ok(()),
            Err(Error::Paused | Error::Cancelled) => return Err(Error::Paused),
            Err(err) => {
                last_error = Some(err);
                tokio::time::sleep(delay).await;
                delay = delay.saturating_mul(2);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| Error::InvalidResponse("segment retry failed".to_owned())))
}

async fn download_segment(
    req: &LightDownloadRequest,
    segment: &SegmentRecord,
) -> Result<(), Error> {
    ensure_active(req).await?;
    let tmp_path = tmp_segment_path(&req.dest_path, segment.idx);
    let final_path = segment_path(&req.dest_path, segment.idx);
    let range = format!("bytes={}-{}", segment.range_start, segment.range_end);
    let mut response = req
        .http
        .get(&req.url)
        .header(RANGE, range)
        .send()
        .await?
        .error_for_status()?;

    let mut file = tokio::fs::File::create(&tmp_path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        let _ = req.progress_tx.send(u64::try_from(chunk.len())?);
    }
    file.flush().await?;
    drop(file);
    tokio::fs::rename(tmp_path, final_path).await?;
    let segment_req = req.to_segment_request();
    mark_complete(&segment_req, segment).await?;
    Ok(())
}

async fn ensure_active(req: &LightDownloadRequest) -> Result<(), Error> {
    let storage = req.storage.lock().await;
    match storage.get_job(req.job_id) {
        Ok(job) => match job.state {
            JobState::Paused => Err(Error::Paused),
            JobState::Failed(_) | JobState::Completed => Err(Error::Cancelled),
            JobState::Queued | JobState::Running => Ok(()),
        },
        Err(Error::JobNotFound(_)) => Err(Error::Cancelled),
        Err(err) => Err(err),
    }
}

async fn mark_complete(req: &DownloadRequest, segment: &SegmentRecord) -> Result<(), Error> {
    let mut storage = req.storage.lock().await;
    storage.mark_segment_completed(req.job_id, req.stream_kind, segment.idx)
}

async fn assemble_segments(dest_path: &Path, segments: &[SegmentRecord]) -> Result<(), Error> {
    let tmp_path = dest_path.with_extension("assembling");
    let mut output = tokio::fs::File::create(&tmp_path).await?;
    for segment in segments {
        let bytes = tokio::fs::read(segment_path(dest_path, segment.idx)).await?;
        output.write_all(&bytes).await?;
    }
    output.flush().await?;
    drop(output);
    tokio::fs::rename(tmp_path, dest_path).await?;
    Ok(())
}

async fn existing_complete(path: &Path, expected_len: u64) -> Result<bool, Error> {
    match tokio::fs::metadata(path).await {
        Ok(metadata) => Ok(metadata.len() == expected_len),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err.into()),
    }
}

fn segment_size(content_length: u64, max_segments: usize) -> u64 {
    let target = content_length / u64::try_from(max_segments).unwrap_or(1);
    target.max(MIN_SEGMENT_SIZE)
}

fn segment_path(dest_path: &Path, idx: u32) -> PathBuf {
    PathBuf::from(format!("{}.{}.part", dest_path.display(), idx))
}

fn tmp_segment_path(dest_path: &Path, idx: u32) -> PathBuf {
    PathBuf::from(format!("{}.{}.tmp", dest_path.display(), idx))
}

#[derive(Clone)]
struct LightDownloadRequest {
    job_id: Uuid,
    stream_kind: StreamKind,
    url: String,
    dest_path: PathBuf,
    http: Client,
    storage: Arc<tokio::sync::Mutex<Storage>>,
    progress_tx: mpsc::UnboundedSender<u64>,
}

impl DownloadRequest {
    fn clone_light(&self) -> LightDownloadRequest {
        LightDownloadRequest {
            job_id: self.job_id,
            stream_kind: self.stream_kind,
            url: self.url.clone(),
            dest_path: self.dest_path.clone(),
            http: self.http.clone(),
            storage: Arc::clone(&self.storage),
            progress_tx: self.progress_tx.clone(),
        }
    }
}

impl LightDownloadRequest {
    fn to_segment_request(&self) -> DownloadRequest {
        DownloadRequest {
            job_id: self.job_id,
            stream_kind: self.stream_kind,
            url: self.url.clone(),
            content_length: None,
            dest_path: self.dest_path.clone(),
            max_concurrent_segments: 1,
            http: self.http.clone(),
            storage: Arc::clone(&self.storage),
            progress_tx: self.progress_tx.clone(),
        }
    }
}
