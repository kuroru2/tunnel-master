# SwiftUI Migration Phase 1: Rust Core Extraction

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the Rust backend into a standalone `rust-core/` crate with UniFFI annotations, decoupled from Tauri, while keeping the existing Tauri app working.

**Architecture:** Create a new Rust library crate (`rust-core/`) containing all tunnel management logic. Replace `tauri::AppHandle` event emission with a `TunnelEventHandler` callback trait annotated with `#[uniffi::export(with_foreign)]`. Replace `tauri::async_runtime::spawn` with `tokio::spawn`. Then make `src-tauri/` depend on `rust-core/` to validate the extraction without breaking the current app.

**Tech Stack:** Rust, UniFFI (proc macros), tokio, russh, keyring, serde

**Spec:** `docs/superpowers/specs/2026-03-21-swiftui-migration-design.md`

---

## File Map

### New files (rust-core/)

| File | Responsibility |
|------|---------------|
| `rust-core/Cargo.toml` | Crate manifest with uniffi, tokio, russh, keyring deps |
| `rust-core/uniffi.toml` | UniFFI config (Swift bindings, module name) |
| `rust-core/src/lib.rs` | Crate root, re-exports public API |
| `rust-core/src/api.rs` | `TunnelCore` uniffi::Object — single entry point for Swift |
| `rust-core/src/events.rs` | `TunnelEventHandler` trait + `KiPromptEntry` record |
| `rust-core/src/types.rs` | Shared types with UniFFI annotations |
| `rust-core/src/errors.rs` | `TunnelError` enum (copied, add uniffi::Error) |
| `rust-core/src/keychain.rs` | Credential store (copied verbatim) |
| `rust-core/src/config/mod.rs` | Module re-export |
| `rust-core/src/config/store.rs` | Config persistence (copied verbatim) |
| `rust-core/src/tunnel/mod.rs` | Module re-export |
| `rust-core/src/tunnel/manager.rs` | Tunnel manager actor (modified: callback trait replaces AppHandle) |
| `rust-core/src/tunnel/connection.rs` | SSH connection (modified: callback trait replaces AppHandle) |
| `rust-core/src/tunnel/forwarder.rs` | Port forwarding (copied verbatim) |
| `rust-core/src/tunnel/health.rs` | Health monitor (copied verbatim) |
| `rust-core/src/tunnel/traffic.rs` | Traffic sampling (modified: callback trait replaces AppHandle) |

### Modified files (src-tauri/)

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add `tunnel-core` as path dependency, remove duplicated deps |
| `src-tauri/src/lib.rs` | Import from `tunnel_core` instead of local modules |
| `src-tauri/src/commands.rs` | Import from `tunnel_core` instead of local modules |

---

## Task 1: Scaffold rust-core crate

**Files:**
- Create: `rust-core/Cargo.toml`
- Create: `rust-core/uniffi.toml`
- Create: `rust-core/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "tunnel-core"
version = "0.1.0"
edition = "2021"
rust-version = "1.77.2"

[lib]
crate-type = ["lib", "staticlib", "cdylib"]
name = "tunnel_core"

[dependencies]
uniffi = { version = "0.28", features = ["cli"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
russh = "0.46"
russh-keys = "0.46"
tracing = "0.1"
dirs = "5"
thiserror = "2"
async-trait = "0.1"

[target.'cfg(target_os = "macos")'.dependencies]
keyring = { version = "3", features = ["apple-native"] }

[target.'cfg(target_os = "windows")'.dependencies]
keyring = { version = "3", features = ["windows-native"] }

[target.'cfg(target_os = "linux")'.dependencies]
keyring = { version = "3", features = ["sync-secret-service", "crypto-rust"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create uniffi.toml**

```toml
[bindings.swift]
module_name = "TunnelCore"
```

- [ ] **Step 3: Create placeholder lib.rs**

```rust
pub mod api;
pub mod config;
pub mod errors;
pub mod events;
pub mod keychain;
pub mod tunnel;
pub mod types;

uniffi::setup_scaffolding!();
```

- [ ] **Step 4: Verify crate structure**

Run: `ls -R rust-core/`
Expected: Cargo.toml, uniffi.toml, src/lib.rs

- [ ] **Step 5: Commit**

```bash
git add rust-core/Cargo.toml rust-core/uniffi.toml rust-core/src/lib.rs
git commit -m "chore: scaffold rust-core crate with UniFFI setup"
```

---

## Task 2: Copy clean modules (types, errors, keychain, config)

**Files:**
- Create: `rust-core/src/types.rs` (copy from `src-tauri/src/types.rs`)
- Create: `rust-core/src/errors.rs` (copy from `src-tauri/src/errors.rs`)
- Create: `rust-core/src/keychain.rs` (copy from `src-tauri/src/keychain.rs`)
- Create: `rust-core/src/config/mod.rs` (copy from `src-tauri/src/config/mod.rs`)
- Create: `rust-core/src/config/store.rs` (copy from `src-tauri/src/config/store.rs`)

- [ ] **Step 1: Copy types.rs and add UniFFI annotations**

Copy `src-tauri/src/types.rs` to `rust-core/src/types.rs`. Then modify:

- Add `#[derive(uniffi::Enum)]` to `TunnelType`, `TunnelStatus`, `AuthMethod`
- Add `#[derive(uniffi::Record)]` to `TunnelConfig`, `TunnelInfo`, `Settings`, `AppConfig`
- Add `#[derive(uniffi::Record)]` to `TrafficSample` (from `tunnel/traffic.rs` — move it here)
- Remove `TunnelInput` (stays in Tauri layer — `commands.rs` uses it)
- Remove `TunnelStatusEvent`, `TunnelErrorEvent` (replaced by callback trait)
- Keep all `serde` derives — still needed for config JSON

Example for TunnelStatus:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "lowercase")]
pub enum TunnelStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
    Disconnecting,
}
```

Example for TunnelInfo:
```rust
#[derive(Debug, Clone, Serialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct TunnelInfo {
    pub id: String,
    pub name: String,
    pub status: TunnelStatus,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub error_message: Option<String>,
    pub auth_method: AuthMethod,
    pub jump_host_name: Option<String>,
    pub show_traffic_chart: bool,
}
```

Also add `TrafficSample` (moved from `tunnel/traffic.rs`):
```rust
#[derive(Debug, Clone, Serialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSample {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub timestamp: u64,
}
```

- [ ] **Step 2: Copy errors.rs and add UniFFI annotation**

Copy `src-tauri/src/errors.rs` to `rust-core/src/errors.rs`. Add `#[derive(uniffi::Error)]` to `TunnelError`:

```rust
#[derive(Debug, Error, Serialize, Clone, uniffi::Error)]
pub enum TunnelError {
    // ... all variants unchanged
}
```

Note: UniFFI Error requires all variants to be representable. The existing variants (all with String/u16 fields) are compatible.

- [ ] **Step 3: Copy keychain.rs verbatim**

Copy `src-tauri/src/keychain.rs` to `rust-core/src/keychain.rs`. No changes needed — already Tauri-free.

- [ ] **Step 4: Copy config/ verbatim**

Copy `src-tauri/src/config/mod.rs` and `src-tauri/src/config/store.rs` to `rust-core/src/config/`. No changes needed — already Tauri-free.

Note: `store.rs` references `crate::types::TunnelInput` in `validate_tunnel_input()`. Since we removed `TunnelInput` from types.rs, we have two options:
- Keep `TunnelInput` in rust-core types.rs (it's used by validate logic)
- Move `validate_tunnel_input` to only live in the Tauri layer

Decision: Keep `TunnelInput` and `validate_tunnel_input` in rust-core — the validation logic is useful for any frontend, not just Tauri. Don't add UniFFI annotations to `TunnelInput` though (Swift will use `TunnelConfig` directly).

- [ ] **Step 5: Verify compilation**

Run: `cd rust-core && cargo check 2>&1 | head -20`
Expected: Errors about missing `api.rs`, `events.rs`, `tunnel/` modules (not yet created). No errors from the copied modules.

Create stub files to unblock:
```rust
// rust-core/src/api.rs
// TODO: implement TunnelCore

// rust-core/src/events.rs
// TODO: implement TunnelEventHandler

// rust-core/src/tunnel/mod.rs
pub mod manager;
pub mod connection;
pub mod forwarder;
pub mod health;
pub mod traffic;
```

And empty stubs for tunnel/*.rs files.

- [ ] **Step 6: Commit**

```bash
git add rust-core/src/types.rs rust-core/src/errors.rs rust-core/src/keychain.rs rust-core/src/config/
git commit -m "feat(rust-core): copy clean modules — types, errors, keychain, config"
```

---

## Task 3: Create events.rs — TunnelEventHandler callback trait

**Files:**
- Create: `rust-core/src/events.rs`

- [ ] **Step 1: Write events.rs**

```rust
use std::sync::Arc;
use crate::types::{TrafficSample, TunnelStatus};

/// Entry for a keyboard-interactive prompt.
#[derive(Debug, Clone, uniffi::Record)]
pub struct KiPromptEntry {
    pub text: String,
    pub echo: bool,
}

/// Callback trait implemented by the foreign (Swift) side.
/// Rust calls these methods to push state changes, auth prompts, and traffic updates.
///
/// IMPORTANT: All methods are called from background tokio tasks.
/// The Swift implementation MUST use `Task { @MainActor in ... }` for UI updates
/// and MUST NOT synchronously call back into Rust (deadlock risk).
#[uniffi::export(with_foreign)]
pub trait TunnelEventHandler: Send + Sync {
    fn on_tunnel_state_changed(&self, id: String, status: TunnelStatus, error_message: Option<String>);
    fn on_passphrase_requested(&self, id: String, key_path: String);
    fn on_password_requested(&self, id: String);
    fn on_host_key_verification(&self, id: String, host: String, port: u16, key_type: String, fingerprint: String);
    fn on_keyboard_interactive(
        &self,
        id: String,
        name: String,
        instructions: String,
        prompts: Vec<KiPromptEntry>,
    );
    fn on_traffic_update(&self, id: String, sample: TrafficSample);
    fn on_error(&self, id: String, message: String);
}
```

Note: `on_tunnel_state_changed` includes `error_message` so the Swift side can update both status and error in one callback (avoids race between separate state + error events). `on_host_key_verification` includes `host` and `port` so the Swift side can show them in the dialog and pass them back to `accept_host_key`.

- [ ] **Step 2: Verify compilation**

Run: `cd rust-core && cargo check 2>&1 | head -20`
Expected: events.rs compiles cleanly.

- [ ] **Step 3: Commit**

```bash
git add rust-core/src/events.rs
git commit -m "feat(rust-core): add TunnelEventHandler callback trait with UniFFI"
```

---

## Task 4: Copy and modify tunnel/traffic.rs

**Files:**
- Create: `rust-core/src/tunnel/traffic.rs` (modified copy)

- [ ] **Step 1: Copy and modify traffic.rs**

Copy `src-tauri/src/tunnel/traffic.rs` to `rust-core/src/tunnel/traffic.rs`. Changes:

1. Remove `TrafficSample` struct (moved to `types.rs` in Task 2)
2. Remove `TrafficEvent` struct (replaced by callback)
3. Replace `TrafficSampler::run` signature — `app_handle: tauri::AppHandle` becomes `event_handler: Arc<dyn crate::events::TunnelEventHandler>`
4. Replace `app_handle.emit("tunnel-traffic", &event)` with `event_handler.on_traffic_update(tunnel_id.clone(), sample.clone())`
5. Remove `use tauri::Emitter;`
6. Import from `crate::types::TrafficSample` instead of local definition

```rust
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::debug;

use crate::events::TunnelEventHandler;
use crate::types::TrafficSample;

// TrafficCounters, TrafficHistory, new_traffic_history — unchanged

pub struct TrafficSampler;

impl TrafficSampler {
    pub async fn run(
        tunnel_id: String,
        counters: Arc<TrafficCounters>,
        history: TrafficHistory,
        event_handler: Arc<dyn TunnelEventHandler>,
    ) {
        let interval = std::time::Duration::from_secs(1);

        loop {
            tokio::time::sleep(interval).await;

            let (bytes_in, bytes_out) = counters.take();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let sample = TrafficSample {
                bytes_in,
                bytes_out,
                timestamp,
            };

            // Push to ring buffer
            {
                let mut buf = history.lock().unwrap();
                if buf.len() >= 60 {
                    buf.pop_front();
                }
                buf.push_back(sample.clone());
            }

            // Push to frontend via callback
            event_handler.on_traffic_update(tunnel_id.clone(), sample);

            debug!(
                "Traffic sample for {}: in={} out={}",
                tunnel_id, bytes_in, bytes_out
            );
        }
    }
}
```

- [ ] **Step 2: Run existing traffic tests**

Run: `cd rust-core && cargo test --lib tunnel::traffic 2>&1`
Expected: `counters_increment_and_take`, `counters_concurrent_increment`, `traffic_history_capacity` all pass (they don't depend on Tauri or the sampler).

- [ ] **Step 3: Commit**

```bash
git add rust-core/src/tunnel/traffic.rs
git commit -m "feat(rust-core): extract traffic module — replace AppHandle with callback"
```

---

## Task 5: Copy and modify tunnel/connection.rs

**Files:**
- Create: `rust-core/src/tunnel/connection.rs` (modified copy)

- [ ] **Step 1: Copy and modify connection.rs**

Copy `src-tauri/src/tunnel/connection.rs` to `rust-core/src/tunnel/connection.rs`. Changes:

1. In `AuthCredentials::KeyboardInteractive`, replace `app_handle: tauri::AppHandle` with `event_handler: Arc<dyn crate::events::TunnelEventHandler>`
2. In `authenticate()`, for the `KeyboardInteractive` branch:
   - Remove `use tauri::Emitter;`
   - Replace `app_handle.emit("keyboard-interactive-prompt", &prompt)` with:
     ```rust
     event_handler.on_keyboard_interactive(
         tunnel_id.clone(),
         name,
         instructions,
         prompts.iter().map(|p| crate::events::KiPromptEntry {
             text: p.prompt.clone(),
             echo: p.echo,
         }).collect(),
     );
     ```
3. Remove the local `KeyboardInteractivePrompt` and `KiPromptEntry` structs (moved to `events.rs`)
4. Import `crate::events::KiPromptEntry`

The `AuthCredentials` enum becomes:
```rust
pub enum AuthCredentials {
    Key {
        key_path: String,
        passphrase: Option<String>,
    },
    Password(String),
    Agent,
    KeyboardInteractive {
        ki_slot: KiResponseSlot,
        event_handler: Arc<dyn crate::events::TunnelEventHandler>,
        tunnel_id: String,
    },
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd rust-core && cargo check 2>&1 | head -20`
Expected: connection.rs compiles. No Tauri imports remain.

- [ ] **Step 3: Commit**

```bash
git add rust-core/src/tunnel/connection.rs
git commit -m "feat(rust-core): extract connection module — replace AppHandle with callback"
```

---

## Task 6: Copy and modify tunnel/manager.rs

**Files:**
- Create: `rust-core/src/tunnel/manager.rs` (modified copy)

This is the largest change. The manager actor uses `tauri::AppHandle` for event emission and `tauri::async_runtime::spawn` for the actor task.

- [ ] **Step 1: Copy and modify manager.rs**

Copy `src-tauri/src/tunnel/manager.rs` to `rust-core/src/tunnel/manager.rs`. Changes:

1. **Replace `tauri::async_runtime::spawn` with `tokio::spawn`** in `spawn_manager()`

2. **Replace `app_handle` parameter with `event_handler`** in `spawn_manager()` and `TunnelManagerActor`:
   ```rust
   pub fn spawn_manager(
       config: AppConfig,
       event_handler: Arc<dyn TunnelEventHandler>,
   ) -> ManagerHandle {
       let (tx, rx) = mpsc::channel(32);
       let manager_tx = tx.clone();

       tokio::spawn(async move {
           let mut manager = TunnelManagerActor::new(config, event_handler, manager_tx);
           manager.run(rx).await;
       });

       tx
   }
   ```

3. **Simplify `TunnelManagerActor` struct** — remove `event_tx`, `error_tx`, `app_handle`. Add `event_handler`:
   ```rust
   struct TunnelManagerActor {
       tunnels: HashMap<String, TunnelState>,
       tunnel_order: Vec<String>,
       event_handler: Arc<dyn TunnelEventHandler>,
       manager_tx: mpsc::Sender<ManagerCommand>,
       settings: Settings,
   }
   ```

4. **Replace `emit_status()`**:
   ```rust
   fn emit_status(&self, id: &str, status: &TunnelStatus) {
       let error_message = self.tunnels.get(id).and_then(|t| t.error_message.clone());
       self.event_handler.on_tunnel_state_changed(
           id.to_string(),
           status.clone(),
           error_message,
       );
   }
   ```

5. **Replace `emit_error()`**:
   ```rust
   fn emit_error(&self, id: &str, message: &str, _code: &str) {
       self.event_handler.on_error(id.to_string(), message.to_string());
   }
   ```

6. **Update `build_credentials()` for KeyboardInteractive**:
   ```rust
   AuthMethod::KeyboardInteractive => {
       let ki_slot: KiResponseSlot = Arc::new(std::sync::Mutex::new(None));
       let tunnel = self.tunnels.get_mut(tunnel_id).unwrap();
       tunnel.ki_slot = Some(ki_slot.clone());
       Ok(AuthCredentials::KeyboardInteractive {
           ki_slot,
           event_handler: self.event_handler.clone(),
           tunnel_id: tunnel_id.to_string(),
       })
   }
   ```
   (Remove the `app_handle.clone().ok_or_else(...)` guard — event_handler is always present.)

7. **Update traffic sampler spawn** in `handle_connect()`:
   ```rust
   let sampler_event_handler = self.event_handler.clone();
   let sampler_counters = traffic_counters.clone();
   let sampler_history = {
       let tunnel = self.tunnels.get(id).unwrap();
       tunnel.traffic_history.clone()
   };
   let sampler_tunnel_id = id.to_string();
   let sampler_handle = tokio::spawn(async move {
       traffic::TrafficSampler::run(
           sampler_tunnel_id,
           sampler_counters,
           sampler_history,
           sampler_event_handler,
       ).await;
   });
   ```
   (Remove the `if let Some(sampler_app_handle)` guard — always spawn sampler.)

8. **Handle PasswordRequired error** — when `build_credentials` returns `PasswordRequired`, emit `on_password_requested` callback:
   In `handle_connect()`, after the credentials error handling for direct connection:
   ```rust
   Err(TunnelError::PasswordRequired(tid)) => {
       self.event_handler.on_password_requested(tid);
       // Don't set error state — waiting for user input
       return Ok(());
   }
   ```
   Similarly for the HostKeyUnknown error:
   ```rust
   Err(TunnelError::HostKeyUnknown { host, port, key_type, fingerprint }) => {
       self.event_handler.on_host_key_verification(
           id.to_string(), host, port, key_type, fingerprint,
       );
       return Ok(());
   }
   ```

- [ ] **Step 2: Run existing manager tests**

Run: `cd rust-core && cargo test --lib tunnel::manager 2>&1`
Expected: The existing tests need updating — `spawn_manager` signature changed. Update test helper:
```rust
fn test_config() -> AppConfig { /* unchanged */ }

struct NoopEventHandler;
impl TunnelEventHandler for NoopEventHandler {
    fn on_tunnel_state_changed(&self, _: String, _: TunnelStatus, _: Option<String>) {}
    fn on_passphrase_requested(&self, _: String, _: String) {}
    fn on_password_requested(&self, _: String) {}
    fn on_host_key_verification(&self, _: String, _: String, _: u16, _: String, _: String) {}
    fn on_keyboard_interactive(&self, _: String, _: String, _: String, _: Vec<KiPromptEntry>) {}
    fn on_traffic_update(&self, _: String, _: TrafficSample) {}
    fn on_error(&self, _: String, _: String) {}
}

// Then in tests:
let handler = Arc::new(NoopEventHandler);
let handle = spawn_manager(test_config(), handler);
```

Run tests again. Expected: `list_tunnels_returns_all`, `connect_unknown_tunnel_returns_error`, `reload_config_adds_new_tunnels`, `connect_to_unreachable_host_returns_error` all pass.

- [ ] **Step 3: Commit**

```bash
git add rust-core/src/tunnel/manager.rs
git commit -m "feat(rust-core): extract manager module — replace AppHandle with callback trait"
```

---

## Task 7: Copy clean tunnel modules (forwarder, health)

**Files:**
- Create: `rust-core/src/tunnel/forwarder.rs` (copy verbatim)
- Create: `rust-core/src/tunnel/health.rs` (copy verbatim)
- Create: `rust-core/src/tunnel/mod.rs`

- [ ] **Step 1: Copy forwarder.rs and health.rs verbatim**

These modules have zero Tauri dependencies. Copy directly.

- [ ] **Step 2: Create tunnel/mod.rs**

```rust
pub mod connection;
pub mod forwarder;
pub mod health;
pub mod manager;
pub mod traffic;
```

- [ ] **Step 3: Verify full crate compiles**

Run: `cd rust-core && cargo check 2>&1`
Expected: Clean compilation (or minor fixups needed).

- [ ] **Step 4: Run all tests**

Run: `cd rust-core && cargo test 2>&1`
Expected: All existing tests pass (types, errors, config/store, keychain, manager, traffic).

- [ ] **Step 5: Commit**

```bash
git add rust-core/src/tunnel/
git commit -m "feat(rust-core): copy forwarder and health modules, complete tunnel extraction"
```

---

## Task 8: Create api.rs — TunnelCore entry point

**Files:**
- Create: `rust-core/src/api.rs`

- [ ] **Step 1: Write TunnelCore struct**

`TunnelCore` is the UniFFI Object that wraps the manager handle and config store. It provides the public API surface for Swift.

```rust
use std::sync::{Arc, Mutex};

use tokio::sync::oneshot;

use crate::config::store::{generate_id, validate_tunnel_input, ConfigStore};
use crate::errors::TunnelError;
use crate::events::TunnelEventHandler;
use crate::keychain;
use crate::tunnel::connection::accept_pending_host_key;
use crate::tunnel::manager::{ManagerCommand, ManagerHandle, spawn_manager};
use crate::types::{AppConfig, TrafficSample, TunnelConfig, TunnelInfo, TunnelInput};

#[derive(uniffi::Object)]
pub struct TunnelCore {
    manager: ManagerHandle,
    config_store: Mutex<ConfigStore>,
    runtime: tokio::runtime::Handle,
}

#[uniffi::export]
impl TunnelCore {
    /// Create a new TunnelCore. Loads config and starts the tunnel manager.
    /// `event_handler` is the Swift-side callback implementation.
    #[uniffi::constructor]
    pub fn new(event_handler: Arc<dyn TunnelEventHandler>) -> Self {
        // Get or create a tokio runtime
        let runtime = tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                let handle = rt.handle().clone();
                std::thread::spawn(move || rt.block_on(std::future::pending::<()>()));
                handle
            });

        let config_store = ConfigStore::new(ConfigStore::default_path());
        let config = config_store.load().unwrap_or_else(|_| AppConfig {
            version: 1,
            tunnels: vec![],
            settings: crate::types::Settings::default(),
        });

        let manager = runtime.block_on(async {
            spawn_manager(config, event_handler)
        });

        Self {
            manager,
            config_store: Mutex::new(config_store),
            runtime,
        }
    }

    // -- Tunnel state --

    pub fn list_tunnels(&self) -> Vec<TunnelInfo> {
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            self.manager.send(ManagerCommand::ListTunnels { reply: tx }).await
        });
        self.runtime.block_on(async { rx.await.unwrap_or_default() })
    }

    pub fn connect(&self, id: String) {
        let manager = self.manager.clone();
        self.runtime.spawn(async move {
            let (tx, rx) = oneshot::channel();
            let _ = manager.send(ManagerCommand::Connect { id, reply: tx }).await;
            let _ = rx.await;
        });
    }

    pub fn disconnect(&self, id: String) {
        let manager = self.manager.clone();
        self.runtime.spawn(async move {
            let (tx, rx) = oneshot::channel();
            let _ = manager.send(ManagerCommand::Disconnect { id, reply: tx }).await;
            let _ = rx.await;
        });
    }

    // -- Config CRUD --

    pub fn get_tunnel_config(&self, id: String) -> Option<TunnelConfig> {
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            self.manager.send(ManagerCommand::GetTunnelConfig { id, reply: tx }).await
        });
        self.runtime.block_on(async { rx.await.ok().and_then(|r| r.ok()) })
    }

    pub fn add_tunnel(&self, config: TunnelConfig) {
        // Persist to disk first (load-mutate-save pattern from commands.rs)
        {
            let store = self.config_store.lock().unwrap();
            if let Ok(mut app_config) = store.load() {
                app_config.tunnels.push(config.clone());
                let _ = store.save(&app_config);
            }
        }
        // Then add to manager
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::AddTunnel { config, reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn update_tunnel(&self, id: String, config: TunnelConfig) {
        // Persist to disk
        {
            let store = self.config_store.lock().unwrap();
            if let Ok(mut app_config) = store.load() {
                if let Some(pos) = app_config.tunnels.iter().position(|t| t.id == id) {
                    app_config.tunnels[pos] = config.clone();
                    let _ = store.save(&app_config);
                }
            }
        }
        // Update in manager
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::UpdateTunnel { config, reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn delete_tunnel(&self, id: String) {
        // Persist to disk
        {
            let store = self.config_store.lock().unwrap();
            if let Ok(mut app_config) = store.load() {
                app_config.tunnels.retain(|t| t.id != id);
                let _ = store.save(&app_config);
            }
        }
        // Remove from manager
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::RemoveTunnel { id, reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn reorder_tunnels(&self, ids: Vec<String>) {
        // Persist to disk
        {
            let store = self.config_store.lock().unwrap();
            if let Ok(mut app_config) = store.load() {
                let mut reordered = Vec::new();
                for id in &ids {
                    if let Some(tc) = app_config.tunnels.iter().find(|t| &t.id == id) {
                        reordered.push(tc.clone());
                    }
                }
                app_config.tunnels = reordered;
                let _ = store.save(&app_config);
            }
        }
        // Update manager ordering
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::ReorderTunnels { ids, reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn reload_config(&self) {
        let store = self.config_store.lock().unwrap();
        if let Ok(config) = store.load() {
            let manager = self.manager.clone();
            let (tx, rx) = oneshot::channel();
            let _ = self.runtime.block_on(async {
                manager.send(ManagerCommand::ReloadConfig { config, reply: tx }).await
            });
            let _ = self.runtime.block_on(async { rx.await });
        }
    }

    // -- Traffic --

    pub fn get_traffic_history(&self, id: String) -> Vec<TrafficSample> {
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            self.manager.send(ManagerCommand::GetTrafficHistory { id, reply: tx }).await
        });
        self.runtime.block_on(async {
            rx.await.ok().and_then(|r| r.ok()).unwrap_or_default()
        })
    }

    // -- Auth responses --

    pub fn accept_host_key(&self, host: String, port: u16) {
        let _ = accept_pending_host_key(&host, port);
    }

    pub fn submit_passphrase(&self, id: String, passphrase: String) {
        // Store in keychain and reconnect
        let key_path = self.get_key_path(&id);
        if let Some(kp) = key_path {
            let _ = keychain::set_passphrase(&kp, &passphrase);
        }
        self.connect(id);
    }

    pub fn submit_password(&self, id: String, password: String) {
        let _ = keychain::store_password(&id, &password);
        self.connect(id);
    }

    pub fn respond_keyboard_interactive(&self, id: String, responses: Vec<String>) {
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::RespondKeyboardInteractive {
                id, responses, reply: tx,
            }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn cancel_auth(&self, id: String) {
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::CancelKeyboardInteractive {
                id, reply: tx,
            }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    // -- Keychain --

    pub fn store_passphrase(&self, id: String, passphrase: String) {
        let key_path = self.get_key_path(&id);
        if let Some(kp) = key_path {
            let _ = keychain::set_passphrase(&kp, &passphrase);
        }
    }

    pub fn store_password(&self, id: String, password: String) {
        let _ = keychain::store_password(&id, &password);
    }

    pub fn clear_credential(&self, id: String) {
        keychain::delete_password(&id);
    }

    // -- Lifecycle --

    pub fn shutdown(&self) {
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::Shutdown { reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }
}

impl TunnelCore {
    fn get_key_path(&self, id: &str) -> Option<String> {
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            self.manager.send(ManagerCommand::GetKeyPath {
                id: id.to_string(),
                reply: tx,
            }).await
        });
        self.runtime.block_on(async {
            rx.await.ok().and_then(|r| r.ok())
        })
    }

}
```

Config persistence uses the same load-mutate-save pattern as the current `commands.rs`. Each CRUD method loads the config from disk, applies the change, saves, then sends the command to the manager.

- [ ] **Step 2: Verify compilation**

Run: `cd rust-core && cargo check 2>&1`
Expected: Clean compilation.

- [ ] **Step 3: Commit**

```bash
git add rust-core/src/api.rs
git commit -m "feat(rust-core): add TunnelCore API entry point with UniFFI annotations"
```

---

## Task 9: Finalize lib.rs and verify full crate

**Files:**
- Modify: `rust-core/src/lib.rs`

- [ ] **Step 1: Update lib.rs with proper exports**

```rust
pub mod api;
pub mod config;
pub mod errors;
pub mod events;
pub mod keychain;
pub mod tunnel;
pub mod types;

pub use api::TunnelCore;
pub use events::TunnelEventHandler;
pub use types::*;

uniffi::setup_scaffolding!();
```

- [ ] **Step 2: Run full test suite**

Run: `cd rust-core && cargo test 2>&1`
Expected: All tests pass:
- `types::tests::*` (6 tests)
- `errors::tests::*` (4 tests)
- `config::store::tests::*` (12 tests)
- `tunnel::traffic::tests::*` (3 tests)
- `tunnel::manager::tests::*` (4 tests)

- [ ] **Step 3: Verify UniFFI scaffolding generates**

Run: `cd rust-core && cargo build 2>&1`
Expected: Clean build. UniFFI scaffolding compiles.

- [ ] **Step 4: Commit**

```bash
git add rust-core/src/lib.rs
git commit -m "feat(rust-core): finalize lib.rs exports and UniFFI scaffolding"
```

---

## Task 10: Wire src-tauri to depend on rust-core

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

This task validates the extraction — the Tauri app should still work after importing from `tunnel_core` instead of local modules.

- [ ] **Step 1: Add rust-core as path dependency in src-tauri/Cargo.toml**

Add under `[dependencies]`:
```toml
tunnel-core = { path = "../rust-core" }
```

- [ ] **Step 2: Update src-tauri imports to use tunnel_core**

In `src-tauri/src/commands.rs`, replace:
```rust
use crate::config::store::{generate_id, validate_tunnel_input, ConfigStore};
use crate::keychain;
use crate::tunnel::connection::accept_pending_host_key;
use crate::tunnel::manager::{ManagerCommand, ManagerHandle};
use crate::tunnel::traffic::TrafficSample;
use crate::types::{TunnelConfig, TunnelInfo, TunnelInput};
```
With:
```rust
use tunnel_core::config::store::{generate_id, validate_tunnel_input, ConfigStore};
use tunnel_core::keychain;
use tunnel_core::tunnel::connection::accept_pending_host_key;
use tunnel_core::tunnel::manager::{ManagerCommand, ManagerHandle};
use tunnel_core::types::{TrafficSample, TunnelConfig, TunnelInfo, TunnelInput};
```

Similarly update `src-tauri/src/lib.rs` to import from `tunnel_core`.

- [ ] **Step 3: Remove duplicated modules from src-tauri**

Delete from `src-tauri/src/`:
- `types.rs`
- `errors.rs`
- `keychain.rs`
- `config/` (entire directory)
- `tunnel/` (entire directory)

Keep only:
- `lib.rs` (Tauri app setup, NSPanel, tray)
- `commands.rs` (Tauri IPC commands)

- [ ] **Step 4: Build and test the Tauri app**

Run: `cd src-tauri && cargo build 2>&1`
Expected: Tauri app builds successfully with `tunnel_core` dependency.

Run: `cd src-tauri && cargo test 2>&1`
Expected: Any remaining Tauri-layer tests pass.

- [ ] **Step 5: Smoke test the running app**

Run: `npm run tauri dev`
Expected: App launches, tray icon appears, tunnel list loads. Basic connect/disconnect still works if you have a test server.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/src/commands.rs
git rm -r src-tauri/src/types.rs src-tauri/src/errors.rs src-tauri/src/keychain.rs src-tauri/src/config/ src-tauri/src/tunnel/
git commit -m "refactor: wire src-tauri to depend on rust-core, remove duplicated modules"
```

---

## Task 11: Verify UniFFI Swift package generation

**Files:** None (verification only)

- [ ] **Step 1: Install cargo-swift if not present**

Run: `cargo install cargo-swift 2>&1`

- [ ] **Step 2: Generate Swift package**

Run: `cd rust-core && cargo swift package --name TunnelCore 2>&1`
Expected: Generates a Swift Package with:
- `TunnelCore.swift` (generated bindings)
- `TunnelCoreFFI.xcframework` (compiled library)
- `Package.swift`

This validates that all UniFFI annotations are correct and the Swift bindings generate successfully.

- [ ] **Step 3: Inspect generated Swift bindings**

Check that the generated `TunnelCore.swift` contains:
- `class TunnelCore` with `init(eventHandler:)` constructor
- `protocol TunnelEventHandler` with all callback methods
- `struct TunnelInfo`, `struct TunnelConfig`, `struct TrafficSample` records
- `enum TunnelStatus`, `enum AuthMethod` enums

Run: `grep -c "class TunnelCore\|protocol TunnelEventHandler\|struct TunnelInfo\|enum TunnelStatus" <path-to-generated>/TunnelCore.swift`
Expected: 4 matches (one per type)

- [ ] **Step 4: Commit (if any config adjustments were needed)**

```bash
git add rust-core/
git commit -m "chore: verify UniFFI Swift package generation"
```

---

## Summary

After completing all 11 tasks:
- `rust-core/` is a standalone Rust crate with UniFFI annotations
- All core logic (tunnel management, SSH, config, keychain, traffic) is extracted
- `tauri::AppHandle` is fully replaced by `TunnelEventHandler` callback trait
- `src-tauri/` depends on `rust-core/` and still works
- `cargo swift package` generates valid Swift bindings

**Next:** Phase 2 plan (Scaffold SwiftUI App) — creates the Xcode project and gets a basic tunnel list displaying.
