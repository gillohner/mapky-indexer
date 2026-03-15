use serde::{Deserialize, Serialize};

pub const NEO4J_URI: &str = "bolt://localhost:7687";
pub const NEO4J_USER: &str = "neo4j";
pub const NEO4J_PASS: &str = "12345678";
pub const POSTGRES_URL: &str = "postgres://mapky:mapky@localhost:5432/mapky";
pub const POSTGRES_MAX_CONNECTIONS: u32 = 10;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Neo4JConfig {
    pub uri: String,
    #[serde(default = "default_neo4j_user")]
    pub user: String,
    pub password: String,
}

fn default_neo4j_user() -> String {
    String::from("neo4j")
}

impl Default for Neo4JConfig {
    fn default() -> Self {
        Self {
            uri: String::from(NEO4J_URI),
            user: String::from(NEO4J_USER),
            password: String::from(NEO4J_PASS),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct PostgresConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_max_connections() -> u32 {
    POSTGRES_MAX_CONNECTIONS
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            url: String::from(POSTGRES_URL),
            max_connections: POSTGRES_MAX_CONNECTIONS,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct DatabaseConfig {
    pub neo4j: Neo4JConfig,
    pub postgres: PostgresConfig,
}
