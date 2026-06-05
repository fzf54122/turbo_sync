use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to resolve platform config directories")]
    MissingProjectDirs,
    #[error("failed to create directory {path}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read config file {path}")]
    ReadConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write config file {path}")]
    WriteConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}")]
    ParseConfig {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to serialize config")]
    SerializeConfig(#[from] toml::ser::Error),
    #[error("failed to generate TLS certificate: {0}")]
    Cert(String),
}

pub type Result<T> = std::result::Result<T, Error>;
