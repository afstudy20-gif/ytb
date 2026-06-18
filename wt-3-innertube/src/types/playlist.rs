//! Playlist-detail types.

/// One entry in a [`PlaylistDetails`] listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistVideo {
    /// 11-character YouTube video ID.
    pub id: String,
    /// Video title.
    pub title: String,
    /// Channel display name.
    pub author: String,
    /// Channel ID.
    pub channel_id: String,
    /// Length text (`M:SS` or `H:MM:SS`).
    pub length_text: Option<String>,
    /// Length in seconds, parsed where possible.
    pub length_seconds: Option<u64>,
    /// Thumbnail URL.
    pub thumbnail_url: Option<String>,
    /// 1-indexed position of this video in the playlist.
    pub index: Option<u64>,
}

/// Top-level playlist page details.
#[derive(Debug, Clone, Default)]
pub struct PlaylistDetails {
    /// Playlist ID (`PL...` or `RD...` for radios).
    pub id: String,
    /// Playlist title.
    pub title: String,
    /// Channel display name of the playlist owner.
    pub author: Option<String>,
    /// Channel ID of the playlist owner.
    pub channel_id: Option<String>,
    /// Total number of videos, when known.
    pub video_count: Option<u64>,
    /// View count text for the playlist, when reported.
    pub view_count_text: Option<String>,
    /// First page of playlist entries.
    pub videos: Vec<PlaylistVideo>,
    /// Continuation token for fetching more entries.
    pub continuation: Option<String>,
}
