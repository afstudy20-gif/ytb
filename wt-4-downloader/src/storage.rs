use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::job::{DownloadKind, Job, JobState, Progress};
use crate::segment::SegmentRecord;
use crate::types::{StreamKind, StreamMap, VideoMeta};
use crate::Error;

pub(crate) struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let conn = Connection::open(path)?;
        let storage = Self { conn };
        storage.init()?;
        Ok(storage)
    }

    pub fn reset_running(&mut self) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE jobs SET state = ?1, updated_at = ?2 WHERE state = ?3",
            params![
                serde_json::to_string(&JobState::Queued)?,
                now_unix(),
                serde_json::to_string(&JobState::Running)?,
            ],
        )?;
        Ok(())
    }

    pub fn insert_job(
        &mut self,
        job: &Job,
        streams: &StreamMap,
        meta: &VideoMeta,
    ) -> Result<(), Error> {
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO jobs
            (id, video_id, title, thumbnail_url, kind_json, state, bytes_done,
             bytes_total, speed_bps, output_path, stream_map_json, meta_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                job.id.to_string(),
                job.video_id,
                job.title,
                job.thumbnail_url,
                serde_json::to_string(&job.kind)?,
                serde_json::to_string(&job.state)?,
                i64::try_from(job.progress.bytes_downloaded)?,
                job.progress.bytes_total.map(i64::try_from).transpose()?,
                job.progress.speed_bps,
                optional_path(&job.output_path),
                serde_json::to_string(streams)?,
                serde_json::to_string(meta)?,
                now,
                now,
            ],
        )?;
        Ok(())
    }

    pub fn try_start(&mut self, id: Uuid) -> Result<bool, Error> {
        let changed = self.conn.execute(
            "UPDATE jobs SET state = ?1, updated_at = ?2 WHERE id = ?3 AND state = ?4",
            params![
                serde_json::to_string(&JobState::Running)?,
                now_unix(),
                id.to_string(),
                serde_json::to_string(&JobState::Queued)?,
            ],
        )?;
        Ok(changed == 1)
    }

    pub fn update_state(&mut self, id: Uuid, state: &JobState) -> Result<bool, Error> {
        self.ensure_job(id)?;
        let changed = self.conn.execute(
            "UPDATE jobs SET state = ?1, updated_at = ?2 WHERE id = ?3",
            params![serde_json::to_string(state)?, now_unix(), id.to_string()],
        )?;
        Ok(changed == 1)
    }

    pub fn update_progress(&mut self, id: Uuid, progress: &Progress) -> Result<(), Error> {
        self.ensure_job(id)?;
        self.conn.execute(
            "UPDATE jobs SET bytes_done = ?1, bytes_total = ?2, speed_bps = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                i64::try_from(progress.bytes_downloaded)?,
                progress.bytes_total.map(i64::try_from).transpose()?,
                progress.speed_bps,
                now_unix(),
                id.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn set_completed(&mut self, id: Uuid, path: &Path) -> Result<(), Error> {
        self.ensure_job(id)?;
        self.conn.execute(
            "UPDATE jobs SET state = ?1, output_path = ?2, updated_at = ?3 WHERE id = ?4",
            params![
                serde_json::to_string(&JobState::Completed)?,
                path.to_string_lossy(),
                now_unix(),
                id.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn set_failed(&mut self, id: Uuid, reason: &str) -> Result<(), Error> {
        if self.get_job(id).is_err() {
            return Ok(());
        }
        self.update_state(id, &JobState::Failed(reason.to_owned()))?;
        Ok(())
    }

    pub fn delete_job(&mut self, id: Uuid) -> Result<(), Error> {
        self.ensure_job(id)?;
        self.conn.execute(
            "DELETE FROM segments WHERE job_id = ?1",
            params![id.to_string()],
        )?;
        self.conn
            .execute("DELETE FROM jobs WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }

    pub fn get_job(&self, id: Uuid) -> Result<Job, Error> {
        self.conn
            .query_row(
                "SELECT * FROM jobs WHERE id = ?1",
                params![id.to_string()],
                row_to_job,
            )
            .optional()?
            .ok_or(Error::JobNotFound(id))
    }

    pub fn get_payload(&self, id: Uuid) -> Result<(StreamMap, VideoMeta), Error> {
        self.ensure_job(id)?;
        self.conn
            .query_row(
                "SELECT stream_map_json, meta_json FROM jobs WHERE id = ?1",
                params![id.to_string()],
                |row| {
                    let streams_json: String = row.get(0)?;
                    let meta_json: String = row.get(1)?;
                    Ok((streams_json, meta_json))
                },
            )
            .map_err(Error::from)
            .and_then(|(streams_json, meta_json)| {
                Ok((
                    serde_json::from_str(&streams_json)?,
                    serde_json::from_str(&meta_json)?,
                ))
            })
    }

    pub fn list_jobs(&self) -> Result<Vec<Job>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM jobs ORDER BY created_at ASC")?;
        let rows = stmt.query_map([], row_to_job)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    pub fn queued_job_ids(&self) -> Result<Vec<Uuid>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM jobs WHERE state = ?1 ORDER BY created_at ASC")?;
        let rows = stmt.query_map(params![serde_json::to_string(&JobState::Queued)?], |row| {
            row.get::<_, String>(0)
        })?;
        rows.map(|row| {
            let id = row.map_err(Error::from)?;
            Uuid::parse_str(&id).map_err(Error::from)
        })
        .collect()
    }

    pub fn ensure_segments(
        &mut self,
        job_id: Uuid,
        stream_kind: StreamKind,
        content_length: u64,
        segment_size: u64,
    ) -> Result<Vec<SegmentRecord>, Error> {
        if self.segments(job_id, stream_kind)?.is_empty() {
            let mut start = 0_u64;
            let mut idx = 0_u32;
            while start < content_length {
                let end = start
                    .saturating_add(segment_size)
                    .saturating_sub(1)
                    .min(content_length.saturating_sub(1));
                self.conn.execute(
                    "INSERT OR IGNORE INTO segments
                    (job_id, stream_kind, segment_idx, range_start, range_end, completed)
                    VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                    params![
                        job_id.to_string(),
                        stream_kind.as_str(),
                        i64::from(idx),
                        i64::try_from(start)?,
                        i64::try_from(end)?,
                    ],
                )?;
                idx = idx.saturating_add(1);
                start = end.saturating_add(1);
            }
        }
        self.segments(job_id, stream_kind)
    }

    pub fn mark_segment_completed(
        &mut self,
        job_id: Uuid,
        stream_kind: StreamKind,
        idx: u32,
    ) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE segments SET completed = 1
             WHERE job_id = ?1 AND stream_kind = ?2 AND segment_idx = ?3",
            params![job_id.to_string(), stream_kind.as_str(), i64::from(idx)],
        )?;
        Ok(())
    }

    fn segments(&self, job_id: Uuid, stream_kind: StreamKind) -> Result<Vec<SegmentRecord>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT segment_idx, range_start, range_end, completed
             FROM segments
             WHERE job_id = ?1 AND stream_kind = ?2
             ORDER BY segment_idx ASC",
        )?;
        let rows = stmt.query_map(params![job_id.to_string(), stream_kind.as_str()], |row| {
            Ok(SegmentRecord {
                idx: u32::try_from(row.get::<_, i64>(0)?).map_err(int_err)?,
                range_start: u64::try_from(row.get::<_, i64>(1)?).map_err(int_err)?,
                range_end: u64::try_from(row.get::<_, i64>(2)?).map_err(int_err)?,
                completed: row.get::<_, i64>(3)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    fn ensure_job(&self, id: Uuid) -> Result<(), Error> {
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM jobs WHERE id = ?1",
                params![id.to_string()],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if exists {
            Ok(())
        } else {
            Err(Error::JobNotFound(id))
        }
    }

    fn init(&self) -> Result<(), Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                video_id TEXT NOT NULL,
                title TEXT NOT NULL,
                thumbnail_url TEXT,
                kind_json TEXT NOT NULL,
                state TEXT NOT NULL,
                bytes_done INTEGER NOT NULL DEFAULT 0,
                bytes_total INTEGER,
                speed_bps REAL NOT NULL DEFAULT 0,
                output_path TEXT,
                stream_map_json TEXT NOT NULL,
                meta_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS segments (
                job_id TEXT NOT NULL,
                stream_kind TEXT NOT NULL,
                segment_idx INTEGER NOT NULL,
                range_start INTEGER NOT NULL,
                range_end INTEGER NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (job_id, stream_kind, segment_idx)
            );",
        )?;
        Ok(())
    }
}

fn row_to_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    let id: String = row.get("id")?;
    let kind_json: String = row.get("kind_json")?;
    let state_json: String = row.get("state")?;
    let bytes_done: i64 = row.get("bytes_done")?;
    let bytes_total: Option<i64> = row.get("bytes_total")?;
    let output_path: Option<String> = row.get("output_path")?;
    Ok(Job {
        id: Uuid::parse_str(&id).map_err(sql_err)?,
        video_id: row.get("video_id")?,
        title: row.get("title")?,
        thumbnail_url: row.get("thumbnail_url")?,
        kind: serde_json::from_str::<DownloadKind>(&kind_json).map_err(sql_err)?,
        state: serde_json::from_str::<JobState>(&state_json).map_err(sql_err)?,
        progress: Progress {
            bytes_downloaded: u64::try_from(bytes_done).map_err(sql_err)?,
            bytes_total: bytes_total
                .map(u64::try_from)
                .transpose()
                .map_err(sql_err)?,
            eta_seconds: None,
            speed_bps: row.get("speed_bps")?,
        },
        output_path: output_path.map(PathBuf::from),
    })
}

fn optional_path(path: &Option<PathBuf>) -> Option<String> {
    path.as_ref()
        .map(|path| path.to_string_lossy().into_owned())
}

fn now_unix() -> i64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    i64::try_from(duration.as_secs()).map_or(i64::MAX, |seconds| seconds)
}

fn sql_err(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn int_err(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Integer, Box::new(error))
}
