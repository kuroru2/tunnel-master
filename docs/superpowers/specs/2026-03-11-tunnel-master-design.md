# Tunnel Master — Design Spec

macOS menu bar app for managing SSH tunnels, built with Rust + Tauri v2.

## Problem

SSH tunnels for development (databases, caches, internal services) require manual terminal commands. When VPN drops, tunnels die and leave zombie processes or hung terminals. Existing tools like Shuttle just wrap terminal SSH commands. We need a dedicated app that owns the SSH connection lifecycle and handles failures gracefully.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust (backend) + TypeScript (frontend) | Rust's safety for connection management, TS for fast UI iteration |
| Framework | Tauri v2 | Lightweight system tray + webview, not Electron-heavy |
| SSH library | `russh` | Pure Rust, async, well-maintained |
| UI model | Menu bar popover | Fast, unobtrusive for 3-5 tunnels |
| Config | Self-contained JSON | No dependency on `~/.ssh/config`, evolves toward Termius model |
| Auth (POC) | SSH keys + passphrase via macOS Keychain | Covers most real-world setups |
| Tunnel type (POC) | Local port forwarding (`-L`) | Most common use case |

## Architecture

### Components

**UI Layer (TypeScript/Tauri WebView)**
- `App.tsx` — main popover UI
- `TunnelList.tsx` — renders tunnel list with status indicators
- `TunnelItem.tsx` — single tunnel row with connect/disconnect toggle
- `useTunnels.ts` — hook bridging Tauri IPC commands and events

**Core Layer (Rust)**
- `TunnelManager` — owns all tunnel lifecycle; maps tunnel IDs to active connections; coordinates connect/disconnect operations
- `SshConnection` — wraps `russh` client; handles TCP connection, SSH handshake, and authentication
- `PortForwarder` — binds a local TCP port, accepts connections, pipes data through the SSH channel to the remote host:port
- `HealthMonitor` — sends SSH keepalive pings on an interval; detects dead connections (missed pings, TCP errors); triggers graceful cleanup
- `ConfigStore` — reads/writes/validates the JSON config file
- `keychain.rs` — macOS Keychain integration for SSH key passphrases

### Communication

- **Frontend → Backend:** Tauri commands (IPC). Frontend calls Rust functions like `connect_tunnel(id)`, `disconnect_tunnel(id)`, `list_tunnels()`.
- **Backend → Frontend:** Tauri events. Rust pushes status updates (`tunnel-status-changed`, `tunnel-error`) that the frontend subscribes to.

## Tunnel Lifecycle

1. **User clicks connect** → Tauri command `connect_tunnel(id)`
2. **Load credentials** → read SSH key from `keyPath`, fetch passphrase from macOS Keychain
3. **SSH connect** → TCP connection → SSH handshake → authenticate with key
4. **Start forwarding** → bind local port → accept incoming connections → pipe through SSH channel to `remoteHost:remotePort`
5. **Monitor** → keepalive pings every 15s; if no response within 30s, mark as dead
6. **Disconnect (manual or detected dead)** → close SSH channel → stop accepting connections → unbind local port → update state → emit event to UI

## Tunnel States

```
Disconnected → Connecting → Connected → Disconnecting → Disconnected
                                ↓
                              Error → Disconnecting → Disconnected
```

States:
- **Disconnected** — idle, no resources held
- **Connecting** — SSH handshake in progress
- **Connected** — forwarding active, health monitor running
- **Error** — dead connection detected, triggers cleanup
- **Disconnecting** — graceful teardown in progress

## Config Format

Location: `~/.tunnel-master/config.json`

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
    }
  ],
  "settings": {
    "keepaliveIntervalSecs": 15,
    "connectionTimeoutSecs": 30,
    "launchAtLogin": false
  }
}
```

Fields:
- `id` — unique identifier, used as key in TunnelManager
- `name` — display name in UI
- `host`, `port`, `user` — SSH server connection details
- `keyPath` — path to SSH private key
- `type` — tunnel type: `"local"` (POC), later `"reverse"`, `"dynamic"`
- `localPort` — port to bind locally
- `remoteHost`, `remotePort` — destination on the remote network
- `autoConnect` — connect automatically on app launch (post-POC)
- `settings.keepaliveIntervalSecs` — how often to send SSH keepalive
- `settings.connectionTimeoutSecs` — how long to wait before declaring connection dead

## Project Structure

```
tunnel-master/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs             # Tauri entry point
│   │   ├── commands.rs         # Tauri IPC command handlers
│   │   ├── tunnel/
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs      # TunnelManager
│   │   │   ├── connection.rs   # SshConnection (russh wrapper)
│   │   │   ├── forwarder.rs    # PortForwarder
│   │   │   └── health.rs       # HealthMonitor
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   └── store.rs        # ConfigStore
│   │   └── keychain.rs         # macOS Keychain integration
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.tsx
│   ├── components/
│   │   ├── TunnelList.tsx
│   │   └── TunnelItem.tsx
│   ├── hooks/
│   │   └── useTunnels.ts
│   └── types.ts
├── package.json
└── config.example.json
```

## POC Scope

### Included
- System tray icon with popover window
- Tunnel list with connect/disconnect toggles
- Local port forwarding (`-L`)
- SSH key auth with passphrase (macOS Keychain)
- Self-contained JSON config
- Health monitoring (keepalive) + graceful cleanup on drop
- Connection status indicators (color-coded)
- Tray icon reflects overall status (all connected / some / none)

### Deferred
- Reverse (`-R`) / dynamic (`-D`) / SOCKS tunnels
- Auto-reconnect with exponential backoff
- Jump host / ProxyJump support
- SSH key generation and management
- Import from `~/.ssh/config`
- macOS notifications on connect/disconnect/error
- Launch at login
- In-app config editor (edit tunnels through UI)

## Error Handling

- **Connection timeout:** fail after `connectionTimeoutSecs`, set state to Error, clean up
- **Auth failure:** surface error message in UI (wrong key, denied), set state to Disconnected
- **Port already in use:** detect on bind, surface error with the conflicting port number
- **VPN/network drop:** detected by HealthMonitor (missed keepalives), trigger graceful disconnect — close channels, unbind ports, update UI. No zombie processes.
- **App quit:** disconnect all tunnels gracefully before exit (Tauri shutdown hook)

## Testing Strategy

- **Unit tests:** ConfigStore (JSON parsing, validation), tunnel state machine transitions
- **Integration tests:** SshConnection + PortForwarder against a local SSH server (e.g., Docker)
- **Manual testing:** connect/disconnect cycle, kill VPN to test graceful cleanup, verify no port leaks
