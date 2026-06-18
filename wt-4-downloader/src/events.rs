use std::path::PathBuf;

use uuid::Uuid;

use crate::Progress;

#[derive(Clone, Debug)]
pub enum Event {
    Queued(Uuid),
    Started(Uuid),
    Progress(Uuid, Progress),
    Completed(Uuid, PathBuf),
    Failed(Uuid, String),
    Paused(Uuid),
    Resumed(Uuid),
}
