use clap::Parser;
use mapky_common::types::DynError;
use mapky_webapi::MapkyApi;
use mapky_watcher::MapkyWatcherBuilder;
use mapky_common::WatcherConfig;
use mapkyd::cli::{Cli, MapkyCommands};
use mapkyd::DaemonLauncher;

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let cli = Cli::parse();
    let command = Cli::receive_command(cli);

    match command {
        MapkyCommands::Api { config_dir } => {
            MapkyApi::start_from_daemon(config_dir, None).await?;
        }
        MapkyCommands::Watcher { config_dir } => {
            let config = mapky_common::DaemonConfig::read_or_create_config_file(config_dir).await?;
            let watcher_config = WatcherConfig::from(config.clone());
            let builder = MapkyWatcherBuilder::with_stack(watcher_config, &config.stack);
            builder.start(None).await?;
        }
        MapkyCommands::Run { config_dir } => {
            DaemonLauncher::start(config_dir, None).await?;
        }
        MapkyCommands::ResetDb {
            config_dir,
            neo4j_only,
            pg_only,
        } => {
            mapkyd::reset::reset_databases(config_dir, neo4j_only, pg_only).await?;
        }
    }

    Ok(())
}
