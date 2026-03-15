use crate::models::homeserver::HomeserverDetails;
use crate::models::place::PlaceDetails;
use crate::models::post::PostDetails;
use crate::models::user::UserDetails;
use neo4rs::{query, Query};

/// Create or update a user node.
pub fn create_user(user: &UserDetails) -> Query {
    query(
        "MERGE (u:User {id: $id})
         SET u.name = $name, u.indexed_at = $indexed_at",
    )
    .param("id", user.id.clone())
    .param("name", user.name.clone())
    .param("indexed_at", user.indexed_at)
}

/// MERGE a Place node by OSM canonical reference.
/// Uses `point({latitude, longitude})` for the spatial index.
/// Returns `flag` = true if updated (already existed), false if created.
pub fn create_place(place: &PlaceDetails) -> Query {
    query(
        "MERGE (p:Place {osm_canonical: $osm_canonical})
         ON CREATE SET
             p.osm_type = $osm_type,
             p.osm_id = $osm_id,
             p.location = point({latitude: $lat, longitude: $lon}),
             p.lat = $lat,
             p.lon = $lon,
             p.review_count = 0,
             p.avg_rating = 0.0,
             p.tag_count = 0,
             p.photo_count = 0,
             p.indexed_at = $indexed_at
         ON MATCH SET
             p.location = point({latitude: $lat, longitude: $lon}),
             p.lat = $lat,
             p.lon = $lon
         RETURN p.indexed_at <> $indexed_at AS flag",
    )
    .param("osm_canonical", place.osm_canonical.clone())
    .param("osm_type", place.osm_type.clone())
    .param("osm_id", place.osm_id)
    .param("lat", place.lat)
    .param("lon", place.lon)
    .param("indexed_at", place.indexed_at)
}

/// Create a post node with AUTHORED and ABOUT relationships.
/// Returns `flag` = true if updated, false if created.
pub fn create_post(post: &PostDetails) -> Query {
    query(
        "MATCH (author:User {id: $author_id})
         MATCH (place:Place {osm_canonical: $osm_canonical})
         OPTIONAL MATCH (author)-[:AUTHORED]->(existing:Post {id: $post_id})
         MERGE (author)-[:AUTHORED]->(p:Post {id: $post_id})
         MERGE (p)-[:ABOUT]->(place)
         ON CREATE SET
             p.indexed_at = $indexed_at
         SET p.content = $content,
             p.rating = $rating
         RETURN existing IS NOT NULL AS flag",
    )
    .param("author_id", post.author_id.clone())
    .param("post_id", post.id.clone())
    .param("osm_canonical", post.osm_canonical.clone())
    .param("content", post.content.clone().unwrap_or_default())
    .param("rating", post.rating.map(|r| r as i64).unwrap_or(0))
    .param("indexed_at", post.indexed_at)
}

/// MERGE a Homeserver node by its public key.
pub fn create_homeserver(hs: &HomeserverDetails) -> Query {
    query(
        "MERGE (hs:Homeserver {id: $id})
         SET hs.indexed_at = $indexed_at",
    )
    .param("id", hs.id.clone())
    .param("indexed_at", hs.indexed_at)
}

/// Link a user to their homeserver via REGISTERED_ON relationship.
/// Creates the User node if it doesn't exist.
pub fn link_user_to_homeserver(user_id: &str, homeserver_id: &str) -> Query {
    query(
        "MERGE (u:User {id: $user_id})
         WITH u
         MATCH (hs:Homeserver {id: $hs_id})
         MERGE (u)-[:REGISTERED_ON]->(hs)",
    )
    .param("user_id", user_id)
    .param("hs_id", homeserver_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user_query_builds() {
        let user = UserDetails {
            id: "test_user_pk".to_string(),
            name: "Test User".to_string(),
            indexed_at: 1000,
        };
        let q = create_user(&user);
        // Verify the query was constructed without panic
        drop(q);
    }

    #[test]
    fn test_create_place_query_builds() {
        let place = PlaceDetails {
            osm_canonical: "node/123456".to_string(),
            osm_type: "node".to_string(),
            osm_id: 123456,
            lat: 40.7128,
            lon: -74.006,
            review_count: 0,
            avg_rating: 0.0,
            tag_count: 0,
            photo_count: 0,
            indexed_at: 1000,
        };
        let q = create_place(&place);
        drop(q);
    }

    #[test]
    fn test_create_homeserver_query_builds() {
        let hs = HomeserverDetails {
            id: "test_hs_pk".to_string(),
            indexed_at: 1000,
        };
        let q = create_homeserver(&hs);
        drop(q);
    }

    #[test]
    fn test_create_post_query_builds() {
        let post = PostDetails {
            id: "0000000000001".to_string(),
            author_id: "test_user_pk".to_string(),
            osm_canonical: "node/123456".to_string(),
            content: Some("Great place!".to_string()),
            rating: Some(8),
            indexed_at: 1000,
        };
        let q = create_post(&post);
        drop(q);
    }

    #[test]
    fn test_link_user_to_homeserver_query_builds() {
        let q = link_user_to_homeserver("user_pk", "hs_pk");
        drop(q);
    }
}
