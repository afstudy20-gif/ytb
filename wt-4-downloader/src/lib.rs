#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]

mod config;
mod error;
mod events;
mod job;
mod queue;
mod segment;
mod storage;
mod types;

pub mod mux;

use std::path::PathBuf;
use std::sync::Arc;

use reqwest::Client;
use tokio::sync::{broadcast, Notify};
use uuid::Uuid;

pub use config::Config;
pub use error::Error;
pub use events::Event;
pub use job::{DownloadKind, Job, JobState, Progress};
pub use types::{
    AudioFormat, AudioQuality, Stream, StreamKind, StreamMap, VideoMeta, VideoQuality,
};

use crate::storage::Storage;

#[derive(Clone)]
pub struct Downloader {
    inner: Arc<Inner>,
}

pub(crate) struct Inner {
    pub cfg: Config,
    pub http: Client,
    pub storage: Arc<tokio::sync::Mutex<Storage>>,
    pub events: broadcast::Sender<Event>,
    pub notify: Notify,
}

impl Downloader {
    /// Opens the persistent queue, resets interrupted jobs, and starts workers
    /// when called inside a Tokio runtime.
    pub fn new(cfg: Config) -> Result<Self, Error> {
        cfg.validate()?;
        std::fs::create_dir_all(&cfg.output_dir)?;
        let db_path = cfg.output_dir.join(".downloader.db");
        let mut storage = Storage::open(db_path)?;
        storage.reset_running()?;

        let (events, _) = broadcast::channel(512);
        let inner = Arc::new(Inner {
            cfg,
            http: Client::new(),
            storage: Arc::new(tokio::sync::Mutex::new(storage)),
            events,
            notify: Notify::new(),
        });

        queue::start_scheduler(Arc::clone(&inner));
        Ok(Self { inner })
    }

    /// Enqueues a job using caller-provided stream data. The `StreamMap` and
    /// `VideoMeta` stubs intentionally mirror the fields commonly consumed from
    /// Innertube adaptive format responses; the integration crate can map its
    /// richer model into these structs without coupling this crate to Innertube.
    pub async fn enqueue(
        &self,
        video_id: &str,
        kind: DownloadKind,
        streams: StreamMap,
        meta: VideoMeta,
    ) -> Result<Uuid, Error> {
        let id = Uuid::new_v4();
        let job = Job::new(
            id,
            video_id.to_owned(),
            meta.title.clone(),
            meta.thumbnail_url.clone(),
            kind,
        );
        {
            let mut storage = self.inner.storage.lock().await;
            storage.insert_job(&job, &streams, &meta)?;
        }
        self.emit(Event::Queued(id));
        self.inner.notify.notify_waiters();
        Ok(id)
    }

    pub async fn pause(&self, id: Uuid) -> Result<(), Error> {
        let changed = {
            let mut storage = self.inner.storage.lock().await;
            storage.update_state(id, &JobState::Paused)?
        };
        if changed {
            self.emit(Event::Paused(id));
            self.inner.notify.notify_waiters();
        }
        Ok(())
    }

    pub async fn resume(&self, id: Uuid) -> Result<(), Error> {
        {
            let mut storage = self.inner.storage.lock().await;
            storage.update_state(id, &JobState::Queued)?;
        }
        self.emit(Event::Resumed(id));
        self.inner.notify.notify_waiters();
        Ok(())
    }

    pub async fn cancel(&self, id: Uuid) -> Result<(), Error> {
        self.delete(id, false).await
    }

    pub async fn delete(&self, id: Uuid, delete_file: bool) -> Result<(), Error> {
        let output_path = {
            let mut storage = self.inner.storage.lock().await;
            let job = storage.get_job(id)?;
            storage.delete_job(id)?;
            job.output_path
        };
        if delete_file {
            if let Some(path) = output_path {
                remove_if_exists(path).await?;
            }
        }
        self.inner.notify.notify_waiters();
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<Job>, Error> {
        let storage = self.inner.storage.lock().await;
        storage.list_jobs()
    }

    #[must_use]
    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }

    fn emit(&self, event: Event) {
        let _ = self.inner.events.send(event);
    }
}

async fn remove_if_exists(path: PathBuf) -> Result<(), Error> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}
