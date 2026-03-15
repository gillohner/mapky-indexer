use neo4rs::{query, Query};

/// Retrieve places within a bounding box using Neo4j spatial `point.withinBBox()`.
pub fn get_places_in_viewport(
    min_lat: f64,
    min_lon: f64,
    max_lat: f64,
    max_lon: f64,
    limit: u32,
) -> Query {
    query(
        "MATCH (p:Place)
         WHERE point.withinBBox(
             p.location,
             point({latitude: $min_lat, longitude: $min_lon}),
             point({latitude: $max_lat, longitude: $max_lon})
         )
         RETURN {
             osm_canonical: p.osm_canonical,
             osm_type: p.osm_type,
             osm_id: p.osm_id,
             lat: p.lat,
             lon: p.lon,
             review_count: p.review_count,
             avg_rating: p.avg_rating,
             tag_count: p.tag_count,
             photo_count: p.photo_count,
             indexed_at: p.indexed_at
         } AS place
         LIMIT $limit",
    )
    .param("min_lat", min_lat)
    .param("min_lon", min_lon)
    .param("max_lat", max_lat)
    .param("max_lon", max_lon)
    .param("limit", limit as i64)
}

/// Retrieve a post by author ID and post ID.
pub fn get_post_by_id(author_id: &str, post_id: &str) -> Query {
    query(
        "MATCH (u:User {id: $author_id})-[:AUTHORED]->(p:Post {id: $post_id})
         OPTIONAL MATCH (p)-[:ABOUT]->(place:Place)
         RETURN {
             id: p.id,
             author_id: u.id,
             osm_canonical: place.osm_canonical,
             content: p.content,
             rating: p.rating,
             indexed_at: p.indexed_at
         } AS details",
    )
    .param("author_id", author_id)
    .param("post_id", post_id)
}

/// Retrieve a homeserver by its public key.
pub fn get_homeserver_by_id(id: &str) -> Query {
    query("MATCH (hs:Homeserver {id: $id}) RETURN hs.id AS id").param("id", id)
}

/// Retrieve all known homeserver IDs as a collected list.
pub fn get_all_homeservers() -> Query {
    query("MATCH (hs:Homeserver) WITH collect(hs.id) AS ids RETURN ids")
}

/// Retrieve all user IDs registered on a specific homeserver.
pub fn get_users_for_homeserver(homeserver_id: &str) -> Query {
    query(
        "MATCH (u:User)-[:REGISTERED_ON]->(hs:Homeserver {id: $hs_id})
         WITH collect(u.id) AS ids RETURN ids",
    )
    .param("hs_id", homeserver_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_query_builds() {
        let q = get_places_in_viewport(40.0, -74.0, 41.0, -73.0, 200);
        drop(q);
    }

    #[test]
    fn test_get_homeserver_by_id_query_builds() {
        let q = get_homeserver_by_id("test_hs_pk");
        drop(q);
    }

    #[test]
    fn test_get_all_homeservers_query_builds() {
        let q = get_all_homeservers();
        drop(q);
    }

    #[test]
    fn test_get_post_query_builds() {
        let q = get_post_by_id("user123", "post456");
        drop(q);
    }

    #[test]
    fn test_get_users_for_homeserver_query_builds() {
        let q = get_users_for_homeserver("test_hs_pk");
        drop(q);
    }
}
