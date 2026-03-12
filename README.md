# Tunnel Master

A lightweight SSH tunnel manager that lives in your system tray. Create, manage, and monitor SSH port-forwarding tunnels without touching the terminal.

Built with [Tauri v2](https://v2.tauri.app), Rust, React, and Tailwind CSS.

## Features

- **System tray app** — runs in the background, click the tray icon to open
- **One-click connect/disconnect** — start and stop tunnels instantly
- **In-app config editor** — add, edit, and delete tunnels from the UI
- **Native file picker** — browse for SSH key files
- **Passphrase support** — stores SSH key passphrases in macOS Keychain
- **Auto-reconnect ready** — health monitoring with keepalive detection
- **Cross-platform** — macOS (NSPanel popover), Linux, and Windows

### macOS extras

- Non-activating panel overlay (no space switching)
- Works on fullscreen spaces
- Click-outside to dismiss
- Keychain integration for SSH passphrases

## Install

Download the latest release from [Releases](../../releases):

| Platform | Format |
|----------|--------|
| macOS | `.dmg` |
| Linux | `.deb`, `.AppImage` |
| Windows | `.msi`, `.exe` |

No runtime dependencies — everything is bundled in the binary.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) >= 20
- macOS: Xcode Command Line Tools
- Linux: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`

### Run

```bash
npm install
npx tauri dev
```

### Test

```bash
# Rust tests
cd src-tauri && cargo test

# TypeScript check
npx tsc --noEmit

# Lint
npm run lint
```

### Build

```bash
npx tauri build
```

## Configuration

Tunnels are stored in `~/.tunnel-master/config.json`. You can edit this file directly or use the in-app editor (click the pencil icon).

```json
{
  "version": 1,
  "tunnels": [
    {
      "id": "my-database",
      "name": "My Database",
      "host": "bastion.example.com",
      "port": 22,
      "user": "sergio",
      "keyPath": "~/.ssh/id_ed25519",
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
}
```

## Architecture

```
src-tauri/src/
├── lib.rs              # App setup, tray icon, NSPanel (macOS)
├── commands.rs         # Tauri IPC commands (CRUD, connect, disconnect)
├── config/store.rs     # Config file load/save with atomic writes
├── tunnel/
│   ├── manager.rs      # Actor-based tunnel lifecycle management
│   ├── connection.rs   # SSH connection via russh
│   ├── forwarder.rs    # TCP port forwarding
│   └── health.rs       # Keepalive health monitoring
├── keychain.rs         # Passphrase storage (macOS Keychain)
├── errors.rs           # Error types
└── types.rs            # Shared data types

src/
├── App.tsx             # View state machine (normal/edit-list/edit-form)
├── components/
│   ├── TunnelList.tsx  # Main tunnel list with connect/disconnect
│   ├── EditList.tsx    # Edit mode with two-step delete
│   ├── EditForm.tsx    # Grouped form for add/edit tunnel
│   └── PassphraseDialog.tsx
├── hooks/useTunnels.ts # React hook for tunnel state and CRUD
└── types.ts            # TypeScript type definitions
```

## License

MIT
