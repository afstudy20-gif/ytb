//! Search-related types: filters, items, results, and the [`Continuable`]
//! trait.

/// Optional filters passed to [`crate::InnerTube::search`].
///
/// Each field maps directly to a YouTube search parameter that InnerTube
/// recognises through its `sortFilter`/`param` blob. `Default` yields an
/// unfiltered search equivalent to the basic search box.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchFilter {
    /// Top-level kind filter.
    pub kind: SearchKind,
    /// Upload-date window filter.
    pub upload_date: Option<UploadDate>,
    /// Duration filter.
    pub duration: Option<Duration>,
    /// Sort order.
    pub sort_by: SortBy,
}

/// Top-level content-kind filter for a search.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchKind {
    /// All results, no kind filter.
    #[default]
    All,
    /// Videos only.
    Video,
    /// Channels only.
    Channel,
    /// Playlists only.
    Playlist,
}

/// Upload-date window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadDate {
    /// Last hour.
    LastHour,
    /// Today.
    Today,
    /// This week.
    ThisWeek,
    /// This month.
    ThisMonth,
    /// This year.
    ThisYear,
}

/// Video length filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Duration {
    /// Under 4 minutes.
    Short,
    /// 4–20 minutes.
    Medium,
    /// Over 20 minutes.
    Long,
}

/// Result sort order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortBy {
    /// Sort by relevance (default).
    #[default]
    Relevance,
    /// Sort by upload date (newest first).
    UploadDate,
    /// Sort by view count.
    ViewCount,
    /// Sort by rating.
    Rating,
}

/// A single item in a [`SearchResults`] list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchItem {
    /// A video result.
    Video(SearchResultVideo),
    /// A channel result.
    Channel(SearchResultChannel),
    /// A playlist result.
    Playlist(SearchResultPlaylist),
    /// A shelf (e.g. "Learning" or "For you" header). Carries the title and
    /// the contained item IDs.
    Shelf {
        /// Display title of the shelf.
        title: String,
        /// Items surfaced inside the shelf.
        items: Vec<SearchItem>,
    },
}

/// Concrete video result from search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultVideo {
    /// 11-character YouTube video ID.
    pub id: String,
    /// Video title.
    pub title: String,
    /// Channel display name.
    pub author: String,
    /// Channel ID.
    pub channel_id: String,
    /// Description snippet shown under the title.
    pub description: Option<String>,
    /// Length text, `None` for live/upcoming.
    pub length_text: Option<String>,
    /// View count text exactly as InnerTube returns it.
    pub view_count_text: Option<String>,
    /// Upload-date text exactly as InnerTube returns it.
    pub published_text: Option<String>,
    /// Thumbnail URL.
    pub thumbnail_url: Option<String>,
    /// `true` if live at the time of the search.
    pub is_live: bool,
}

/// Channel result from search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultChannel {
    /// Channel ID (`UC...`).
    pub id: String,
    /// Display name.
    pub title: String,
    /// Subscriber count text, e.g. `1.2M subscribers`.
    pub subscriber_count_text: Option<String>,
    /// Channel description snippet.
    pub description: Option<String>,
    /// Avatar URL.
    pub avatar_url: Option<String>,
}

/// Playlist result from search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultPlaylist {
    /// Playlist ID (`PL...`).
    pub id: String,
    /// Playlist title.
    pub title: String,
    /// First listed video thumbnail (used by InnerTube as the cover).
    pub thumbnail_url: Option<String>,
    /// Channel display name of the playlist owner.
    pub author: Option<String>,
    /// Number of videos in the playlist, when InnerTube exposes it.
    pub video_count: Option<u64>,
}

/// A page of search results.
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    /// Ordered results for this page.
    pub items: Vec<SearchItem>,
    /// Continuation token to fetch the next page via
    /// [`crate::InnerTube::continuation`], when available.
    pub continuation: Option<String>,
    /// Estimated total result count, when InnerTube reports one.
    pub estimated_results: Option<u64>,
}

/// Trait implemented by response types that may carry a continuation token.
///
/// Used by [`crate::InnerTube::continuation`] to return a typed next page
/// without the caller having to know which endpoint the token came from.
/// Currently implemented for [`SearchResults`].
pub trait Continuable {
    /// Extract the continuation token, if any, that should be used to fetch
    /// the next page.
    fn continuation_token(&self) -> Option<&str>;
}

impl Continuable for SearchResults {
    fn continuation_token(&self) -> Option<&str> {
        self.continuation.as_deref()
    }
}
