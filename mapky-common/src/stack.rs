use crate::config::{Level, StackConfig};
use crate::db::{Neo4jConnector, PgConnector};
use crate::types::DynError;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, Registry};

pub struct StackManager;

impl StackManager {
    pub async fn setup(name: &str, config: &StackConfig) -> Result<(), DynError> {
        Self::setup_logging(name, config.log_level);

        Neo4jConnector::init(&config.db.neo4j).await?;
        PgConnector::init(&config.db.postgres).await?;

        Ok(())
    }

    fn setup_logging(name: &str, log_level: Level) {
        let _ = tracing_log::LogTracer::init();

        let env_filter = EnvFilter::new(log_level.as_str());
        let fmt_layer = fmt::layer().compact().with_line_number(true);
        let subscriber = Registry::default().with(env_filter).with(fmt_layer);

        if tracing::subscriber::set_global_default(subscriber).is_ok() {
            info!("{name} logging initialized at level {}", log_level.as_str());
        }
    }
}
