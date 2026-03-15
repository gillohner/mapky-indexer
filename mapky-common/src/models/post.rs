use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use mapky_app_specs::MapkyAppPost;

/// Indexed representation of a post (review, comment, question).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PostDetails {
    pub id: String,
    pub author_id: String,
    pub osm_canonical: String,
    pub content: Option<String>,
    pub rating: Option<u8>,
    pub indexed_at: i64,
}

impl PostDetails {
    /// Convert a MapkyAppPost from a homeserver event into a PostDetails.
    pub fn from_homeserver(post: &MapkyAppPost, author_id: &str, post_id: &str) -> Self {
        Self {
            id: post_id.to_string(),
            author_id: author_id.to_string(),
            osm_canonical: post.place.canonical(),
            content: post.content.clone(),
            rating: post.rating,
            indexed_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapky_app_specs::{OsmElementType, OsmRef};

    #[test]
    fn test_post_details_from_homeserver() {
        let post = MapkyAppPost::new(
            OsmRef::new(OsmElementType::Node, 123456),
            Some("Great place!".to_string()),
            Some(8),
            None,
            None,
        );
        let details = PostDetails::from_homeserver(&post, "user_pk", "0000000000001");
        assert_eq!(details.id, "0000000000001");
        assert_eq!(details.author_id, "user_pk");
        assert_eq!(details.osm_canonical, "node/123456");
        assert_eq!(details.content.as_deref(), Some("Great place!"));
        assert_eq!(details.rating, Some(8));
    }

    #[test]
    fn test_post_details_serde_roundtrip() {
        let details = PostDetails {
            id: "001".to_string(),
            author_id: "user1".to_string(),
            osm_canonical: "node/42".to_string(),
            content: Some("Test".to_string()),
            rating: Some(7),
            indexed_at: 1000,
        };
        let json = serde_json::to_string(&details).unwrap();
        let parsed: PostDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, details.id);
        assert_eq!(parsed.rating, details.rating);
    }
}
