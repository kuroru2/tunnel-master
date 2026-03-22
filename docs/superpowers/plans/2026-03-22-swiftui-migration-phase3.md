# SwiftUI Migration Phase 3: Feature Parity

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Achieve feature parity with the Tauri v0.5.3 app — auth dialogs, edit mode (add/edit/delete/reorder), traffic sparklines, and tray icon.

**Architecture:** Each feature is a self-contained SwiftUI view + ViewModel methods. Auth dialogs use `.sheet` modifiers on ContentView. Edit mode adds two new views (EditListView, EditFormView). Traffic sparklines use SwiftUI `Path`. All features wire through the existing `TunnelViewModel` → `TunnelCore` → Rust pipeline.

**Tech Stack:** SwiftUI, @Observable, NSOpenPanel (file picker)

**Spec:** `docs/superpowers/specs/2026-03-21-swiftui-migration-design.md`

**Prerequisites:** Phase 2 complete (working SwiftUI app with tunnel list and connect/disconnect)

---

## File Map

| File | Responsibility | Task |
|------|---------------|------|
| `macos-app/TunnelMaster/Dialogs/PassphraseDialog.swift` | SSH key passphrase entry | 1 |
| `macos-app/TunnelMaster/Dialogs/PasswordDialog.swift` | SSH password entry | 1 |
| `macos-app/TunnelMaster/Dialogs/HostKeyDialog.swift` | TOFU host key verification | 1 |
| `macos-app/TunnelMaster/Dialogs/KeyboardInteractiveDialog.swift` | Multi-prompt KI auth | 1 |
| `macos-app/TunnelMaster/ContentView.swift` | Add `.sheet` for dialogs | 1 |
| `macos-app/TunnelMaster/TunnelViewModel.swift` | Add dialog submit/cancel methods, CRUD methods, traffic data | 1,2,3 |
| `macos-app/TunnelMaster/Views/EditListView.swift` | Tunnel list with reorder, delete, add | 2 |
| `macos-app/TunnelMaster/Views/EditFormView.swift` | Add/edit tunnel config form | 3 |
| `macos-app/TunnelMaster/Views/TrafficSparkline.swift` | SVG-like sparkline via SwiftUI Path | 4 |
| `macos-app/TunnelMaster/Views/TunnelRow.swift` | Add sparkline overlay | 4 |

---

## Task 1: Auth dialogs

**Files:**
- Create: `macos-app/TunnelMaster/Dialogs/PassphraseDialog.swift`
- Create: `macos-app/TunnelMaster/Dialogs/PasswordDialog.swift`
- Create: `macos-app/TunnelMaster/Dialogs/HostKeyDialog.swift`
- Create: `macos-app/TunnelMaster/Dialogs/KeyboardInteractiveDialog.swift`
- Modify: `macos-app/TunnelMaster/ContentView.swift`
- Modify: `macos-app/TunnelMaster/TunnelViewModel.swift`

- [ ] **Step 1: Add dialog action methods to TunnelViewModel**

Add these methods to TunnelViewModel:

```swift
// MARK: - Dialog actions

func submitPassphrase(_ passphrase: String, tunnelId: String) {
    core?.submitPassphrase(id: tunnelId, passphrase: passphrase)
    activeDialog = nil
}

func submitPassword(_ password: String, tunnelId: String) {
    core?.submitPassword(id: tunnelId, password: password)
    activeDialog = nil
}

func acceptHostKey(host: String, port: UInt16) {
    core?.acceptHostKey(host: host, port: port)
    activeDialog = nil
    // Reconnect the tunnel that triggered this
    if case .hostKey(let tunnelId, _, _, _, _) = activeDialog {
        core?.connect(id: tunnelId)
    }
}

func respondKeyboardInteractive(_ responses: [String], tunnelId: String) {
    core?.respondKeyboardInteractive(id: tunnelId, responses: responses)
    activeDialog = nil
}

func cancelDialog() {
    if case .keyboardInteractive(let tid, _, _, _) = activeDialog {
        core?.cancelAuth(id: tid)
    }
    activeDialog = nil
}
```

Note: `acceptHostKey` needs to store the tunnelId before clearing the dialog, then reconnect. Fix ordering: save tunnelId first, clear dialog, then reconnect.

- [ ] **Step 2: Create PassphraseDialog.swift**

```swift
import SwiftUI

struct PassphraseDialog: View {
    let tunnelId: String
    let keyPath: String
    let onSubmit: (String, String) -> Void  // (passphrase, tunnelId)
    let onCancel: () -> Void

    @State private var passphrase = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Passphrase Required")
                .font(.headline)

            Text("Enter the passphrase for the SSH key. It will be stored securely.")
                .font(.caption)
                .foregroundStyle(.secondary)

            SecureField("SSH key passphrase", text: $passphrase)
                .textFieldStyle(.roundedBorder)
                .onSubmit { submit() }

            HStack {
                Spacer()
                Button("Cancel") { onCancel() }
                    .keyboardShortcut(.cancelAction)
                Button("Unlock") { submit() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(passphrase.isEmpty)
            }
        }
        .padding()
        .frame(width: 300)
    }

    private func submit() {
        guard !passphrase.isEmpty else { return }
        onSubmit(passphrase, tunnelId)
    }
}
```

- [ ] **Step 3: Create PasswordDialog.swift**

```swift
import SwiftUI

struct PasswordDialog: View {
    let tunnelId: String
    let onSubmit: (String, String) -> Void  // (password, tunnelId)
    let onCancel: () -> Void

    @State private var password = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Password Required")
                .font(.headline)

            Text("Enter the SSH password. It will be stored securely.")
                .font(.caption)
                .foregroundStyle(.secondary)

            SecureField("SSH password", text: $password)
                .textFieldStyle(.roundedBorder)
                .onSubmit { submit() }

            HStack {
                Spacer()
                Button("Cancel") { onCancel() }
                    .keyboardShortcut(.cancelAction)
                Button("Connect") { submit() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(password.isEmpty)
            }
        }
        .padding()
        .frame(width: 300)
    }

    private func submit() {
        guard !password.isEmpty else { return }
        onSubmit(password, tunnelId)
    }
}
```

- [ ] **Step 4: Create HostKeyDialog.swift**

```swift
import SwiftUI

struct HostKeyDialog: View {
    let tunnelId: String
    let host: String
    let port: UInt16
    let keyType: String
    let fingerprint: String
    let onAccept: (String, UInt16) -> Void  // (host, port)
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Unknown Host")
                .font(.headline)

            Text("The authenticity of \(host):\(port) can't be established.")
                .font(.caption)
                .foregroundStyle(.secondary)

            GroupBox {
                VStack(alignment: .leading, spacing: 4) {
                    Text("\(keyType) fingerprint:")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text("SHA256:\(fingerprint)")
                        .font(.system(.caption, design: .monospaced))
                        .textSelection(.enabled)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }

            Text("Are you sure you want to continue connecting?")
                .font(.caption)
                .foregroundStyle(.secondary)

            HStack {
                Spacer()
                Button("Cancel") { onCancel() }
                    .keyboardShortcut(.cancelAction)
                Button("Trust & Connect") { onAccept(host, port) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
        .frame(width: 340)
    }
}
```

- [ ] **Step 5: Create KeyboardInteractiveDialog.swift**

```swift
import SwiftUI

struct KeyboardInteractiveDialog: View {
    let tunnelId: String
    let name: String
    let instructions: String
    let prompts: [KiPromptEntry]
    let onSubmit: ([String], String) -> Void  // (responses, tunnelId)
    let onCancel: () -> Void

    @State private var responses: [String]

    init(tunnelId: String, name: String, instructions: String, prompts: [KiPromptEntry],
         onSubmit: @escaping ([String], String) -> Void, onCancel: @escaping () -> Void) {
        self.tunnelId = tunnelId
        self.name = name
        self.instructions = instructions
        self.prompts = prompts
        self.onSubmit = onSubmit
        self.onCancel = onCancel
        self._responses = State(initialValue: Array(repeating: "", count: prompts.count))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(name.isEmpty ? "Authentication Required" : name)
                .font(.headline)

            if !instructions.isEmpty {
                Text(instructions)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            ForEach(Array(prompts.enumerated()), id: \.offset) { index, prompt in
                VStack(alignment: .leading, spacing: 4) {
                    Text(prompt.text)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    if prompt.echo {
                        TextField("", text: $responses[index])
                            .textFieldStyle(.roundedBorder)
                    } else {
                        SecureField("", text: $responses[index])
                            .textFieldStyle(.roundedBorder)
                    }
                }
            }

            HStack {
                Spacer()
                Button("Cancel") { onCancel() }
                    .keyboardShortcut(.cancelAction)
                Button("Submit") { onSubmit(responses, tunnelId) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
        .frame(width: 300)
    }
}
```

- [ ] **Step 6: Wire dialogs into ContentView with .sheet**

Add to ContentView, after `.background(...)`:

```swift
.sheet(item: $viewModel.activeDialog) { dialog in
    switch dialog {
    case .passphrase(let tid, let keyPath):
        PassphraseDialog(
            tunnelId: tid, keyPath: keyPath,
            onSubmit: viewModel.submitPassphrase,
            onCancel: viewModel.cancelDialog
        )
    case .password(let tid):
        PasswordDialog(
            tunnelId: tid,
            onSubmit: viewModel.submitPassword,
            onCancel: viewModel.cancelDialog
        )
    case .hostKey(let tid, let host, let port, let keyType, let fingerprint):
        HostKeyDialog(
            tunnelId: tid, host: host, port: port,
            keyType: keyType, fingerprint: fingerprint,
            onAccept: viewModel.acceptHostKey,
            onCancel: viewModel.cancelDialog
        )
    case .keyboardInteractive(let tid, let name, let instructions, let prompts):
        KeyboardInteractiveDialog(
            tunnelId: tid, name: name,
            instructions: instructions, prompts: prompts,
            onSubmit: viewModel.respondKeyboardInteractive,
            onCancel: viewModel.cancelDialog
        )
    }
}
```

- [ ] **Step 7: Build and verify**

Run: `cd macos-app && xcodegen generate && xcodebuild -project TunnelMaster.xcodeproj -scheme TunnelMaster -configuration Debug build 2>&1 | tail -5`
Expected: BUILD SUCCEEDED

- [ ] **Step 8: Commit**

```bash
git add macos-app/TunnelMaster/Dialogs/ macos-app/TunnelMaster/ContentView.swift macos-app/TunnelMaster/TunnelViewModel.swift
git commit -m "feat(macos-app): add auth dialogs — passphrase, password, host key, keyboard-interactive"
```

---

## Task 2: Edit list view (reorder, delete)

**Files:**
- Create: `macos-app/TunnelMaster/Views/EditListView.swift`
- Modify: `macos-app/TunnelMaster/ContentView.swift`
- Modify: `macos-app/TunnelMaster/TunnelViewModel.swift`

- [ ] **Step 1: Add CRUD methods to TunnelViewModel**

```swift
// MARK: - CRUD operations

func deleteTunnel(id: String) {
    core?.deleteTunnel(id: id)
    refreshTunnels()
}

func reorderTunnels(ids: [String]) {
    core?.reorderTunnels(ids: ids)
    refreshTunnels()
}
```

- [ ] **Step 2: Create EditListView.swift**

The React EditList has: header (+ Add, title, Done), tunnel rows with drag handle + minus button + tap-to-edit + slide-in delete confirm. SwiftUI equivalent uses `List` with `.onMove` and `.onDelete` for simplicity, plus custom row content.

```swift
import SwiftUI

struct EditListView: View {
    @Bindable var viewModel: TunnelViewModel
    @State private var confirmingDelete: String? = nil

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Button("+ Add") {
                    viewModel.currentView = .editForm(tunnelId: nil)
                }
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)

                Spacer()

                Text("Edit Tunnels")
                    .font(.headline)

                Spacer()

                Button("Done") {
                    viewModel.currentView = .list
                }
                .buttonStyle(.plain)
                .fontWeight(.semibold)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)

            Divider()

            if viewModel.tunnels.isEmpty {
                VStack {
                    Spacer()
                    Text("No tunnels configured")
                        .foregroundStyle(.secondary)
                    Button("Add your first tunnel") {
                        viewModel.currentView = .editForm(tunnelId: nil)
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(.secondary)
                    .font(.caption)
                    Spacer()
                }
                .frame(maxHeight: .infinity)
            } else {
                List {
                    ForEach(viewModel.tunnels, id: \.id) { tunnel in
                        HStack {
                            // Minus / delete button
                            Button {
                                if confirmingDelete == tunnel.id {
                                    viewModel.deleteTunnel(id: tunnel.id)
                                    confirmingDelete = nil
                                } else {
                                    confirmingDelete = tunnel.id
                                }
                            } label: {
                                Image(systemName: confirmingDelete == tunnel.id ? "trash.fill" : "minus.circle.fill")
                                    .foregroundStyle(confirmingDelete == tunnel.id ? .white : .red)
                                    .background(confirmingDelete == tunnel.id ? Color.red.clipShape(Circle()) : nil)
                            }
                            .buttonStyle(.plain)

                            // Tap to edit
                            Button {
                                viewModel.currentView = .editForm(tunnelId: tunnel.id)
                            } label: {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(tunnel.name)
                                        .font(.body)
                                        .lineLimit(1)
                                    Text("localhost:\(tunnel.localPort) → \(tunnel.remoteHost):\(tunnel.remotePort)")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                        .lineLimit(1)
                                }
                            }
                            .buttonStyle(.plain)

                            Spacer()

                            Image(systemName: "chevron.right")
                                .font(.caption)
                                .foregroundStyle(.tertiary)
                        }
                    }
                    .onMove { from, to in
                        var ids = viewModel.tunnels.map(\.id)
                        ids.move(fromOffsets: from, toOffset: to)
                        viewModel.reorderTunnels(ids: ids)
                    }
                }
                .listStyle(.plain)
            }
        }
    }
}
```

- [ ] **Step 3: Update ContentView to use EditListView**

Replace the `.editList` placeholder:

```swift
case .editList:
    EditListView(viewModel: viewModel)
```

- [ ] **Step 4: Build and verify**

Run: `cd macos-app && xcodegen generate && xcodebuild ... build`
Expected: BUILD SUCCEEDED

- [ ] **Step 5: Commit**

```bash
git add macos-app/TunnelMaster/Views/EditListView.swift macos-app/TunnelMaster/ContentView.swift macos-app/TunnelMaster/TunnelViewModel.swift
git commit -m "feat(macos-app): add edit list view with reorder and delete"
```

---

## Task 3: Edit form view (add/edit tunnel)

**Files:**
- Create: `macos-app/TunnelMaster/Views/EditFormView.swift`
- Modify: `macos-app/TunnelMaster/ContentView.swift`
- Modify: `macos-app/TunnelMaster/TunnelViewModel.swift`

- [ ] **Step 1: Add form-related methods to TunnelViewModel**

```swift
func getTunnelConfig(id: String) -> TunnelConfig? {
    return core?.getTunnelConfig(id: id)
}

func addTunnel(config: TunnelConfig) {
    core?.addTunnel(config: config)
    refreshTunnels()
    currentView = .editList
}

func updateTunnel(id: String, config: TunnelConfig) {
    core?.updateTunnel(id: id, config: config)
    refreshTunnels()
    currentView = .editList
}
```

- [ ] **Step 2: Create EditFormView.swift**

The form mirrors the React EditForm: Connection section (name, host, port, user, auth method, key path with file picker, jump host), Port Forwarding section (local port, remote host, remote port), Options (auto connect, traffic chart).

```swift
import SwiftUI
import AppKit

struct EditFormView: View {
    @Bindable var viewModel: TunnelViewModel
    let tunnelId: String?

    @State private var name = ""
    @State private var host = ""
    @State private var port: UInt16 = 22
    @State private var user = ""
    @State private var authMethod: AuthMethod = .key
    @State private var keyPath = ""
    @State private var jumpHost: String? = nil
    @State private var localPort: UInt16 = 0
    @State private var remoteHost = ""
    @State private var remotePort: UInt16 = 0
    @State private var autoConnect = false
    @State private var showTrafficChart = true
    @State private var error: String? = nil
    @State private var saving = false

    var isValid: Bool {
        !name.trimmingCharacters(in: .whitespaces).isEmpty &&
        !host.trimmingCharacters(in: .whitespaces).isEmpty &&
        !user.trimmingCharacters(in: .whitespaces).isEmpty &&
        localPort > 0 &&
        !remoteHost.trimmingCharacters(in: .whitespaces).isEmpty &&
        remotePort > 0
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Button("‹ Back") {
                    viewModel.currentView = .editList
                }
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)

                Spacer()

                Text(tunnelId != nil ? "Edit Tunnel" : "New Tunnel")
                    .font(.headline)

                Spacer()

                Button("Save") { save() }
                    .buttonStyle(.plain)
                    .fontWeight(.semibold)
                    .disabled(!isValid || saving)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)

            Divider()

            // Error
            if let error {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .padding(.horizontal, 12)
                    .padding(.top, 8)
            }

            // Form
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    // Connection
                    Section("Connection") {
                        FormField("Name", text: $name)
                        FormField("Host", text: $host)
                        FormField("Port", value: $port)
                        FormField("Username", text: $user)

                        // Auth method picker
                        HStack {
                            Text("Auth")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .frame(width: 70, alignment: .leading)
                            Picker("", selection: $authMethod) {
                                Text("Key").tag(AuthMethod.key)
                                Text("Password").tag(AuthMethod.password)
                                Text("Agent").tag(AuthMethod.agent)
                                Text("2FA").tag(AuthMethod.keyboardInteractive)
                            }
                            .pickerStyle(.segmented)
                            .labelsHidden()
                        }

                        if authMethod == .key {
                            HStack {
                                Text("Key")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .frame(width: 70, alignment: .leading)
                                TextField("~/.ssh/id_rsa", text: $keyPath)
                                    .textFieldStyle(.roundedBorder)
                                    .font(.system(.body, design: .monospaced))
                                Button {
                                    pickKeyFile()
                                } label: {
                                    Image(systemName: "folder")
                                }
                            }
                        }

                        // Jump host
                        HStack {
                            Text("Jump")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .frame(width: 70, alignment: .leading)
                            Picker("None", selection: Binding(
                                get: { jumpHost ?? "" },
                                set: { jumpHost = $0.isEmpty ? nil : $0 }
                            )) {
                                Text("None").tag("")
                                ForEach(viewModel.tunnels.filter { $0.id != tunnelId }, id: \.id) { t in
                                    Text(t.name).tag(t.id)
                                }
                            }
                        }
                    }

                    // Port Forwarding
                    Section("Port Forwarding") {
                        FormField("Local Port", value: $localPort)
                        FormField("Remote Host", text: $remoteHost)
                        FormField("Remote Port", value: $remotePort)
                    }

                    // Options
                    Section("Options") {
                        Toggle("Auto Connect", isOn: $autoConnect)
                        Toggle("Traffic Chart", isOn: $showTrafficChart)
                    }
                }
                .padding(12)
            }
        }
        .onAppear { loadExisting() }
    }

    private func loadExisting() {
        guard let id = tunnelId, let config = viewModel.getTunnelConfig(id: id) else { return }
        name = config.name
        host = config.host
        port = config.port
        user = config.user
        authMethod = config.authMethod
        keyPath = config.keyPath
        jumpHost = config.jumpHost
        localPort = config.localPort
        remoteHost = config.remoteHost
        remotePort = config.remotePort
        autoConnect = config.autoConnect
        showTrafficChart = config.showTrafficChart
    }

    private func save() {
        saving = true
        error = nil

        let config = TunnelConfig(
            id: tunnelId ?? UUID().uuidString,
            name: name, host: host, port: port, user: user,
            authMethod: authMethod, keyPath: keyPath,
            tunnelType: .local,
            localPort: localPort, remoteHost: remoteHost, remotePort: remotePort,
            autoConnect: autoConnect, jumpHost: jumpHost,
            showTrafficChart: showTrafficChart
        )

        if let id = tunnelId {
            viewModel.updateTunnel(id: id, config: config)
        } else {
            viewModel.addTunnel(config: config)
        }
    }

    private func pickKeyFile() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.directoryURL = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".ssh")
        if panel.runModal() == .OK, let url = panel.url {
            keyPath = url.path
        }
    }
}

// MARK: - Helpers

private struct FormField: View {
    let label: String
    @Binding var text: String

    init(_ label: String, text: Binding<String>) {
        self.label = label
        self._text = text
    }

    var body: some View {
        HStack {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(width: 70, alignment: .leading)
            TextField("", text: $text)
                .textFieldStyle(.roundedBorder)
        }
    }
}

extension FormField {
    init(_ label: String, value: Binding<UInt16>) {
        self.label = label
        self._text = Binding(
            get: { value.wrappedValue == 0 ? "" : String(value.wrappedValue) },
            set: { value.wrappedValue = UInt16($0) ?? 0 }
        )
    }
}
```

Note: `TunnelConfig` ID generation for new tunnels — the Rust side's `add_tunnel` uses `config.id` as-is. The React app generates slugified IDs from the name. For now, use UUID; we can improve later. The important thing is that config persistence works.

Actually, looking at the Rust `api.rs`, `add_tunnel` passes config straight to the manager. The ID must be set. For new tunnels, generate a slug-based ID like the Tauri commands.rs does. Add a helper to ViewModel:

```swift
private func generateId(from name: String) -> String {
    let slug = name.lowercased()
        .replacingOccurrences(of: "[^a-z0-9]", with: "-", options: .regularExpression)
        .replacingOccurrences(of: "-+", with: "-", options: .regularExpression)
        .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
    let existingIds = Set(tunnels.map(\.id))
    if !existingIds.contains(slug) { return slug }
    var n = 2
    while existingIds.contains("\(slug)-\(n)") { n += 1 }
    return "\(slug)-\(n)"
}
```

Update the `save()` method to use `generateId` for new tunnels instead of UUID.

- [ ] **Step 3: Update ContentView for editForm**

Replace the `.editForm` placeholder:

```swift
case .editForm(let tunnelId):
    EditFormView(viewModel: viewModel, tunnelId: tunnelId)
```

- [ ] **Step 4: Build and verify**

Run build. Expected: BUILD SUCCEEDED.

- [ ] **Step 5: Commit**

```bash
git add macos-app/TunnelMaster/Views/EditFormView.swift macos-app/TunnelMaster/ContentView.swift macos-app/TunnelMaster/TunnelViewModel.swift
git commit -m "feat(macos-app): add edit form view with add/edit tunnel support"
```

---

## Task 4: Traffic sparklines

**Files:**
- Create: `macos-app/TunnelMaster/Views/TrafficSparkline.swift`
- Modify: `macos-app/TunnelMaster/Views/TunnelRow.swift`
- Modify: `macos-app/TunnelMaster/TunnelViewModel.swift`

- [ ] **Step 1: Add traffic data to TunnelViewModel**

```swift
// Add property
var trafficHistory: [String: [TrafficSample]] = [:]

// Update onTrafficUpdate
func onTrafficUpdate(id: String, sample: TrafficSample) {
    Task { @MainActor in
        var history = self.trafficHistory[id] ?? []
        history.append(sample)
        if history.count > 60 { history.removeFirst(history.count - 60) }
        self.trafficHistory[id] = history
    }
}

// Add method to get history (loads initial from Rust)
func getTrafficHistory(id: String) -> [TrafficSample] {
    if let cached = trafficHistory[id], !cached.isEmpty {
        return cached
    }
    guard let core else { return [] }
    let history = core.getTrafficHistory(id: id)
    Task { @MainActor in
        self.trafficHistory[id] = history
    }
    return history
}
```

- [ ] **Step 2: Create TrafficSparkline.swift**

Port the React SVG sparkline to SwiftUI Path:

```swift
import SwiftUI

struct TrafficSparkline: View {
    let samples: [TrafficSample]

    private let maxPoints = 60

    var body: some View {
        if samples.count >= 2 {
            Canvas { context, size in
                let maxVal = max(100, samples.map { max($0.bytesIn, $0.bytesOut) }.max() ?? 100)

                // Download line (green)
                let downloadPath = buildPath(samples: samples, size: size, maxVal: maxVal) { $0.bytesIn }
                context.stroke(downloadPath, with: .color(.green.opacity(0.5)), lineWidth: 1.5)

                // Upload line (blue, dashed)
                let uploadPath = buildPath(samples: samples, size: size, maxVal: maxVal) { $0.bytesOut }
                context.stroke(uploadPath, with: .color(.blue.opacity(0.4)), style: StrokeStyle(lineWidth: 1, dash: [3, 2]))
            }
            .allowsHitTesting(false)
            .opacity(0.25)
        }
    }

    private func buildPath(samples: [TrafficSample], size: CGSize, maxVal: UInt64, getValue: (TrafficSample) -> UInt64) -> Path {
        Path { path in
            let step = size.width / CGFloat(maxPoints - 1)
            let startIndex = max(0, maxPoints - samples.count)

            for (i, sample) in samples.enumerated() {
                let x = CGFloat(startIndex + i) * step
                let y = size.height - (CGFloat(getValue(sample)) / CGFloat(maxVal)) * (size.height - 4)
                if i == 0 {
                    path.move(to: CGPoint(x: x, y: y))
                } else {
                    path.addLine(to: CGPoint(x: x, y: y))
                }
            }
        }
    }
}
```

- [ ] **Step 3: Add sparkline to TunnelRow**

Modify TunnelRow to show sparkline as a background overlay when the tunnel is connected and `showTrafficChart` is true:

```swift
// Add to TunnelRow, wrap the existing HStack in a ZStack:
var body: some View {
    ZStack {
        if tunnel.status == .connected && tunnel.showTrafficChart {
            TrafficSparkline(samples: samples)
        }

        HStack(spacing: 8) {
            // ... existing content unchanged
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
    }
    .contentShape(Rectangle())
}
```

TunnelRow needs a `samples` property:
```swift
let samples: [TrafficSample]
```

Update TunnelListView's ForEach to pass samples:
```swift
TunnelRow(tunnel: tunnel, samples: viewModel.trafficHistory[tunnel.id] ?? []) {
    viewModel.toggleConnection(id: tunnel.id)
}
```

- [ ] **Step 4: Build and verify**

Run build. Expected: BUILD SUCCEEDED.

- [ ] **Step 5: Commit**

```bash
git add macos-app/TunnelMaster/Views/TrafficSparkline.swift macos-app/TunnelMaster/Views/TunnelRow.swift macos-app/TunnelMaster/Views/TunnelListView.swift macos-app/TunnelMaster/TunnelViewModel.swift
git commit -m "feat(macos-app): add traffic sparkline charts"
```

---

## Task 5: Tray icon

**Files:**
- Modify: `macos-app/TunnelMaster/TunnelMasterApp.swift`

- [ ] **Step 1: Use custom tray icon**

The current app uses `Image(systemName: "network")`. The Tauri app has a custom tray icon at `src-tauri/icons/tray-icon.png`. For now, use an SF Symbol that looks like a tunnel/network. We can replace with the custom icon later.

Update the MenuBarExtra label to show connected count:

```swift
MenuBarExtra {
    ContentView(viewModel: viewModel)
        .frame(width: 320, height: 400)
} label: {
    let connected = viewModel.tunnels.filter { $0.status == .connected }.count
    if connected > 0 {
        Label("\(connected)", systemImage: "network")
    } else {
        Image(systemName: "network")
    }
}
.menuBarExtraStyle(.window)
```

- [ ] **Step 2: Build and verify**

Run build. Expected: BUILD SUCCEEDED.

- [ ] **Step 3: Commit**

```bash
git add macos-app/TunnelMaster/TunnelMasterApp.swift
git commit -m "feat(macos-app): show connected count in tray icon"
```

---

## Summary

After completing all 5 tasks:
- Auth dialogs (passphrase, password, host key, keyboard-interactive) show as sheets
- Edit list with reorder (drag) and delete (two-tap confirm)
- Edit form with all fields (connection, port forwarding, options, file picker, jump host)
- Traffic sparklines as background overlay on connected tunnels
- Tray icon shows connected count

**Feature parity achieved.** Next: Phase 4 (Cleanup) — remove src-tauri/, src/, update release workflow, bump to v1.0.
