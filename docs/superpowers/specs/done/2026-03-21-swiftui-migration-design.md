# SwiftUI Migration Design Spec

## Overview

Migrate Tunnel Master from Tauri (Rust + React WebView) to a native macOS app using SwiftUI `MenuBarExtra` with Rust backend via UniFFI. Feature parity with the current v0.5.3 Tauri app is the goal; native UI polish comes later.

## Motivation

WebView-in-tray on macOS has fundamental limitations:
- NSPanel doesn't keep the menu bar visible in fullscreen
- NSPopover also fails to keep the menu bar visible
- NSMenu with WebView blocks the main thread (modal event loop freezes rendering)
- All alternatives tested in `feat/nspopover-tray` branch — none worked satisfactorily

SwiftUI's `MenuBarExtra` with `.window` style provides a system-managed popover that solves all of these issues natively.

## Constraints

- **macOS 14+ (Sonoma)** minimum deployment target — enables `@Observable`
- **No App Store** — direct distribution via GitHub Releases, unsigned
- **Feature parity first** — no new features, no UI redesign during migration
- **Side-by-side development** — Tauri app kept intact until SwiftUI version reaches parity
- **Swift Package Manager** for dependencies

## Architecture

### Repository Structure

```
tunnel-master/
├── macos-app/                    ← NEW: Xcode project
│   ├── TunnelMaster.xcodeproj
│   ├── TunnelMaster/
│   │   ├── TunnelMasterApp.swift
│   │   ├── ContentView.swift
│   │   ├── TunnelViewModel.swift
│   │   ├── Views/
│   │   │   ├── TunnelListView.swift
│   │   │   ├── TunnelRow.swift
│   │   │   ├── TrafficSparkline.swift
│   │   │   ├── EditListView.swift
│   │   │   └── EditFormView.swift
│   │   ├── Dialogs/
│   │   │   ├── PassphraseDialog.swift
│   │   │   ├── PasswordDialog.swift
│   │   │   ├── HostKeyDialog.swift
│   │   │   └── KeyboardInteractiveDialog.swift
│   │   └── Assets.xcassets/
│   └── Packages/
│       └── TunnelCore/           ← Generated Swift Package (cargo-swift output)
├── rust-core/                    ← NEW: Extracted Rust library crate
│   ├── Cargo.toml
│   ├── uniffi.toml
│   └── src/
│       ├── lib.rs
│       ├── api.rs
│       ├── events.rs
│       ├── types.rs
│       ├── errors.rs
│       ├── keychain.rs
│       ├── tunnel/
│       │   ├── manager.rs
│       │   ├── connection.rs
│       │   ├── forwarder.rs
│       │   ├── health.rs
│       │   └── traffic.rs
│       └── config/
│           └── store.rs
├── src-tauri/                    ← KEPT until parity confirmed
├── src/                          ← KEPT until parity confirmed
```

### Data Flow

```
SwiftUI Views
    ↕ @Observable binding
TunnelViewModel (implements TunnelEventHandler protocol)
    ↕ UniFFI-generated Swift bindings
TunnelCore (Rust)
    ├── api.rs    — Swift calls Rust (commands)
    └── events.rs — Rust calls Swift (callbacks)
```

**Swift → Rust (commands):** Direct function calls through UniFFI. SwiftUI views call ViewModel methods, which call `TunnelCore` functions.

**Rust → Swift (callbacks):** `TunnelEventHandler` trait with `#[uniffi::export(with_foreign)]`. Rust calls trait methods when state changes, auth is needed, or traffic updates arrive. The ViewModel implements this protocol and dispatches to `@MainActor` for UI updates.

## Rust Core API

### Command API (`api.rs`)

`TunnelCore` is the single entry point. Created at app startup with the event handler.

`TunnelCore` owns the `ConfigStore` internally — all CRUD operations persist to disk within the Rust layer. Swift never interacts with config files directly.

```rust
#[derive(uniffi::Object)]
pub struct TunnelCore { /* ... */ }

#[uniffi::export]
impl TunnelCore {
    #[uniffi::constructor]
    fn new(event_handler: Arc<dyn TunnelEventHandler>) -> Self;

    // Tunnel state
    fn list_tunnels(&self) -> Vec<TunnelInfo>;
    fn connect(&self, id: String);
    fn disconnect(&self, id: String);

    // Config CRUD (persists to disk internally)
    fn get_tunnel_config(&self, id: String) -> Option<TunnelConfig>;
    fn add_tunnel(&self, config: TunnelConfig);
    fn update_tunnel(&self, id: String, config: TunnelConfig);
    fn delete_tunnel(&self, id: String);
    fn reorder_tunnels(&self, ids: Vec<String>);
    fn reload_config(&self);

    // Traffic
    fn get_traffic_history(&self, id: String) -> Vec<TrafficSample>;

    // Auth responses
    fn accept_host_key(&self, host: String, port: u16);
    fn submit_passphrase(&self, id: String, passphrase: String);
    fn submit_password(&self, id: String, password: String);
    fn respond_keyboard_interactive(&self, id: String, responses: Vec<String>);
    fn cancel_auth(&self, id: String);

    // Keychain
    fn store_passphrase(&self, id: String, passphrase: String);
    fn store_password(&self, id: String, password: String);
    fn clear_credential(&self, id: String);

    // Lifecycle — disconnects all tunnels, cleans up. App exit is Swift's responsibility.
    fn shutdown(&self);
}
```

**Note:** File picker (`pick_key_file`) is handled entirely on the Swift side via `NSOpenPanel`. The selected path is passed to `add_tunnel`/`update_tunnel` in the `TunnelConfig`.

### Event Trait (`events.rs`)

```rust
#[derive(uniffi::Record)]
pub struct KiPromptEntry {
    pub text: String,
    pub echo: bool,
}

#[uniffi::export(with_foreign)]
pub trait TunnelEventHandler: Send + Sync {
    fn on_tunnel_state_changed(&self, id: String, status: TunnelStatus, error_message: Option<String>);
    fn on_passphrase_requested(&self, id: String, key_path: String);
    fn on_password_requested(&self, id: String);
    fn on_host_key_verification(&self, id: String, host: String, port: u16, key_type: String, fingerprint: String);
    fn on_keyboard_interactive(
        &self, id: String, name: String, instructions: String, prompts: Vec<KiPromptEntry>
    );
    fn on_traffic_update(&self, id: String, sample: TrafficSample);
    fn on_error(&self, id: String, message: String);
}
```

### Tauri Coupling Points

Six modules are already Tauri-free and copy directly: `config/store.rs`, `keychain.rs`, `types.rs`, `errors.rs`, `tunnel/forwarder.rs`, `tunnel/health.rs`.

Three modules need changes:

| Module | Tauri Usage | Replacement |
|--------|------------|-------------|
| `tunnel/manager.rs` | `tauri::AppHandle` for events, `tauri::async_runtime::spawn` | `Arc<dyn TunnelEventHandler>`, `tokio::spawn` |
| `tunnel/connection.rs` | `tauri::AppHandle` + `tauri::Emitter` for auth prompts | `Arc<dyn TunnelEventHandler>` callbacks |
| `tunnel/traffic.rs` | `tauri::AppHandle` + `tauri::Emitter` for traffic events | `Arc<dyn TunnelEventHandler>` callbacks |

Two modules stay Tauri-only (not extracted): `commands.rs`, `lib.rs`.

### Platform-Specific Notes

- **Shutdown:** `TunnelCore::shutdown()` only disconnects tunnels and cleans up resources. App termination is handled by SwiftUI / `NSApplication.terminate`. The current Tauri code calls `std::process::exit(0)` — that moves to the Swift side.
- **Tray tooltip:** Connected tunnel count logic (currently in Tauri `lib.rs`) moves to the SwiftUI ViewModel. The ViewModel computes the count from its `tunnels` array and updates the `MenuBarExtra` label.
- **File picker:** `pick_key_file` becomes a Swift-side `NSOpenPanel` call. No Rust involvement.

## SwiftUI Views

### Component Mapping

| React Component | SwiftUI View | Notes |
|----------------|-------------|-------|
| `App.tsx` | `ContentView` | `ViewMode` enum instead of string state |
| `TunnelList.tsx` | `TunnelListView` | SwiftUI `List` with `ForEach` |
| `TunnelItem.tsx` | `TunnelRow` | `HStack` with status circle + `Toggle` |
| `TrafficSparkline.tsx` | `TrafficSparkline` | SwiftUI `Canvas` or `Path` |
| `EditList.tsx` | `EditListView` | `List` with `.onMove`, `.onDelete` |
| `EditForm.tsx` | `EditFormView` | SwiftUI `Form` with sections |
| `CustomSelect.tsx` | — | Native `Picker`, no custom component needed |
| `useTunnels.ts` hook | `TunnelViewModel` | `@Observable` class |
| 4 Dialog components | 4 Dialog views | `.sheet` / `.alert` modifiers |

### State Management

```swift
@Observable
class TunnelViewModel: TunnelEventHandler {
    var tunnels: [TunnelInfo] = []
    var currentView: ViewMode = .list
    var activeDialog: DialogState? = nil
    var editingTunnel: TunnelConfig? = nil

    private var core: TunnelCore

    init() {
        self.core = TunnelCore(eventHandler: self)
    }

    // TunnelEventHandler protocol — dispatches to @MainActor
    func onTunnelStateChanged(id: String, state: TunnelState) {
        Task { @MainActor in
            // update tunnels array
        }
    }
}
```

### App Entry Point

```swift
@main
struct TunnelMasterApp: App {
    @State private var viewModel = TunnelViewModel()

    var body: some Scene {
        MenuBarExtra("Tunnel Master", image: "tray-icon") {
            ContentView(viewModel: viewModel)
        }
        .menuBarExtraStyle(.window)
    }
}
```

## Build Pipeline

1. **Build Rust:** `cd rust-core && cargo swift package` → generates `TunnelCore` Swift Package into `macos-app/Packages/TunnelCore/`
2. **Build Swift:** Open `TunnelMaster.xcodeproj`, which depends on local `TunnelCore` package → Xcode builds and links
3. **Release:** `cargo swift package` + `xcodebuild archive` → export unsigned `.app` → zip for GitHub Release

A `build.sh` script will wrap steps 1-2 for convenience.

## Migration Phases

### Phase 1: Extract Rust Core
- Create `rust-core/` crate with modules copied from `src-tauri/src/`
- Replace `tauri::AppHandle` with `Arc<dyn TunnelEventHandler>` callback trait
- Replace `tauri::async_runtime::spawn` with `tokio::spawn`
- Add UniFFI proc-macro annotations to public types and functions
- Make `src-tauri/` depend on `rust-core/` so the Tauri app still works (validates extraction)

### Phase 2: Scaffold SwiftUI App
- Run `cargo swift package` to generate Swift bindings
- Create Xcode project with `MenuBarExtra(.window)`
- Implement `TunnelViewModel` with callback protocol
- Get a basic tunnel list displaying (read-only) as first milestone

### Phase 3: Feature Parity
- Connect/disconnect toggles
- Traffic sparklines
- Edit mode (add/edit/delete/reorder tunnels)
- Auth dialogs (passphrase, password, host key verification, keyboard-interactive)
- Keychain credential storage
- App icon, tray icon

### Phase 4: Cleanup
- Remove `src-tauri/`, `src/`, and React/Tauri dependencies
- Update GitHub Release workflow
- Bump version to v1.0

## Known Risks & Mitigations

| Risk | Level | Mitigation |
|------|-------|------------|
| **Threading deadlock** — UniFFI foreign callbacks are synchronous. If Swift's callback handler synchronously calls back into Rust on the same thread, re-entrancy deadlock. | High | Rust must always fire callbacks from background tokio tasks (already the case — manager actor loop, traffic sampler). Swift must use `Task { @MainActor in ... }` immediately in every callback method to avoid blocking the Rust thread. Never do synchronous Rust calls inside a callback. |
| **Granular UI updates** — Replacing the entire `tunnels` array on every state change can reset scroll position or lose focus in EditFormView. | Medium | `TunnelInfo` conforms to `Identifiable`. `onTunnelStateChanged` updates only the specific element in the array by `id`, not the whole list. Never call `list_tunnels()` from inside a callback — use the delta. |
| **Keychain migration** — The SwiftUI binary has a different executable identity than the Tauri binary. macOS Keychain items created by Tauri may not be accessible. Both apps are unsigned, but executable identity still differs. | Medium | Accept that users re-enter credentials once after migration. Document this in release notes. The `keyring` crate with service name `"tunnel-master"` may or may not carry over — test during Phase 3. |
| **TunnelCore lifecycle** — `MenuBarExtra` views can be recreated by the system. Multiple `TunnelCore` instances would bind the same SSH ports. | Medium | `TunnelViewModel` must be created once at the `App` level via `@State`. Use two-phase init (create `TunnelCore` in a separate `start()` method, not in `init()`) to avoid Swift's self-before-init restriction. |
| **App Sandbox default** — New Xcode projects enable App Sandbox by default. Rust's `std::fs` cannot read SSH key paths inside a sandbox without `startAccessingSecurityScopedResource()`. | Low | Disable App Sandbox in Xcode "Signing & Capabilities" tab. We distribute unsigned via GitHub Releases — no sandbox needed. |
| **Sparkline performance** — Many tunnels emitting traffic callbacks could cause stutter. | Low | Already mitigated: traffic sampler runs at 1Hz. Even 20 tunnels = 20 callbacks/second, trivial for SwiftUI Canvas. No throttling changes needed. |

## References

- [Ockam Portals: Swift + Rust architecture](https://dev.to/build-trust/how-we-built-a-swift-app-that-uses-rust-102f)
- [TantalusPath: Rust to Swift state syncing via UniFFI](https://www.tantaluspath.com/tech/rust_to_swift_state_syncing/)
- [UniFFI documentation](https://mozilla.github.io/uniffi-rs/)
- [cargo-swift](https://github.com/nicklimmern/cargo-swift)
- [SwiftUI MenuBarExtra](https://developer.apple.com/documentation/swiftui/menubarextra)
