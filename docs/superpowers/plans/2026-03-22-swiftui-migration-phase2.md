# SwiftUI Migration Phase 2: Scaffold SwiftUI App

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a working SwiftUI MenuBarExtra app that displays the tunnel list (read-only) using the rust-core library from Phase 1.

**Architecture:** Use XcodeGen to generate the Xcode project from a YAML spec. The app links the Rust static library (`libtunnel_core.a`) and imports UniFFI-generated Swift bindings. A `TunnelViewModel` implements the `TunnelEventHandler` protocol to receive state changes from Rust. The first milestone is a read-only tunnel list with connect/disconnect toggles.

**Tech Stack:** Swift 6.1, SwiftUI, XcodeGen, UniFFI-generated bindings, Rust static library

**Spec:** `docs/superpowers/specs/2026-03-21-swiftui-migration-design.md`

**Prerequisites:** Phase 1 complete (rust-core crate with UniFFI, `libtunnel_core.a` built, Swift bindings generated)

---

## Build Pipeline

Since `cargo-swift` requires rustup (we have Homebrew Rust), we use a manual approach:

1. `cd rust-core && cargo build --release` → produces `target/release/libtunnel_core.a`
2. `cargo run --bin uniffi-bindgen generate --library target/release/libtunnel_core.dylib --language swift --out-dir macos-app/TunnelMaster/Generated` → generates Swift bindings
3. XcodeGen generates `TunnelMaster.xcodeproj` from `macos-app/project.yml`
4. `xcodebuild` builds the app

A `build.sh` script wraps steps 1-3.

## File Map

### New files (macos-app/)

| File | Responsibility |
|------|---------------|
| `macos-app/project.yml` | XcodeGen project spec |
| `macos-app/build.sh` | Build Rust + generate bindings + generate xcodeproj |
| `macos-app/TunnelMaster/TunnelMasterApp.swift` | @main, MenuBarExtra with .window style |
| `macos-app/TunnelMaster/ContentView.swift` | Routes between ViewMode states |
| `macos-app/TunnelMaster/TunnelViewModel.swift` | @Observable, implements TunnelEventHandler |
| `macos-app/TunnelMaster/Views/TunnelListView.swift` | Tunnel list with status + toggles |
| `macos-app/TunnelMaster/Views/TunnelRow.swift` | Single tunnel row |
| `macos-app/TunnelMaster/Generated/TunnelCore.swift` | UniFFI-generated bindings (auto-generated) |
| `macos-app/TunnelMaster/Generated/TunnelCoreFFI.h` | UniFFI-generated C header (auto-generated) |
| `macos-app/TunnelMaster/Generated/TunnelCoreFFI.modulemap` | Module map for C header (auto-generated) |
| `macos-app/TunnelMaster/Assets.xcassets/` | App icons, tray icon |
| `macos-app/TunnelMaster/TunnelMaster.entitlements` | Entitlements (App Sandbox OFF) |
| `macos-app/TunnelMaster/Info.plist` | App metadata, LSUIElement=YES (no dock icon) |

---

## Task 1: Build script and UniFFI bindings generation

**Files:**
- Create: `macos-app/build.sh`
- Create: `macos-app/TunnelMaster/Generated/` (populated by script)

- [ ] **Step 1: Create the build script**

```bash
#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RUST_CORE="$PROJECT_ROOT/rust-core"
GENERATED_DIR="$SCRIPT_DIR/TunnelMaster/Generated"

echo "==> Building rust-core (release)..."
cd "$RUST_CORE"
cargo build --release

echo "==> Generating Swift bindings..."
mkdir -p "$GENERATED_DIR"
cargo run --bin uniffi-bindgen generate \
    --library target/release/libtunnel_core.dylib \
    --language swift \
    --out-dir "$GENERATED_DIR"

echo "==> Generating Xcode project..."
cd "$SCRIPT_DIR"
xcodegen generate

echo "==> Done! Open TunnelMaster.xcodeproj or build with:"
echo "    xcodebuild -project TunnelMaster.xcodeproj -scheme TunnelMaster -configuration Debug build"
```

- [ ] **Step 2: Make executable and test Rust build + bindings generation**

Run: `chmod +x macos-app/build.sh`
Run: `cd macos-app && ./build.sh` (will fail at xcodegen — project.yml doesn't exist yet. That's expected.)
Verify: `ls macos-app/TunnelMaster/Generated/TunnelCore.swift` exists

- [ ] **Step 3: Commit**

```bash
git add macos-app/build.sh
git commit -m "chore: add build script for Rust + UniFFI + XcodeGen pipeline"
```

---

## Task 2: XcodeGen project spec

**Files:**
- Create: `macos-app/project.yml`
- Create: `macos-app/TunnelMaster/TunnelMaster.entitlements`
- Create: `macos-app/TunnelMaster/Info.plist`

- [ ] **Step 1: Create project.yml**

```yaml
name: TunnelMaster
options:
  bundleIdPrefix: com.kuroru2
  deploymentTarget:
    macOS: "14.0"
  xcodeVersion: "16.4"
  generateEmptyDirectories: true

settings:
  base:
    SWIFT_VERSION: "6.0"
    MACOSX_DEPLOYMENT_TARGET: "14.0"
    LIBRARY_SEARCH_PATHS:
      - "$(PROJECT_DIR)/../rust-core/target/release"
    SWIFT_OBJC_BRIDGING_HEADER: ""
    OTHER_LDFLAGS:
      - "-ltunnel_core"
      - "-framework Security"
      - "-framework CoreFoundation"
    IMPORT_PATHS:
      - "$(PROJECT_DIR)/TunnelMaster/Generated"

targets:
  TunnelMaster:
    type: application
    platform: macOS
    sources:
      - path: TunnelMaster
        excludes:
          - "Generated/TunnelCoreFFI.h"
          - "Generated/TunnelCoreFFI.modulemap"
    settings:
      base:
        INFOPLIST_FILE: TunnelMaster/Info.plist
        CODE_SIGN_ENTITLEMENTS: TunnelMaster/TunnelMaster.entitlements
        CODE_SIGNING_ALLOWED: "NO"
        PRODUCT_BUNDLE_IDENTIFIER: com.kuroru2.tunnel-master
        PRODUCT_NAME: "Tunnel Master"
        SWIFT_OBJC_INTEROP_MODE: "objcxx"
    preBuildScripts:
      - name: "Import C Module"
        script: |
          # Make the TunnelCoreFFI C module available to Swift
          MODULEMAP_DIR="${BUILT_PRODUCTS_DIR}/TunnelCoreFFI"
          mkdir -p "$MODULEMAP_DIR"
          cp "${PROJECT_DIR}/TunnelMaster/Generated/TunnelCoreFFI.h" "$MODULEMAP_DIR/"
          cp "${PROJECT_DIR}/TunnelMaster/Generated/TunnelCoreFFI.modulemap" "$MODULEMAP_DIR/module.modulemap"
        inputFiles: []
        outputFiles: []
```

Note: The IMPORT_PATHS and preBuildScript are needed so Swift can `import TunnelCoreFFI` from the generated bindings. The TunnelCore.swift file has `#if canImport(TunnelCoreFFI) / import TunnelCoreFFI` at the top.

- [ ] **Step 2: Create Info.plist**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Tunnel Master</string>
    <key>CFBundleDisplayName</key>
    <string>Tunnel Master</string>
    <key>CFBundleIdentifier</key>
    <string>com.kuroru2.tunnel-master</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>LSUIElement</key>
    <true/>
</dict>
</plist>
```

`LSUIElement = YES` makes the app a menu bar agent (no Dock icon).

- [ ] **Step 3: Create entitlements (sandbox OFF)**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <false/>
</dict>
</plist>
```

- [ ] **Step 4: Run xcodegen to verify project generates**

Run: `cd macos-app && xcodegen generate`
Expected: `TunnelMaster.xcodeproj` created successfully

- [ ] **Step 5: Commit**

```bash
git add macos-app/project.yml macos-app/TunnelMaster/Info.plist macos-app/TunnelMaster/TunnelMaster.entitlements
git commit -m "chore: add XcodeGen project spec, Info.plist, entitlements"
```

---

## Task 3: App entry point and placeholder views

**Files:**
- Create: `macos-app/TunnelMaster/TunnelMasterApp.swift`
- Create: `macos-app/TunnelMaster/ContentView.swift`

- [ ] **Step 1: Create TunnelMasterApp.swift**

```swift
import SwiftUI

@main
struct TunnelMasterApp: App {
    @State private var viewModel = TunnelViewModel()

    var body: some Scene {
        MenuBarExtra {
            ContentView(viewModel: viewModel)
                .frame(width: 320, height: 400)
        } label: {
            Image(systemName: "network")
        }
        .menuBarExtraStyle(.window)
    }
}
```

Notes:
- Uses `Image(systemName: "network")` as placeholder tray icon (custom icon comes later)
- Frame matches current Tauri app dimensions (320x400)
- `.menuBarExtraStyle(.window)` gives us the popover panel

- [ ] **Step 2: Create ContentView.swift**

```swift
import SwiftUI

enum ViewMode {
    case list
    case editList
    case editForm(tunnelId: String?)
}

struct ContentView: View {
    @Bindable var viewModel: TunnelViewModel

    var body: some View {
        VStack(spacing: 0) {
            switch viewModel.currentView {
            case .list:
                TunnelListView(viewModel: viewModel)
            case .editList:
                Text("Edit List — coming in Phase 3")
            case .editForm:
                Text("Edit Form — coming in Phase 3")
            }
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
```

- [ ] **Step 3: Verify these files have no syntax errors**

This can't be verified until TunnelViewModel exists (Task 4). Move on.

- [ ] **Step 4: Commit**

```bash
git add macos-app/TunnelMaster/TunnelMasterApp.swift macos-app/TunnelMaster/ContentView.swift
git commit -m "feat(macos-app): add app entry point with MenuBarExtra and ContentView"
```

---

## Task 4: TunnelViewModel

**Files:**
- Create: `macos-app/TunnelMaster/TunnelViewModel.swift`

This is the most important file — it bridges Rust and SwiftUI.

- [ ] **Step 1: Create TunnelViewModel.swift**

```swift
import Foundation
import Observation

@Observable
final class TunnelViewModel: TunnelEventHandler {
    var tunnels: [TunnelInfo] = []
    var currentView: ViewMode = .list
    var activeDialog: DialogState? = nil

    private var core: TunnelCore?

    enum DialogState: Identifiable {
        case passphrase(tunnelId: String, keyPath: String)
        case password(tunnelId: String)
        case hostKey(tunnelId: String, host: String, port: UInt16, keyType: String, fingerprint: String)
        case keyboardInteractive(tunnelId: String, name: String, instructions: String, prompts: [KiPromptEntry])

        var id: String {
            switch self {
            case .passphrase(let tid, _): return "passphrase-\(tid)"
            case .password(let tid): return "password-\(tid)"
            case .hostKey(let tid, _, _, _, _): return "hostkey-\(tid)"
            case .keyboardInteractive(let tid, _, _, _): return "ki-\(tid)"
            }
        }
    }

    func start() {
        core = TunnelCore(eventHandler: self)
        refreshTunnels()
    }

    func shutdown() {
        core?.shutdown()
        core = nil
    }

    // MARK: - Public API (called by views)

    func refreshTunnels() {
        guard let core else { return }
        let list = core.listTunnels()
        Task { @MainActor in
            self.tunnels = list
        }
    }

    func toggleConnection(id: String) {
        guard let core else { return }
        if let tunnel = tunnels.first(where: { $0.id == id }) {
            switch tunnel.status {
            case .connected, .connecting:
                core.disconnect(id: id)
            case .disconnected, .error:
                core.connect(id: id)
            case .disconnecting:
                break // wait
            }
        }
    }

    // MARK: - TunnelEventHandler (called by Rust on background thread)

    func onTunnelStateChanged(id: String, status: TunnelStatus, errorMessage: String?) {
        Task { @MainActor in
            if let idx = self.tunnels.firstIndex(where: { $0.id == id }) {
                // Update in-place to avoid array rebuild
                var updated = self.tunnels[idx]
                updated = TunnelInfo(
                    id: updated.id,
                    name: updated.name,
                    status: status,
                    localPort: updated.localPort,
                    remoteHost: updated.remoteHost,
                    remotePort: updated.remotePort,
                    errorMessage: errorMessage,
                    authMethod: updated.authMethod,
                    jumpHostName: updated.jumpHostName,
                    showTrafficChart: updated.showTrafficChart
                )
                self.tunnels[idx] = updated
            } else {
                // New tunnel or full refresh needed
                self.refreshTunnels()
            }
        }
    }

    func onPassphraseRequested(id: String, keyPath: String) {
        Task { @MainActor in
            self.activeDialog = .passphrase(tunnelId: id, keyPath: keyPath)
        }
    }

    func onPasswordRequested(id: String) {
        Task { @MainActor in
            self.activeDialog = .password(tunnelId: id)
        }
    }

    func onHostKeyVerification(id: String, host: String, port: UInt16, keyType: String, fingerprint: String) {
        Task { @MainActor in
            self.activeDialog = .hostKey(tunnelId: id, host: host, port: port, keyType: keyType, fingerprint: fingerprint)
        }
    }

    func onKeyboardInteractive(id: String, name: String, instructions: String, prompts: [KiPromptEntry]) {
        Task { @MainActor in
            self.activeDialog = .keyboardInteractive(tunnelId: id, name: name, instructions: instructions, prompts: prompts)
        }
    }

    func onTrafficUpdate(id: String, sample: TrafficSample) {
        // Phase 3: traffic sparklines. For now, no-op.
    }

    func onError(id: String, message: String) {
        Task { @MainActor in
            if let idx = self.tunnels.firstIndex(where: { $0.id == id }) {
                var updated = self.tunnels[idx]
                updated = TunnelInfo(
                    id: updated.id,
                    name: updated.name,
                    status: .error,
                    localPort: updated.localPort,
                    remoteHost: updated.remoteHost,
                    remotePort: updated.remotePort,
                    errorMessage: message,
                    authMethod: updated.authMethod,
                    jumpHostName: updated.jumpHostName,
                    showTrafficChart: updated.showTrafficChart
                )
                self.tunnels[idx] = updated
            }
        }
    }
}
```

Key design notes:
- **Two-phase init**: `start()` is called separately from `init()` to avoid Swift's self-before-init restriction (spec risk #4).
- **`Task { @MainActor in }`**: All callbacks dispatch to main thread (spec risk #1 — threading deadlock prevention).
- **Granular updates**: `onTunnelStateChanged` updates the specific element by index, not the whole array (spec risk #2).
- **DialogState enum**: Captures all the data needed for auth dialogs. Phase 2 won't show the dialogs yet, but the state is tracked.

- [ ] **Step 2: Commit**

```bash
git add macos-app/TunnelMaster/TunnelViewModel.swift
git commit -m "feat(macos-app): add TunnelViewModel with TunnelEventHandler implementation"
```

---

## Task 5: Tunnel list views

**Files:**
- Create: `macos-app/TunnelMaster/Views/TunnelListView.swift`
- Create: `macos-app/TunnelMaster/Views/TunnelRow.swift`

- [ ] **Step 1: Create TunnelListView.swift**

```swift
import SwiftUI

struct TunnelListView: View {
    @Bindable var viewModel: TunnelViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Tunnels")
                    .font(.headline)
                Spacer()
                Button {
                    viewModel.currentView = .editList
                } label: {
                    Image(systemName: "pencil")
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)

            Divider()

            // Tunnel list
            if viewModel.tunnels.isEmpty {
                VStack {
                    Spacer()
                    Text("No tunnels configured")
                        .foregroundStyle(.secondary)
                    Text("Click the pencil icon to add one")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                    Spacer()
                }
                .frame(maxHeight: .infinity)
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(viewModel.tunnels, id: \.id) { tunnel in
                            TunnelRow(tunnel: tunnel) {
                                viewModel.toggleConnection(id: tunnel.id)
                            }
                            Divider()
                        }
                    }
                }
            }

            Divider()

            // Footer
            HStack {
                let connectedCount = viewModel.tunnels.filter {
                    $0.status == .connected
                }.count
                Text("\(connectedCount)/\(viewModel.tunnels.count) connected")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Quit") {
                    viewModel.shutdown()
                    NSApplication.shared.terminate(nil)
                }
                .buttonStyle(.plain)
                .font(.caption)
                .foregroundStyle(.secondary)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
        }
    }
}
```

- [ ] **Step 2: Create TunnelRow.swift**

```swift
import SwiftUI

struct TunnelRow: View {
    let tunnel: TunnelInfo
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            // Status indicator
            Circle()
                .fill(statusColor)
                .frame(width: 8, height: 8)

            // Tunnel info
            VStack(alignment: .leading, spacing: 2) {
                Text(tunnel.name)
                    .font(.body)
                    .lineLimit(1)

                Text("localhost:\(tunnel.localPort) → \(tunnel.remoteHost):\(tunnel.remotePort)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            // Error indicator
            if tunnel.status == .error, let msg = tunnel.errorMessage {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(.orange)
                    .help(msg)
            }

            // Connect toggle
            if tunnel.status == .connecting || tunnel.status == .disconnecting {
                ProgressView()
                    .controlSize(.small)
                    .frame(width: 24)
            } else {
                Toggle("", isOn: Binding(
                    get: { tunnel.status == .connected },
                    set: { _ in onToggle() }
                ))
                .toggleStyle(.switch)
                .controlSize(.small)
                .labelsHidden()
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .contentShape(Rectangle())
    }

    private var statusColor: Color {
        switch tunnel.status {
        case .connected: return .green
        case .connecting, .disconnecting: return .yellow
        case .error: return .red
        case .disconnected: return .gray
        }
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add macos-app/TunnelMaster/Views/
git commit -m "feat(macos-app): add TunnelListView and TunnelRow"
```

---

## Task 6: Assets and tray icon

**Files:**
- Create: `macos-app/TunnelMaster/Assets.xcassets/Contents.json`
- Create: `macos-app/TunnelMaster/Assets.xcassets/AppIcon.appiconset/Contents.json`

- [ ] **Step 1: Create asset catalog structure**

```bash
mkdir -p macos-app/TunnelMaster/Assets.xcassets/AppIcon.appiconset
```

Create `macos-app/TunnelMaster/Assets.xcassets/Contents.json`:
```json
{
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
```

Create `macos-app/TunnelMaster/Assets.xcassets/AppIcon.appiconset/Contents.json`:
```json
{
  "images" : [
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "16x16"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "16x16"
    },
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "32x32"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "32x32"
    },
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "128x128"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "128x128"
    },
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "256x256"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "256x256"
    },
    {
      "idiom" : "mac",
      "scale" : "1x",
      "size" : "512x512"
    },
    {
      "idiom" : "mac",
      "scale" : "2x",
      "size" : "512x512"
    }
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
```

No actual icon images for now — the app will use a default icon. Custom icons come in Phase 3.

- [ ] **Step 2: Commit**

```bash
git add macos-app/TunnelMaster/Assets.xcassets/
git commit -m "chore(macos-app): add asset catalog skeleton"
```

---

## Task 7: Build and run the app

This is the integration task — wire everything together and verify the app builds and runs.

**Files:**
- Modify: `macos-app/TunnelMasterApp.swift` (may need adjustments)
- Modify: `macos-app/project.yml` (may need adjustments)

- [ ] **Step 1: Run the full build script**

Run: `cd macos-app && ./build.sh`
Expected: Rust builds, bindings generate, xcodegen creates project

- [ ] **Step 2: Build with xcodebuild**

Run: `cd macos-app && xcodebuild -project TunnelMaster.xcodeproj -scheme TunnelMaster -configuration Debug build 2>&1 | tail -20`

If it fails, diagnose and fix. Common issues:
- Missing IMPORT_PATHS for the TunnelCoreFFI module
- Library search path not finding libtunnel_core.a
- Swift 6 strict concurrency warnings/errors on the generated TunnelCore.swift (may need `SWIFT_STRICT_CONCURRENCY = minimal` in project.yml)

- [ ] **Step 3: Run the app**

Run: `open macos-app/build/Debug/Tunnel\ Master.app` (or wherever xcodebuild outputs it)
Expected: Menu bar icon appears. Clicking shows the tunnel list (populated from `~/.tunnel-master/config.json` if it exists, empty state otherwise).

- [ ] **Step 4: Fix any issues and iterate**

This step may require multiple iterations. The goal is:
1. App compiles with zero errors
2. App launches and shows menu bar icon
3. Clicking the icon shows the popover with tunnel list
4. Tunnels from config.json are displayed
5. Connect/disconnect toggles work

- [ ] **Step 5: Commit working state**

```bash
git add macos-app/
git commit -m "feat(macos-app): working SwiftUI MenuBarExtra app with tunnel list"
```

---

## Summary

After completing all 7 tasks:
- `macos-app/` contains a working SwiftUI app with XcodeGen project
- App shows in the menu bar, popover displays tunnel list
- Connect/disconnect toggles work via TunnelViewModel → TunnelCore → Rust
- Auth dialogs are tracked in state but not displayed yet (Phase 3)
- Traffic sparklines are not displayed yet (Phase 3)
- Edit mode is not implemented yet (Phase 3)

**Next:** Phase 3 plan (Feature Parity) — adds edit mode, auth dialogs, traffic sparklines, and remaining features.
