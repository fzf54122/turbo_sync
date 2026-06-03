use std::{
    env, fs,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};

const CONFIG_ENV: &str = "TURBOSYNC_CONFIG";
const DB_ENV: &str = "TURBOSYNC_DB";
const DEFAULT_AGENT_ADDR: &str = "127.0.0.1:38745";
const DEFAULT_TRANSPORT_ADDR: &str = "0.0.0.0:38746";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
    pub db_file: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct AppConfig {
    pub node_id: String,
    pub agent_addr: String,
    pub transport_addr: String,
    pub db_path: PathBuf,
}

impl AppConfig {
    #[must_use]
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            agent_addr: DEFAULT_AGENT_ADDR.to_owned(),
            transport_addr: DEFAULT_TRANSPORT_ADDR.to_owned(),
            db_path,
        }
    }
}

pub fn resolve_paths() -> Result<ConfigPaths> {
    let project_dirs =
        ProjectDirs::from("dev", "TurboSync", "TurboSync").ok_or(Error::MissingProjectDirs)?;
    let config_file =
        env_path(CONFIG_ENV).unwrap_or_else(|| project_dirs.config_dir().join("config.toml"));
    let data_dir = project_dirs.data_dir().to_path_buf();
    let db_file = env_path(DB_ENV).unwrap_or_else(|| data_dir.join("turbosync.db"));

    Ok(ConfigPaths {
        config_file,
        data_dir,
        db_file,
    })
}

pub fn init_config() -> Result<AppConfig> {
    init_config_at(&resolve_paths()?)
}

pub fn init_config_at(paths: &ConfigPaths) -> Result<AppConfig> {
    create_parent_dir(&paths.config_file)?;
    fs::create_dir_all(&paths.data_dir).map_err(|source| Error::CreateDir {
        path: paths.data_dir.clone(),
        source,
    })?;
    create_parent_dir(&paths.db_file)?;

    if paths.config_file.exists() {
        return load_config_at(paths);
    }

    let config = AppConfig::new(paths.db_file.clone());
    let encoded = toml::to_string_pretty(&config)?;
    fs::write(&paths.config_file, encoded).map_err(|source| Error::WriteConfig {
        path: paths.config_file.clone(),
        source,
    })?;

    Ok(config)
}

pub fn load_config_at(paths: &ConfigPaths) -> Result<AppConfig> {
    let raw = fs::read_to_string(&paths.config_file).map_err(|source| Error::ReadConfig {
        path: paths.config_file.clone(),
        source,
    })?;
    toml::from_str(&raw).map_err(|source| Error::ParseConfig {
        path: paths.config_file.clone(),
        source,
    })
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

fn create_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_config_writes_and_reuses_existing_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let paths = ConfigPaths {
            config_file: temp_dir.path().join("config/config.toml"),
            data_dir: temp_dir.path().join("data"),
            db_file: temp_dir.path().join("data/turbosync.db"),
        };

        let first = init_config_at(&paths).unwrap();
        let second = init_config_at(&paths).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.db_path, paths.db_file);
        assert!(paths.config_file.exists());
        assert!(paths.data_dir.exists());
    }
}
