use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Minimal indexed user representation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDetails {
    pub id: String,
    pub name: String,
    pub indexed_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_details_serde_roundtrip() {
        let user = UserDetails {
            id: "user_pk_123".to_string(),
            name: "Test User".to_string(),
            indexed_at: 1000,
        };
        let json = serde_json::to_string(&user).unwrap();
        let parsed: UserDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, user.id);
        assert_eq!(parsed.name, user.name);
    }
}
