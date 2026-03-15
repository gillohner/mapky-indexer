use crate::types::DynError;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::path::Path;
use tokio::fs;

pub trait ConfigLoader<T>
where
    T: DeserializeOwned + Send + Sync + Debug,
{
    /// Parses the struct from a TOML string
    fn try_from_str(value: &str) -> Result<T, DynError> {
        let config: T = toml::from_str(value)?;
        Ok(config)
    }

    /// Loads the struct from a TOML file
    fn load(path: impl AsRef<Path> + Send) -> impl std::future::Future<Output = Result<T, DynError>> + Send {
        async move {
            let config_file_path = path.as_ref();
            let s = fs::read_to_string(config_file_path)
                .await
                .map_err(|e| format!("Failed to read config file {:?}: {}", config_file_path, e))?;

            let config = Self::try_from_str(&s)
                .map_err(|e| format!("Failed to parse config file {:?}: {}", config_file_path, e))?;

            Ok(config)
        }
    }
}
