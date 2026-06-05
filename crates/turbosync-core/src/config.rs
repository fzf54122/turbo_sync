use std::{
    env, fs,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    pub cert_dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct AppConfig {
    pub node_id: String,
    pub agent_addr: String,
    pub transport_addr: String,
    pub db_path: PathBuf,
    #[serde(default)]
    pub cert_dir: PathBuf,
    #[serde(default)]
    pub cert_fingerprint: String,
}

impl AppConfig {
    #[must_use]
    pub fn new(db_path: PathBuf, cert_dir: PathBuf, cert_fingerprint: String) -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            agent_addr: DEFAULT_AGENT_ADDR.to_owned(),
            transport_addr: DEFAULT_TRANSPORT_ADDR.to_owned(),
            db_path,
            cert_dir,
            cert_fingerprint,
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
    let cert_dir = data_dir.join("cert");

    Ok(ConfigPaths {
        config_file,
        data_dir,
        db_file,
        cert_dir,
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
        let mut config = load_config_at(paths)?;
        if config.cert_fingerprint.is_empty() {
            let fingerprint = ensure_agent_cert(&paths.cert_dir)?;
            config.cert_dir = paths.cert_dir.clone();
            config.cert_fingerprint = fingerprint;
            save_config_at(paths, &config)?;
        }
        return Ok(config);
    }

    let fingerprint = ensure_agent_cert(&paths.cert_dir)?;
    let config = AppConfig::new(paths.db_file.clone(), paths.cert_dir.clone(), fingerprint);
    save_config_at(paths, &config)?;

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

fn save_config_at(paths: &ConfigPaths, config: &AppConfig) -> Result<()> {
    let encoded = toml::to_string_pretty(config)?;
    fs::write(&paths.config_file, encoded).map_err(|source| Error::WriteConfig {
        path: paths.config_file.clone(),
        source,
    })?;
    Ok(())
}

/// Ensure agent TLS certificate exists; returns the SHA256 fingerprint.
pub fn ensure_agent_cert(cert_dir: &Path) -> Result<String> {
    let cert_path = cert_dir.join("cert.der");
    if cert_path.exists() {
        let der = fs::read(&cert_path).map_err(|source| Error::ReadConfig {
            path: cert_path.clone(),
            source,
        })?;
        return Ok(compute_cert_fingerprint(&der));
    }

    let (cert, key) = generate_self_signed_cert()?;
    fs::create_dir_all(cert_dir).map_err(|source| Error::CreateDir {
        path: cert_dir.to_path_buf(),
        source,
    })?;
    fs::write(&cert_path, cert.as_ref()).map_err(|source| Error::WriteConfig {
        path: cert_path.clone(),
        source,
    })?;
    let key_path = cert_dir.join("key.der");
    let key_bytes: Vec<u8> = key.secret_der().to_vec();
    fs::write(&key_path, &key_bytes).map_err(|source| Error::WriteConfig {
        path: key_path.clone(),
        source,
    })?;

    Ok(compute_cert_fingerprint(cert.as_ref()))
}

/// Load the persisted agent certificate and private key.
pub fn load_agent_cert(
    cert_dir: &Path,
) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
    let cert_path = cert_dir.join("cert.der");
    let key_path = cert_dir.join("key.der");

    let cert_der = fs::read(&cert_path).map_err(|source| Error::ReadConfig {
        path: cert_path,
        source,
    })?;
    let key_der = fs::read(&key_path).map_err(|source| Error::ReadConfig {
        path: key_path,
        source,
    })?;

    let cert = CertificateDer::from(cert_der);
    let key = PrivatePkcs8KeyDer::from(key_der).into();
    Ok((cert, key))
}

fn generate_self_signed_cert() -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
    let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| Error::Cert(e.to_string()))?;
    let cert = rcgen::CertificateParams::new(vec!["turbosync".into()])
        .map_err(|e| Error::Cert(e.to_string()))?
        .self_signed(&key)
        .map_err(|e| Error::Cert(e.to_string()))?;
    let key_bytes = key.serialized_der().to_vec();
    let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_bytes));
    Ok((cert.der().clone(), key_der))
}

pub fn compute_cert_fingerprint(der: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(der);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
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
            cert_dir: temp_dir.path().join("data/cert"),
        };

        let first = init_config_at(&paths).unwrap();
        let second = init_config_at(&paths).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.db_path, paths.db_file);
        assert!(!first.cert_fingerprint.is_empty());
        assert!(paths.config_file.exists());
        assert!(paths.data_dir.exists());
        assert!(paths.cert_dir.join("cert.der").exists());
        assert!(paths.cert_dir.join("key.der").exists());
    }
}
