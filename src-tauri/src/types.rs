use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelType {
    Local,
    Reverse,
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
    Disconnecting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: String,
    #[serde(rename = "type")]
    pub tunnel_type: TunnelType,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    #[serde(default)]
    pub auto_connect: bool,
}

/// Input for creating/updating a tunnel — no id field
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelInput {
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub user: String,
    #[serde(default)]
    pub key_path: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    #[serde(default)]
    pub auto_connect: bool,
}

fn default_ssh_port() -> u16 { 22 }

impl TunnelInput {
    pub fn to_config(self, id: String) -> TunnelConfig {
        TunnelConfig {
            id,
            name: self.name,
            host: self.host,
            port: self.port,
            user: self.user,
            key_path: self.key_path,
            tunnel_type: TunnelType::Local,
            local_port: self.local_port,
            remote_host: self.remote_host,
            remote_port: self.remote_port,
            auto_connect: self.auto_connect,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_keepalive_interval")]
    pub keepalive_interval_secs: u64,
    #[serde(default = "default_keepalive_timeout")]
    pub keepalive_timeout_secs: u64,
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,
    #[serde(default)]
    pub launch_at_login: bool,
}

fn default_keepalive_interval() -> u64 { 15 }
fn default_keepalive_timeout() -> u64 { 30 }
fn default_connection_timeout() -> u64 { 10 }

impl Default for Settings {
    fn default() -> Self {
        Self {
            keepalive_interval_secs: 15,
            keepalive_timeout_secs: 30,
            connection_timeout_secs: 10,
            launch_at_login: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub tunnels: Vec<TunnelConfig>,
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelInfo {
    pub id: String,
    pub name: String,
    pub status: TunnelStatus,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatusEvent {
    pub id: String,
    pub status: TunnelStatus,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TunnelErrorEvent {
    pub id: String,
    pub message: String,
    pub code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tunnel_status_serializes_to_lowercase() {
        let status = TunnelStatus::Connected;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"connected\"");
    }

    #[test]
    fn tunnel_type_serializes_to_lowercase() {
        let t = TunnelType::Local;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"local\"");
    }

    #[test]
    fn settings_default_values() {
        let s = Settings::default();
        assert_eq!(s.keepalive_interval_secs, 15);
        assert_eq!(s.keepalive_timeout_secs, 30);
        assert_eq!(s.connection_timeout_secs, 10);
        assert!(!s.launch_at_login);
    }

    #[test]
    fn tunnel_input_to_config() {
        let input = TunnelInput {
            name: "Test".to_string(),
            host: "example.com".to_string(),
            port: 22,
            user: "user".to_string(),
            key_path: "".to_string(),
            local_port: 5432,
            remote_host: "db.internal".to_string(),
            remote_port: 5432,
            auto_connect: false,
        };
        let config = input.to_config("test".to_string());
        assert_eq!(config.id, "test");
        assert_eq!(config.tunnel_type, TunnelType::Local);
        assert_eq!(config.name, "Test");
    }

    #[test]
    fn tunnel_config_deserializes_from_json() {
        let json = r#"{
            "id": "test",
            "name": "Test",
            "host": "example.com",
            "port": 22,
            "user": "user",
            "keyPath": "~/.ssh/id_rsa",
            "type": "local",
            "localPort": 5432,
            "remoteHost": "db.internal",
            "remotePort": 5432,
            "autoConnect": false
        }"#;
        let config: TunnelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.id, "test");
        assert_eq!(config.tunnel_type, TunnelType::Local);
        assert_eq!(config.local_port, 5432);
    }
}
