# Extended Authentication Methods & ProxyJump — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add password, SSH agent, keyboard-interactive (2FA) auth methods and ProxyJump support to Tunnel Master.

**Architecture:** New `AuthMethod` enum and `AuthCredentials` enum drive auth dispatch in a redesigned `SshConnection::connect()`. ProxyJump uses `connect_stream()` over an SSH channel. Keyboard-interactive uses a shared `oneshot` channel between the russh handler and Tauri commands. Frontend adds auth method selector, jump host dropdown, and new dialogs.

**Tech Stack:** Rust (russh 0.46, russh-keys 0.46, keyring 3, tokio), TypeScript/React, Tailwind CSS v4, Tauri v2

**Spec:** `docs/superpowers/specs/2026-03-12-auth-methods-design.md`

---

## File Structure

### Modified files:
- `src-tauri/src/types.rs` — Add `AuthMethod` enum, new fields on `TunnelConfig`, `TunnelInput`, `TunnelInfo`
- `src-tauri/src/errors.rs` — Add `PasswordRequired`, `AgentUnavailable`, `JumpHostNotFound`, `JumpHostFailed` variants
- `src-tauri/src/keychain.rs` — Add `get_password`, `store_password`, `delete_password`
- `src-tauri/src/tunnel/connection.rs` — Add `AuthCredentials`, `KiResponseSlot`, rewrite `connect()`, add `connect_stream()`
- `src-tauri/src/tunnel/manager.rs` — Add `app_handle`, new commands, rewrite `handle_connect`, jump chain validation, `tunnel_to_info`
- `src-tauri/src/config/store.rs` — Gate key_path validation on `AuthMethod::Key`
- `src-tauri/src/commands.rs` — Add `store_password_for_tunnel`, `respond_keyboard_interactive`, `cancel_keyboard_interactive`; update `delete_tunnel`
- `src-tauri/src/lib.rs` — Pass `app_handle` to `spawn_manager`, register new commands
- `src/types.ts` — Add `authMethod`, `jumpHost`, `jumpHostName` fields
- `src/hooks/useTunnels.ts` — Handle `PASSWORD_REQUIRED:` errors, add KI event listener and handlers
- `src/components/EditForm.tsx` — Auth method selector, conditional key path, jump host dropdown
- `src/components/TunnelItem.tsx` — Show "via {jumpHostName}" subtitle
- `src/App.tsx` — Wire up PasswordDialog and KeyboardInteractiveDialog

### New files:
- `src/components/PasswordDialog.tsx` — Password prompt dialog
- `src/components/KeyboardInteractiveDialog.tsx` — 2FA prompt dialog

---

## Chunk 1: Backend Foundation (Types, Errors, Keychain)

### Task 1: Add AuthMethod enum and update TunnelConfig

**Files:**
- Modify: `src-tauri/src/types.rs`
- Modify: `src-tauri/src/tunnel/manager.rs` (to_info stub + test literals)
- Modify: `src-tauri/src/config/store.rs` (test literals only)

- [ ] **Step 1: Add AuthMethod enum before TunnelConfig**

Add after the `TunnelStatus` enum (after line 21):

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMethod {
    #[default]
    Key,
    Password,
    Agent,
    KeyboardInteractive,
}
```

- [ ] **Step 2: Add auth_method and jump_host to TunnelConfig**

Add two new fields to `TunnelConfig` struct. After the `user` field (line 31), add:

```rust
    #[serde(default)]
    pub auth_method: AuthMethod,
```

After the `auto_connect` field (line 38-39), add:

```rust
    #[serde(default)]
    pub jump_host: Option<String>,
```

- [ ] **Step 3: Add auth_method and jump_host to TunnelInput**

Add two new fields to `TunnelInput` struct. After `key_path` (line 51), add:

```rust
    #[serde(default)]
    pub auth_method: AuthMethod,
```

After `auto_connect` (line 56), add:

```rust
    #[serde(default)]
    pub jump_host: Option<String>,
```

- [ ] **Step 4: Update TunnelInput::to_config()**

In `to_config()` (lines 62-76), add the two new fields to the TunnelConfig construction:

```rust
    pub fn to_config(self, id: String) -> TunnelConfig {
        TunnelConfig {
            id,
            name: self.name,
            host: self.host,
            port: self.port,
            user: self.user,
            auth_method: self.auth_method,
            key_path: self.key_path,
            tunnel_type: TunnelType::Local,
            local_port: self.local_port,
            remote_host: self.remote_host,
            remote_port: self.remote_port,
            auto_connect: self.auto_connect,
            jump_host: self.jump_host,
        }
    }
```

- [ ] **Step 5: Add auth_method and jump_host_name to TunnelInfo**

Add two new fields to `TunnelInfo` struct (after line 123):

```rust
    pub auth_method: AuthMethod,
    pub jump_host_name: Option<String>,
```

- [ ] **Step 6: Add temporary to_info() stub for new TunnelInfo fields**

The existing `TunnelState::to_info()` in `manager.rs` doesn't know about the new `auth_method` and `jump_host_name` fields yet (that rewrite comes in Task 6). To keep the crate compiling between Tasks 1-6, add temporary dummy values in `to_info()`.

In `src-tauri/src/tunnel/manager.rs`, find `to_info()` (around line 88-98) and add the two new fields with defaults:

```rust
    fn to_info(&self) -> TunnelInfo {
        TunnelInfo {
            id: self.config.id.clone(),
            name: self.config.name.clone(),
            status: self.status.clone(),
            local_port: self.config.local_port,
            remote_host: self.config.remote_host.clone(),
            remote_port: self.config.remote_port,
            error_message: self.error_message.clone(),
            auth_method: self.config.auth_method.clone(),  // NEW
            jump_host_name: None,                          // NEW (placeholder until Task 6)
        }
    }
```

Add `use crate::types::AuthMethod;` if not already imported.

- [ ] **Step 7: Update tests in types.rs**

Update all `TunnelConfig` literals in `types.rs` tests to include the new fields. Add after existing fields:

```rust
    auth_method: AuthMethod::Key,
    jump_host: None,
```

Update `TunnelInput` literals in `types.rs` tests — for tests with `key_path: ""`, change to use `AuthMethod::Password`:

```rust
    auth_method: AuthMethod::Password,  // empty key_path is valid for non-Key auth
    jump_host: None,
```

For the `tunnel_input_to_config` test, add:
```rust
    auth_method: AuthMethod::Key,
    jump_host: None,
```

Add a new test for AuthMethod serialization:
```rust
    #[test]
    fn auth_method_serializes_kebab_case() {
        let m = AuthMethod::KeyboardInteractive;
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, "\"keyboard-interactive\"");
    }

    #[test]
    fn auth_method_default_is_key() {
        let m = AuthMethod::default();
        assert_eq!(m, AuthMethod::Key);
    }
```

- [ ] **Step 8: Update test literals in manager.rs and config/store.rs**

**IMPORTANT:** All `TunnelConfig` and `TunnelInput` struct literals across the crate must be updated in the same commit to keep compilation working.

In `src-tauri/src/tunnel/manager.rs`, update the `test_config()` helper (around lines 578-603) and all `TunnelConfig` struct literals in `reload_config_adds_new_tunnels` and `connect_to_unreachable_host_returns_error` tests (around lines 644-656) to include:

```rust
    auth_method: AuthMethod::Key,
    jump_host: None,
```

Add `use crate::types::AuthMethod;` to the test module imports.

In `src-tauri/src/config/store.rs`, update all `TunnelInput` struct literals in the validation tests (around lines 319-398) to include the new fields. For tests with `key_path: ""`, use `AuthMethod::Password`:

```rust
    auth_method: AuthMethod::Password,
    jump_host: None,
```

For `validate_input_valid` which has a real key path, use `AuthMethod::Key`.

Add `use crate::types::AuthMethod;` to the test imports (update the existing `use crate::types::TunnelInput;` line).

- [ ] **Step 9: Run full test suite**

Run: `cd src-tauri && cargo test --lib`
Expected: All tests pass (types, config, and manager tests)

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/tunnel/manager.rs src-tauri/src/config/store.rs
git commit -m "feat: add AuthMethod enum, auth_method and jump_host fields to types"
```

---

### Task 2: Add new error variants

**Files:**
- Modify: `src-tauri/src/errors.rs`

- [ ] **Step 1: Add four new error variants**

Add before `TunnelNotFound` (before line 38):

```rust
    #[error("PASSWORD_REQUIRED:{0}")]
    PasswordRequired(String),

    #[error("SSH agent not available: {0}")]
    AgentUnavailable(String),

    #[error("Jump host not found: {0}")]
    JumpHostNotFound(String),

    #[error("Jump host connection failed: {0}")]
    JumpHostFailed(String),
```

- [ ] **Step 2: Add tests for new error variants**

Add to the `tests` module:

```rust
    #[test]
    fn password_required_format() {
        let err = TunnelError::PasswordRequired("my-tunnel".into());
        assert_eq!(err.to_string(), "PASSWORD_REQUIRED:my-tunnel");
    }

    #[test]
    fn agent_unavailable_format() {
        let err = TunnelError::AgentUnavailable("not found".into());
        assert!(err.to_string().contains("SSH agent not available"));
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib errors`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/errors.rs
git commit -m "feat: add PasswordRequired, AgentUnavailable, JumpHost error variants"
```

---

### Task 3: Add password keychain functions

**Files:**
- Modify: `src-tauri/src/keychain.rs`

- [ ] **Step 1: Add get_password, store_password, delete_password**

Add after the existing `set_passphrase` function (after line 31):

```rust
pub fn get_password(tunnel_id: &str) -> Option<String> {
    let key = format!("password/{}", tunnel_id);
    let entry = keyring::Entry::new(SERVICE_NAME, &key).ok()?;
    match entry.get_password() {
        Ok(password) => {
            debug!("Retrieved password from credential store for tunnel {}", tunnel_id);
            Some(password)
        }
        Err(keyring::Error::NoEntry) => {
            debug!("No password stored for tunnel {}", tunnel_id);
            None
        }
        Err(e) => {
            debug!("Credential store error for tunnel {}: {}", tunnel_id, e);
            None
        }
    }
}

pub fn store_password(tunnel_id: &str, password: &str) -> Result<(), String> {
    let key = format!("password/{}", tunnel_id);
    let entry = keyring::Entry::new(SERVICE_NAME, &key).map_err(|e| e.to_string())?;
    entry.set_password(password).map_err(|e| e.to_string())
}

pub fn delete_password(tunnel_id: &str) {
    let key = format!("password/{}", tunnel_id);
    if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, &key) {
        let _ = entry.delete_credential();
        debug!("Deleted password for tunnel {}", tunnel_id);
    }
}
```

Add `use tracing::debug;` to the imports at the top if not already present.

- [ ] **Step 2: Add keychain unit tests**

Add a `#[cfg(test)]` module at the bottom of `keychain.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_password_returns_none_when_not_stored() {
        // Use a unique ID that won't collide with real entries
        let result = get_password("test-nonexistent-tunnel-12345");
        assert!(result.is_none());
    }

    #[test]
    fn store_and_get_password_roundtrip() {
        let id = "test-roundtrip-tunnel-keychain";
        // Clean up first in case of prior failed test
        delete_password(id);

        store_password(id, "my-secret-pw").expect("store should succeed");
        let retrieved = get_password(id);
        assert_eq!(retrieved, Some("my-secret-pw".to_string()));

        // Clean up
        delete_password(id);
        assert!(get_password(id).is_none());
    }

    #[test]
    fn delete_password_is_idempotent() {
        // Should not panic even if no entry exists
        delete_password("test-nonexistent-delete-12345");
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib keychain`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/keychain.rs
git commit -m "feat: add password get/store/delete to keychain with tests"
```

---

### Task 4: Update config validation

**Files:**
- Modify: `src-tauri/src/config/store.rs`

- [ ] **Step 1: Gate key_path validation on AuthMethod::Key**

Replace lines 155-163 (the key_path validation block):

```rust
    // Validate keyPath if provided
    if !input.key_path.is_empty() {
        let expanded = ConfigStore::expand_tilde(&input.key_path);
        if !expanded.exists() {
            return Err(TunnelError::ConfigInvalid(
                format!("keyPath '{}' does not exist", input.key_path),
            ));
        }
    }
```

With:

```rust
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
```

- [ ] **Step 2: Update tests that use empty key_path**

Note: The `TunnelInput` test literals in `store.rs` were already updated with `auth_method` and `jump_host` fields in Task 1 Step 8. This step only needs to update the `validate_input_valid` test to use `AuthMethod::Key` (since it now requires a non-empty key_path), and add the new `validate_input_key_auth_requires_key_path` test.

In tests `validate_input_valid`, `validate_input_empty_name`, `validate_input_port_conflict`, `validate_input_port_conflict_self_excluded`, `validate_input_port_zero`: verify `auth_method` and `jump_host` fields are present. For tests with `key_path: ""`, use `AuthMethod::Password`:

```rust
    use crate::types::{AuthMethod, TunnelInput};

    // In validate_input_valid:
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
```

Apply the same pattern to all other TunnelInput test literals.

Add a new test for key auth requiring key_path:

```rust
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
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib config`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/config/store.rs
git commit -m "feat: gate key_path validation on AuthMethod::Key"
```

---

## Chunk 2: Connection Rewrite

### Task 5: Add AuthCredentials and KiResponseSlot, rewrite SshConnection::connect()

**Files:**
- Modify: `src-tauri/src/tunnel/connection.rs`

This is the largest single change. The file goes from 245 lines to ~450 lines.

- [ ] **Step 1: Add new imports and type aliases**

Replace the imports block (lines 1-12) with:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use russh::client;
use russh::*;
use russh_keys::key;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{debug, info, warn};

use crate::config::store::ConfigStore;
use crate::errors::TunnelError;

/// Slot for keyboard-interactive response synchronization.
/// The handler stores a oneshot::Sender here; the Tauri command reads it to send responses.
pub type KiResponseSlot = Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<Vec<String>>>>>;

/// Authentication credentials for SSH connections.
pub enum AuthCredentials {
    Key {
        key_path: String,
        passphrase: Option<String>,
    },
    Password(String),
    Agent,
    KeyboardInteractive {
        ki_slot: KiResponseSlot,
        app_handle: tauri::AppHandle,
        tunnel_id: String,
    },
}
```

- [ ] **Step 2: Add keyboard-interactive event types**

Add after `AuthCredentials`:

```rust
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardInteractivePrompt {
    pub tunnel_id: String,
    pub name: String,
    pub instructions: String,
    pub prompts: Vec<KiPromptEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiPromptEntry {
    pub text: String,
    pub echo: bool,
}
```

- [ ] **Step 3: Update SshClientHandler with new fields**

Replace the `SshClientHandler` struct (lines 53-57) with:

```rust
struct SshClientHandler {
    host: String,
    port: u16,
    check_result: Arc<std::sync::Mutex<Option<HostKeyCheckResult>>>,
    ki_slot: KiResponseSlot,
    app_handle: Option<tauri::AppHandle>,
    tunnel_id: String,
}
```

- [ ] **Step 4: Add auth_keyboard_interactive to Handler impl**

Add this method to the `Handler` impl block, after `check_server_key` (after line 104):

```rust
    async fn auth_keyboard_interactive(
        &mut self,
        name: &str,
        instructions: &str,
        prompts: &[(std::borrow::Cow<'_, str>, bool)],
    ) -> Result<Vec<String>, Self::Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Store sender so respond_keyboard_interactive can find it
        *self.ki_slot.lock().unwrap() = Some(tx);

        // Emit event to frontend
        if let Some(ref handle) = self.app_handle {
            let prompt = KeyboardInteractivePrompt {
                tunnel_id: self.tunnel_id.clone(),
                name: name.to_string(),
                instructions: instructions.to_string(),
                prompts: prompts
                    .iter()
                    .map(|(text, echo)| KiPromptEntry {
                        text: text.to_string(),
                        echo: *echo,
                    })
                    .collect(),
            };
            let _ = handle.emit("keyboard-interactive-prompt", &prompt);
        }

        // Block until frontend responds or cancels
        rx.await.map_err(|_| russh::Error::Disconnect)
    }
```

- [ ] **Step 5: Rewrite SshConnection::connect()**

Replace the entire `connect()` method (lines 109-201) with the new signature and implementation. This is the core change:

```rust
    /// Connect to an SSH server and authenticate.
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        credentials: AuthCredentials,
        timeout_secs: u64,
    ) -> Result<Self, TunnelError> {
        info!("Connecting to {}@{}:{}", user, host, port);

        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(timeout_secs * 3)),
            keepalive_interval: Some(Duration::from_secs(15)),
            keepalive_max: 3,
            ..Default::default()
        });

        let check_result = Arc::new(std::sync::Mutex::new(None));
        let ki_slot = match &credentials {
            AuthCredentials::KeyboardInteractive { ki_slot, .. } => ki_slot.clone(),
            _ => Arc::new(std::sync::Mutex::new(None)),
        };
        let (app_handle, tunnel_id) = match &credentials {
            AuthCredentials::KeyboardInteractive {
                app_handle,
                tunnel_id,
                ..
            } => (Some(app_handle.clone()), tunnel_id.clone()),
            _ => (None, String::new()),
        };

        let handler = SshClientHandler {
            host: host.to_string(),
            port,
            check_result: check_result.clone(),
            ki_slot,
            app_handle,
            tunnel_id,
        };

        // Connect with timeout
        let addr = format!("{}:{}", host, port);
        let mut session = match tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            client::connect(config, &addr, handler),
        )
        .await
        {
            Ok(Ok(session)) => session,
            Ok(Err(e)) => {
                // Check if this was a host key rejection
                if let Some(result) = check_result.lock().unwrap().take() {
                    match result {
                        HostKeyCheckResult::Unknown(pubkey) => {
                            let fingerprint = pubkey.fingerprint();
                            let key_type = pubkey.name().to_string();
                            let map_key = format!("{}:{}", host, port);
                            PENDING_HOST_KEYS
                                .lock()
                                .unwrap()
                                .insert(map_key, pubkey);
                            return Err(TunnelError::HostKeyUnknown {
                                host: host.to_string(),
                                port,
                                key_type,
                                fingerprint,
                            });
                        }
                        HostKeyCheckResult::Changed => {
                            return Err(TunnelError::HostKeyChanged {
                                host: host.to_string(),
                                port,
                            });
                        }
                    }
                }
                return Err(TunnelError::SshError(format!(
                    "Connection failed: {}",
                    e
                )));
            }
            Err(_) => return Err(TunnelError::ConnectionTimeout),
        };

        // Authenticate based on credentials
        Self::authenticate(&mut session, user, credentials).await?;

        info!("SSH connection established to {}:{}", host, port);
        Ok(Self { session })
    }
```

- [ ] **Step 6: Add connect_stream() method**

Add after `connect()`:

```rust
    /// Connect over an existing stream (for ProxyJump).
    pub async fn connect_stream<R: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        stream: R,
        host: &str,
        port: u16,
        user: &str,
        credentials: AuthCredentials,
        timeout_secs: u64,
    ) -> Result<Self, TunnelError> {
        info!("Connecting via stream to {}@{}:{}", user, host, port);

        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(timeout_secs * 3)),
            keepalive_interval: Some(Duration::from_secs(15)),
            keepalive_max: 3,
            ..Default::default()
        });

        let check_result = Arc::new(std::sync::Mutex::new(None));
        let ki_slot = match &credentials {
            AuthCredentials::KeyboardInteractive { ki_slot, .. } => ki_slot.clone(),
            _ => Arc::new(std::sync::Mutex::new(None)),
        };
        let (app_handle, tunnel_id) = match &credentials {
            AuthCredentials::KeyboardInteractive {
                app_handle,
                tunnel_id,
                ..
            } => (Some(app_handle.clone()), tunnel_id.clone()),
            _ => (None, String::new()),
        };

        let handler = SshClientHandler {
            host: host.to_string(),
            port,
            check_result: check_result.clone(),
            ki_slot,
            app_handle,
            tunnel_id,
        };

        let mut session = match tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            client::connect_stream(config, stream, handler),
        )
        .await
        {
            Ok(Ok(session)) => session,
            Ok(Err(e)) => {
                if let Some(result) = check_result.lock().unwrap().take() {
                    match result {
                        HostKeyCheckResult::Unknown(pubkey) => {
                            let fingerprint = pubkey.fingerprint();
                            let key_type = pubkey.name().to_string();
                            let map_key = format!("{}:{}", host, port);
                            PENDING_HOST_KEYS
                                .lock()
                                .unwrap()
                                .insert(map_key, pubkey);
                            return Err(TunnelError::HostKeyUnknown {
                                host: host.to_string(),
                                port,
                                key_type,
                                fingerprint,
                            });
                        }
                        HostKeyCheckResult::Changed => {
                            return Err(TunnelError::HostKeyChanged {
                                host: host.to_string(),
                                port,
                            });
                        }
                    }
                }
                return Err(TunnelError::SshError(format!(
                    "Stream connection failed: {}",
                    e
                )));
            }
            Err(_) => return Err(TunnelError::ConnectionTimeout),
        };

        Self::authenticate(&mut session, user, credentials).await?;

        info!("SSH stream connection established to {}:{}", host, port);
        Ok(Self { session })
    }
```

- [ ] **Step 7: Add authenticate() helper**

Add a private method that handles all auth dispatch:

```rust
    /// Dispatch authentication based on credentials type.
    async fn authenticate(
        session: &mut client::Handle<SshClientHandler>,
        user: &str,
        credentials: AuthCredentials,
    ) -> Result<(), TunnelError> {
        match credentials {
            AuthCredentials::Key {
                key_path,
                passphrase,
            } => {
                let expanded = ConfigStore::expand_tilde(&key_path);
                let key_pair =
                    russh_keys::load_secret_key(&expanded, passphrase.as_deref())
                        .map_err(|e| {
                            TunnelError::AuthFailed(format!("Failed to load key: {}", e))
                        })?;
                let ok = session
                    .authenticate_publickey(user, Arc::new(key_pair))
                    .await
                    .map_err(|e| {
                        TunnelError::AuthFailed(format!("Auth error: {}", e))
                    })?;
                if !ok {
                    return Err(TunnelError::AuthFailed(
                        "Server rejected public key".to_string(),
                    ));
                }
            }
            AuthCredentials::Password(password) => {
                let ok = session
                    .authenticate_password(user, &password)
                    .await
                    .map_err(|e| {
                        TunnelError::AuthFailed(format!("Auth error: {}", e))
                    })?;
                if !ok {
                    return Err(TunnelError::AuthFailed(
                        "Server rejected password".to_string(),
                    ));
                }
            }
            AuthCredentials::Agent => {
                Self::authenticate_with_agent(session, user).await?;
            }
            AuthCredentials::KeyboardInteractive { .. } => {
                let ok = session
                    .authenticate_keyboard_interactive_start(user, None)
                    .await
                    .map_err(|e| {
                        TunnelError::AuthFailed(format!("KI auth error: {}", e))
                    })?;
                if !ok {
                    return Err(TunnelError::AuthFailed(
                        "Server rejected authentication".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
```

- [ ] **Step 8: Add SSH agent authentication helper**

Add after `authenticate()`:

```rust
    /// Authenticate using the SSH agent.
    async fn authenticate_with_agent(
        session: &mut client::Handle<SshClientHandler>,
        user: &str,
    ) -> Result<(), TunnelError> {
        #[cfg(unix)]
        let mut agent = russh_keys::agent::client::AgentClient::connect_env()
            .await
            .map_err(|e| {
                TunnelError::AgentUnavailable(format!(
                    "SSH agent not available — {}. Try launching from a terminal or ensure your agent is running.",
                    e
                ))
            })?;

        #[cfg(windows)]
        let mut agent = {
            let pipe = tokio::net::windows::named_pipe::ClientOptions::new()
                .open(r"\\.\pipe\openssh-ssh-agent")
                .map_err(|e| {
                    TunnelError::AgentUnavailable(format!(
                        "SSH agent not available — {}",
                        e
                    ))
                })?;
            russh_keys::agent::client::AgentClient::connect(pipe)
        };

        let identities = agent.request_identities().await.map_err(|e| {
            TunnelError::AgentUnavailable(format!("Failed to list agent keys: {}", e))
        })?;

        if identities.is_empty() {
            return Err(TunnelError::AgentUnavailable(
                "SSH agent has no keys loaded".to_string(),
            ));
        }

        let mut accepted = false;
        for (pubkey, _comment) in identities {
            let (returned_agent, result) =
                session.authenticate_future(user, pubkey, agent).await;
            agent = returned_agent;
            match result {
                Ok(true) => {
                    accepted = true;
                    break;
                }
                Ok(false) => continue,
                Err(e) => {
                    return Err(TunnelError::AuthFailed(format!(
                        "Agent auth error: {}",
                        e
                    )));
                }
            }
        }

        if !accepted {
            return Err(TunnelError::AuthFailed(
                "No agent key accepted by server".to_string(),
            ));
        }

        Ok(())
    }
```

- [ ] **Step 9: Run cargo check**

Run: `cd src-tauri && cargo check`
Expected: Compiles (manager.rs will have warnings about unused new types until Task 6)

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/tunnel/connection.rs
git commit -m "feat: rewrite SshConnection with AuthCredentials, connect_stream, multi-auth"
```

---

## Chunk 3: Manager Rewrite

### Task 6: Update TunnelManagerActor and handle_connect

**Files:**
- Modify: `src-tauri/src/tunnel/manager.rs`

- [ ] **Step 1: Update imports**

Add to imports (line 13):

```rust
use crate::tunnel::connection::{AuthCredentials, KiResponseSlot};
use crate::types::{AppConfig, AuthMethod, TunnelConfig, TunnelInfo, TunnelStatus, TunnelStatusEvent};
```

Remove `TunnelConfig` and `TunnelInfo` from the existing use line if they're already there, to avoid duplication.

- [ ] **Step 2: Add new ManagerCommand variants**

Add before `Shutdown` in the enum (before line 58):

```rust
    RespondKeyboardInteractive {
        tunnel_id: String,
        responses: Vec<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    CancelKeyboardInteractive {
        tunnel_id: String,
    },
```

- [ ] **Step 3: Update TunnelState with new fields**

Add to `TunnelState` struct (after line 71):

```rust
    /// Jump host SSH connection, if using ProxyJump
    jump_connection: Option<Arc<SshConnection>>,
    /// Keyboard-interactive response slot
    ki_slot: Option<KiResponseSlot>,
```

Update `TunnelState::new()` to initialize the new fields:

```rust
    fn new(config: TunnelConfig) -> Self {
        Self {
            config,
            status: TunnelStatus::Disconnected,
            error_message: None,
            abort_handles: Vec::new(),
            ssh_connection: None,
            jump_connection: None,
            ki_slot: None,
            generation: 0,
        }
    }
```

- [ ] **Step 4: Remove to_info() from TunnelState, keep a private helper**

Remove the `to_info()` method from `TunnelState` (lines 88-98). We'll add `tunnel_to_info()` to the actor instead.

- [ ] **Step 5: Add app_handle to TunnelManagerActor**

Use `Option<tauri::AppHandle>` for testability — tests pass `None`, production passes `Some(handle)`.

Add field to `TunnelManagerActor` struct (after line 124):

```rust
    app_handle: Option<tauri::AppHandle>,
```

Update `new()` to accept and store it:

```rust
    fn new(
        config: AppConfig,
        event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
        error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
        manager_tx: mpsc::Sender<ManagerCommand>,
        app_handle: Option<tauri::AppHandle>,
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
            app_handle,
        }
    }
```

- [ ] **Step 6: Update spawn_manager() signature**

Update `spawn_manager` to accept `Option<tauri::AppHandle>`:

```rust
pub fn spawn_manager(
    config: AppConfig,
    event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
    error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
    app_handle: Option<tauri::AppHandle>,
) -> ManagerHandle {
    let (tx, rx) = mpsc::channel(32);
    let manager_tx = tx.clone();

    tauri::async_runtime::spawn(async move {
        let mut manager = TunnelManagerActor::new(config, event_tx, error_tx, manager_tx, app_handle);
        manager.run(rx).await;
    });

    tx
}
```

- [ ] **Step 7: Add tunnel_to_info() method**

Add to `TunnelManagerActor` impl:

```rust
    fn tunnel_to_info(&self, state: &TunnelState) -> TunnelInfo {
        let jump_host_name = state.config.jump_host.as_ref().and_then(|jh_id| {
            self.tunnels.get(jh_id).map(|jh| jh.config.name.clone())
        });
        TunnelInfo {
            id: state.config.id.clone(),
            name: state.config.name.clone(),
            status: state.status.clone(),
            local_port: state.config.local_port,
            remote_host: state.config.remote_host.clone(),
            remote_port: state.config.remote_port,
            error_message: state.error_message.clone(),
            auth_method: state.config.auth_method.clone(),
            jump_host_name,
        }
    }
```

- [ ] **Step 8: Replace all t.to_info() calls with self.tunnel_to_info(t)**

In `run()` `ListTunnels` handler:
```rust
let infos: Vec<TunnelInfo> = self.tunnels.values().map(|t| self.tunnel_to_info(t)).collect();
```

In `handle_add_tunnel()`:
```rust
let info = self.tunnel_to_info(&state);
```

In `handle_update_tunnel()`:
```rust
let info = self.tunnel_to_info(&state);
```

- [ ] **Step 9: Add validate_jump_chain()**

Add to `TunnelManagerActor` impl:

```rust
    fn validate_jump_chain(&self, tunnel_id: &str) -> Result<(), TunnelError> {
        let mut visited = vec![tunnel_id.to_string()]; // Include self to detect self-reference
        let mut current = self.tunnels.get(tunnel_id)
            .and_then(|s| s.config.jump_host.clone());
        while let Some(id) = current {
            if visited.contains(&id) {
                return Err(TunnelError::JumpHostFailed(
                    "Circular jump host reference".into(),
                ));
            }
            if visited.len() > 5 {
                return Err(TunnelError::JumpHostFailed(
                    "Jump chain too deep (max 5)".into(),
                ));
            }
            visited.push(id.clone());
            current = self.tunnels.get(&id)
                .and_then(|s| s.config.jump_host.clone());
        }
        Ok(())
    }
```

- [ ] **Step 10: Add handler for new commands in run()**

Add match arms in `run()` before the `Shutdown` arm:

```rust
                ManagerCommand::RespondKeyboardInteractive {
                    tunnel_id,
                    responses,
                    reply,
                } => {
                    let result = if let Some(state) = self.tunnels.get(&tunnel_id) {
                        if let Some(ref ki_slot) = state.ki_slot {
                            if let Some(tx) = ki_slot.lock().unwrap().take() {
                                tx.send(responses)
                                    .map_err(|_| "KI channel closed".to_string())
                            } else {
                                Err("No pending KI prompt".to_string())
                            }
                        } else {
                            Err("No KI slot for tunnel".to_string())
                        }
                    } else {
                        Err("Tunnel not found".to_string())
                    };
                    let _ = reply.send(result);
                }

                ManagerCommand::CancelKeyboardInteractive { tunnel_id } => {
                    if let Some(state) = self.tunnels.get(&tunnel_id) {
                        if let Some(ref ki_slot) = state.ki_slot {
                            let _ = ki_slot.lock().unwrap().take();
                        }
                    }
                }
```

- [ ] **Step 11: Rewrite handle_connect()**

Replace the entire `handle_connect()` method with auth-method-aware version:

```rust
    async fn handle_connect(&mut self, id: &str) -> Result<(), TunnelError> {
        let (config, local_port, remote_host, remote_port) = {
            let tunnel = self
                .tunnels
                .get_mut(id)
                .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

            if tunnel.status == TunnelStatus::Connected
                || tunnel.status == TunnelStatus::Connecting
            {
                debug!("Tunnel {} already connected/connecting", id);
                return Ok(());
            }

            tunnel.status = TunnelStatus::Connecting;
            tunnel.error_message = None;

            (
                tunnel.config.clone(),
                tunnel.config.local_port,
                tunnel.config.remote_host.clone(),
                tunnel.config.remote_port,
            )
        };
        self.emit_status(id, &TunnelStatus::Connecting);

        let timeout_secs = self.settings.connection_timeout_secs;
        let keepalive_interval = self.settings.keepalive_interval_secs;
        let keepalive_timeout = self.settings.keepalive_timeout_secs;

        // Build credentials
        let credentials = self.build_credentials(id, &config)?;

        // Validate and handle jump host
        if config.jump_host.is_some() {
            self.validate_jump_chain(id)?;
        }

        // Connect (with or without jump host)
        let (ssh, jump_ssh) = match config.jump_host {
            Some(ref jump_id) => {
                match self.connect_via_jump(jump_id, &config, credentials, timeout_secs).await {
                    Ok(result) => result,
                    Err(e) => {
                        let error_msg = e.to_string();
                        if let Some(tunnel) = self.tunnels.get_mut(id) {
                            tunnel.status = TunnelStatus::Disconnected;
                            tunnel.error_message = Some(error_msg);
                        }
                        self.emit_status(id, &TunnelStatus::Disconnected);
                        return Err(e);
                    }
                }
            }
            None => {
                match SshConnection::connect(
                    &config.host, config.port, &config.user, credentials, timeout_secs,
                ).await {
                    Ok(conn) => (Arc::new(conn), None),
                    Err(e) => {
                        let error_msg = e.to_string();
                        if let Some(tunnel) = self.tunnels.get_mut(id) {
                            tunnel.status = TunnelStatus::Disconnected;
                            tunnel.error_message = Some(error_msg);
                        }
                        self.emit_status(id, &TunnelStatus::Disconnected);
                        return Err(e);
                    }
                }
            }
        };

        // Increment generation
        let generation = {
            let tunnel = self.tunnels.get_mut(id).unwrap();
            tunnel.generation += 1;
            tunnel.generation
        };

        // Death channel, forwarder, health monitor (same as before)
        let (death_tx, mut death_rx) = mpsc::channel::<String>(1);
        let manager_tx = self.manager_tx.clone();
        let tunnel_id = id.to_string();

        let death_handle = tokio::spawn(async move {
            if let Some(error) = death_rx.recv().await {
                let _ = manager_tx
                    .send(ManagerCommand::TunnelDied {
                        id: tunnel_id,
                        error,
                        generation,
                    })
                    .await;
            }
        });

        let fwd_ssh = ssh.clone();
        let fwd_death_tx = death_tx.clone();
        let fwd_remote_host = remote_host.clone();
        let fwd_tunnel_id = id.to_string();
        let forwarder_handle = tokio::spawn(async move {
            if let Err(e) = PortForwarder::start(
                fwd_ssh, local_port, fwd_remote_host, remote_port,
                fwd_death_tx.clone(), fwd_tunnel_id,
            ).await {
                warn!("Port forwarder exited with error: {}", e);
                let _ = fwd_death_tx.send(format!("Port forwarder failed: {}", e)).await;
            }
        });

        let health_ssh = ssh.clone();
        let health_tunnel_id = id.to_string();
        let health_death_tx = death_tx;
        let health_handle = tokio::spawn(async move {
            HealthMonitor::run(
                health_ssh, health_tunnel_id,
                keepalive_interval, keepalive_timeout, health_death_tx,
            ).await;
        });

        // Store state
        {
            let tunnel = self.tunnels.get_mut(id).unwrap();
            for handle in tunnel.abort_handles.drain(..) {
                handle.abort();
            }
            tunnel.abort_handles.push(forwarder_handle.abort_handle());
            tunnel.abort_handles.push(health_handle.abort_handle());
            tunnel.abort_handles.push(death_handle.abort_handle());
            tunnel.ssh_connection = Some(ssh);
            tunnel.jump_connection = jump_ssh;
            tunnel.status = TunnelStatus::Connected;
        }
        self.emit_status(id, &TunnelStatus::Connected);

        info!("Tunnel {} connected", id);
        Ok(())
    }
```

- [ ] **Step 12: Add build_credentials() helper**

```rust
    fn build_credentials(
        &mut self,
        tunnel_id: &str,
        config: &TunnelConfig,
    ) -> Result<AuthCredentials, TunnelError> {
        match config.auth_method {
            AuthMethod::Key => {
                let expanded = ConfigStore::expand_tilde(&config.key_path);
                let passphrase = keychain::get_passphrase(
                    expanded.to_string_lossy().as_ref(),
                );
                Ok(AuthCredentials::Key {
                    key_path: config.key_path.clone(),
                    passphrase,
                })
            }
            AuthMethod::Password => {
                match keychain::get_password(&config.id) {
                    Some(pw) => Ok(AuthCredentials::Password(pw)),
                    None => Err(TunnelError::PasswordRequired(config.id.clone())),
                }
            }
            AuthMethod::Agent => Ok(AuthCredentials::Agent),
            AuthMethod::KeyboardInteractive => {
                let app_handle = self.app_handle.clone().ok_or_else(|| {
                    TunnelError::AuthFailed("Keyboard-interactive auth requires app_handle".into())
                })?;
                let ki_slot: KiResponseSlot =
                    Arc::new(std::sync::Mutex::new(None));
                // Store in tunnel state for respond command
                if let Some(state) = self.tunnels.get_mut(tunnel_id) {
                    state.ki_slot = Some(ki_slot.clone());
                }
                Ok(AuthCredentials::KeyboardInteractive {
                    ki_slot,
                    app_handle,
                    tunnel_id: config.id.clone(),
                })
            }
        }
    }
```

- [ ] **Step 13: Add connect_via_jump() helper**

```rust
    async fn connect_via_jump(
        &mut self,
        jump_id: &str,
        dest_config: &TunnelConfig,
        dest_credentials: AuthCredentials,
        timeout_secs: u64,
    ) -> Result<(Arc<SshConnection>, Option<Arc<SshConnection>>), TunnelError> {
        // Resolve jump host config
        let jump_config = self
            .tunnels
            .get(jump_id)
            .ok_or_else(|| TunnelError::JumpHostNotFound(jump_id.to_string()))?
            .config
            .clone();

        // Build jump host credentials
        let jump_credentials = match jump_config.auth_method {
            AuthMethod::Key => {
                let expanded = ConfigStore::expand_tilde(&jump_config.key_path);
                let passphrase = keychain::get_passphrase(
                    expanded.to_string_lossy().as_ref(),
                );
                AuthCredentials::Key {
                    key_path: jump_config.key_path.clone(),
                    passphrase,
                }
            }
            AuthMethod::Password => {
                match keychain::get_password(&jump_config.id) {
                    Some(pw) => AuthCredentials::Password(pw),
                    None => return Err(TunnelError::JumpHostFailed(
                        format!("Password required for jump host '{}'", jump_config.name),
                    )),
                }
            }
            AuthMethod::Agent => AuthCredentials::Agent,
            AuthMethod::KeyboardInteractive => {
                // KI for jump hosts requires its own ki_slot + app_handle
                let app_handle = self.app_handle.clone().ok_or_else(|| {
                    TunnelError::JumpHostFailed("KI auth for jump host requires app_handle".into())
                })?;
                let ki_slot: KiResponseSlot = Arc::new(std::sync::Mutex::new(None));
                // Store ki_slot in jump host's tunnel state so respond command can find it
                if let Some(jh_state) = self.tunnels.get_mut(&jump_config.id) {
                    jh_state.ki_slot = Some(ki_slot.clone());
                }
                AuthCredentials::KeyboardInteractive {
                    ki_slot,
                    app_handle,
                    tunnel_id: jump_config.id.clone(),
                }
            }
        };

        // Connect to jump host
        let jump_ssh = SshConnection::connect(
            &jump_config.host,
            jump_config.port,
            &jump_config.user,
            jump_credentials,
            timeout_secs,
        )
        .await
        .map_err(|e| TunnelError::JumpHostFailed(format!("Jump host connection failed: {}", e)))?;

        let jump_ssh = Arc::new(jump_ssh);

        // Open channel through jump host to destination
        // Note: open_direct_tcpip() already exists in SshConnection (connection.rs:204-227)
        let channel = jump_ssh
            .open_direct_tcpip(
                &dest_config.host,
                dest_config.port,
                "127.0.0.1",
                0,
            )
            .await
            .map_err(|e| TunnelError::JumpHostFailed(format!("Failed to open channel: {}", e)))?;

        // Connect destination SSH over the channel stream
        let dest_ssh = SshConnection::connect_stream(
            channel.into_stream(),
            &dest_config.host,
            dest_config.port,
            &dest_config.user,
            dest_credentials,
            timeout_secs,
        )
        .await?;

        Ok((Arc::new(dest_ssh), Some(jump_ssh)))
    }
```

- [ ] **Step 14: Update handle_disconnect() to clean up jump_connection**

After `ssh_connection.take()` disconnect (line 387-389), add:

```rust
            if let Some(jump) = tunnel.jump_connection.take() {
                jump.disconnect().await;
            }
            tunnel.ki_slot = None;
```

Do the same in `handle_tunnel_died()` cleanup block (after line 429-431).

- [ ] **Step 15: Update handle_remove_tunnel()**

Replace `handle_remove_tunnel()` with:

```rust
    async fn handle_remove_tunnel(&mut self, id: &str) -> Result<(), TunnelError> {
        // Disconnect if currently connected
        if let Some(tunnel) = self.tunnels.get(id) {
            if tunnel.status == TunnelStatus::Connected || tunnel.status == TunnelStatus::Connecting {
                info!("Disconnecting tunnel '{}' before removal", id);
                self.handle_disconnect(id).await.ok();
            }
        }

        // Disconnect dependent tunnels
        let dependents: Vec<String> = self
            .tunnels
            .iter()
            .filter(|(_, s)| s.config.jump_host.as_deref() == Some(id))
            .filter(|(_, s)| {
                s.status == TunnelStatus::Connected || s.status == TunnelStatus::Connecting
            })
            .map(|(dep_id, _)| dep_id.clone())
            .collect();
        for dep_id in &dependents {
            info!("Disconnecting dependent tunnel '{}' before removing jump host", dep_id);
            self.handle_disconnect(dep_id).await.ok();
        }

        // Clear dangling jump host references
        for state in self.tunnels.values_mut() {
            if state.config.jump_host.as_deref() == Some(id) {
                state.config.jump_host = None;
            }
        }

        // Delete password from keyring
        keychain::delete_password(id);

        match self.tunnels.remove(id) {
            Some(_) => {
                info!("Removed tunnel '{}'", id);
                Ok(())
            }
            None => Err(TunnelError::TunnelNotFound(id.to_string())),
        }
    }
```

- [ ] **Step 16: Update tests**

Update `test_config()` and all `TunnelConfig` literals in tests to include new fields:

```rust
    auth_method: AuthMethod::Key,
    jump_host: None,
```

Add `use crate::types::AuthMethod;` to test imports.

Update `spawn_manager` calls in tests — they now accept `Option<tauri::AppHandle>`. Tests pass `None`:

```rust
let manager = spawn_manager(config, Some(event_tx), Some(error_tx), None);
```

This works because `app_handle` is `Option<tauri::AppHandle>` — `None` is valid for tests. KI auth will return an error if `app_handle` is `None`, which is correct behavior for tests that don't need KI.

- [ ] **Step 17: Run tests**

Run: `cd src-tauri && cargo test --lib`
Expected: All tests pass (with possible ignored tests)

- [ ] **Step 18: Commit**

```bash
git add src-tauri/src/tunnel/manager.rs
git commit -m "feat: rewrite manager with auth dispatch, ProxyJump, KI commands"
```

---

## Chunk 4: Commands, lib.rs Wiring, Frontend

### Task 7: Add new Tauri commands and update delete_tunnel

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Add imports**

Ensure these are in the imports at the top of `commands.rs`:

```rust
use tokio::sync::oneshot;
use crate::tunnel::manager::ManagerCommand;
```

If `ManagerCommand` is already imported, just add `oneshot`.

- [ ] **Step 2: Add store_password_for_tunnel command**

```rust
#[tauri::command]
pub async fn store_password_for_tunnel(
    id: String,
    password: String,
) -> Result<(), String> {
    crate::keychain::store_password(&id, &password)
}
```

- [ ] **Step 3: Add respond_keyboard_interactive command**

```rust
#[tauri::command]
pub async fn respond_keyboard_interactive(
    id: String,
    responses: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::RespondKeyboardInteractive {
            tunnel_id: id,
            responses,
            reply: reply_tx,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;
    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))?
}
```

- [ ] **Step 4: Add cancel_keyboard_interactive command**

```rust
#[tauri::command]
pub async fn cancel_keyboard_interactive(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .manager
        .send(ManagerCommand::CancelKeyboardInteractive {
            tunnel_id: id,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))
}
```

- [ ] **Step 5: Update delete_tunnel to clear dangling references on disk**

In `delete_tunnel`, after `app_config.tunnels.retain(|t| t.id != id);` add:

```rust
    // Clear dangling jump host references on disk
    for tunnel in &mut app_config.tunnels {
        if tunnel.jump_host.as_deref() == Some(id.as_str()) {
            tunnel.jump_host = None;
        }
    }
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add password, KI commands; update delete_tunnel for dangling refs"
```

---

### Task 8: Wire up lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Update spawn_manager call**

Change line 219 from:
```rust
let manager = spawn_manager(config, Some(event_tx), Some(error_tx));
```
to:
```rust
let manager = spawn_manager(config, Some(event_tx), Some(error_tx), Some(app.handle().clone()));
```

- [ ] **Step 2: Register new commands**

Add to the `invoke_handler` list (lines 291-304):
```rust
            commands::store_password_for_tunnel,
            commands::respond_keyboard_interactive,
            commands::cancel_keyboard_interactive,
```

- [ ] **Step 3: Run cargo build**

Run: `cd src-tauri && cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire app_handle to manager, register new auth commands"
```

---

### Task 9: Update frontend types

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Add AuthMethod type and update interfaces**

```typescript
export type AuthMethod = "key" | "password" | "agent" | "keyboard-interactive";

export interface TunnelInfo {
  id: string;
  name: string;
  status: TunnelStatus;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  errorMessage: string | null;
  authMethod: AuthMethod;
  jumpHostName: string | null;
}

export interface TunnelInput {
  name: string;
  host: string;
  port: number;
  user: string;
  keyPath: string;
  authMethod: AuthMethod;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  autoConnect: boolean;
  jumpHost: string | null;
}

export interface TunnelConfig {
  id: string;
  name: string;
  host: string;
  port: number;
  user: string;
  keyPath: string;
  authMethod: AuthMethod;
  type: "local" | "reverse" | "dynamic";
  localPort: number;
  remoteHost: string;
  remotePort: number;
  autoConnect: boolean;
  jumpHost: string | null;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/types.ts
git commit -m "feat: add authMethod and jumpHost to frontend types"
```

---

### Task 10: Update useTunnels hook

**Files:**
- Modify: `src/hooks/useTunnels.ts`

- [ ] **Step 1: Add passwordPrompt state and handler**

Add state after `hostKeyPrompt`:

```typescript
  const [passwordPrompt, setPasswordPrompt] = useState<{
    tunnelId: string;
    tunnelName: string;
  } | null>(null);
```

- [ ] **Step 2: Add PASSWORD_REQUIRED handling in connect()**

In the `connect` callback, add a branch before the `else` block (after the HOST_KEY_CHANGED check):

```typescript
      } else if (errMsg.startsWith("PASSWORD_REQUIRED:")) {
        const tunnelId = errMsg.substring("PASSWORD_REQUIRED:".length);
        const name = tunnels.find((t) => t.id === tunnelId)?.name ?? tunnelId;
        setPasswordPrompt({ tunnelId, tunnelName: name });
```

- [ ] **Step 3: Add submitPassword and cancelPassword**

```typescript
  const submitPassword = useCallback(
    async (password: string) => {
      if (!passwordPrompt) return;
      const { tunnelId } = passwordPrompt;
      setPasswordPrompt(null);
      try {
        await invoke("store_password_for_tunnel", {
          id: tunnelId,
          password,
        });
        await invoke("connect_tunnel", { id: tunnelId });
      } catch (e) {
        setError(String(e));
      }
    },
    [passwordPrompt]
  );

  const cancelPassword = useCallback(() => {
    setPasswordPrompt(null);
  }, []);
```

- [ ] **Step 4: Add keyboard-interactive state and handlers**

```typescript
  const [kiPrompt, setKiPrompt] = useState<{
    tunnelId: string;
    name: string;
    instructions: string;
    prompts: Array<{ text: string; echo: boolean }>;
  } | null>(null);
```

Add event listener in the `useEffect` (alongside the tunnel-status-changed listener):

```typescript
    const unlistenKi = listen<{
      tunnelId: string;
      name: string;
      instructions: string;
      prompts: Array<{ text: string; echo: boolean }>;
    }>("keyboard-interactive-prompt", (event) => {
      setKiPrompt(event.payload);
    });
```

Update the cleanup:
```typescript
    return () => {
      unlisten.then((fn) => fn());
      unlistenKi.then((fn) => fn());
    };
```

Add handlers:
```typescript
  const respondKeyboardInteractive = useCallback(
    async (responses: string[]) => {
      if (!kiPrompt) return;
      const { tunnelId } = kiPrompt;
      setKiPrompt(null);
      try {
        await invoke("respond_keyboard_interactive", {
          id: tunnelId,
          responses,
        });
      } catch (e) {
        setError(String(e));
      }
    },
    [kiPrompt]
  );

  const cancelKeyboardInteractive = useCallback(async () => {
    if (!kiPrompt) return;
    const { tunnelId } = kiPrompt;
    setKiPrompt(null);
    try {
      await invoke("cancel_keyboard_interactive", { id: tunnelId });
    } catch (e) {
      setError(String(e));
    }
  }, [kiPrompt]);
```

- [ ] **Step 5: Add new exports to return object**

```typescript
  return {
    // ... existing ...
    passwordPrompt,
    submitPassword,
    cancelPassword,
    kiPrompt,
    respondKeyboardInteractive,
    cancelKeyboardInteractive,
  };
```

- [ ] **Step 6: Commit**

```bash
git add src/hooks/useTunnels.ts
git commit -m "feat: add password and KI prompt handling to useTunnels"
```

---

### Task 11: Create PasswordDialog and KeyboardInteractiveDialog

**Files:**
- Create: `src/components/PasswordDialog.tsx`
- Create: `src/components/KeyboardInteractiveDialog.tsx`

- [ ] **Step 1: Create PasswordDialog**

```typescript
import { useState } from "react";

interface PasswordDialogProps {
  tunnelName: string;
  onSubmit: (password: string) => void;
  onCancel: () => void;
}

export function PasswordDialog({
  tunnelName,
  onSubmit,
  onCancel,
}: PasswordDialogProps) {
  const [password, setPassword] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (password) {
      onSubmit(password);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
      <form
        onSubmit={handleSubmit}
        className="bg-white dark:bg-[#1a1a1a] rounded-xl p-4 mx-3 w-full border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.08)]"
      >
        <h3 className="text-sm font-semibold mb-1">Password Required</h3>
        <p className="text-xs text-[#999] dark:text-[#666] mb-3">
          Enter the SSH password for{" "}
          <span className="text-[#1a1a1a] dark:text-[#e5e5e5]">
            {tunnelName}
          </span>
          . It will be stored securely.
        </p>
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="SSH password"
          autoFocus
          className="w-full px-3 py-2 bg-[#fafafa] dark:bg-[#0f0f0f] border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)] rounded-md text-sm placeholder-[#bbb] dark:placeholder-[#555] focus:outline-none focus:ring-1 focus:ring-[#bbb] dark:focus:ring-[#555] mb-3"
        />
        <div className="flex gap-2 justify-end">
          <button
            type="button"
            onClick={onCancel}
            className="px-3 py-1.5 text-xs text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999] rounded"
          >
            Cancel
          </button>
          <button
            type="submit"
            className="px-3 py-1.5 text-xs font-medium bg-[#1a1a1a] dark:bg-[#e5e5e5] text-[#fafafa] dark:text-[#0f0f0f] rounded-md hover:opacity-90"
          >
            Connect
          </button>
        </div>
      </form>
    </div>
  );
}
```

- [ ] **Step 2: Create KeyboardInteractiveDialog**

```typescript
import { useState } from "react";

interface KeyboardInteractiveDialogProps {
  name: string;
  instructions: string;
  prompts: Array<{ text: string; echo: boolean }>;
  onSubmit: (responses: string[]) => void;
  onCancel: () => void;
}

export function KeyboardInteractiveDialog({
  name,
  instructions,
  prompts,
  onSubmit,
  onCancel,
}: KeyboardInteractiveDialogProps) {
  const [responses, setResponses] = useState<string[]>(
    prompts.map(() => "")
  );

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(responses);
  };

  const updateResponse = (index: number, value: string) => {
    setResponses((prev) => {
      const next = [...prev];
      next[index] = value;
      return next;
    });
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
      <form
        onSubmit={handleSubmit}
        className="bg-white dark:bg-[#1a1a1a] rounded-xl p-4 mx-3 w-full border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.08)]"
      >
        <h3 className="text-sm font-semibold mb-1">
          {name || "Authentication Required"}
        </h3>
        {instructions && (
          <p className="text-xs text-[#999] dark:text-[#666] mb-3">
            {instructions}
          </p>
        )}
        {prompts.map((prompt, i) => (
          <div key={i} className="mb-3">
            <label className="text-xs text-[#999] dark:text-[#666] mb-1 block">
              {prompt.text}
            </label>
            <input
              type={prompt.echo ? "text" : "password"}
              value={responses[i]}
              onChange={(e) => updateResponse(i, e.target.value)}
              autoFocus={i === 0}
              className="w-full px-3 py-2 bg-[#fafafa] dark:bg-[#0f0f0f] border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)] rounded-md text-sm placeholder-[#bbb] dark:placeholder-[#555] focus:outline-none focus:ring-1 focus:ring-[#bbb] dark:focus:ring-[#555]"
            />
          </div>
        ))}
        <div className="flex gap-2 justify-end">
          <button
            type="button"
            onClick={onCancel}
            className="px-3 py-1.5 text-xs text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999] rounded"
          >
            Cancel
          </button>
          <button
            type="submit"
            className="px-3 py-1.5 text-xs font-medium bg-[#1a1a1a] dark:bg-[#e5e5e5] text-[#fafafa] dark:text-[#0f0f0f] rounded-md hover:opacity-90"
          >
            Submit
          </button>
        </div>
      </form>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add src/components/PasswordDialog.tsx src/components/KeyboardInteractiveDialog.tsx
git commit -m "feat: add PasswordDialog and KeyboardInteractiveDialog components"
```

---

### Task 12: Update EditForm with auth method selector and jump host dropdown

**Files:**
- Modify: `src/components/EditForm.tsx`

- [ ] **Step 1: Update emptyForm and form loading**

Update `emptyForm` to include new fields:

```typescript
const emptyForm: TunnelInput = {
  name: "",
  host: "",
  port: 22,
  user: "",
  keyPath: "",
  authMethod: "key",
  localPort: 0,
  remoteHost: "",
  remotePort: 0,
  autoConnect: false,
  jumpHost: null,
};
```

Update the `getTunnelConfig` then-callback to include new fields:

```typescript
          setForm({
            name: config.name,
            host: config.host,
            port: config.port,
            user: config.user,
            keyPath: config.keyPath,
            authMethod: config.authMethod ?? "key",
            localPort: config.localPort,
            remoteHost: config.remoteHost,
            remotePort: config.remotePort,
            autoConnect: config.autoConnect,
            jumpHost: config.jumpHost ?? null,
          });
```

- [ ] **Step 2: Add tunnels prop for jump host dropdown**

Update `EditFormProps` to accept tunnels list:

```typescript
interface EditFormProps {
  tunnelId: string | null;
  tunnels: TunnelInfo[];  // NEW — for jump host dropdown
  getTunnelConfig: (id: string) => Promise<TunnelConfig>;
  onSave: (input: TunnelInput, id: string | null) => Promise<void>;
  onBack: () => void;
}
```

Import `TunnelInfo` from types:
```typescript
import type { TunnelInput, TunnelConfig, TunnelInfo, AuthMethod } from "../types";
```

- [ ] **Step 3: Add auth method selector after Username row**

After the Username `FormRow` (line 125) and before the Key path `div` (line 126), add:

```tsx
          {/* Auth Method */}
          <div className="flex items-center px-3 py-2 border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
            <label className="text-sm text-[#999] dark:text-[#666] w-[70px] flex-shrink-0">Auth</label>
            <div className="flex gap-1 flex-1">
              {(["key", "password", "agent", "keyboard-interactive"] as AuthMethod[]).map((method) => {
                const labels: Record<AuthMethod, string> = {
                  key: "Key",
                  password: "Password",
                  agent: "Agent",
                  "keyboard-interactive": "2FA",
                };
                return (
                  <button
                    key={method}
                    type="button"
                    onClick={() => updateField("authMethod", method)}
                    className={`px-2 py-1 text-xs rounded-md transition-colors ${
                      form.authMethod === method
                        ? "bg-[#1a1a1a] dark:bg-[#e5e5e5] text-white dark:text-[#0f0f0f] font-medium"
                        : "text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999]"
                    }`}
                  >
                    {labels[method]}
                  </button>
                );
              })}
            </div>
          </div>
```

- [ ] **Step 4: Conditionally show key path only for "key" auth**

Wrap the existing Key path `div` (lines 126-153) in a conditional:

```tsx
          {form.authMethod === "key" && (
            <div className="flex items-center px-3 py-2">
              {/* ... existing key path UI ... */}
            </div>
          )}
```

- [ ] **Step 5: Add Jump Host dropdown after the connection section**

After the connection section's closing `</div>` (line 154), add:

```tsx
          {/* Jump Host */}
          <div className="flex items-center px-3 py-2 border-t border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
            <label className="text-sm text-[#999] dark:text-[#666] w-[70px] flex-shrink-0">Jump</label>
            <select
              value={form.jumpHost ?? ""}
              onChange={(e) => updateField("jumpHost", e.target.value || null)}
              className="flex-1 bg-transparent text-sm outline-none text-[#1a1a1a] dark:text-[#e5e5e5]"
            >
              <option value="">None</option>
              {tunnels
                .filter((t) => t.id !== tunnelId)
                .map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
            </select>
          </div>
```

Note: This goes inside the connection section `div`, before its closing tag. The jump host dropdown should be the last row in the Connection section.

**Loop prevention:** The frontend excludes self from the dropdown. Full circular reference detection is handled server-side by `validate_jump_chain()` in the manager, which runs at connect time and returns a clear error. This is simpler and more reliable than trying to replicate chain-walking logic in the frontend (which would need the full jump_host graph that TunnelInfo doesn't carry).

- [ ] **Step 6: Commit**

```bash
git add src/components/EditForm.tsx
git commit -m "feat: add auth method selector and jump host dropdown to EditForm"
```

---

### Task 13: Update TunnelItem to show jump host

**Files:**
- Modify: `src/components/TunnelItem.tsx`

- [ ] **Step 1: Add "via {jumpHostName}" to subtitle**

After the forwarding spec line (line 40-41), add:

```tsx
          {tunnel.jumpHostName && (
            <div className="text-xs text-[#999] dark:text-[#555] truncate">
              via {tunnel.jumpHostName}
            </div>
          )}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/TunnelItem.tsx
git commit -m "feat: show 'via jump host' in TunnelItem subtitle"
```

---

### Task 14: Wire everything up in App.tsx

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Destructure new values from useTunnels**

Add to the destructuring:

```typescript
    passwordPrompt,
    submitPassword,
    cancelPassword,
    kiPrompt,
    respondKeyboardInteractive,
    cancelKeyboardInteractive,
```

- [ ] **Step 2: Pass tunnels to EditForm**

Add `tunnels` prop to EditForm:

```tsx
      <EditForm
        tunnelId={view.tunnelId}
        tunnels={tunnels}
        getTunnelConfig={getTunnelConfig}
        onSave={handleSave}
        onBack={() => setView({ kind: "edit-list" })}
      />
```

- [ ] **Step 3: Add PasswordDialog and KeyboardInteractiveDialog**

Import the new components:
```typescript
import { PasswordDialog } from "./components/PasswordDialog";
import { KeyboardInteractiveDialog } from "./components/KeyboardInteractiveDialog";
```

Add after the HostKeyDialog block (after line 148):

```tsx
      {/* Password dialog */}
      {passwordPrompt && (
        <PasswordDialog
          tunnelName={passwordPrompt.tunnelName}
          onSubmit={submitPassword}
          onCancel={cancelPassword}
        />
      )}

      {/* Keyboard-interactive dialog */}
      {kiPrompt && (
        <KeyboardInteractiveDialog
          name={kiPrompt.name}
          instructions={kiPrompt.instructions}
          prompts={kiPrompt.prompts}
          onSubmit={respondKeyboardInteractive}
          onCancel={cancelKeyboardInteractive}
        />
      )}
```

- [ ] **Step 4: Build and verify**

Run: `npm run build && cd src-tauri && cargo build`
Expected: Everything compiles

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx
git commit -m "feat: wire up PasswordDialog and KeyboardInteractiveDialog in App"
```

---

## Chunk 5: Final Integration

### Task 15: Full build and manual smoke test

- [ ] **Step 1: Run full test suite**

Run: `cd src-tauri && cargo test --lib`
Expected: All tests pass

- [ ] **Step 2: Run frontend build**

Run: `npm run build`
Expected: No TypeScript errors

- [ ] **Step 3: Run dev mode**

Run: `npm run tauri dev`
Expected: App launches, tray icon appears

- [ ] **Step 4: Manual smoke tests**
- Create a new tunnel with Key auth (existing behavior works)
- Create a tunnel with Password auth (should prompt for password on connect)
- Create a tunnel with Agent auth (if SSH agent running, should connect; if not, should show agent unavailable error)
- Create a tunnel with 2FA/KI auth against a KI-enabled server (should show KI dialog with prompts)
- Edit a tunnel and change auth method (key path hides/shows)
- Select a jump host from dropdown (self should not appear in list)
- Attempt circular jump host reference (should get error from backend at connect time)
- Delete a tunnel that's used as a jump host (dependent tunnels should disconnect, references cleared)

- [ ] **Step 5: Final commit and tag**

```bash
git add src-tauri/src/types.rs src-tauri/src/errors.rs src-tauri/src/keychain.rs \
  src-tauri/src/tunnel/connection.rs src-tauri/src/tunnel/manager.rs \
  src-tauri/src/config/store.rs src-tauri/src/commands.rs src-tauri/src/lib.rs \
  src/types.ts src/hooks/useTunnels.ts src/App.tsx \
  src/components/EditForm.tsx src/components/TunnelItem.tsx \
  src/components/PasswordDialog.tsx src/components/KeyboardInteractiveDialog.tsx
git commit -m "feat: complete auth methods and ProxyJump implementation"
```
