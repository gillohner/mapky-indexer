use serde::{Deserialize, Serialize};
use std::error::Error;
use utoipa::{IntoParams, ToSchema};

pub type DynError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Pagination {
    pub skip: u32,
    pub limit: u32,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            skip: 0,
            limit: 20,
        }
    }
}

/// Bounding box query for spatial viewport queries.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ViewportQuery {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
    #[serde(default = "default_zoom")]
    pub zoom: u8,
    #[serde(default = "default_viewport_limit")]
    pub limit: u32,
}

fn default_zoom() -> u8 {
    14
}

fn default_viewport_limit() -> u32 {
    200
}

impl ViewportQuery {
    pub fn validate(&self) -> Result<(), String> {
        if self.min_lat < -90.0 || self.min_lat > 90.0 {
            return Err(format!("min_lat out of range: {}", self.min_lat));
        }
        if self.max_lat < -90.0 || self.max_lat > 90.0 {
            return Err(format!("max_lat out of range: {}", self.max_lat));
        }
        if self.min_lon < -180.0 || self.min_lon > 180.0 {
            return Err(format!("min_lon out of range: {}", self.min_lon));
        }
        if self.max_lon < -180.0 || self.max_lon > 180.0 {
            return Err(format!("max_lon out of range: {}", self.max_lon));
        }
        if self.min_lat >= self.max_lat {
            return Err("min_lat must be less than max_lat".into());
        }
        if self.min_lon >= self.max_lon {
            return Err("min_lon must be less than max_lon".into());
        }
        if self.limit == 0 || self.limit > 1000 {
            return Err(format!("limit must be 1..1000, got {}", self.limit));
        }
        Ok(())
    }
}

/// Query parameters for listing posts on a place.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PlacePostsQuery {
    /// Number of results to skip (default 0).
    #[serde(default)]
    pub skip: Option<u32>,
    /// Maximum results to return (default 20, max 100).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Filter: "reviews" for posts with rating only. Omit for all posts.
    #[serde(default)]
    pub kind: Option<String>,
}

impl PlacePostsQuery {
    pub fn skip(&self) -> i64 {
        self.skip.unwrap_or(0) as i64
    }

    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).min(100) as i64
    }

    pub fn reviews_only(&self) -> bool {
        self.kind.as_deref() == Some("reviews")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_query_valid() {
        let q = ViewportQuery {
            min_lat: 40.0,
            min_lon: -74.0,
            max_lat: 41.0,
            max_lon: -73.0,
            zoom: 14,
            limit: 200,
        };
        assert!(q.validate().is_ok());
    }

    #[test]
    fn test_viewport_query_invalid_lat() {
        let q = ViewportQuery {
            min_lat: 91.0,
            min_lon: -74.0,
            max_lat: 41.0,
            max_lon: -73.0,
            zoom: 14,
            limit: 200,
        };
        assert!(q.validate().is_err());
    }

    #[test]
    fn test_viewport_query_inverted_lat() {
        let q = ViewportQuery {
            min_lat: 41.0,
            min_lon: -74.0,
            max_lat: 40.0,
            max_lon: -73.0,
            zoom: 14,
            limit: 200,
        };
        assert!(q.validate().unwrap_err().contains("min_lat must be less"));
    }

    #[test]
    fn test_viewport_query_zero_limit() {
        let q = ViewportQuery {
            min_lat: 40.0,
            min_lon: -74.0,
            max_lat: 41.0,
            max_lon: -73.0,
            zoom: 14,
            limit: 0,
        };
        assert!(q.validate().is_err());
    }

    #[test]
    fn test_pagination_default() {
        let p = Pagination::default();
        assert_eq!(p.skip, 0);
        assert_eq!(p.limit, 20);
    }

    #[test]
    fn test_place_posts_query_defaults() {
        let q = PlacePostsQuery {
            skip: None,
            limit: None,
            kind: None,
        };
        assert_eq!(q.skip(), 0);
        assert_eq!(q.limit(), 20);
        assert!(!q.reviews_only());
    }

    #[test]
    fn test_place_posts_query_reviews_only() {
        let q = PlacePostsQuery {
            skip: Some(5),
            limit: Some(200),
            kind: Some("reviews".to_string()),
        };
        assert_eq!(q.skip(), 5);
        assert_eq!(q.limit(), 100); // capped at 100
        assert!(q.reviews_only());
    }
}
