use std::path::PathBuf;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("job not found: {0}")]
    JobNotFound(uuid::Uuid),
    #[error("missing stream: {0}")]
    MissingStream(String),
    #[error("invalid HTTP response: {0}")]
    InvalidResponse(String),
    #[error("job paused")]
    Paused,
    #[error("job cancelled")]
    Cancelled,
    #[error("ffmpeg failed: {0}")]
    Ffmpeg(String),
    #[error("ffmpeg binary not found at {0}")]
    FfmpegNotFound(PathBuf),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("database error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("integer conversion failed: {0}")]
    IntConversion(#[from] std::num::TryFromIntError),
    #[error("tokio runtime is required for background workers")]
    RuntimeRequired,
}
