# Config Editor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add in-app CRUD for tunnel configurations — add, edit, delete tunnels via an iOS-style edit mode without touching the JSON file.

**Architecture:** New Tauri commands (`add_tunnel`, `update_tunnel`, `delete_tunnel`, `get_tunnel_config`) backed by new manager commands. Frontend gains two new views (EditList, EditForm) managed by a view state machine in App.tsx. ConfigStore gets atomic `save()`. All validation on the Rust side.

**Tech Stack:** Rust/Tauri (backend), React/TypeScript/Tailwind (frontend)

**Spec:** `docs/superpowers/specs/2026-03-11-config-editor-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/src/types.rs` | Modify | Add `TunnelInput` struct |
| `src-tauri/src/config/store.rs` | Modify | Add `save()` with atomic write, add `slugify()` and `generate_id()` |
| `src-tauri/src/tunnel/manager.rs` | Modify | Add `AddTunnel`, `UpdateTunnel`, `RemoveTunnel`, `GetTunnelConfig` commands |
| `src-tauri/src/commands.rs` | Modify | Add `add_tunnel`, `update_tunnel`, `delete_tunnel`, `get_tunnel_config` commands |
| `src-tauri/src/lib.rs` | Modify | Register new commands in `invoke_handler` |
| `src/types.ts` | Modify | Add `TunnelInput` and `TunnelConfig` types |
| `src/hooks/useTunnels.ts` | Modify | Add CRUD functions |
| `src/App.tsx` | Modify | View state machine, conditional rendering |
| `src/components/EditList.tsx` | Create | iOS-style edit mode tunnel list |
| `src/components/EditForm.tsx` | Create | Grouped form for add/edit tunnel |

---

## Chunk 1: Backend — Types, ConfigStore, Manager

### Task 1: Add `TunnelInput` type and `slugify`/`generate_id` helpers

**Files:**
- Modify: `src-tauri/src/types.rs`
- Modify: `src-tauri/src/config/store.rs`

- [ ] **Step 1: Write tests for slugify and generate_id**

Add to `src-tauri/src/config/store.rs` tests module:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test config::store::tests -- --nocapture`
Expected: FAIL — `slugify` and `generate_id` not found

- [ ] **Step 3: Implement slugify and generate_id**

Add to `src-tauri/src/config/store.rs` (above `impl ConfigStore`):

```rust
pub fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse consecutive hyphens and trim
    let mut result = String::new();
    let mut prev_hyphen = true; // start true to trim leading
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test config::store::tests -- --nocapture`
Expected: PASS

- [ ] **Step 5: Add `TunnelInput` type**

Add to `src-tauri/src/types.rs` after the `TunnelConfig` struct:

```rust
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
```

Also add a `to_config` method on `TunnelInput`:

```rust
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
```

Add test:

```rust
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
```

- [ ] **Step 6: Run all tests**

Run: `cd src-tauri && cargo test -- --nocapture`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/config/store.rs
git commit -m "feat: add TunnelInput type and slugify/generate_id helpers"
```

---

### Task 2: Add `save()` to ConfigStore with atomic write

**Files:**
- Modify: `src-tauri/src/config/store.rs`

- [ ] **Step 1: Write test for save**

Add to `src-tauri/src/config/store.rs` tests module:

```rust
#[test]
fn save_and_reload() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.json");
    fs::write(&path, sample_config_json()).unwrap();

    let store = ConfigStore::new(path);
    let mut config = store.load().unwrap();

    // Modify and save
    config.tunnels[0].name = "Modified".to_string();
    store.save(&config).unwrap();

    // Reload and verify
    let reloaded = store.load().unwrap();
    assert_eq!(reloaded.tunnels[0].name, "Modified");
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test config::store::tests::save -- --nocapture`
Expected: FAIL — `save` method not found

- [ ] **Step 3: Implement save**

Add to `impl ConfigStore` in `src-tauri/src/config/store.rs`:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test config::store::tests -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config/store.rs
git commit -m "feat: add ConfigStore::save() with atomic write"
```

---

### Task 3: Add validation helper

**Files:**
- Modify: `src-tauri/src/config/store.rs`

- [ ] **Step 1: Write tests for validation**

Add to `src-tauri/src/config/store.rs` tests module:

```rust
use crate::types::TunnelInput;

#[test]
fn validate_input_valid() {
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
    assert!(validate_tunnel_input(&input, &[], None).is_ok());
}

#[test]
fn validate_input_empty_name() {
    let input = TunnelInput {
        name: "".to_string(),
        host: "example.com".to_string(),
        port: 22,
        user: "user".to_string(),
        key_path: "".to_string(),
        local_port: 5432,
        remote_host: "db.internal".to_string(),
        remote_port: 5432,
        auto_connect: false,
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
        local_port: 5432,
        remote_host: "db.internal".to_string(),
        remote_port: 5432,
        auto_connect: false,
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
        local_port: 5432,
        remote_host: "db.internal".to_string(),
        remote_port: 5432,
        auto_connect: false,
    };
    let existing = vec![("self-id".to_string(), 5432u16)];
    // When updating "self-id", its own port should not conflict
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
        local_port: 0,
        remote_host: "db.internal".to_string(),
        remote_port: 5432,
        auto_connect: false,
    };
    let err = validate_tunnel_input(&input, &[], None).unwrap_err();
    assert!(err.to_string().contains("localPort"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test config::store::tests::validate -- --nocapture`
Expected: FAIL — `validate_tunnel_input` not found

- [ ] **Step 3: Implement validate_tunnel_input**

Add to `src-tauri/src/config/store.rs` (as a public function):

```rust
use crate::types::TunnelInput;

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

    // Validate keyPath if provided
    if !input.key_path.is_empty() {
        let expanded = ConfigStore::expand_tilde(&input.key_path);
        if !expanded.exists() {
            return Err(TunnelError::ConfigInvalid(
                format!("keyPath '{}' does not exist", input.key_path),
            ));
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test config::store::tests -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config/store.rs
git commit -m "feat: add validate_tunnel_input with port conflict and field checks"
```

---

### Task 4: Add manager commands — AddTunnel, UpdateTunnel, RemoveTunnel, GetTunnelConfig

**Files:**
- Modify: `src-tauri/src/tunnel/manager.rs`

- [ ] **Step 1: Add new variants to `ManagerCommand` enum**

Add to the `ManagerCommand` enum in `src-tauri/src/tunnel/manager.rs`:

```rust
AddTunnel {
    config: TunnelConfig,
    reply: oneshot::Sender<Result<TunnelInfo, TunnelError>>,
},
UpdateTunnel {
    config: TunnelConfig,
    reply: oneshot::Sender<Result<TunnelInfo, TunnelError>>,
},
RemoveTunnel {
    id: String,
    reply: oneshot::Sender<Result<(), TunnelError>>,
},
GetTunnelConfig {
    id: String,
    reply: oneshot::Sender<Result<TunnelConfig, TunnelError>>,
},
```

- [ ] **Step 2: Handle new commands in the actor `run` loop**

Add match arms in the `run` method's `match cmd` block:

```rust
ManagerCommand::AddTunnel { config, reply } => {
    let result = self.handle_add_tunnel(config);
    let _ = reply.send(result);
}

ManagerCommand::UpdateTunnel { config, reply } => {
    let result = self.handle_update_tunnel(config).await;
    let _ = reply.send(result);
}

ManagerCommand::RemoveTunnel { id, reply } => {
    let result = self.handle_remove_tunnel(&id).await;
    let _ = reply.send(result);
}

ManagerCommand::GetTunnelConfig { id, reply } => {
    let result = match self.tunnels.get(&id) {
        Some(t) => Ok(t.config.clone()),
        None => Err(TunnelError::TunnelNotFound(id)),
    };
    let _ = reply.send(result);
}
```

- [ ] **Step 3: Implement handler methods**

Add to `impl TunnelManagerActor`:

```rust
fn handle_add_tunnel(&mut self, config: TunnelConfig) -> Result<TunnelInfo, TunnelError> {
    if self.tunnels.contains_key(&config.id) {
        return Err(TunnelError::ConfigInvalid(
            format!("Tunnel '{}' already exists", config.id),
        ));
    }
    let state = TunnelState::new(config);
    let info = state.to_info();
    self.tunnels.insert(info.id.clone(), state);
    info!("Added tunnel '{}'", info.id);
    Ok(info)
}

async fn handle_update_tunnel(&mut self, config: TunnelConfig) -> Result<TunnelInfo, TunnelError> {
    let id = config.id.clone();

    // Disconnect if currently connected
    if let Some(tunnel) = self.tunnels.get(&id) {
        if tunnel.status == TunnelStatus::Connected || tunnel.status == TunnelStatus::Connecting {
            info!("Disconnecting tunnel '{}' before update", id);
            self.handle_disconnect(&id).await.ok();
        }
    } else {
        return Err(TunnelError::TunnelNotFound(id));
    }

    // Replace with new config, reset state
    let state = TunnelState::new(config);
    let info = state.to_info();
    self.tunnels.insert(id.clone(), state);
    self.emit_status(&id, &TunnelStatus::Disconnected);
    info!("Updated tunnel '{}'", id);
    Ok(info)
}

async fn handle_remove_tunnel(&mut self, id: &str) -> Result<(), TunnelError> {
    // Disconnect if currently connected
    if let Some(tunnel) = self.tunnels.get(id) {
        if tunnel.status == TunnelStatus::Connected || tunnel.status == TunnelStatus::Connecting {
            info!("Disconnecting tunnel '{}' before removal", id);
            self.handle_disconnect(id).await.ok();
        }
    }

    match self.tunnels.remove(id) {
        Some(_) => {
            info!("Removed tunnel '{}'", id);
            Ok(())
        }
        None => Err(TunnelError::TunnelNotFound(id.to_string())),
    }
}
```

- [ ] **Step 4: Verify build**

Run: `cd src-tauri && cargo build`
Expected: Build succeeds

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/tunnel/manager.rs
git commit -m "feat: add AddTunnel, UpdateTunnel, RemoveTunnel, GetTunnelConfig manager commands"
```

---

### Task 5: Add Tauri commands — add_tunnel, update_tunnel, delete_tunnel, get_tunnel_config

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add new commands to commands.rs**

Add the following to `src-tauri/src/commands.rs`. Add `use crate::types::{TunnelConfig, TunnelInput};` to imports and `use crate::config::store::{generate_id, validate_tunnel_input};`.

```rust
#[tauri::command]
pub async fn add_tunnel(
    input: TunnelInput,
    state: State<'_, AppState>,
) -> Result<TunnelInfo, String> {
    // Collect existing IDs and ports for validation
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::ListTunnels { reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    let existing = reply_rx.await.map_err(|e| format!("Manager error: {}", e))?;

    let existing_ids: Vec<String> = existing.iter().map(|t| t.id.clone()).collect();
    let existing_ports: Vec<(String, u16)> = existing.iter().map(|t| (t.id.clone(), t.local_port)).collect();

    // Validate
    validate_tunnel_input(&input, &existing_ports, None).map_err(|e| e.to_string())?;

    // Generate ID
    let id = generate_id(&input.name, &existing_ids);
    let config = input.to_config(id);

    // Save to disk
    {
        let mut app_config = state.config_store.load().map_err(|e| e.to_string())?;
        app_config.tunnels.push(config.clone());
        state.config_store.save(&app_config).map_err(|e| e.to_string())?;
    }

    // Add to manager
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::AddTunnel { config, reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    reply_rx.await.map_err(|e| format!("Manager error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tunnel(
    id: String,
    input: TunnelInput,
    state: State<'_, AppState>,
) -> Result<TunnelInfo, String> {
    // Collect existing ports for validation (exclude self)
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::ListTunnels { reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    let existing = reply_rx.await.map_err(|e| format!("Manager error: {}", e))?;
    let existing_ports: Vec<(String, u16)> = existing.iter().map(|t| (t.id.clone(), t.local_port)).collect();

    // Validate (exclude self from port conflict check)
    validate_tunnel_input(&input, &existing_ports, Some(&id)).map_err(|e| e.to_string())?;

    // Preserve original ID
    let config = input.to_config(id.clone());

    // Save to disk
    {
        let mut app_config = state.config_store.load().map_err(|e| e.to_string())?;
        if let Some(pos) = app_config.tunnels.iter().position(|t| t.id == id) {
            app_config.tunnels[pos] = config.clone();
        } else {
            return Err(format!("Tunnel '{}' not found in config", id));
        }
        state.config_store.save(&app_config).map_err(|e| e.to_string())?;
    }

    // Update in manager
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::UpdateTunnel { config, reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    reply_rx.await.map_err(|e| format!("Manager error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tunnel(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Remove from manager (disconnects if connected)
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::RemoveTunnel { id: id.clone(), reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    reply_rx.await.map_err(|e| format!("Manager error: {}", e))?.map_err(|e| e.to_string())?;

    // Remove from config on disk
    let mut app_config = state.config_store.load().map_err(|e| e.to_string())?;
    app_config.tunnels.retain(|t| t.id != id);
    state.config_store.save(&app_config).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_tunnel_config(
    id: String,
    state: State<'_, AppState>,
) -> Result<TunnelConfig, String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::GetTunnelConfig { id, reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    reply_rx.await.map_err(|e| format!("Manager error: {}", e))?.map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Register commands in lib.rs**

In `src-tauri/src/lib.rs`, update the `invoke_handler` to include the new commands:

```rust
.invoke_handler(tauri::generate_handler![
    commands::list_tunnels,
    commands::connect_tunnel,
    commands::disconnect_tunnel,
    commands::store_passphrase_for_tunnel,
    commands::reload_config,
    commands::add_tunnel,
    commands::update_tunnel,
    commands::delete_tunnel,
    commands::get_tunnel_config,
])
```

- [ ] **Step 3: Verify build**

Run: `cd src-tauri && cargo build`
Expected: Build succeeds

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add CRUD Tauri commands — add_tunnel, update_tunnel, delete_tunnel, get_tunnel_config"
```

---

## Chunk 2: Frontend — Types, Hook, Views

### Task 6: Add TypeScript types and hook functions

**Files:**
- Modify: `src/types.ts`
- Modify: `src/hooks/useTunnels.ts`

- [ ] **Step 1: Add types to types.ts**

Add to `src/types.ts`:

```typescript
export interface TunnelInput {
  name: string;
  host: string;
  port: number;
  user: string;
  keyPath: string;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  autoConnect: boolean;
}

export interface TunnelConfig {
  id: string;
  name: string;
  host: string;
  port: number;
  user: string;
  keyPath: string;
  type: "local" | "reverse" | "dynamic";
  localPort: number;
  remoteHost: string;
  remotePort: number;
  autoConnect: boolean;
}
```

- [ ] **Step 2: Add CRUD functions to useTunnels hook**

Add the following functions inside `useTunnels()` in `src/hooks/useTunnels.ts`, and add `TunnelConfig` and `TunnelInput` to the type imports:

```typescript
const addTunnel = useCallback(async (input: TunnelInput) => {
  try {
    setError(null);
    await invoke("add_tunnel", { input });
    await fetchTunnels();
  } catch (e) {
    setError(String(e));
    throw e;
  }
}, [fetchTunnels]);

const updateTunnel = useCallback(async (id: string, input: TunnelInput) => {
  try {
    setError(null);
    await invoke("update_tunnel", { id, input });
    await fetchTunnels();
  } catch (e) {
    setError(String(e));
    throw e;
  }
}, [fetchTunnels]);

const deleteTunnel = useCallback(async (id: string) => {
  try {
    setError(null);
    await invoke("delete_tunnel", { id });
    await fetchTunnels();
  } catch (e) {
    setError(String(e));
    throw e;
  }
}, [fetchTunnels]);

const getTunnelConfig = useCallback(async (id: string): Promise<TunnelConfig> => {
  return await invoke<TunnelConfig>("get_tunnel_config", { id });
}, []);
```

Add these to the return object:

```typescript
return {
  tunnels,
  loading,
  error,
  connect,
  disconnect,
  reload,
  passphrasePrompt,
  submitPassphrase,
  cancelPassphrase,
  addTunnel,
  updateTunnel,
  deleteTunnel,
  getTunnelConfig,
};
```

- [ ] **Step 3: Verify build**

Run: `npm run build`
Expected: Build succeeds (TypeScript compiles)

- [ ] **Step 4: Commit**

```bash
git add src/types.ts src/hooks/useTunnels.ts
git commit -m "feat: add TunnelInput/TunnelConfig types and CRUD functions to useTunnels hook"
```

---

### Task 7: Create EditList component

**Files:**
- Create: `src/components/EditList.tsx`

- [ ] **Step 1: Create EditList.tsx**

Create `src/components/EditList.tsx`:

```tsx
import { useState } from "react";
import type { TunnelInfo } from "../types";

interface EditListProps {
  tunnels: TunnelInfo[];
  onEdit: (id: string) => void;
  onAdd: () => void;
  onDelete: (id: string) => Promise<void>;
  onDone: () => void;
}

export function EditList({ tunnels, onEdit, onAdd, onDelete, onDone }: EditListProps) {
  const [confirmingDelete, setConfirmingDelete] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);

  const handleMinusClick = (id: string) => {
    setConfirmingDelete(confirmingDelete === id ? null : id);
  };

  const handleDelete = async (id: string) => {
    setDeleting(id);
    try {
      await onDelete(id);
    } finally {
      setDeleting(null);
      setConfirmingDelete(null);
    }
  };

  return (
    <div className="h-screen flex flex-col bg-gray-900 text-white select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <button
          onClick={onAdd}
          className="text-sm text-blue-400 hover:text-blue-300"
        >
          + Add
        </button>
        <h1 className="text-sm font-semibold">Edit Tunnels</h1>
        <button
          onClick={onDone}
          className="text-sm font-semibold text-blue-400 hover:text-blue-300"
        >
          Done
        </button>
      </div>

      {/* Tunnel list */}
      <div className="flex-1 overflow-y-auto p-2">
        {tunnels.length === 0 ? (
          <div className="py-8 text-center">
            <p className="text-gray-400 text-sm">No tunnels configured</p>
            <button
              onClick={onAdd}
              className="mt-2 text-blue-400 text-sm hover:text-blue-300"
            >
              Add your first tunnel
            </button>
          </div>
        ) : (
          <div className="space-y-0.5">
            {tunnels.map((tunnel) => (
              <div key={tunnel.id} className="flex items-center rounded-lg hover:bg-white/5 transition-colors">
                {/* Delete minus button */}
                <button
                  onClick={() => handleMinusClick(tunnel.id)}
                  className="flex-shrink-0 w-8 h-8 flex items-center justify-center ml-1"
                  disabled={deleting === tunnel.id}
                >
                  <div className="w-5 h-5 rounded-full bg-red-500 flex items-center justify-center text-white text-sm font-bold leading-none">
                    &minus;
                  </div>
                </button>

                {/* Tunnel info — clickable to edit */}
                <button
                  onClick={() => onEdit(tunnel.id)}
                  className="flex-1 flex items-center justify-between px-2 py-2.5 text-left"
                >
                  <div className="min-w-0">
                    <p className="text-sm text-white truncate">{tunnel.name}</p>
                    <p className="text-xs text-gray-500 truncate">
                      localhost:{tunnel.localPort} → {tunnel.remoteHost}:{tunnel.remotePort}
                    </p>
                  </div>
                  <span className="text-gray-600 text-lg ml-2">&rsaquo;</span>
                </button>

                {/* Slide-in delete confirmation */}
                {confirmingDelete === tunnel.id && (
                  <button
                    onClick={() => handleDelete(tunnel.id)}
                    disabled={deleting === tunnel.id}
                    className="flex-shrink-0 bg-red-500 text-white text-xs px-3 py-1.5 rounded-md mr-2 hover:bg-red-600 disabled:opacity-50"
                  >
                    {deleting === tunnel.id ? "..." : "Delete"}
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

Run: `npm run build`
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add src/components/EditList.tsx
git commit -m "feat: add EditList component with iOS-style two-step delete"
```

---

### Task 8: Create EditForm component

**Files:**
- Create: `src/components/EditForm.tsx`

- [ ] **Step 1: Create EditForm.tsx**

Create `src/components/EditForm.tsx`:

```tsx
import { useState, useEffect } from "react";
import type { TunnelInput, TunnelConfig } from "../types";

interface EditFormProps {
  tunnelId: string | null; // null = new tunnel
  getTunnelConfig: (id: string) => Promise<TunnelConfig>;
  onSave: (input: TunnelInput, id: string | null) => Promise<void>;
  onBack: () => void;
}

const emptyForm: TunnelInput = {
  name: "",
  host: "",
  port: 22,
  user: "",
  keyPath: "",
  localPort: 0,
  remoteHost: "",
  remotePort: 0,
  autoConnect: false,
};

export function EditForm({ tunnelId, getTunnelConfig, onSave, onBack }: EditFormProps) {
  const [form, setForm] = useState<TunnelInput>(emptyForm);
  const [loading, setLoading] = useState(!!tunnelId);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (tunnelId) {
      getTunnelConfig(tunnelId)
        .then((config) => {
          setForm({
            name: config.name,
            host: config.host,
            port: config.port,
            user: config.user,
            keyPath: config.keyPath,
            localPort: config.localPort,
            remoteHost: config.remoteHost,
            remotePort: config.remotePort,
            autoConnect: config.autoConnect,
          });
        })
        .catch((e) => setError(String(e)))
        .finally(() => setLoading(false));
    }
  }, [tunnelId, getTunnelConfig]);

  const isValid =
    form.name.trim() !== "" &&
    form.host.trim() !== "" &&
    form.user.trim() !== "" &&
    form.localPort > 0 &&
    form.remoteHost.trim() !== "" &&
    form.remotePort > 0;

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave(form, tunnelId);
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  const updateField = <K extends keyof TunnelInput>(key: K, value: TunnelInput[K]) => {
    setForm((prev) => ({ ...prev, [key]: value }));
  };

  if (loading) {
    return (
      <div className="h-screen flex items-center justify-center bg-gray-900">
        <p className="text-gray-400 text-sm">Loading...</p>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-gray-900 text-white select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <button
          onClick={onBack}
          className="text-sm text-blue-400 hover:text-blue-300"
        >
          &lsaquo; Back
        </button>
        <h1 className="text-sm font-semibold">
          {tunnelId ? "Edit Tunnel" : "New Tunnel"}
        </h1>
        <button
          onClick={handleSave}
          disabled={!isValid || saving}
          className="text-sm font-semibold text-blue-400 hover:text-blue-300 disabled:text-gray-600 disabled:cursor-not-allowed"
        >
          {saving ? "..." : "Save"}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="mx-3 mt-2 px-3 py-2 bg-red-500/20 border border-red-500/30 rounded-md">
          <p className="text-xs text-red-400">{error}</p>
        </div>
      )}

      {/* Form */}
      <div className="flex-1 overflow-y-auto px-4 py-3">
        {/* Connection section */}
        <SectionLabel>Connection</SectionLabel>
        <div className="bg-white/5 rounded-lg overflow-hidden mb-4">
          <FormRow label="Name" value={form.name} onChange={(v) => updateField("name", v)} />
          <FormRow label="Host" value={form.host} onChange={(v) => updateField("host", v)} />
          <FormRow
            label="Port"
            value={String(form.port)}
            onChange={(v) => updateField("port", parseInt(v) || 0)}
            type="number"
          />
          <FormRow label="Username" value={form.user} onChange={(v) => updateField("user", v)} />
          <FormRow
            label="Key Path"
            value={form.keyPath}
            onChange={(v) => updateField("keyPath", v)}
            placeholder="~/.ssh/id_rsa"
            last
          />
        </div>

        {/* Port Forwarding section */}
        <SectionLabel>Port Forwarding</SectionLabel>
        <div className="bg-white/5 rounded-lg overflow-hidden mb-4">
          <FormRow
            label="Local Port"
            value={form.localPort === 0 ? "" : String(form.localPort)}
            onChange={(v) => updateField("localPort", parseInt(v) || 0)}
            type="number"
            placeholder="e.g. 5432"
          />
          <FormRow
            label="Remote Host"
            value={form.remoteHost}
            onChange={(v) => updateField("remoteHost", v)}
            placeholder="e.g. localhost"
          />
          <FormRow
            label="Remote Port"
            value={form.remotePort === 0 ? "" : String(form.remotePort)}
            onChange={(v) => updateField("remotePort", parseInt(v) || 0)}
            type="number"
            placeholder="e.g. 5432"
            last
          />
        </div>

        {/* Options section */}
        <SectionLabel>Options</SectionLabel>
        <div className="bg-white/5 rounded-lg overflow-hidden">
          <div className="flex items-center justify-between px-3 py-2.5">
            <span className="text-sm text-gray-300">Auto Connect</span>
            <button
              onClick={() => updateField("autoConnect", !form.autoConnect)}
              className={`w-10 h-6 rounded-full relative transition-colors ${
                form.autoConnect ? "bg-green-500" : "bg-gray-600"
              }`}
            >
              <div
                className={`w-5 h-5 rounded-full bg-white absolute top-0.5 transition-transform ${
                  form.autoConnect ? "translate-x-[18px]" : "translate-x-0.5"
                }`}
              />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-xs text-gray-500 uppercase tracking-wider mb-1.5 px-1">
      {children}
    </p>
  );
}

interface FormRowProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
  placeholder?: string;
  last?: boolean;
}

function FormRow({ label, value, onChange, type = "text", placeholder, last }: FormRowProps) {
  return (
    <div
      className={`flex items-center px-3 py-2 ${
        last ? "" : "border-b border-white/5"
      }`}
    >
      <label className="text-sm text-gray-400 w-24 flex-shrink-0">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="flex-1 bg-transparent text-sm text-white outline-none placeholder-gray-600"
      />
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

Run: `npm run build`
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add src/components/EditForm.tsx
git commit -m "feat: add EditForm component with iOS Settings-style grouped fields"
```

---

### Task 9: Wire up view state machine in App.tsx

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Rewrite App.tsx with view state machine**

Replace `src/App.tsx` with:

```tsx
import { useState } from "react";
import { TunnelList } from "./components/TunnelList";
import { PassphraseDialog } from "./components/PassphraseDialog";
import { EditList } from "./components/EditList";
import { EditForm } from "./components/EditForm";
import { useTunnels } from "./hooks/useTunnels";
import type { TunnelInput } from "./types";

type View =
  | { kind: "normal" }
  | { kind: "edit-list" }
  | { kind: "edit-form"; tunnelId: string | null };

function App() {
  const {
    tunnels,
    loading,
    error,
    connect,
    disconnect,
    passphrasePrompt,
    submitPassphrase,
    cancelPassphrase,
    addTunnel,
    updateTunnel,
    deleteTunnel,
    getTunnelConfig,
  } = useTunnels();

  const [view, setView] = useState<View>({ kind: "normal" });

  const handleSave = async (input: TunnelInput, id: string | null) => {
    if (id) {
      await updateTunnel(id, input);
    } else {
      await addTunnel(input);
    }
    setView({ kind: "edit-list" });
  };

  // Edit list view
  if (view.kind === "edit-list") {
    return (
      <EditList
        tunnels={tunnels}
        onEdit={(id) => setView({ kind: "edit-form", tunnelId: id })}
        onAdd={() => setView({ kind: "edit-form", tunnelId: null })}
        onDelete={deleteTunnel}
        onDone={() => setView({ kind: "normal" })}
      />
    );
  }

  // Edit form view
  if (view.kind === "edit-form") {
    return (
      <EditForm
        tunnelId={view.tunnelId}
        getTunnelConfig={getTunnelConfig}
        onSave={handleSave}
        onBack={() => setView({ kind: "edit-list" })}
      />
    );
  }

  // Normal view
  const connectedCount = tunnels.filter((t) => t.status === "connected").length;
  const totalCount = tunnels.length;

  return (
    <div className="h-screen flex flex-col bg-gray-900 text-white select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <div>
          <h1 className="text-sm font-semibold">Tunnel Master</h1>
          <p className="text-xs text-gray-400">
            {connectedCount}/{totalCount} active
          </p>
        </div>
        <button
          onClick={() => setView({ kind: "edit-list" })}
          className="text-sm text-blue-400 hover:text-blue-300"
        >
          Edit
        </button>
      </div>

      {/* Error banner */}
      {error && (
        <div className="mx-3 mt-2 px-3 py-2 bg-red-500/20 border border-red-500/30 rounded-md">
          <p className="text-xs text-red-400">{error}</p>
        </div>
      )}

      {/* Content — scrollable */}
      <div className="flex-1 overflow-y-auto p-2">
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

      {/* Passphrase dialog */}
      {passphrasePrompt && (
        <PassphraseDialog
          tunnelId={passphrasePrompt.tunnelId}
          onSubmit={submitPassphrase}
          onCancel={cancelPassphrase}
        />
      )}
    </div>
  );
}

export default App;
```

- [ ] **Step 2: Verify build**

Run: `npm run build`
Expected: Build succeeds

- [ ] **Step 3: Manual test**

Run: `npx tauri dev`

Test the following flows:
1. Normal view shows "Edit" button in header
2. Click "Edit" → switches to edit list view with "+ Add", "Done", tunnel rows with minus buttons
3. Click a tunnel row → switches to edit form with fields populated
4. Click "Back" → returns to edit list
5. Click "+ Add" → switches to empty edit form
6. Fill fields, click "Save" → tunnel added, returns to edit list
7. Click minus on a tunnel → "Delete" button appears → click "Delete" → tunnel removed
8. Click "Done" → returns to normal view

- [ ] **Step 4: Commit**

```bash
git add src/App.tsx
git commit -m "feat: wire up view state machine — normal, edit-list, edit-form views"
```
