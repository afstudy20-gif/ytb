//! Public data types returned by [`crate::InnerTube`].
//!
//! The submodules group types by the InnerTube endpoint they originate from.
//! All types implement `Clone`, `Debug`, and `serde::Serialize` (hand-rolled
//! `Debug` where InnerTube would otherwise leak huge JSON blobs) so they can
//! be passed across threads and surfaced to logs safely.

pub mod channel;
pub mod playlist;
pub mod search;
pub mod stream;
pub mod video;

pub use channel::{ChannelBadge, ChannelDetails};
pub use playlist::{PlaylistDetails, PlaylistVideo};
pub use search::{
    Continuable, Duration, SearchFilter, SearchItem, SearchKind, SearchResults,
    SearchResultChannel, SearchResultPlaylist, SearchResultVideo, SortBy, UploadDate,
};
pub use stream::{Stream, StreamMap};
pub use video::{Caption, VideoDetails, VideoSummary};

/// Re-export a handful of primitives reused across modules so callers do not
/// need to reach into `serde_json` directly.
pub use serde_json::Value as JsonValue;
