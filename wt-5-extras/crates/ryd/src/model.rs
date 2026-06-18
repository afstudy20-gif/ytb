//! Request/response models for the Return YouTube Dislike API.

use serde::{Deserialize, Serialize};

/// Dislike / like payload returned by `GET /votes?videoId=...`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Votes {
    /// YouTube video id this record describes.
    pub id: String,
    /// Unix timestamp (seconds) the record was first created.
    #[serde(rename = "dateCreated")]
    pub date_created: f64,
    /// Like count at the time of the snapshot.
    pub likes: i64,
    /// Dislike count at the time of the snapshot.
    pub dislikes: i64,
    /// 1..=5 average rating derived from the above.
    pub rating: f64,
    /// View count at the time of the snapshot (0 when unknown).
    #[serde(rename = "viewCount")]
    pub view_count: i64,
    /// Whether the upstream record has been marked deleted.
    pub deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn votes_decode_from_camel_case() {
        let raw = r#"{
            "id":"abc",
            "dateCreated":1.0,
            "likes":10,
            "dislikes":2,
            "rating":4.5,
            "viewCount":1000,
            "deleted":false
        }"#;
        let v: Votes = serde_json::from_str(raw).unwrap();
        assert_eq!(v.id, "abc");
        assert_eq!(v.likes, 10);
        assert_eq!(v.dislikes, 2);
        assert_eq!(v.rating, 4.5);
        assert_eq!(v.view_count, 1000);
        assert!(!v.deleted);
    }
}
