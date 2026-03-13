use std::path::PathBuf;
use tracing::debug;

use crate::errors::TunnelError;
use crate::types::AppConfig;
use crate::types::TunnelInput;

pub fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let mut result = String::new();
    let mut prev_hyphen = true;
    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen {
                result.push('-');
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }
    result.trim_end_matches('-').to_string()
}

pub fn generate_id(name: &str, existing_ids: &[String]) -> String {
    let base = slugify(name);
    if !existing_ids.contains(&base) {
        return base;
    }
    let mut n = 2;
    loop {
        let candidate = format!("{}-{}", base, n);
        if !existing_ids.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

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

    pub fn save(&self, config: &AppConfig) -> Result<(), TunnelError> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TunnelError::ConfigInvalid(format!("Cannot create config dir: {}", e)))?;
        }

        let json = serde_json::to_string_pretty(config)
            .map_err(|e| TunnelError::ConfigInvalid(format!("Serialization error: {}", e)))?;

        // Atomic write: write to temp file, then rename
        let tmp_path = self.path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &json)
            .map_err(|e| TunnelError::ConfigInvalid(format!("Write error: {}", e)))?;
        std::fs::rename(&tmp_path, &self.path)
            .map_err(|e| TunnelError::ConfigInvalid(format!("Rename error: {}", e)))?;

        debug!("Saved config with {} tunnels", config.tunnels.len());
        Ok(())
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

/// Validate tunnel input. `existing_ports` is a list of (id, localPort) for conflict checking.
/// `exclude_id` is set when updating — the tunnel's own port is excluded from conflict check.
pub fn validate_tunnel_input(
    input: &TunnelInput,
    existing_ports: &[(String, u16)],
    exclude_id: Option<&str>,
) -> Result<(), TunnelError> {
    if input.name.trim().is_empty() {
        return Err(TunnelError::ConfigInvalid("name is required".to_string()));
    }
    if input.host.trim().is_empty() {
        return Err(TunnelError::ConfigInvalid("host is required".to_string()));
    }
    if input.user.trim().is_empty() {
        return Err(TunnelError::ConfigInvalid("user is required".to_string()));
    }
    if input.port == 0 {
        return Err(TunnelError::ConfigInvalid("port must be 1-65535".to_string()));
    }
    if input.local_port == 0 {
        return Err(TunnelError::ConfigInvalid("localPort must be 1-65535".to_string()));
    }
    if input.remote_host.trim().is_empty() {
        return Err(TunnelError::ConfigInvalid("remoteHost is required".to_string()));
    }
    if input.remote_port == 0 {
        return Err(TunnelError::ConfigInvalid("remotePort must be 1-65535".to_string()));
    }

    // Check localPort conflict with other tunnels
    for (id, port) in existing_ports {
        if *port == input.local_port {
            if let Some(excl) = exclude_id {
                if id == excl {
                    continue;
                }
            }
            return Err(TunnelError::ConfigInvalid(
                format!("localPort {} is already used by tunnel '{}'", input.local_port, id),
            ));
        }
    }

    // Validate keyPath only for Key auth
    if input.auth_method == crate::types::AuthMethod::Key {
        if input.key_path.is_empty() {
            return Err(TunnelError::ConfigInvalid(
                "Key path is required for key authentication".to_string(),
            ));
        }
        let expanded = ConfigStore::expand_tilde(&input.key_path);
        if !expanded.exists() {
            return Err(TunnelError::ConfigInvalid(
                format!("keyPath '{}' does not exist", input.key_path),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Settings;
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

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("ORA Web"), "ora-web");
    }

    #[test]
    fn slugify_special_chars() {
        assert_eq!(slugify("ORA Web (prod)"), "ora-web-prod");
    }

    #[test]
    fn slugify_consecutive_hyphens() {
        assert_eq!(slugify("my--tunnel---name"), "my-tunnel-name");
    }

    #[test]
    fn slugify_leading_trailing() {
        assert_eq!(slugify("  --hello-- "), "hello");
    }

    #[test]
    fn generate_id_no_conflict() {
        let existing: Vec<String> = vec![];
        assert_eq!(generate_id("ORA Web", &existing), "ora-web");
    }

    #[test]
    fn generate_id_with_conflict() {
        let existing = vec!["ora-web".to_string()];
        assert_eq!(generate_id("ORA Web", &existing), "ora-web-2");
    }

    #[test]
    fn generate_id_multiple_conflicts() {
        let existing = vec!["ora-web".to_string(), "ora-web-2".to_string()];
        assert_eq!(generate_id("ORA Web", &existing), "ora-web-3");
    }

    #[test]
    fn save_and_reload() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, sample_config_json()).unwrap();

        let store = ConfigStore::new(path);
        let mut config = store.load().unwrap();

        config.tunnels[0].name = "Modified".to_string();
        store.save(&config).unwrap();

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.tunnels[0].name, "Modified");
    }

    use crate::types::{AuthMethod, TunnelInput};

    #[test]
    fn validate_input_valid() {
        let input = TunnelInput {
            name: "Test".to_string(),
            host: "example.com".to_string(),
            port: 22,
            user: "user".to_string(),
            key_path: "".to_string(),
            auth_method: AuthMethod::Password,
            local_port: 5432,
            remote_host: "db.internal".to_string(),
            remote_port: 5432,
            auto_connect: false,
            jump_host: None,
        };
        assert!(validate_tunnel_input(&input, &[], None).is_ok());
    }

    #[test]
    fn validate_input_key_auth_requires_key_path() {
        let input = TunnelInput {
            name: "Test".to_string(),
            host: "example.com".to_string(),
            port: 22,
            user: "user".to_string(),
            key_path: "".to_string(),
            auth_method: AuthMethod::Key,
            local_port: 5432,
            remote_host: "db.internal".to_string(),
            remote_port: 5432,
            auto_connect: false,
            jump_host: None,
        };
        let err = validate_tunnel_input(&input, &[], None).unwrap_err();
        assert!(err.to_string().contains("Key path is required"));
    }

    #[test]
    fn validate_input_empty_name() {
        let input = TunnelInput {
            name: "".to_string(),
            host: "example.com".to_string(),
            port: 22,
            user: "user".to_string(),
            key_path: "".to_string(),
            auth_method: AuthMethod::Password,
            local_port: 5432,
            remote_host: "db.internal".to_string(),
            remote_port: 5432,
            auto_connect: false,
            jump_host: None,
        };
        let err = validate_tunnel_input(&input, &[], None).unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn validate_input_port_conflict() {
        let input = TunnelInput {
            name: "Test".to_string(),
            host: "example.com".to_string(),
            port: 22,
            user: "user".to_string(),
            key_path: "".to_string(),
            auth_method: AuthMethod::Password,
            local_port: 5432,
            remote_host: "db.internal".to_string(),
            remote_port: 5432,
            auto_connect: false,
            jump_host: None,
        };
        let existing = vec![("other".to_string(), 5432u16)];
        let err = validate_tunnel_input(&input, &existing, None).unwrap_err();
        assert!(err.to_string().contains("5432"));
    }

    #[test]
    fn validate_input_port_conflict_self_excluded() {
        let input = TunnelInput {
            name: "Test".to_string(),
            host: "example.com".to_string(),
            port: 22,
            user: "user".to_string(),
            key_path: "".to_string(),
            auth_method: AuthMethod::Password,
            local_port: 5432,
            remote_host: "db.internal".to_string(),
            remote_port: 5432,
            auto_connect: false,
            jump_host: None,
        };
        let existing = vec![("self-id".to_string(), 5432u16)];
        assert!(validate_tunnel_input(&input, &existing, Some("self-id")).is_ok());
    }

    #[test]
    fn validate_input_port_zero() {
        let input = TunnelInput {
            name: "Test".to_string(),
            host: "example.com".to_string(),
            port: 22,
            user: "user".to_string(),
            key_path: "".to_string(),
            auth_method: AuthMethod::Password,
            local_port: 0,
            remote_host: "db.internal".to_string(),
            remote_port: 5432,
            auto_connect: false,
            jump_host: None,
        };
        let err = validate_tunnel_input(&input, &[], None).unwrap_err();
        assert!(err.to_string().contains("localPort"));
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("subdir").join("config.json");

        let store = ConfigStore::new(path.clone());
        let config = AppConfig {
            version: 1,
            tunnels: vec![],
            settings: Settings::default(),
        };
        store.save(&config).unwrap();

        assert!(path.exists());
        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.tunnels.len(), 0);
    }
}
