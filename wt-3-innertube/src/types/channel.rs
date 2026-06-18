//! Channel-detail types.

use crate::types::video::VideoSummary;

/// A badge that YouTube attaches to a channel (e.g. "Verified").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelBadge {
    /// Internal style key InnerTube uses (e.g. `BADGE_STYLE_TYPE_VERIFIED`).
    pub style: String,
    /// Display label.
    pub label: String,
}

/// Top-level channel page details.
#[derive(Debug, Clone, Default)]
pub struct ChannelDetails {
    /// Channel ID (`UC...`).
    pub id: String,
    /// Display name.
    pub title: String,
    /// Channel description (the "About" blurb).
    pub description: String,
    /// Subscriber count text exactly as shown on the channel page.
    pub subscriber_count_text: Option<String>,
    /// Total video count text, e.g. `1.2K videos`.
    pub video_count_text: Option<String>,
    /// Country/region the channel lists itself in, if any.
    pub country: Option<String>,
    /// Avatar URL (highest resolution available in the response).
    pub avatar_url: Option<String>,
    /// Banner URL (channel art).
    pub banner_url: Option<String>,
    /// Badges (verified, etc.).
    pub badges: Vec<ChannelBadge>,
    /// First page of "videos" tab entries.
    pub videos: Vec<VideoSummary>,
    /// Continuation token for fetching more videos.
    pub videos_continuation: Option<String>,
}
