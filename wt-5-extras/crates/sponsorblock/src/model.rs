//! Request/response models for the `SponsorBlock` API.

use serde::{Deserialize, Serialize};

/// `SponsorBlock` segment categories.
///
/// See <https://wiki.sponsor.ajay.app/w/API_Docs#Categories>.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Paid promotion integrated into the content.
    Sponsor,
    /// Unpaid self-promotion or merch plugs.
    SelfPromo,
    /// "Don't forget to like and subscribe" type interludes.
    Interaction,
    /// Intro animation / title sequence.
    Intro,
    /// Credits / endcards.
    Outro,
    /// Preview / recap of an earlier part of the video.
    Preview,
    /// Off-topic music segment in a non-music video.
    MusicOfftopic,
    /// Tangential filler that adds nothing.
    Filler,
}

impl Category {
    /// Wire string `SponsorBlock` expects in `categories`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Sponsor => "sponsor",
            Category::SelfPromo => "selfpromo",
            Category::Interaction => "interaction",
            Category::Intro => "intro",
            Category::Outro => "outro",
            Category::Preview => "preview",
            Category::MusicOfftopic => "music_offtopic",
            Category::Filler => "filler",
        }
    }
}

/// Action type returned on each segment (only `skip` is currently produced
/// by the upstream API but the field is reserved for mute/preview).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Skip,
    Mute,
    #[serde(other)]
    Other,
}

/// A single sponsored segment returned by the API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Segment {
    /// Public UUID identifying this segment (used for voting).
    #[serde(rename = "UUID")]
    pub uuid: String,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
    /// Category string as returned by the server.
    pub category: String,
    /// Action type (`skip`, `mute`, …).
    #[serde(rename = "actionType")]
    pub action_type: String,
    /// Total duration of the source video at the time of submission.
    #[serde(rename = "videoDuration", default)]
    pub video_duration: Option<f64>,
    /// Whether the segment is locked against further votes.
    #[serde(default)]
    pub locked: i32,
    /// Net score: upvotes minus downvotes.
    #[serde(default)]
    pub votes: i32,
}

/// Input for [`crate::Client::submit`].
#[derive(Debug, Clone, Serialize)]
pub struct NewSegment {
    /// Segment start in seconds.
    pub start: f64,
    /// Segment end in seconds.
    pub end: f64,
    /// Category string (use a [`Category::as_str`] value).
    pub category: String,
    /// Action type — almost always `"skip"`.
    #[serde(rename = "userAgent")]
    pub user_agent: String,
}

/// Vote type for [`crate::Client::vote`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    /// Upvote — segment is correct.
    Up,
    /// Downvote — segment is wrong / mistimed.
    Down,
    /// Mark as the segment should not be skipped (no-op vote, value 0).
    Skip,
}

impl Vote {
    pub(crate) const fn as_i64(self) -> i64 {
        match self {
            Vote::Up => 1,
            Vote::Down => 0,
            Vote::Skip => 20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_serializes_to_snake_case() {
        let v = serde_json::to_string(&Category::MusicOfftopic).unwrap();
        assert_eq!(v, "\"music_offtopic\"");
    }

    #[test]
    fn segment_decodes_from_fixture_payload() {
        let raw = r#"[
            {"UUID":"abc","start":1.0,"end":2.5,"category":"sponsor","actionType":"skip","videoDuration":120.0,"locked":1,"votes":12}
        ]"#;
        let parsed: Vec<Segment> = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.len(), 1);
        let s = &parsed[0];
        assert_eq!(s.uuid, "abc");
        approx_eq(s.start, 1.0);
        approx_eq(s.end, 2.5);
        assert_eq!(s.video_duration.map(|f| f as i64), Some(120));
        assert_eq!(s.locked, 1);
        assert_eq!(s.votes, 12);
    }

    /// Compare two f64s with a tight tolerance, to satisfy `clippy::float_cmp`.
    fn approx_eq(a: f64, b: f64) {
        assert!(
            (a - b).abs() < 1e-9,
            "floats differ: {a} vs {b}",
        );
    }
}
