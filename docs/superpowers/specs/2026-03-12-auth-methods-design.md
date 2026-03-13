# Extended Authentication Methods & ProxyJump — Design Spec

## Problem

Tunnel Master currently only supports SSH public key authentication. Real-world SSH environments use a mix of auth methods — password, SSH agent, keyboard-interactive (2FA) — and often require multi-hop connections through bastion/jump hosts. Users who don't use key-based auth or need to reach hosts behind a bastion can't use the app.

## Scope

Add four authentication methods and ProxyJump support:

1. **Key auth** — existing, no changes needed
2. **Password auth** — username/password, stored in keyring
3. **SSH Agent** — use keys from the running ssh-agent
4. **Keyboard-interactive (2FA/MFA)** — server-driven prompts
5. **ProxyJump** — chain SSH connections through a jump host

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Auth method selection | Explicit dropdown in form | Avoids confusing auto-detect failures; matches Termius/PuTTY UX |
| Password storage | Keyring (same as passphrases) | Never stored in config file; cross-platform via `keyring` crate |
| Jump host config | Reference another tunnel by ID | Reuses existing config; single source of truth for credentials |
| Jump host connections | One per tunnel (not shared) | Simpler lifecycle; no shared state between tunnels |
| SSH agent client | `russh_keys::agent::client::AgentClient` | Already a dependency; `connect_env()` on Unix, manual named pipe on Windows |
| SSH over channel | `russh::client::connect_stream` | Accepts any `AsyncRead+AsyncWrite`; channel's `into_stream()` provides this |
| Config migration | `serde(default)` for new fields | Existing configs without `authMethod` default to `"key"`; no breaking change |
| Keyboard-interactive sync | Shared `Arc<Mutex<Option<oneshot::Sender>>>` between handler and TunnelState | Handler stores tx; Tauri command reads from TunnelState to send responses |
| Windows SSH agent | Named pipe `\\.\pipe\openssh-ssh-agent` via `tokio::net::windows::named_pipe` | Standard OpenSSH for Windows agent socket; `connect_env()` is Unix-only |

## Config Schema Changes

### TunnelConfig (Rust)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    #[serde(default)]                   // NEW — defaults to Key
    pub auth_method: AuthMethod,
    pub key_path: String,               // existing — used when auth_method = Key
    #[serde(rename = "type")]
    pub tunnel_type: TunnelType,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    #[serde(default)]
    pub auto_connect: bool,
    #[serde(default)]                   // NEW — ID of another tunnel, or None
    pub jump_host: Option<String>,
}

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

### TunnelInput (frontend → backend)

```typescript
interface TunnelInput {
  name: string;
  host: string;
  port: number;
  user: string;
  authMethod: "key" | "password" | "agent" | "keyboard-interactive";
  keyPath: string;        // only relevant for authMethod "key"
  localPort: number;
  remoteHost: string;
  remotePort: number;
  autoConnect: boolean;
  jumpHost: string | null; // tunnel ID or null
}
```

### TunnelInput (Rust)

```rust
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
    #[serde(default)]
    pub auth_method: AuthMethod,     // NEW
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    #[serde(default)]
    pub auto_connect: bool,
    #[serde(default)]
    pub jump_host: Option<String>,   // NEW
}

impl TunnelInput {
    pub fn to_config(self, id: String) -> TunnelConfig {
        TunnelConfig {
            id,
            name: self.name,
            host: self.host,
            port: self.port,
            user: self.user,
            auth_method: self.auth_method,  // NEW
            key_path: self.key_path,
            tunnel_type: TunnelType::Local,
            local_port: self.local_port,
            remote_host: self.remote_host,
            remote_port: self.remote_port,
            auto_connect: self.auto_connect,
            jump_host: self.jump_host,      // NEW
        }
    }
}
```

### TunnelInfo

Add `auth_method` and `jump_host_name` so the frontend can display auth type and "via {name}" without cross-referencing IDs:

```rust
pub struct TunnelInfo {
    pub id: String,
    pub name: String,
    pub status: TunnelStatus,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub error_message: Option<String>,
    pub auth_method: AuthMethod,          // NEW
    pub jump_host_name: Option<String>,   // NEW — resolved name for display
}
```

The `to_info()` method moves from `TunnelState` to `TunnelManagerActor` (renamed `tunnel_to_info`) so it can look up the jump host name. All call sites (e.g., `ListTunnels` handler) change from `t.to_info()` to `self.tunnel_to_info(t)`:

```rust
impl TunnelManagerActor {
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
}
```

### Validation Changes

The existing `validate_tunnel_input` in `config/store.rs` checks `key_path` unconditionally. This must be gated on `auth_method`:

```rust
fn validate_tunnel_input(input: &TunnelInput) -> Result<(), TunnelError> {
    // Only validate key_path when auth method is Key
    if input.auth_method == AuthMethod::Key {
        if input.key_path.is_empty() {
            return Err(TunnelError::ConfigInvalid("Key path is required for key authentication".into()));
        }
        let expanded = ConfigStore::expand_tilde(&input.key_path);
        if !std::path::Path::new(&expanded).exists() {
            return Err(TunnelError::ConfigInvalid("Key file not found".into()));
        }
    }
    // ... other validations unchanged ...
    Ok(())
}
```

Note: existing tests that create `TunnelInput` with `key_path: ""` and expect validation to pass will need updating — either set `auth_method: AuthMethod::Password` (or similar) or provide a valid `key_path`.

### Migration

Existing configs without `authMethod` deserialize as `"key"` via `#[serde(default)]`. Existing configs without `jumpHost` deserialize as `null`. No migration code needed.

## Authentication Flows

### Key Auth (existing)

```
Connect
→ load private key from keyPath
→ get passphrase from keyring (if encrypted, prompt user if missing)
→ session.authenticate_publickey(user, key)
→ Connected
```

No changes to existing flow.

### Password Auth

```
Connect
→ get password from keyring (keyed by "password/{tunnel_id}")
→ if missing: return TunnelError::PasswordRequired(tunnel_id)
→ UI detects "PASSWORD_REQUIRED:{tunnel_id}" in error message
→ UI shows password dialog (reuses PassphraseDialog pattern)
→ user enters password → store_password_for_tunnel command → keyring
→ retry Connect
→ session.authenticate_password(user, password)
→ if auth rejected (returns false): return TunnelError::AuthFailed("Server rejected password")
→ Connected
```

New error variant: `PasswordRequired(String)` — serializes as `"PASSWORD_REQUIRED:{tunnel_id}"`

Frontend detection: error message starts with `PASSWORD_REQUIRED:` prefix, same pattern as existing `UNKNOWN_HOST_KEY:` and `PASSPHRASE_REQUIRED:`.

Credential store key: `tunnel-master` service, `password/<tunnel_id>` user.

### SSH Agent

```
Connect
→ Unix: AgentClient::connect_env().await  (reads SSH_AUTH_SOCK)
→ Windows: open named pipe \\.\pipe\openssh-ssh-agent, then AgentClient::connect(stream)
→ list keys: agent.request_identities().await → Vec<(PublicKey, String)>
→ try each key: session.authenticate_future(user, key, agent).await
→   authenticate_future returns (agent, Result<bool>) — reuse agent for next key
→ if any key accepted: Connected
→ if all rejected: TunnelError::AuthFailed("No agent key accepted by server")
```

**Actual russh API:**
- `AgentClient::connect_env() -> Result<AgentClient<UnixStream>>` — Unix only
- `AgentClient::connect(stream) -> AgentClient<S>` — for any `AsyncRead+AsyncWrite` stream
- `agent.request_identities() -> Result<Vec<(PublicKey, String)>>` — list loaded keys
- `session.authenticate_future(user, pubkey, signer) -> (signer, Result<bool>)` — requires a `PublicKey` argument in addition to the signer; iterate over agent keys

If `SSH_AUTH_SOCK` is not set (Unix) or the named pipe is unreachable (Windows): return `TunnelError::AgentUnavailable("SSH agent not available")`.

No credentials to store — the agent handles key management.

**Note on GUI launches:** When the app is launched from a desktop shortcut (not a terminal), `SSH_AUTH_SOCK` may not be inherited. The error message should be clear about this (`"SSH agent not available — SSH_AUTH_SOCK not set. Try launching from a terminal or ensure your agent is running."`). A future enhancement could allow specifying a custom agent socket path in Settings, but this is out of scope for now.

### Keyboard-Interactive (2FA)

Two-phase flow requiring synchronization between the russh `Handler` callback and the frontend UI.

#### Synchronization Architecture

The challenge: russh's `Handler::auth_keyboard_interactive` callback runs inside the SSH transport task. It must await a response from the frontend before returning. The Tauri command `respond_keyboard_interactive` runs in a separate task and needs to deliver that response.

Solution: a shared `Arc<std::sync::Mutex<Option<oneshot::Sender<Vec<String>>>>>` that lives **outside** the handler, created before `connect()`/`connect_stream()` is called, and stored in both the handler and in `TunnelState`.

```rust
// Created before the SSH connection:
type KiResponseSlot = Arc<std::sync::Mutex<Option<oneshot::Sender<Vec<String>>>>>;

let ki_slot: KiResponseSlot = Arc::new(std::sync::Mutex::new(None));

// Clone for the handler (moved into russh transport):
let handler = SshClientHandler {
    host: host.to_string(),
    port,
    check_result: check_result.clone(),
    ki_slot: ki_slot.clone(),               // handler writes Sender here
    app_handle: Some(app_handle.clone()),   // Some for KI, None for other auth methods
    tunnel_id: tunnel_id.to_string(),
};

// The same ki_slot is stored in TunnelState after connect():
// tunnel_state.ki_slot = Some(ki_slot);
```

Why `std::sync::Mutex`: the handler's `auth_keyboard_interactive` is async but only locks the mutex briefly to swap `None`↔`Some(tx)` — no `.await` while holding the lock. The Tauri command similarly does a brief lock-swap. `std::sync::Mutex` is safe and simpler here.

#### Handler Callback

```rust
#[async_trait]
impl client::Handler for SshClientHandler {
    async fn auth_keyboard_interactive(
        &mut self,
        name: &str,
        instructions: &str,
        prompts: &[(Cow<'_, str>, bool)],
    ) -> Result<Vec<String>, Self::Error> {
        // Create oneshot for this round
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Store sender so respond_keyboard_interactive command can find it
        *self.ki_slot.lock().unwrap() = Some(tx);

        // Emit event to frontend
        let prompt = KeyboardInteractivePrompt {
            tunnel_id: self.tunnel_id.clone(),
            name: name.to_string(),
            instructions: instructions.to_string(),
            prompts: prompts.iter().map(|(text, echo)| KiPromptEntry {
                text: text.to_string(),
                echo: *echo,
            }).collect(),
        };
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit("keyboard-interactive-prompt", &prompt);
        }

        // Block until frontend responds or cancels
        rx.await.map_err(|_| russh::Error::Disconnect)
    }
}
```

#### Auth Completion

`authenticate_keyboard_interactive` is not a single call — russh drives the exchange through repeated `Handler::auth_keyboard_interactive` callbacks. The `connect()` method does NOT return until auth completes or fails. The flow:

1. Caller calls `session.authenticate_keyboard_interactive_start(user, None).await`
2. This triggers the server exchange; russh calls `handler.auth_keyboard_interactive()` for each prompt round
3. The handler awaits the oneshot (blocks until the frontend responds)
4. russh sends the response, server may send more prompts (repeating step 2-3)
5. When the server sends `SSH_MSG_USERAUTH_SUCCESS`, russh returns from the final callback
6. `authenticate_keyboard_interactive_start` ultimately resolves

However, `authenticate_keyboard_interactive_start` returns `bool` (accepted/rejected) — it does NOT return until the full exchange completes. So `connect()` can simply check the return value:

```rust
AuthMethod::KeyboardInteractive => {
    let accepted = session
        .authenticate_keyboard_interactive_start(user, None)
        .await
        .map_err(|e| TunnelError::AuthFailed(format!("KI auth error: {}", e)))?;
    if !accepted {
        return Err(TunnelError::AuthFailed("Server rejected authentication".into()));
    }
}
```

If the user cancels the dialog, the `cancel_keyboard_interactive` command drops the `ki_slot`'s sender, causing `rx.await` to return `Err` in the handler, which propagates as a disconnect.

#### Event Payload

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardInteractivePrompt {
    pub tunnel_id: String,
    pub name: String,        // server-provided name (often empty)
    pub instructions: String, // server-provided instructions
    pub prompts: Vec<KiPromptEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiPromptEntry {
    pub text: String,
    pub echo: bool,  // true = show input, false = mask like password
}
```

### Auth Credentials Enum

To avoid a flat parameter list that couples unrelated auth methods, use an enum:

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
        app_handle: tauri::AppHandle,
        tunnel_id: String,
    },
}
```

### Connect Method Redesign

The current `SshConnection::connect()` is replaced with a new signature:

```rust
impl SshConnection {
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        credentials: AuthCredentials,
        timeout_secs: u64,
    ) -> Result<Self, TunnelError> {
        // 1. TCP connect + host key check (same as today)
        let check_result = Arc::new(std::sync::Mutex::new(None));
        let ki_slot = match &credentials {
            AuthCredentials::KeyboardInteractive { ki_slot, .. } => ki_slot.clone(),
            _ => Arc::new(std::sync::Mutex::new(None)),
        };
        let (app_handle, tunnel_id) = match &credentials {
            AuthCredentials::KeyboardInteractive { app_handle, tunnel_id, .. } =>
                (Some(app_handle.clone()), tunnel_id.clone()),
            _ => (None, String::new()),
        };

        let handler = SshClientHandler {
            host: host.to_string(),
            port,
            check_result: check_result.clone(),
            ki_slot,
            app_handle,  // Option<tauri::AppHandle> — None for non-KI auth
            tunnel_id,
        };

        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(timeout_secs * 3)),
            keepalive_interval: Some(Duration::from_secs(15)),
            keepalive_max: 3,
            ..Default::default()
        });

        let addr = format!("{}:{}", host, port);
        let mut session = /* connect with timeout, same as today */;

        // 2. Dispatch auth based on credentials
        match credentials {
            AuthCredentials::Key { key_path, passphrase } => {
                let expanded = ConfigStore::expand_tilde(&key_path);
                let key_pair = russh_keys::load_secret_key(&expanded, passphrase.as_deref())?;
                let ok = session.authenticate_publickey(user, Arc::new(key_pair)).await?;
                if !ok { return Err(TunnelError::AuthFailed("Server rejected public key".into())); }
            }
            AuthCredentials::Password(password) => {
                let ok = session.authenticate_password(user, &password).await?;
                if !ok { return Err(TunnelError::AuthFailed("Server rejected password".into())); }
            }
            AuthCredentials::Agent => {
                #[cfg(unix)]
                let mut agent = russh_keys::agent::client::AgentClient::connect_env().await
                    .map_err(|e| TunnelError::AgentUnavailable(e.to_string()))?;
                #[cfg(windows)]
                let mut agent = {
                    let pipe = tokio::net::windows::named_pipe::ClientOptions::new()
                        .open(r"\\.\pipe\openssh-ssh-agent")
                        .map_err(|e| TunnelError::AgentUnavailable(e.to_string()))?;
                    russh_keys::agent::client::AgentClient::connect(pipe)
                };

                let identities = agent.request_identities().await
                    .map_err(|e| TunnelError::AgentUnavailable(e.to_string()))?;

                let mut accepted = false;
                for (pubkey, _comment) in identities {
                    let (returned_agent, result) =
                        session.authenticate_future(user, pubkey, agent).await;
                    agent = returned_agent;
                    match result {
                        Ok(true) => { accepted = true; break; }
                        Ok(false) => continue,
                        Err(e) => return Err(TunnelError::AuthFailed(e.to_string())),
                    }
                }
                if !accepted {
                    return Err(TunnelError::AuthFailed("No agent key accepted by server".into()));
                }
            }
            AuthCredentials::KeyboardInteractive { .. } => {
                let ok = session.authenticate_keyboard_interactive_start(user, None).await
                    .map_err(|e| TunnelError::AuthFailed(format!("KI auth error: {}", e)))?;
                if !ok {
                    return Err(TunnelError::AuthFailed("Server rejected authentication".into()));
                }
            }
        }

        Ok(Self { session })
    }

    /// Connect over an existing stream (for ProxyJump).
    pub async fn connect_stream<R: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        stream: R,
        user: &str,
        credentials: AuthCredentials,
        timeout_secs: u64,
    ) -> Result<Self, TunnelError> {
        // Same as connect() but uses client::connect_stream(config, stream, handler)
        // instead of client::connect(config, addr, handler).
        // The timeout wraps the entire connect_stream + auth sequence,
        // since the SSH handshake over a jump channel can hang.
        // Host key check and auth dispatch are identical to connect().
    }
}
```

### TunnelManagerActor Changes

The manager needs an `app_handle` field for keyboard-interactive event emission:

```rust
struct TunnelManagerActor {
    tunnels: HashMap<String, TunnelState>,
    settings: Settings,
    event_tx: Option<UnboundedSender<TunnelStatusEvent>>,
    error_tx: Option<UnboundedSender<TunnelErrorEvent>>,
    app_handle: tauri::AppHandle,  // NEW — for KI events
}
```

`spawn_manager` gains an `app_handle` parameter. In `lib.rs`, pass `app.handle().clone()` when calling `spawn_manager`.

### Manager's handle_connect Rewrite

The manager's `handle_connect` must build `AuthCredentials` from the tunnel config:

```rust
async fn handle_connect(&mut self, tunnel_id: &str) {
    let state = self.tunnels.get(tunnel_id).unwrap();
    let config = &state.config;

    // Build credentials based on auth method
    let credentials = match config.auth_method {
        AuthMethod::Key => {
            let passphrase = crate::keychain::get_passphrase(&config.key_path);
            AuthCredentials::Key {
                key_path: config.key_path.clone(),
                passphrase,
            }
        }
        AuthMethod::Password => {
            match crate::keychain::get_password(&config.id) {
                Some(pw) => AuthCredentials::Password(pw),
                None => {
                    self.set_error(tunnel_id, TunnelError::PasswordRequired(config.id.clone()));
                    return;
                }
            }
        }
        AuthMethod::Agent => AuthCredentials::Agent,
        AuthMethod::KeyboardInteractive => {
            let ki_slot = Arc::new(std::sync::Mutex::new(None));
            // Store ki_slot in tunnel state for respond_keyboard_interactive to access
            self.tunnels.get_mut(tunnel_id).unwrap().ki_slot = Some(ki_slot.clone());
            AuthCredentials::KeyboardInteractive {
                ki_slot,
                app_handle: self.app_handle.clone(),
                tunnel_id: config.id.clone(),
            }
        }
    };

    // Handle jump host if configured
    let ssh = if let Some(ref jump_id) = config.jump_host {
        // ... see ProxyJump section ...
    } else {
        SshConnection::connect(
            &config.host, config.port, &config.user,
            credentials, self.settings.connection_timeout_secs,
        ).await
    };

    // ... rest of connect flow (port forwarding, health monitor) ...
}
```

## ProxyJump

### Connection Flow

When a tunnel has `jumpHost` set:

1. **Circular reference check** — Walk the jump chain collecting visited IDs. If any ID appears twice, return `TunnelError::JumpHostFailed("Circular jump host reference")`. Maximum chain depth: 5 hops (prevents accidental deep recursion even without cycles).

```rust
fn validate_jump_chain(&self, tunnel_id: &str) -> Result<Vec<String>, TunnelError> {
    let mut visited = Vec::new();
    let mut current = Some(tunnel_id.to_string());
    while let Some(id) = current {
        if visited.contains(&id) {
            return Err(TunnelError::JumpHostFailed("Circular jump host reference".into()));
        }
        if visited.len() >= 5 {
            return Err(TunnelError::JumpHostFailed("Jump chain too deep (max 5)".into()));
        }
        visited.push(id.clone());
        current = self.tunnels.get(&id)
            .and_then(|s| s.config.jump_host.clone());
    }
    Ok(visited)
}
```

2. **Resolve jump host** — Look up the referenced tunnel's config by ID. If not found, return `TunnelError::JumpHostNotFound(jump_id)`.

3. **Connect to jump host** — Full SSH connection using the jump host's own auth method (key, password, agent, etc.). Uses `SshConnection::connect()`. If the jump host itself has a `jumpHost`, recurse (the chain was already validated in step 1). Host key verification triggers normally — the existing `HostKeyUnknown` error includes `host` and `port`, so the frontend knows which host the TOFU prompt is for.

4. **Open channel** — `jump_session.channel_open_direct_tcpip(destination_host, destination_port, "127.0.0.1", 0)`

5. **SSH over channel** — `SshConnection::connect_stream(channel.into_stream(), user, credentials, timeout)` to establish the SSH session to the final destination through the jump channel.

6. **Authenticate** — Using the tunnel's own auth method on the destination session (handled inside `connect_stream`).

7. **Port forward** — Normal port forwarding on the destination session.

### Jump Session Lifecycle

The jump session stays alive because its `Handle` is held in `TunnelState.jump_connection`. The `channel.into_stream()` produces a `ChannelStream` that reads/writes through the jump session's transport. As long as the `Handle` is not dropped and no disconnect is sent, the transport task continues running and the channel remains usable.

The jump session's built-in keepalive (`keepalive_interval: 15s`) keeps the jump connection alive independently. The health monitor only monitors the inner (destination) session — if the jump connection drops, the inner session's transport will fail, which the health monitor detects.

### TunnelState Changes

```rust
struct TunnelState {
    config: TunnelConfig,
    status: TunnelStatus,
    error_message: Option<String>,
    abort_handles: Vec<tokio::task::AbortHandle>,
    ssh_connection: Option<Arc<SshConnection>>,
    jump_connection: Option<Arc<SshConnection>>,  // NEW
    ki_slot: Option<KiResponseSlot>,              // NEW — for keyboard-interactive
    generation: u64,
}
```

On disconnect, both `ssh_connection` and `jump_connection` are disconnected (jump last, after inner).

### Dangling Jump Host References

When a tunnel is deleted, any other tunnels referencing it as a jump host must be cleaned up:

The existing `handle_remove_tunnel` in the manager gains cleanup steps:

```rust
// In handle_remove_tunnel (manager.rs):
fn handle_remove_tunnel(&mut self, tunnel_id: &str) {
    // 1. Disconnect if connected (existing)
    // 2. Disconnect dependent tunnels whose jump host is being deleted
    let dependents: Vec<String> = self.tunnels.iter()
        .filter(|(_, s)| s.config.jump_host.as_deref() == Some(tunnel_id))
        .filter(|(_, s)| s.status == TunnelStatus::Connected || s.status == TunnelStatus::Connecting)
        .map(|(id, _)| id.clone())
        .collect();
    for dep_id in &dependents {
        self.disconnect_tunnel(dep_id).await;
    }
    // 3. Clear dangling jump host references in other tunnels (in-memory)
    for state in self.tunnels.values_mut() {
        if state.config.jump_host.as_deref() == Some(tunnel_id) {
            state.config.jump_host = None;
        }
    }
    // 4. Remove from map (existing)
    self.tunnels.remove(tunnel_id);
    // 5. Delete password from keyring (if any)
    crate::keychain::delete_password(tunnel_id);
}
```

The `delete_tunnel` Tauri command in `commands.rs` must also clear dangling `jumpHost` references on disk. The current code loads `app_config`, filters out the deleted tunnel, and saves. It must also clear `jumpHost` on any remaining tunnels that referenced the deleted ID:

```rust
// In delete_tunnel command (commands.rs):
app_config.tunnels.retain(|t| t.id != id);
// Clear dangling jump host references on disk
for tunnel in &mut app_config.tunnels {
    if tunnel.jump_host.as_deref() == Some(&id) {
        tunnel.jump_host = None;
    }
}
config_store.save(&app_config)?;
```

This keeps the in-memory state (manager) and on-disk state (config.json) consistent.

## Frontend Changes

### EditForm

**Auth Method selector** — Below the Username field in the Connection section:
- Segmented control or dropdown: `Key | Password | Agent | 2FA`
- Default: `Key`
- When `Key`: show Key path + browse button (existing)
- When `Password`: hide Key path (password prompted on first connect)
- When `Agent`: hide Key path
- When `2FA`: hide Key path

**Jump Host dropdown** — New field at the bottom of the Connection section:
- Options: `None` + list of other tunnel names (excluding self, and excluding tunnels that would create a loop)
- Label: "Jump Host"
- Loop prevention: when building the dropdown options, walk each candidate's jump chain — if the current tunnel's ID appears anywhere in that chain, exclude it. This prevents both direct loops (A↔B) and indirect loops (A→B→C→A).
- When a jump host is selected, the TunnelItem shows "via {jump_name}" in its subtitle

### TunnelInput (TypeScript)

Add `authMethod` and `jumpHost` fields to the existing `TunnelInput` type.

### New Dialog: KeyboardInteractiveDialog

Similar to PassphraseDialog:
- Shows server-provided prompt text (e.g. "Verification code:")
- Each prompt gets its own input field (password-masked if `echo` is false, visible if true)
- Submit / Cancel buttons
- If multiple prompts, show all with labels

### Password Prompt

Reuse the existing PassphraseDialog pattern:
- Error message starts with `PASSWORD_REQUIRED:` → show password dialog
- User enters password → `store_password_for_tunnel` command → keyring
- Retry connect

### TunnelItem

When a tunnel has a jump host configured, show "via {jump_name}" in the subtitle using `jumpHostName` from `TunnelInfo`:
```
:5432 → db.internal:5432
via My Bastion
```

## New Tauri Commands

| Command | Args | Returns | Description |
|---------|------|---------|-------------|
| `store_password_for_tunnel` | `id, password` | `()` | Store SSH password in keyring |
| `respond_keyboard_interactive` | `id, responses` | `()` | Send 2FA responses to server via ki_slot |
| `cancel_keyboard_interactive` | `id` | `()` | Drop ki_slot sender to cancel pending 2FA |

### respond_keyboard_interactive Implementation

This command accesses the `ki_slot` stored in `TunnelState`:

```rust
#[tauri::command]
async fn respond_keyboard_interactive(
    id: String,
    responses: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    state.manager.send(ManagerCommand::RespondKeyboardInteractive {
        tunnel_id: id,
        responses,
        reply: reply_tx,
    }).await.map_err(|e| e.to_string())?;
    reply_rx.await.map_err(|e| e.to_string())?
}
```

The manager handles this by extracting the `oneshot::Sender` from `ki_slot` and sending the responses through it.

### cancel_keyboard_interactive Implementation

```rust
#[tauri::command]
async fn cancel_keyboard_interactive(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.manager.send(ManagerCommand::CancelKeyboardInteractive {
        tunnel_id: id,
    }).await.map_err(|e| e.to_string())
}
```

### New ManagerCommand Variants

```rust
enum ManagerCommand {
    // ... existing variants ...

    RespondKeyboardInteractive {
        tunnel_id: String,
        responses: Vec<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    CancelKeyboardInteractive {
        tunnel_id: String,
    },
}
```

Manager handling:

```rust
ManagerCommand::RespondKeyboardInteractive { tunnel_id, responses, reply } => {
    let result = if let Some(state) = self.tunnels.get(&tunnel_id) {
        if let Some(ref ki_slot) = state.ki_slot {
            if let Some(tx) = ki_slot.lock().unwrap().take() {
                tx.send(responses).map_err(|_| "KI channel closed".to_string())
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
    // Drop the sender to signal cancellation
    if let Some(state) = self.tunnels.get(&tunnel_id) {
        if let Some(ref ki_slot) = state.ki_slot {
            let _ = ki_slot.lock().unwrap().take(); // drop sender → rx returns Err
        }
    }
}
```

## New Events

| Event | Payload | Description |
|-------|---------|-------------|
| `keyboard-interactive-prompt` | `{ tunnelId, name, instructions, prompts: [{text, echo}] }` | Server requests user input during 2FA |

## Error Variants

New additions to `TunnelError`:

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

## Credential Storage

All secrets go through the cross-platform `keyring` crate:

| Secret | Keyring key |
|--------|-------------|
| Key passphrase | `tunnel-master` service, `<expanded_key_path>` user (existing) |
| SSH password | `tunnel-master` service, `password/<tunnel_id>` user |

Passwords are never written to the config file.

### Credential Cleanup

New keychain functions needed:

```rust
// keychain.rs additions:
pub fn get_password(tunnel_id: &str) -> Option<String> {
    let key = format!("password/{}", tunnel_id);
    let entry = keyring::Entry::new(SERVICE_NAME, &key).ok()?;
    entry.get_password().ok()
}

pub fn store_password(tunnel_id: &str, password: &str) -> Result<(), String> {
    let key = format!("password/{}", tunnel_id);
    let entry = keyring::Entry::new(SERVICE_NAME, &key).map_err(|e| e.to_string())?;
    entry.set_password(password).map_err(|e| e.to_string())
}

pub fn delete_password(tunnel_id: &str) {
    let key = format!("password/{}", tunnel_id);
    if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, &key) {
        let _ = entry.delete_credential(); // ignore errors (may not exist)
    }
}
```

On tunnel deletion: call `delete_password(tunnel_id)`. Passphrase cleanup is not needed since passphrases are keyed by key path (which may be shared across tunnels).

## Testing Strategy

- **Unit tests:** Auth credential building, jump chain validation (circular detection, depth limit), dangling reference cleanup, keychain get/store/delete for passwords
- **Integration tests:** Use a mock SSH server (e.g., `russh::server` or a Docker container with `sshd`) to verify:
  - Password auth flow (correct password accepted, wrong rejected)
  - SSH agent auth (with `ssh-agent` running)
  - Keyboard-interactive 2FA prompt/response flow — this requires a mock server that sends KI challenges, and a test harness that feeds responses through the `ki_slot` oneshot channel
  - ProxyJump through a jump host container to a destination container
- **Frontend tests:** Manual verification of form conditional visibility (auth method selector hiding/showing key path), jump host dropdown loop prevention, KI dialog rendering

## What's NOT Included

- **SSH certificates** — russh supports `authenticate_openssh_cert` but no current demand. Easy to add later as another `AuthMethod` variant.
- **GSSAPI/Kerberos** — Niche enterprise requirement. russh doesn't support it natively.
- **SOCKS/HTTP proxy** — Connecting through corporate proxies. Separate concern from auth methods.
- **Shared jump connections** — Each tunnel gets its own jump connection. Sharing adds complexity for minimal benefit.
- **Multi-hop UI** — Jump hosts can technically chain (A→B→C→D) via recursion, but we only test and document single-hop jumps. Max chain depth enforced at 5.
