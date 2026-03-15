use clap::{Parser, Subcommand};
use std::path::PathBuf;

fn default_config_dir() -> PathBuf {
    PathBuf::from("./config/local")
}

#[derive(Parser, Debug)]
#[command(name = "mapkyd")]
#[command(about = "MapKy Indexer Daemon", long_about = None)]
pub struct Cli {
    /// Directory containing `config.toml`
    #[arg(short, long, default_value_os_t = default_config_dir())]
    pub config_dir: PathBuf,

    #[command(subcommand)]
    pub command: Option<MapkyCommands>,
}

impl Cli {
    pub fn receive_command(cli: Cli) -> MapkyCommands {
        match cli.command {
            None => MapkyCommands::Run {
                config_dir: cli.config_dir,
            },
            Some(command) => command,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum MapkyCommands {
    /// Run the REST API service only
    Api {
        #[arg(short, long, default_value_os_t = default_config_dir())]
        config_dir: PathBuf,
    },

    /// Run the event watcher only
    Watcher {
        #[arg(short, long, default_value_os_t = default_config_dir())]
        config_dir: PathBuf,
    },

    /// Run both API and Watcher (default when no subcommand is given)
    #[command(hide = true)]
    Run {
        #[arg(short, long, default_value_os_t = default_config_dir())]
        config_dir: PathBuf,
    },

    /// Wipe database contents and recreate schema
    ResetDb {
        #[arg(short, long, default_value_os_t = default_config_dir())]
        config_dir: PathBuf,
        /// Only reset Neo4j
        #[arg(long)]
        neo4j_only: bool,
        /// Only reset PostgreSQL
        #[arg(long)]
        pg_only: bool,
    },
}
