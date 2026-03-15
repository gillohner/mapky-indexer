use neo4rs::{query, Query};

/// Delete a post and its relationships from the graph.
pub fn delete_post(author_id: &str, post_id: &str) -> Query {
    query(
        "MATCH (u:User {id: $author_id})-[:AUTHORED]->(p:Post {id: $post_id})
         DETACH DELETE p
         RETURN u IS NOT NULL AS flag",
    )
    .param("author_id", author_id)
    .param("post_id", post_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_post_query_builds() {
        let q = delete_post("user123", "post456");
        drop(q);
    }
}
