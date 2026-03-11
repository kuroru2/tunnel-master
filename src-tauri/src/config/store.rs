use std::path::{Path, PathBuf};
use tracing::{debug, error};

use crate::errors::TunnelError;
use crate::types::AppConfig;

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> PathBuf {
        let home = dirs::home_dir().expect("Could not determine home directory");
        home.join(".tunnel-master").join("config.json")
    }

    pub fn load(&self) -> Result<AppConfig, TunnelError> {
        if !self.path.exists() {
            return Err(TunnelError::ConfigNotFound);
        }

        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| TunnelError::ConfigInvalid(e.to_string()))?;

        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| TunnelError::ConfigInvalid(e.to_string()))?;

        if config.version != 1 {
            return Err(TunnelError::ConfigInvalid(format!(
                "Unsupported config version: {}. Expected 1.",
                config.version
            )));
        }

        debug!("Loaded config with {} tunnels", config.tunnels.len());
        Ok(config)
    }

    pub fn expand_tilde(path: &str) -> PathBuf {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&path[2..]);
            }
        }
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn sample_config_json() -> &'static str {
        r#"{
            "version": 1,
            "tunnels": [
                {
                    "id": "dev-db",
                    "name": "Dev Database",
                    "host": "bastion.example.com",
                    "port": 22,
                    "user": "sergio",
                    "keyPath": "~/.ssh/id_rsa",
                    "type": "local",
                    "localPort": 5432,
                    "remoteHost": "db.internal",
                    "remotePort": 5432,
                    "autoConnect": false
                }
            ],
            "settings": {
                "keepaliveIntervalSecs": 15,
                "keepaliveTimeoutSecs": 30,
                "connectionTimeoutSecs": 10,
                "launchAtLogin": false
            }
        }"#
    }

    #[test]
    fn load_valid_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, sample_config_json()).unwrap();

        let store = ConfigStore::new(path);
        let config = store.load().unwrap();

        assert_eq!(config.version, 1);
        assert_eq!(config.tunnels.len(), 1);
        assert_eq!(config.tunnels[0].id, "dev-db");
        assert_eq!(config.tunnels[0].local_port, 5432);
        assert_eq!(config.settings.keepalive_interval_secs, 15);
    }

    #[test]
    fn load_missing_file_returns_config_not_found() {
        let store = ConfigStore::new(PathBuf::from("/nonexistent/config.json"));
        let result = store.load();
        assert!(matches!(result, Err(TunnelError::ConfigNotFound)));
    }

    #[test]
    fn load_invalid_json_returns_config_invalid() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, "not json").unwrap();

        let store = ConfigStore::new(path);
        let result = store.load();
        assert!(matches!(result, Err(TunnelError::ConfigInvalid(_))));
    }

    #[test]
    fn load_unsupported_version_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        let json = r#"{"version": 99, "tunnels": [], "settings": {"keepaliveIntervalSecs": 15, "keepaliveTimeoutSecs": 30, "connectionTimeoutSecs": 10, "launchAtLogin": false}}"#;
        fs::write(&path, json).unwrap();

        let store = ConfigStore::new(path);
        let result = store.load();
        assert!(matches!(result, Err(TunnelError::ConfigInvalid(_))));
    }

    #[test]
    fn expand_tilde_replaces_home() {
        let expanded = ConfigStore::expand_tilde("~/.ssh/id_rsa");
        let home = dirs::home_dir().unwrap();
        assert_eq!(expanded, home.join(".ssh/id_rsa"));
    }

    #[test]
    fn expand_tilde_leaves_absolute_paths() {
        let expanded = ConfigStore::expand_tilde("/etc/ssh/key");
        assert_eq!(expanded, PathBuf::from("/etc/ssh/key"));
    }
}
