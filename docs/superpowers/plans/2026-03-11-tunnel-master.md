# Tunnel Master Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a macOS menu bar app that manages SSH tunnels with graceful lifecycle management, using Rust + Tauri v2.

**Architecture:** Rust backend with an actor-pattern TunnelManager (tokio mpsc channel) orchestrating per-tunnel SshConnection, PortForwarder, and HealthMonitor tasks. TypeScript/React frontend in a Tauri popover window communicates via IPC commands and events.

**Tech Stack:** Rust, Tauri v2, russh, tokio, serde, tracing, dirs, security-framework | TypeScript, React, Vite, Tailwind CSS, @tauri-apps/api

**Spec:** `docs/superpowers/specs/2026-03-11-tunnel-master-design.md`

---

## File Map

### Rust Backend (`src-tauri/`)

| File | Responsibility |
|------|---------------|
| `src/main.rs` | Tauri app entry point, managed state setup, system tray, shutdown hook |
| `src/errors.rs` | `TunnelError` enum, serialization for Tauri IPC |
| `src/types.rs` | Shared types: `TunnelConfig`, `TunnelInfo`, `TunnelStatus`, `Settings`, `AppConfig` |
| `src/commands.rs` | Tauri `#[command]` handlers: list, connect, disconnect, reload |
| `src/config/mod.rs` | Re-exports |
| `src/config/store.rs` | `ConfigStore`: JSON read/write/validate, tilde expansion |
| `src/tunnel/mod.rs` | Re-exports |
| `src/tunnel/manager.rs` | `TunnelManager`: actor loop, mpsc receiver, orchestrates lifecycle |
| `src/tunnel/connection.rs` | `SshConnection`: russh client wrapper, auth, channel management |
| `src/tunnel/forwarder.rs` | `PortForwarder`: local TCP listener, bidirectional pipe through SSH channel |
| `src/tunnel/health.rs` | `HealthMonitor`: per-tunnel keepalive pings, dead connection detection |
| `src/keychain.rs` | macOS Keychain read/write for SSH key passphrases |
| `Cargo.toml` | Rust dependencies |
| `tauri.conf.json` | Tauri app config (window, permissions, tray) |

### TypeScript Frontend (`src/`)

| File | Responsibility |
|------|---------------|
| `src/main.tsx` | React entry point |
| `src/App.tsx` | Root component, renders TunnelList |
| `src/types.ts` | TypeScript types mirroring Rust types |
| `src/hooks/useTunnels.ts` | Hook: Tauri IPC calls + event subscriptions |
| `src/components/TunnelList.tsx` | Renders list of TunnelItem components |
| `src/components/TunnelItem.tsx` | Single tunnel row: name, status indicator, toggle button |
| `src/index.css` | Tailwind imports + popover-specific styles |

### Root

| File | Responsibility |
|------|---------------|
| `package.json` | Node dependencies, scripts |
| `vite.config.ts` | Vite config for Tauri |
| `tailwind.config.js` | Tailwind config |
| `postcss.config.js` | PostCSS for Tailwind |
| `tsconfig.json` | TypeScript config |
| `index.html` | HTML entry point |
| `config.example.json` | Example config file |

---

## Chunk 1: Project Scaffolding & Foundation

### Task 1: Scaffold Tauri v2 + Vite + React project

**Files:**
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `tailwind.config.js`, `postcss.config.js`, `index.html`, `src/main.tsx`, `src/App.tsx`, `src/index.css`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/build.rs`

**Prerequisites:** Rust toolchain, Node.js 18+, and Tauri CLI must be installed.

- [ ] **Step 1: Verify prerequisites are installed**

Run:
```bash
rustc --version && cargo --version && node --version && npm --version
```
Expected: version numbers for all four tools.

- [ ] **Step 2: Install Tauri CLI globally**

Run:
```bash
cargo install tauri-cli --version "^2"
```
Expected: `tauri-cli` installed or already up to date.

- [ ] **Step 3: Create Vite + React + TypeScript project**

Run from the project root (`/Users/sergiiolyva/ctbto/projects/tunnel-master`):
```bash
npm create vite@latest . -- --template react-ts
```
If prompted about non-empty directory, choose to continue (only `.git`, `.idea`, `docs`, `.gitignore` exist).

Expected: `package.json`, `vite.config.ts`, `tsconfig.json`, `src/`, `index.html` created.

- [ ] **Step 4: Install frontend dependencies**

Run:
```bash
npm install
npm install -D tailwindcss @tailwindcss/vite
npm install @tauri-apps/api @tauri-apps/plugin-shell
```

- [ ] **Step 5: Configure Tailwind CSS**

Replace `src/index.css` with:
```css
@import "tailwindcss";
```

Update `vite.config.ts`:
```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "esnext",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
```

- [ ] **Step 6: Initialize Tauri in the project**

Run:
```bash
cargo tauri init
```

When prompted:
- App name: `tunnel-master`
- Window title: `Tunnel Master`
- Web assets path: `../dist`
- Dev server URL: `http://localhost:1420`
- Frontend dev command: `npm run dev`
- Frontend build command: `npm run build`

Expected: `src-tauri/` directory created with `Cargo.toml`, `tauri.conf.json`, `src/main.rs`, `build.rs`.

- [ ] **Step 7: Add Rust dependencies to Cargo.toml**

Edit `src-tauri/Cargo.toml` — add to `[dependencies]`:
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-build = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
russh = "0.46"
russh-keys = "0.46"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dirs = "5"
security-framework = "2"
thiserror = "2"
async-trait = "0.1"
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 8: Write minimal App.tsx placeholder**

Replace `src/App.tsx`:
```tsx
function App() {
  return (
    <div className="min-h-screen bg-gray-900 text-white p-4">
      <h1 className="text-lg font-semibold">Tunnel Master</h1>
      <p className="text-gray-400 text-sm mt-1">No tunnels configured</p>
    </div>
  );
}

export default App;
```

- [ ] **Step 9: Verify the project compiles and runs**

Run:
```bash
cargo tauri dev
```
Expected: Tauri app window opens showing "Tunnel Master" heading. Close the window to stop.

- [ ] **Step 10: Commit scaffolding**

```bash
git add -A
git commit -m "feat: scaffold Tauri v2 + Vite + React + Tailwind project"
```

---

### Task 2: Define shared types and error types (Rust)

**Files:**
- Create: `src-tauri/src/types.rs`
- Create: `src-tauri/src/errors.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write tests for TunnelStatus serialization**

Create `src-tauri/src/types.rs`:
```rust
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
    pub auto_connect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub keepalive_interval_secs: u64,
    pub keepalive_timeout_secs: u64,
    pub connection_timeout_secs: u64,
    pub launch_at_login: bool,
}

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
```

- [ ] **Step 2: Run tests to verify they pass**

Run:
```bash
cd src-tauri && cargo test types::tests -- --nocapture
```
Expected: 4 tests pass.

- [ ] **Step 3: Create error types**

Create `src-tauri/src/errors.rs`:
```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
pub enum TunnelError {
    #[error("Config file not found")]
    ConfigNotFound,

    #[error("Invalid config: {0}")]
    ConfigInvalid(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Port {0} is already in use")]
    PortInUse(u16),

    #[error("Connection timed out")]
    ConnectionTimeout,

    #[error("SSH error: {0}")]
    SshError(String),

    #[error("Tunnel not found: {0}")]
    TunnelNotFound(String),
}

// Tauri requires IntoString for command error returns
impl From<TunnelError> for String {
    fn from(e: TunnelError) -> String {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        assert_eq!(
            TunnelError::PortInUse(5432).to_string(),
            "Port 5432 is already in use"
        );
        assert_eq!(
            TunnelError::TunnelNotFound("abc".into()).to_string(),
            "Tunnel not found: abc"
        );
    }

    #[test]
    fn error_serializes_to_json() {
        let err = TunnelError::ConnectionTimeout;
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("ConnectionTimeout"));
    }
}
```

- [ ] **Step 4: Run error tests**

Run:
```bash
cd src-tauri && cargo test errors::tests -- --nocapture
```
Expected: 2 tests pass.

- [ ] **Step 5: Wire modules into main.rs**

Update `src-tauri/src/main.rs`:
```rust
mod config;
mod commands;
mod errors;
mod keychain;
mod tunnel;
mod types;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("tunnel_master=debug")
        .init();

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Create stub files so this compiles:
- `src-tauri/src/commands.rs`: `// Tauri command handlers — implemented in Task 7`
- `src-tauri/src/keychain.rs`: `// macOS Keychain integration — implemented in Task 6`
- `src-tauri/src/config/mod.rs`: `pub mod store;`
- `src-tauri/src/config/store.rs`: `// ConfigStore — implemented in Task 3`
- `src-tauri/src/tunnel/mod.rs`:
```rust
pub mod manager;
pub mod connection;
pub mod forwarder;
pub mod health;
```
- `src-tauri/src/tunnel/manager.rs`: `// TunnelManager — implemented in Task 4`
- `src-tauri/src/tunnel/connection.rs`: `// SshConnection — implemented in Task 5`
- `src-tauri/src/tunnel/forwarder.rs`: `// PortForwarder — implemented in Task 5`
- `src-tauri/src/tunnel/health.rs`: `// HealthMonitor — implemented in Task 5`

- [ ] **Step 6: Verify everything compiles**

Run:
```bash
cd src-tauri && cargo build
```
Expected: compiles with no errors (warnings about unused modules are OK).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/errors.rs src-tauri/src/main.rs \
  src-tauri/src/commands.rs src-tauri/src/keychain.rs \
  src-tauri/src/config/ src-tauri/src/tunnel/
git commit -m "feat: add shared types and error definitions"
```

---

### Task 3: ConfigStore — JSON config read/write/validate

**Files:**
- Create: `src-tauri/src/config/store.rs`
- Modify: `src-tauri/src/config/mod.rs`
- Create: `config.example.json`

- [ ] **Step 1: Write failing tests for ConfigStore**

Write `src-tauri/src/config/store.rs`:
```rust
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
```

- [ ] **Step 2: Add tempfile dev-dependency**

Add to `src-tauri/Cargo.toml` under `[dev-dependencies]`:
```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Run tests**

Run:
```bash
cd src-tauri && cargo test config::store::tests -- --nocapture
```
Expected: 6 tests pass.

- [ ] **Step 4: Create config.example.json**

Create `config.example.json` in project root:
```json
{
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
    },
    {
      "id": "dev-redis",
      "name": "Dev Redis",
      "host": "bastion.example.com",
      "port": 22,
      "user": "sergio",
      "keyPath": "~/.ssh/id_rsa",
      "type": "local",
      "localPort": 6379,
      "remoteHost": "redis.internal",
      "remotePort": 6379,
      "autoConnect": false
    }
  ],
  "settings": {
    "keepaliveIntervalSecs": 15,
    "keepaliveTimeoutSecs": 30,
    "connectionTimeoutSecs": 10,
    "launchAtLogin": false
  }
}
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config/ config.example.json src-tauri/Cargo.toml
git commit -m "feat: add ConfigStore with JSON loading and validation"
```

---

## Chunk 2: Core SSH Engine

### Task 4: TunnelManager — actor pattern with state machine

**Files:**
- Create: `src-tauri/src/tunnel/manager.rs`
- Modify: `src-tauri/src/tunnel/mod.rs`

The TunnelManager is the central orchestrator. It receives commands over an mpsc channel and manages tunnel state transitions. For this task, we implement the actor skeleton and state tracking without real SSH connections (those come in Task 5).

- [ ] **Step 1: Write tests for the manager's state tracking**

Write `src-tauri/src/tunnel/manager.rs`:
```rust
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

use crate::errors::TunnelError;
use crate::types::{AppConfig, TunnelConfig, TunnelInfo, TunnelStatus, TunnelStatusEvent};

/// Messages the TunnelManager actor receives
#[derive(Debug)]
pub enum ManagerCommand {
    ListTunnels {
        reply: oneshot::Sender<Vec<TunnelInfo>>,
    },
    Connect {
        id: String,
        reply: oneshot::Sender<Result<(), TunnelError>>,
    },
    Disconnect {
        id: String,
        reply: oneshot::Sender<Result<(), TunnelError>>,
    },
    ReloadConfig {
        config: AppConfig,
        reply: oneshot::Sender<Result<(), TunnelError>>,
    },
    TunnelDied {
        id: String,
        error: String,
    },
}

/// Runtime state for a single tunnel
#[derive(Debug)]
struct TunnelState {
    config: TunnelConfig,
    status: TunnelStatus,
    error_message: Option<String>,
    /// Handle to abort the tunnel's tokio tasks on disconnect
    abort_handle: Option<tokio::task::AbortHandle>,
}

impl TunnelState {
    fn new(config: TunnelConfig) -> Self {
        Self {
            config,
            status: TunnelStatus::Disconnected,
            error_message: None,
            abort_handle: None,
        }
    }

    fn to_info(&self) -> TunnelInfo {
        TunnelInfo {
            id: self.config.id.clone(),
            name: self.config.name.clone(),
            status: self.status.clone(),
            local_port: self.config.local_port,
            remote_host: self.config.remote_host.clone(),
            remote_port: self.config.remote_port,
            error_message: self.error_message.clone(),
        }
    }
}

pub type ManagerHandle = mpsc::Sender<ManagerCommand>;

/// Spawn the TunnelManager actor. Returns a sender to communicate with it.
pub fn spawn_manager(
    config: AppConfig,
    event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
    error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
) -> ManagerHandle {
    let (tx, rx) = mpsc::channel(32);
    let manager_tx = tx.clone();

    tokio::spawn(async move {
        let mut manager = TunnelManagerActor::new(config, event_tx, error_tx, manager_tx);
        manager.run(rx).await;
    });

    tx
}

struct TunnelManagerActor {
    tunnels: HashMap<String, TunnelState>,
    event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
    error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
    manager_tx: mpsc::Sender<ManagerCommand>,
    settings: crate::types::Settings,
}

impl TunnelManagerActor {
    fn new(
        config: AppConfig,
        event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
        error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
        manager_tx: mpsc::Sender<ManagerCommand>,
    ) -> Self {
        let mut tunnels = HashMap::new();
        for tc in config.tunnels {
            tunnels.insert(tc.id.clone(), TunnelState::new(tc));
        }

        Self {
            tunnels,
            event_tx,
            error_tx,
            manager_tx,
            settings: config.settings,
        }
    }

    async fn run(&mut self, mut rx: mpsc::Receiver<ManagerCommand>) {
        info!("TunnelManager started with {} tunnels", self.tunnels.len());

        while let Some(cmd) = rx.recv().await {
            match cmd {
                ManagerCommand::ListTunnels { reply } => {
                    let infos: Vec<TunnelInfo> =
                        self.tunnels.values().map(|t| t.to_info()).collect();
                    let _ = reply.send(infos);
                }

                ManagerCommand::Connect { id, reply } => {
                    let result = self.handle_connect(&id).await;
                    let _ = reply.send(result);
                }

                ManagerCommand::Disconnect { id, reply } => {
                    let result = self.handle_disconnect(&id).await;
                    let _ = reply.send(result);
                }

                ManagerCommand::ReloadConfig { config, reply } => {
                    let result = self.handle_reload(config);
                    let _ = reply.send(result);
                }

                ManagerCommand::TunnelDied { id, error } => {
                    self.handle_tunnel_died(&id, &error).await;
                }
            }
        }

        info!("TunnelManager shutting down, disconnecting all tunnels");
        self.disconnect_all().await;
    }

    async fn handle_connect(&mut self, id: &str) -> Result<(), TunnelError> {
        let tunnel = self
            .tunnels
            .get_mut(id)
            .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

        if tunnel.status == TunnelStatus::Connected || tunnel.status == TunnelStatus::Connecting {
            debug!("Tunnel {} already connected/connecting", id);
            return Ok(());
        }

        tunnel.status = TunnelStatus::Connecting;
        tunnel.error_message = None;
        self.emit_status(id, &TunnelStatus::Connecting);

        // TODO: Task 6 will add real SSH connection logic here.
        // For now, we just transition to Connected to validate the state machine.
        tunnel.status = TunnelStatus::Connected;
        self.emit_status(id, &TunnelStatus::Connected);

        info!("Tunnel {} connected", id);
        Ok(())
    }

    async fn handle_disconnect(&mut self, id: &str) -> Result<(), TunnelError> {
        let tunnel = self
            .tunnels
            .get_mut(id)
            .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

        if tunnel.status == TunnelStatus::Disconnected {
            return Ok(());
        }

        tunnel.status = TunnelStatus::Disconnecting;
        self.emit_status(id, &TunnelStatus::Disconnecting);

        // Abort the tunnel's background tasks if any
        if let Some(handle) = tunnel.abort_handle.take() {
            handle.abort();
        }

        tunnel.status = TunnelStatus::Disconnected;
        tunnel.error_message = None;
        self.emit_status(id, &TunnelStatus::Disconnected);

        info!("Tunnel {} disconnected", id);
        Ok(())
    }

    async fn handle_tunnel_died(&mut self, id: &str, error: &str) {
        if let Some(tunnel) = self.tunnels.get_mut(id) {
            warn!("Tunnel {} died: {}", id, error);
            tunnel.status = TunnelStatus::Error;
            tunnel.error_message = Some(error.to_string());
            self.emit_status(id, &TunnelStatus::Error);

            // Clean up
            if let Some(handle) = tunnel.abort_handle.take() {
                handle.abort();
            }

            tunnel.status = TunnelStatus::Disconnected;
            self.emit_status(id, &TunnelStatus::Disconnected);
        }
    }

    fn handle_reload(&mut self, config: AppConfig) -> Result<(), TunnelError> {
        // Update settings
        self.settings = config.settings;

        // Add new tunnels, update existing ones (but don't touch connected tunnels' state)
        for tc in config.tunnels {
            if let Some(existing) = self.tunnels.get_mut(&tc.id) {
                existing.config = tc;
            } else {
                self.tunnels.insert(tc.id.clone(), TunnelState::new(tc));
            }
        }

        info!("Config reloaded, {} tunnels configured", self.tunnels.len());
        Ok(())
    }

    async fn disconnect_all(&mut self) {
        let ids: Vec<String> = self.tunnels.keys().cloned().collect();
        for id in ids {
            let _ = self.handle_disconnect(&id).await;
        }
    }

    fn emit_status(&self, id: &str, status: &TunnelStatus) {
        if let Some(tx) = &self.event_tx {
            let event = TunnelStatusEvent {
                id: id.to_string(),
                status: status.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            let _ = tx.send(event);
        }
    }

    fn emit_error(&self, id: &str, message: &str, code: &str) {
        if let Some(tx) = &self.error_tx {
            let event = crate::types::TunnelErrorEvent {
                id: id.to_string(),
                message: message.to_string(),
                code: code.to_string(),
            };
            let _ = tx.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Settings, TunnelType};

    fn test_config() -> AppConfig {
        AppConfig {
            version: 1,
            tunnels: vec![
                TunnelConfig {
                    id: "db".into(),
                    name: "Database".into(),
                    host: "example.com".into(),
                    port: 22,
                    user: "user".into(),
                    key_path: "~/.ssh/id_rsa".into(),
                    tunnel_type: TunnelType::Local,
                    local_port: 5432,
                    remote_host: "db.internal".into(),
                    remote_port: 5432,
                    auto_connect: false,
                },
                TunnelConfig {
                    id: "redis".into(),
                    name: "Redis".into(),
                    host: "example.com".into(),
                    port: 22,
                    user: "user".into(),
                    key_path: "~/.ssh/id_rsa".into(),
                    tunnel_type: TunnelType::Local,
                    local_port: 6379,
                    remote_host: "redis.internal".into(),
                    remote_port: 6379,
                    auto_connect: false,
                },
            ],
            settings: Settings::default(),
        }
    }

    #[tokio::test]
    async fn list_tunnels_returns_all() {
        let handle = spawn_manager(test_config(), None, None);
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::ListTunnels { reply: reply_tx })
            .await
            .unwrap();

        let tunnels = reply_rx.await.unwrap();
        assert_eq!(tunnels.len(), 2);
        assert!(tunnels.iter().all(|t| t.status == TunnelStatus::Disconnected));
    }

    #[tokio::test]
    async fn connect_transitions_to_connected() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let handle = spawn_manager(test_config(), Some(event_tx), None);

        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Connect {
                id: "db".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        assert!(reply_rx.await.unwrap().is_ok());

        // Should have received Connecting then Connected events
        let evt1 = event_rx.recv().await.unwrap();
        assert_eq!(evt1.status, TunnelStatus::Connecting);
        let evt2 = event_rx.recv().await.unwrap();
        assert_eq!(evt2.status, TunnelStatus::Connected);
    }

    #[tokio::test]
    async fn disconnect_transitions_to_disconnected() {
        let handle = spawn_manager(test_config(), None, None);

        // Connect first
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Connect {
                id: "db".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // Disconnect
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Disconnect {
                id: "db".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // Verify status
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::ListTunnels { reply: reply_tx })
            .await
            .unwrap();
        let tunnels = reply_rx.await.unwrap();
        let db = tunnels.iter().find(|t| t.id == "db").unwrap();
        assert_eq!(db.status, TunnelStatus::Disconnected);
    }

    #[tokio::test]
    async fn connect_unknown_tunnel_returns_error() {
        let handle = spawn_manager(test_config(), None, None);

        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Connect {
                id: "nonexistent".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        let result = reply_rx.await.unwrap();
        assert!(matches!(result, Err(TunnelError::TunnelNotFound(_))));
    }

    #[tokio::test]
    async fn tunnel_died_transitions_through_error() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let handle = spawn_manager(test_config(), Some(event_tx), None);

        // Connect first
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Connect {
                id: "db".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // Drain connect events
        let _ = event_rx.recv().await; // Connecting
        let _ = event_rx.recv().await; // Connected

        // Simulate tunnel death
        handle
            .send(ManagerCommand::TunnelDied {
                id: "db".into(),
                error: "Connection lost".into(),
            })
            .await
            .unwrap();

        // Should get Error then Disconnected
        let evt = event_rx.recv().await.unwrap();
        assert_eq!(evt.status, TunnelStatus::Error);
        let evt = event_rx.recv().await.unwrap();
        assert_eq!(evt.status, TunnelStatus::Disconnected);
    }

    #[tokio::test]
    async fn reload_config_adds_new_tunnels() {
        let handle = spawn_manager(test_config(), None, None);

        let mut new_config = test_config();
        new_config.tunnels.push(TunnelConfig {
            id: "new".into(),
            name: "New Tunnel".into(),
            host: "example.com".into(),
            port: 22,
            user: "user".into(),
            key_path: "~/.ssh/id_rsa".into(),
            tunnel_type: TunnelType::Local,
            local_port: 8080,
            remote_host: "web.internal".into(),
            remote_port: 80,
            auto_connect: false,
        });

        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::ReloadConfig {
                config: new_config,
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // Verify 3 tunnels now
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::ListTunnels { reply: reply_tx })
            .await
            .unwrap();
        let tunnels = reply_rx.await.unwrap();
        assert_eq!(tunnels.len(), 3);
    }
}
```

- [ ] **Step 2: Run tests**

Run:
```bash
cd src-tauri && cargo test tunnel::manager::tests -- --nocapture
```
Expected: 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/tunnel/manager.rs src-tauri/src/tunnel/mod.rs
git commit -m "feat: add TunnelManager actor with state machine and mpsc commands"
```

---

### Task 5: SSH Connection, Port Forwarding, and Health Monitor

**Files:**
- Create: `src-tauri/src/tunnel/connection.rs`
- Create: `src-tauri/src/tunnel/forwarder.rs`
- Create: `src-tauri/src/tunnel/health.rs`
- Create: `src-tauri/src/keychain.rs`

These are the core SSH components. Integration testing requires a real SSH server, so unit tests focus on the structural/state aspects. Manual testing with a real server validates the SSH flow end-to-end.

- [ ] **Step 1: Implement SshConnection**

Write `src-tauri/src/tunnel/connection.rs`:
```rust
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use russh::client;
use russh::*;
use russh_keys::key;
use tokio::net::TcpStream;
use tracing::{debug, error, info};

use crate::config::store::ConfigStore;
use crate::errors::TunnelError;

/// Wrapper around russh client session
pub struct SshConnection {
    session: client::Handle<SshClientHandler>,
}

/// Handler for russh client callbacks
struct SshClientHandler;

#[async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: In production, verify against known_hosts.
        // For POC, accept all keys.
        Ok(true)
    }
}

impl SshConnection {
    /// Connect to an SSH server and authenticate with a key file.
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        key_path: &str,
        passphrase: Option<&str>,
        timeout_secs: u64,
    ) -> Result<Self, TunnelError> {
        let expanded_key_path = ConfigStore::expand_tilde(key_path);

        info!("Connecting to {}@{}:{}", user, host, port);

        // Load the private key
        let key_pair = if let Some(pass) = passphrase {
            russh_keys::load_secret_key(&expanded_key_path, Some(pass))
                .map_err(|e| TunnelError::AuthFailed(format!("Failed to load key: {}", e)))?
        } else {
            russh_keys::load_secret_key(&expanded_key_path, None)
                .map_err(|e| TunnelError::AuthFailed(format!("Failed to load key: {}", e)))?
        };

        // Configure the SSH client
        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(timeout_secs * 3)),
            keepalive_interval: None, // We handle keepalive ourselves via HealthMonitor
            ..Default::default()
        };

        // Connect with timeout
        let addr = format!("{}:{}", host, port);
        let session = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            client::connect(Arc::new(config), &addr, SshClientHandler),
        )
        .await
        .map_err(|_| TunnelError::ConnectionTimeout)?
        .map_err(|e| TunnelError::SshError(format!("Connection failed: {}", e)))?;

        // Authenticate
        let auth_result = session
            .authenticate_publickey(user, Arc::new(key_pair))
            .await
            .map_err(|e| TunnelError::AuthFailed(format!("Auth error: {}", e)))?;

        if !auth_result {
            return Err(TunnelError::AuthFailed(
                "Server rejected public key".to_string(),
            ));
        }

        info!("SSH connection established to {}:{}", host, port);
        Ok(Self { session })
    }

    /// Request a direct-tcpip channel for local port forwarding.
    pub async fn open_direct_tcpip(
        &self,
        remote_host: &str,
        remote_port: u16,
        local_host: &str,
        local_port: u16,
    ) -> Result<Channel<client::Msg>, TunnelError> {
        let channel = self
            .session
            .channel_open_direct_tcpip(
                remote_host,
                remote_port.into(),
                local_host,
                local_port.into(),
            )
            .await
            .map_err(|e| TunnelError::SshError(format!("Failed to open channel: {}", e)))?;

        debug!(
            "Opened direct-tcpip channel: {}:{} -> {}:{}",
            local_host, local_port, remote_host, remote_port
        );
        Ok(channel)
    }

    /// Send a keepalive request. Returns Ok if server responds.
    pub async fn send_keepalive(&self) -> Result<(), TunnelError> {
        // russh uses global requests for keepalive
        // A simple approach: try to send a "keepalive@openssh.com" request
        self.session
            .send_keepalive(true)
            .await
            .map_err(|e| TunnelError::SshError(format!("Keepalive failed: {}", e)))?;
        Ok(())
    }

    /// Disconnect the SSH session.
    pub async fn disconnect(&self) {
        let _ = self
            .session
            .disconnect(Disconnect::ByApplication, "Client disconnecting", "en")
            .await;
        info!("SSH session disconnected");
    }
}
```

- [ ] **Step 2: Implement PortForwarder**

Write `src-tauri/src/tunnel/forwarder.rs`:
```rust
use std::net::SocketAddr;
use std::sync::Arc;

use russh::client;
use russh::Channel;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::errors::TunnelError;
use crate::tunnel::connection::SshConnection;

/// Listens on a local port and forwards connections through an SSH channel.
pub struct PortForwarder;

impl PortForwarder {
    /// Bind a local port and forward incoming connections to remote_host:remote_port
    /// via the SSH connection. Runs until the cancel token is triggered.
    pub async fn start(
        ssh: Arc<SshConnection>,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
        death_tx: mpsc::Sender<String>,
        tunnel_id: String,
    ) -> Result<(), TunnelError> {
        let addr: SocketAddr = ([127, 0, 0, 1], local_port).into();
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                TunnelError::PortInUse(local_port)
            } else {
                TunnelError::SshError(format!("Failed to bind port {}: {}", local_port, e))
            }
        })?;

        info!("Forwarding localhost:{} -> {}:{}", local_port, remote_host, remote_port);

        loop {
            match listener.accept().await {
                Ok((tcp_stream, peer_addr)) => {
                    debug!("Accepted connection from {} on port {}", peer_addr, local_port);

                    let ssh = ssh.clone();
                    let rh = remote_host.clone();
                    let rp = remote_port;
                    let lp = local_port;
                    let death = death_tx.clone();
                    let tid = tunnel_id.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            ssh, tcp_stream, &rh, rp, lp,
                        ).await {
                            warn!("Connection handling error on tunnel {}: {}", tid, e);
                            // Don't report individual connection errors as tunnel death
                            // The health monitor handles tunnel-level failures
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error on port {}: {}", local_port, e);
                    let _ = death_tx
                        .send(format!("Listener error: {}", e))
                        .await;
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(
        ssh: Arc<SshConnection>,
        mut tcp_stream: tokio::net::TcpStream,
        remote_host: &str,
        remote_port: u16,
        local_port: u16,
    ) -> Result<(), TunnelError> {
        let channel = ssh
            .open_direct_tcpip(remote_host, remote_port, "127.0.0.1", local_port)
            .await?;

        let (mut channel_reader, mut channel_writer) = tokio::io::split(channel.into_stream());
        let (mut tcp_reader, mut tcp_writer) = tcp_stream.split();

        // Bidirectional pipe
        tokio::select! {
            result = tokio::io::copy(&mut tcp_reader, &mut channel_writer) => {
                if let Err(e) = result {
                    debug!("TCP->SSH copy ended: {}", e);
                }
            }
            result = tokio::io::copy(&mut channel_reader, &mut tcp_writer) => {
                if let Err(e) = result {
                    debug!("SSH->TCP copy ended: {}", e);
                }
            }
        }

        debug!("Connection closed on port {}", local_port);
        Ok(())
    }
}
```

- [ ] **Step 3: Implement HealthMonitor**

Write `src-tauri/src/tunnel/health.rs`:
```rust
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::tunnel::connection::SshConnection;

/// Monitors an SSH connection's health by sending periodic keepalive pings.
pub struct HealthMonitor;

impl HealthMonitor {
    /// Run the health monitor loop. Sends keepalive pings at the configured interval.
    /// If a keepalive fails or times out, sends the tunnel ID to death_tx.
    pub async fn run(
        ssh: Arc<SshConnection>,
        tunnel_id: String,
        keepalive_interval_secs: u64,
        keepalive_timeout_secs: u64,
        death_tx: mpsc::Sender<String>,
    ) {
        let interval = Duration::from_secs(keepalive_interval_secs);
        let timeout = Duration::from_secs(keepalive_timeout_secs);

        loop {
            tokio::time::sleep(interval).await;

            let result = tokio::time::timeout(timeout, ssh.send_keepalive()).await;

            match result {
                Ok(Ok(())) => {
                    debug!("Keepalive OK for tunnel {}", tunnel_id);
                }
                Ok(Err(e)) => {
                    warn!("Keepalive error for tunnel {}: {}", tunnel_id, e);
                    let _ = death_tx
                        .send(format!("Keepalive error: {}", e))
                        .await;
                    break;
                }
                Err(_) => {
                    warn!("Keepalive timeout for tunnel {}", tunnel_id);
                    let _ = death_tx
                        .send("Keepalive timeout".to_string())
                        .await;
                    break;
                }
            }
        }
    }
}
```

- [ ] **Step 4: Implement Keychain integration**

Write `src-tauri/src/keychain.rs`:
```rust
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};
use tracing::{debug, warn};

const SERVICE_NAME: &str = "tunnel-master";

/// Retrieve an SSH key passphrase from macOS Keychain.
/// Returns None if no passphrase is stored.
pub fn get_passphrase(key_path: &str) -> Option<String> {
    match get_generic_password(SERVICE_NAME, key_path) {
        Ok(bytes) => {
            let passphrase = String::from_utf8(bytes.to_vec()).ok()?;
            debug!("Retrieved passphrase from Keychain for {}", key_path);
            Some(passphrase)
        }
        Err(e) => {
            debug!("No passphrase in Keychain for {}: {}", key_path, e);
            None
        }
    }
}

/// Store an SSH key passphrase in macOS Keychain.
pub fn set_passphrase(key_path: &str, passphrase: &str) -> Result<(), String> {
    // Delete existing entry if any (set_generic_password fails on duplicates)
    let _ = delete_generic_password(SERVICE_NAME, key_path);

    set_generic_password(SERVICE_NAME, key_path, passphrase.as_bytes())
        .map_err(|e| format!("Failed to store passphrase: {}", e))?;

    debug!("Stored passphrase in Keychain for {}", key_path);
    Ok(())
}

/// Delete an SSH key passphrase from macOS Keychain.
pub fn delete_passphrase(key_path: &str) -> Result<(), String> {
    delete_generic_password(SERVICE_NAME, key_path)
        .map_err(|e| format!("Failed to delete passphrase: {}", e))?;
    debug!("Deleted passphrase from Keychain for {}", key_path);
    Ok(())
}
```

- [ ] **Step 5: Verify everything compiles**

Run:
```bash
cd src-tauri && cargo build
```
Expected: compiles successfully. Warnings about unused code are OK at this stage.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/tunnel/connection.rs src-tauri/src/tunnel/forwarder.rs \
  src-tauri/src/tunnel/health.rs src-tauri/src/keychain.rs
git commit -m "feat: add SshConnection, PortForwarder, HealthMonitor, and Keychain"
```

---

### Task 6: Integrate real SSH into TunnelManager

**Files:**
- Modify: `src-tauri/src/tunnel/manager.rs`

Now we replace the TODO placeholder in `handle_connect` with real SSH connection, port forwarding, and health monitoring.

- [ ] **Step 1: Add SSH fields and imports, then update handle_connect**

First, add imports at the top of `src-tauri/src/tunnel/manager.rs`:
```rust
use std::sync::Arc;
use crate::tunnel::connection::SshConnection;
use crate::tunnel::forwarder::PortForwarder;
use crate::tunnel::health::HealthMonitor;
use crate::config::store::ConfigStore;
use crate::keychain;
```

Add `ssh_connection` field to `TunnelState` struct:
```rust
    ssh_connection: Option<Arc<SshConnection>>,
```
Initialize it as `None` in `TunnelState::new`.

Add `manager_tx` field to `TunnelManagerActor`:
```rust
    manager_tx: mpsc::Sender<ManagerCommand>,
```

Update `spawn_manager` to pass a clone of the sender to the actor:
```rust
pub fn spawn_manager(
    config: AppConfig,
    event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
) -> ManagerHandle {
    let (tx, rx) = mpsc::channel(32);
    let manager_tx = tx.clone();
    tokio::spawn(async move {
        let mut manager = TunnelManagerActor::new(config, event_tx, manager_tx);
        manager.run(rx).await;
    });
    tx
}
```

Update `TunnelManagerActor::new` to accept and store `manager_tx: mpsc::Sender<ManagerCommand>`.

Then replace the `handle_connect` method:
```rust
    async fn handle_connect(&mut self, id: &str) -> Result<(), TunnelError> {
        let tunnel = self
            .tunnels
            .get_mut(id)
            .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

        if tunnel.status == TunnelStatus::Connected || tunnel.status == TunnelStatus::Connecting {
            debug!("Tunnel {} already connected/connecting", id);
            return Ok(());
        }

        tunnel.status = TunnelStatus::Connecting;
        tunnel.error_message = None;
        self.emit_status(id, &TunnelStatus::Connecting);

        let config = tunnel.config.clone();
        let settings = self.settings.clone();

        // Resolve passphrase from Keychain
        let expanded_key_path = ConfigStore::expand_tilde(&config.key_path);
        let passphrase = keychain::get_passphrase(
            expanded_key_path.to_str().unwrap_or(&config.key_path),
        );

        // Connect SSH
        let ssh = match SshConnection::connect(
            &config.host,
            config.port,
            &config.user,
            &config.key_path,
            passphrase.as_deref(),
            settings.connection_timeout_secs,
        )
        .await
        {
            Ok(ssh) => Arc::new(ssh),
            Err(e) => {
                error!("SSH connection failed for tunnel {}: {}", id, e);
                let tunnel = self.tunnels.get_mut(id).unwrap();
                tunnel.status = TunnelStatus::Disconnected;
                tunnel.error_message = Some(e.to_string());
                self.emit_status(id, &TunnelStatus::Disconnected);
                return Err(e);
            }
        };

        // Create a death notification channel for this tunnel
        let (death_tx, mut death_rx) = tokio::sync::mpsc::channel::<String>(1);
        let manager_tx = self.manager_tx.clone();
        let tunnel_id = id.to_string();

        // Spawn death listener that sends TunnelDied to the manager
        tokio::spawn(async move {
            if let Some(error) = death_rx.recv().await {
                let _ = manager_tx
                    .send(ManagerCommand::TunnelDied {
                        id: tunnel_id,
                        error,
                    })
                    .await;
            }
        });

        // Spawn port forwarder
        let ssh_fwd = ssh.clone();
        let death_fwd = death_tx.clone();
        let fwd_id = id.to_string();
        let remote_host = config.remote_host.clone();
        let fwd_handle = tokio::spawn(async move {
            if let Err(e) = PortForwarder::start(
                ssh_fwd,
                config.local_port,
                remote_host,
                config.remote_port,
                death_fwd,
                fwd_id,
            )
            .await
            {
                error!("Port forwarder error: {}", e);
            }
        });

        // Spawn health monitor
        let ssh_health = ssh.clone();
        let death_health = death_tx.clone();
        let health_id = id.to_string();
        let health_handle = tokio::spawn(async move {
            HealthMonitor::run(
                ssh_health,
                health_id,
                settings.keepalive_interval_secs,
                settings.keepalive_timeout_secs,
                death_health,
            )
            .await;
        });

        // Store abort handles for cleanup
        let tunnel = self.tunnels.get_mut(id).unwrap();
        tunnel.status = TunnelStatus::Connected;
        // We need both handles — store them combined
        tunnel.abort_handle = Some(fwd_handle.abort_handle());
        // Store additional handles in a vec or abort both via a JoinSet
        // For simplicity, we'll abort the forwarder; health monitor will
        // notice the SSH session is gone and exit on its own
        tunnel.ssh_connection = Some(ssh);

        self.emit_status(id, &TunnelStatus::Connected);
        info!("Tunnel {} connected", id);
        Ok(())
    }
```

- [ ] **Step 2: Update handle_disconnect to properly close SSH**

Replace the disconnect method to also close the SSH session:
```rust
    async fn handle_disconnect(&mut self, id: &str) -> Result<(), TunnelError> {
        let tunnel = self
            .tunnels
            .get_mut(id)
            .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

        if tunnel.status == TunnelStatus::Disconnected {
            return Ok(());
        }

        tunnel.status = TunnelStatus::Disconnecting;
        self.emit_status(id, &TunnelStatus::Disconnecting);

        // Abort the tunnel's background tasks
        if let Some(handle) = tunnel.abort_handle.take() {
            handle.abort();
        }

        // Close the SSH session
        if let Some(ssh) = tunnel.ssh_connection.take() {
            ssh.disconnect().await;
        }

        tunnel.status = TunnelStatus::Disconnected;
        tunnel.error_message = None;
        self.emit_status(id, &TunnelStatus::Disconnected);

        info!("Tunnel {} disconnected", id);
        Ok(())
    }
```

- [ ] **Step 3: Verify compilation**

Run:
```bash
cd src-tauri && cargo build
```
Expected: compiles successfully.

- [ ] **Step 4: Update existing manager tests for real SSH integration**

The existing tests from Task 4 test `connect` which now attempts a real SSH connection. Since there is no SSH server in tests, update the test strategy:

1. Keep `list_tunnels_returns_all`, `connect_unknown_tunnel_returns_error`, `reload_config_adds_new_tunnels` — these don't depend on successful SSH.
2. Update `connect_transitions_to_connected` — this test will now fail because `handle_connect` tries real SSH. Convert it to test the **error path**: connect to a nonexistent host and verify the tunnel stays `Disconnected` with an error message.
3. `disconnect_transitions_to_disconnected` and `tunnel_died_transitions_through_error` — these need a connected tunnel. Since we can't connect in tests, remove these and note they are covered by manual testing (Task 11).

Updated test for connect error path:
```rust
#[tokio::test]
async fn connect_to_unreachable_host_returns_error() {
    let handle = spawn_manager(test_config(), None);

    let (reply_tx, reply_rx) = oneshot::channel();
    handle
        .send(ManagerCommand::Connect {
            id: "db".into(),
            reply: reply_tx,
        })
        .await
        .unwrap();

    // Should fail since there's no real SSH server
    let result = reply_rx.await.unwrap();
    assert!(result.is_err());

    // Verify tunnel is back to Disconnected (not stuck in Connecting)
    let (reply_tx, reply_rx) = oneshot::channel();
    handle
        .send(ManagerCommand::ListTunnels { reply: reply_tx })
        .await
        .unwrap();
    let tunnels = reply_rx.await.unwrap();
    let db = tunnels.iter().find(|t| t.id == "db").unwrap();
    assert_eq!(db.status, TunnelStatus::Disconnected);
    assert!(db.error_message.is_some());
}
```

Run:
```bash
cd src-tauri && cargo test tunnel::manager::tests -- --nocapture
```
Expected: all remaining tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/tunnel/manager.rs
git commit -m "feat: integrate real SSH connection into TunnelManager lifecycle"
```

---

## Chunk 3: Tauri Integration

### Task 7: Tauri command handlers

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement Tauri commands**

Write `src-tauri/src/commands.rs`:
```rust
use tauri::State;
use tokio::sync::oneshot;

use crate::config::store::ConfigStore;
use crate::errors::TunnelError;
use crate::tunnel::manager::{ManagerCommand, ManagerHandle};
use crate::types::TunnelInfo;

pub struct AppState {
    pub manager: ManagerHandle,
    pub config_store: ConfigStore,
}

#[tauri::command]
pub async fn list_tunnels(state: State<'_, AppState>) -> Result<Vec<TunnelInfo>, String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::ListTunnels { reply: reply_tx })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))
}

#[tauri::command]
pub async fn connect_tunnel(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::Connect {
            id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disconnect_tunnel(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::Disconnect {
            id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reload_config(state: State<'_, AppState>) -> Result<(), String> {
    let config = state
        .config_store
        .load()
        .map_err(|e| e.to_string())?;

    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::ReloadConfig {
            config,
            reply: reply_tx,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))?
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Wire up main.rs with system tray, managed state, and shutdown**

Update `src-tauri/src/main.rs`:
```rust
mod commands;
mod config;
mod errors;
mod keychain;
mod tunnel;
mod types;

use commands::AppState;
use config::store::ConfigStore;
use tunnel::manager::spawn_manager;
use types::TunnelStatusEvent;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("tunnel_master=debug")
        .init();

    let config_store = ConfigStore::new(ConfigStore::default_path());
    let config = config_store.load().unwrap_or_else(|e| {
        tracing::warn!("Could not load config: {}. Starting with empty config.", e);
        types::AppConfig {
            version: 1,
            tunnels: vec![],
            settings: types::Settings::default(),
        }
    });

    tauri::Builder::default()
        .setup(move |app| {
            // Create system tray
            let quit = MenuItem::with_id(app, "quit", "Quit Tunnel Master", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            // Spawn the TunnelManager actor
            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<TunnelStatusEvent>();
            let (error_tx, mut error_rx) = tokio::sync::mpsc::unbounded_channel::<types::TunnelErrorEvent>();
            let manager = spawn_manager(config, Some(event_tx), Some(error_tx));

            // Forward status events to Tauri frontend
            let app_handle = app.handle().clone();
            tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    let _ = app_handle.emit("tunnel-status-changed", &event);
                }
            });

            // Forward error events to Tauri frontend
            let app_handle2 = app.handle().clone();
            tokio::spawn(async move {
                while let Some(event) = error_rx.recv().await {
                    let _ = app_handle2.emit("tunnel-error", &event);
                }
            });

            // Store state for commands
            app.manage(AppState {
                manager,
                config_store,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_tunnels,
            commands::connect_tunnel,
            commands::disconnect_tunnel,
            commands::reload_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Verify compilation**

Run:
```bash
cd src-tauri && cargo build
```
Expected: compiles. Some warnings about unused items may appear.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat: add Tauri command handlers and system tray setup"
```

---

## Chunk 4: Frontend

### Task 8: TypeScript types and useTunnels hook

**Files:**
- Create: `src/types.ts`
- Create: `src/hooks/useTunnels.ts`

- [ ] **Step 1: Define TypeScript types**

Write `src/types.ts`:
```typescript
export type TunnelStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "error"
  | "disconnecting";

export interface TunnelInfo {
  id: string;
  name: string;
  status: TunnelStatus;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  errorMessage: string | null;
}

export interface TunnelStatusEvent {
  id: string;
  status: TunnelStatus;
  timestamp: number;
}

export interface TunnelErrorEvent {
  id: string;
  message: string;
  code: string;
}
```

- [ ] **Step 2: Implement useTunnels hook**

Create directory and write `src/hooks/useTunnels.ts`:
```typescript
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TunnelInfo, TunnelStatusEvent } from "../types";

export function useTunnels() {
  const [tunnels, setTunnels] = useState<TunnelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchTunnels = useCallback(async () => {
    try {
      const result = await invoke<TunnelInfo[]>("list_tunnels");
      setTunnels(result);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchTunnels();

    const unlisten = listen<TunnelStatusEvent>(
      "tunnel-status-changed",
      (_event) => {
        // Re-fetch full list on any status change for simplicity
        fetchTunnels();
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchTunnels]);

  const connect = useCallback(async (id: string) => {
    try {
      await invoke("connect_tunnel", { id });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const disconnect = useCallback(async (id: string) => {
    try {
      await invoke("disconnect_tunnel", { id });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const reload = useCallback(async () => {
    try {
      await invoke("reload_config");
      await fetchTunnels();
    } catch (e) {
      setError(String(e));
    }
  }, [fetchTunnels]);

  return { tunnels, loading, error, connect, disconnect, reload };
}
```

- [ ] **Step 3: Commit**

```bash
mkdir -p src/hooks
git add src/types.ts src/hooks/useTunnels.ts
git commit -m "feat: add TypeScript types and useTunnels hook"
```

---

### Task 9: Frontend components — TunnelItem, TunnelList, App

**Files:**
- Create: `src/components/TunnelItem.tsx`
- Create: `src/components/TunnelList.tsx`
- Modify: `src/App.tsx`
- Modify: `src/index.css`

- [ ] **Step 1: Create TunnelItem component**

Create `src/components/TunnelItem.tsx`:
```tsx
import type { TunnelInfo, TunnelStatus } from "../types";

interface TunnelItemProps {
  tunnel: TunnelInfo;
  onConnect: (id: string) => void;
  onDisconnect: (id: string) => void;
}

const STATUS_COLORS: Record<TunnelStatus, string> = {
  disconnected: "bg-gray-500",
  connecting: "bg-yellow-500 animate-pulse",
  connected: "bg-green-500",
  error: "bg-red-500",
  disconnecting: "bg-yellow-500",
};

const STATUS_LABELS: Record<TunnelStatus, string> = {
  disconnected: "Disconnected",
  connecting: "Connecting...",
  connected: "Connected",
  error: "Error",
  disconnecting: "Disconnecting...",
};

export function TunnelItem({ tunnel, onConnect, onDisconnect }: TunnelItemProps) {
  const isConnected = tunnel.status === "connected";
  const isBusy = tunnel.status === "connecting" || tunnel.status === "disconnecting";

  const handleToggle = () => {
    if (isBusy) return;
    if (isConnected) {
      onDisconnect(tunnel.id);
    } else {
      onConnect(tunnel.id);
    }
  };

  return (
    <div className="flex items-center justify-between px-3 py-2.5 hover:bg-white/5 rounded-lg transition-colors">
      <div className="flex items-center gap-3 min-w-0">
        <div className={`w-2 h-2 rounded-full shrink-0 ${STATUS_COLORS[tunnel.status]}`} />
        <div className="min-w-0">
          <div className="text-sm font-medium text-white truncate">{tunnel.name}</div>
          <div className="text-xs text-gray-400 truncate">
            :{tunnel.localPort} → {tunnel.remoteHost}:{tunnel.remotePort}
          </div>
          {tunnel.errorMessage && (
            <div className="text-xs text-red-400 truncate mt-0.5">{tunnel.errorMessage}</div>
          )}
        </div>
      </div>

      <button
        onClick={handleToggle}
        disabled={isBusy}
        className={`
          shrink-0 ml-3 px-3 py-1 rounded-md text-xs font-medium transition-colors
          ${isBusy ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}
          ${isConnected
            ? "bg-red-500/20 text-red-400 hover:bg-red-500/30"
            : "bg-green-500/20 text-green-400 hover:bg-green-500/30"
          }
        `}
      >
        {isBusy
          ? STATUS_LABELS[tunnel.status]
          : isConnected
          ? "Stop"
          : "Start"}
      </button>
    </div>
  );
}
```

- [ ] **Step 2: Create TunnelList component**

Create `src/components/TunnelList.tsx`:
```tsx
import type { TunnelInfo } from "../types";
import { TunnelItem } from "./TunnelItem";

interface TunnelListProps {
  tunnels: TunnelInfo[];
  onConnect: (id: string) => void;
  onDisconnect: (id: string) => void;
}

export function TunnelList({ tunnels, onConnect, onDisconnect }: TunnelListProps) {
  if (tunnels.length === 0) {
    return (
      <div className="py-8 text-center">
        <p className="text-gray-400 text-sm">No tunnels configured</p>
        <p className="text-gray-500 text-xs mt-1">
          Edit ~/.tunnel-master/config.json to add tunnels
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-0.5">
      {tunnels.map((tunnel) => (
        <TunnelItem
          key={tunnel.id}
          tunnel={tunnel}
          onConnect={onConnect}
          onDisconnect={onDisconnect}
        />
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Update App.tsx**

Replace `src/App.tsx`:
```tsx
import { TunnelList } from "./components/TunnelList";
import { useTunnels } from "./hooks/useTunnels";

function App() {
  const { tunnels, loading, error, connect, disconnect } = useTunnels();

  const connectedCount = tunnels.filter((t) => t.status === "connected").length;
  const totalCount = tunnels.length;

  return (
    <div className="min-h-screen bg-gray-900 text-white select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <div>
          <h1 className="text-sm font-semibold">Tunnel Master</h1>
          <p className="text-xs text-gray-400">
            {connectedCount}/{totalCount} active
          </p>
        </div>
      </div>

      {/* Error banner */}
      {error && (
        <div className="mx-3 mt-2 px-3 py-2 bg-red-500/20 border border-red-500/30 rounded-md">
          <p className="text-xs text-red-400">{error}</p>
        </div>
      )}

      {/* Content */}
      <div className="p-2">
        {loading ? (
          <div className="py-8 text-center">
            <p className="text-gray-400 text-sm">Loading...</p>
          </div>
        ) : (
          <TunnelList
            tunnels={tunnels}
            onConnect={connect}
            onDisconnect={disconnect}
          />
        )}
      </div>
    </div>
  );
}

export default App;
```

- [ ] **Step 4: Update index.css for popover styling**

Replace `src/index.css`:
```css
@import "tailwindcss";

:root {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  font-size: 14px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
}

body {
  margin: 0;
  padding: 0;
  overflow: hidden;
}
```

- [ ] **Step 5: Verify frontend builds**

Run:
```bash
npm run build
```
Expected: builds without errors. Output in `dist/`.

- [ ] **Step 6: Commit**

```bash
mkdir -p src/components
git add src/components/ src/App.tsx src/index.css
git commit -m "feat: add TunnelItem, TunnelList components and update App"
```

---

### Task 10: Tauri window config for popover behavior

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Configure tauri.conf.json for popover window**

Edit `src-tauri/tauri.conf.json`. The key settings:
- Window should be hidden by default (shown on tray click)
- Small fixed size suitable for a popover
- No decorations (frameless)
- Transparent background for popover feel

Set in the `"app"` > `"windows"` array:
```json
{
  "label": "main",
  "title": "Tunnel Master",
  "width": 320,
  "height": 400,
  "resizable": false,
  "decorations": false,
  "transparent": true,
  "visible": false,
  "skipTaskbar": true,
  "alwaysOnTop": true
}
```

Also ensure `"trayIcon"` permission is enabled in the capabilities.

- [ ] **Step 2: Update main.rs to toggle popover on tray icon click**

Add tray icon click handling to show/hide the main window positioned below the tray icon:
```rust
// In the .setup() closure, update the tray builder:
let _tray = TrayIconBuilder::new()
    .icon(app.default_window_icon().unwrap().clone())
    .menu(&menu)
    .on_menu_event(|app, event| {
        if event.id.as_ref() == "quit" {
            app.exit(0);
        }
    })
    .on_tray_icon_event(|tray, event| {
        if let tauri::tray::TrayIconEvent::Click { .. } = event {
            let app = tray.app_handle();
            if let Some(window) = app.get_webview_window("main") {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    // Position window near the tray icon
                    let _ = window.set_focus();
                    let _ = window.show();
                }
            }
        }
    })
    .build(app)?;
```

- [ ] **Step 3: Add dynamic tray icon status updates**

The spec requires the tray icon to reflect overall tunnel status (all connected / some / none). In the event forwarding loop in `main.rs`, after emitting each `tunnel-status-changed` event to the frontend, also update the tray icon tooltip to reflect the current aggregate status:

```rust
// In the event forwarding tokio::spawn block in main.rs setup:
let app_handle = app.handle().clone();
let manager_for_events = manager.clone();
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        let _ = app_handle.emit("tunnel-status-changed", &event);

        // Update tray icon tooltip with aggregate status
        if let Some(tray) = app_handle.tray_by_id("main") {
            // Query current tunnel states
            let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
            if manager_for_events
                .send(ManagerCommand::ListTunnels { reply: reply_tx })
                .await
                .is_ok()
            {
                if let Ok(tunnels) = reply_rx.await {
                    let connected = tunnels.iter().filter(|t| t.status == TunnelStatus::Connected).count();
                    let total = tunnels.len();
                    let tooltip = if connected == 0 {
                        "Tunnel Master — no active tunnels".to_string()
                    } else if connected == total {
                        format!("Tunnel Master — all {} tunnels active", total)
                    } else {
                        format!("Tunnel Master — {}/{} tunnels active", connected, total)
                    };
                    let _ = tray.set_tooltip(Some(&tooltip));
                }
            }
        }
    }
});
```

Note: Set the tray icon ID when building it by adding `.id("main")` to the `TrayIconBuilder` chain.

- [ ] **Step 4: Verify the app runs with popover behavior**

Run:
```bash
cargo tauri dev
```
Expected: App starts with tray icon. Clicking tray icon toggles the popover window. Window shows tunnel list (empty if no config file exists). Tray icon tooltip updates when tunnels connect/disconnect.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/src/main.rs
git commit -m "feat: configure popover window, tray icon toggle, and dynamic status tooltip"
```

---

### Task 11: End-to-end manual test and polish

**Files:**
- Create: `~/.tunnel-master/config.json` (user's home, not in repo)

- [ ] **Step 1: Create a test config file**

Copy `config.example.json` to `~/.tunnel-master/config.json` and edit with real tunnel details for your environment:
```bash
mkdir -p ~/.tunnel-master
cp config.example.json ~/.tunnel-master/config.json
# Edit with your actual SSH details
```

- [ ] **Step 2: Run the app and test tunnel lifecycle**

Run:
```bash
cargo tauri dev
```

Test checklist:
- [ ] Tray icon appears in menu bar
- [ ] Clicking tray icon shows popover with tunnel list
- [ ] Clicking tray icon again hides popover
- [ ] Each tunnel shows name, port mapping, and "Start" button
- [ ] Clicking "Start" transitions to "Connecting..." then "Connected" (green dot)
- [ ] Clicking "Stop" transitions to "Disconnecting..." then "Disconnected" (gray dot)
- [ ] While connected, verify the local port is actually forwarding (e.g., `psql -h localhost -p 5432`)
- [ ] Kill VPN/network to test graceful cleanup — tunnel should transition to Error then Disconnected
- [ ] Quit via tray menu disconnects all tunnels

- [ ] **Step 3: Fix any issues found during manual testing**

Address any compilation errors, UI glitches, or connection issues found in step 2.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete POC — Tunnel Master with SSH tunnel management"
```

---

## Summary

| Task | Description | Dependencies |
|------|-------------|-------------|
| 1 | Scaffold Tauri + Vite + React + Tailwind | None |
| 2 | Shared types and error definitions | Task 1 |
| 3 | ConfigStore (JSON read/validate) | Task 2 |
| 4 | TunnelManager actor (state machine) | Task 2 |
| 5 | SshConnection, PortForwarder, HealthMonitor, Keychain | Task 2 |
| 6 | Integrate real SSH into TunnelManager | Tasks 4, 5 |
| 7 | Tauri command handlers + main.rs wiring | Task 6 |
| 8 | TypeScript types + useTunnels hook | Task 1 |
| 9 | Frontend components (TunnelItem, TunnelList, App) | Task 8 |
| 10 | Popover window config + tray toggle | Task 7 |
| 11 | End-to-end manual testing | All |

**Parallelizable:** Tasks 3, 4, 5 can run in parallel after Task 2. Tasks 8, 9 can run in parallel with Tasks 5, 6, 7 (frontend is independent of backend).
