# NSPanel Full-Screen Menu Bar Fix — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace SwiftUI's `MenuBarExtra(.window)` with a custom `NSStatusItem` + `NSPanel` that keeps the macOS menu bar visible in full-screen mode while the popup is open.

**Architecture:** Create a custom `NSPanel` subclass initialized with `.nonactivatingPanel` so the app never activates (the full-screen app stays "active" and macOS doesn't auto-hide the menu bar). Post `HIToolbox` menu-tracking notifications to pin the menu bar visible. Wire `NSStatusItem` manually with dynamic badge updates driven by the existing `@Observable` `TunnelViewModel`.

**Tech Stack:** AppKit (NSPanel, NSStatusItem, NSHostingView), SwiftUI (ContentView unchanged), macOS 14+

**Known risk:** `com.apple.HIToolbox.beginMenuTrackingNotification` / `endMenuTrackingNotification` are undocumented system notifications. They work today and are used by popular menu bar apps, but could break in future macOS versions.

**Note:** `.sheet()` may not present correctly on a non-activating `NSPanel`. If dialogs break, the fallback is to temporarily activate the app when a dialog is needed (`NSApp.activate()`) and deactivate after dismissal.

---

## File Structure

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `TunnelMaster/StatusBarPanel.swift` | Custom `NSPanel` subclass with `.nonactivatingPanel`, Escape-to-dismiss |
| Create | `TunnelMaster/StatusBarController.swift` | Owns `NSStatusItem` + `StatusBarPanel`, handles click, badge updates, show/hide with menu-tracking notifications |
| Modify | `TunnelMaster/TunnelMasterApp.swift` | Replace `MenuBarExtra` with `NSApplicationDelegateAdaptor` that creates `StatusBarController` |

**No changes needed to:** `ContentView.swift`, `TunnelViewModel.swift`, any Views or Dialogs — they stay exactly as-is.

**XcodeGen:** `project.yml` uses `path: TunnelMaster` which auto-includes all `.swift` files. Run `xcodegen generate` after adding new files so `.xcodeproj` picks them up.

---

### Task 1: Create `StatusBarPanel` — the non-activating NSPanel

**Files:**
- Create: `macos-app/TunnelMaster/StatusBarPanel.swift`

- [ ] **Step 1: Create the NSPanel subclass**

```swift
import AppKit

/// A non-activating panel that appears below the status bar item.
/// Using `.nonactivatingPanel` at init time prevents app activation,
/// which keeps the menu bar visible in full-screen mode.
final class StatusBarPanel: NSPanel {

    /// Called by StatusBarController when the panel should dismiss.
    var onDismiss: (() -> Void)?

    init(contentView: NSView) {
        super.init(
            contentRect: .zero,
            styleMask: [.nonactivatingPanel, .titled, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )

        titleVisibility = .hidden
        titlebarAppearsTransparent = true

        isFloatingPanel = true
        level = .statusBar
        hidesOnDeactivate = false
        isMovableByWindowBackground = false
        animationBehavior = .utilityWindow
        isOpaque = false
        backgroundColor = .clear

        collectionBehavior = [
            .auxiliary,
            .stationary,
            .moveToActiveSpace,
            .fullScreenAuxiliary
        ]

        self.contentView = contentView
    }

    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }

    /// Escape key dismisses the panel.
    override func cancelOperation(_ sender: Any?) {
        onDismiss?()
    }
}
```

- [ ] **Step 2: Regenerate Xcode project and verify it compiles**

Run:
```bash
cd macos-app && xcodegen generate && xcodebuild -project TunnelMaster.xcodeproj -scheme TunnelMaster build 2>&1 | tail -5
```
Expected: `BUILD SUCCEEDED`

- [ ] **Step 3: Commit**

```bash
git add macos-app/TunnelMaster/StatusBarPanel.swift
git commit -m "feat: add StatusBarPanel with nonactivatingPanel for full-screen support"
```

---

### Task 2: Create `StatusBarController` — status item + show/hide logic

**Files:**
- Create: `macos-app/TunnelMaster/StatusBarController.swift`

- [ ] **Step 1: Create the controller**

```swift
import AppKit
import SwiftUI

/// Manages the NSStatusItem and the popup panel lifecycle.
/// Posts HIToolbox menu-tracking notifications to keep the menu bar
/// visible in full-screen mode while the panel is open.
final class StatusBarController {

    private var statusItem: NSStatusItem!
    private var panel: StatusBarPanel!
    private var globalMonitor: Any?
    private var localMonitor: Any?
    private let viewModel: TunnelViewModel
    private var observationTask: Task<Void, Never>?

    init(viewModel: TunnelViewModel) {
        self.viewModel = viewModel
        setupStatusItem()
        setupPanel()
        observeBadge()
    }

    deinit {
        observationTask?.cancel()
        removeMonitors()
    }

    // MARK: - Status Item

    private func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        guard let button = statusItem.button else { return }
        button.image = NSImage(systemSymbolName: "network", accessibilityDescription: "Tunnel Master")
        button.action = #selector(togglePanel)
        button.target = self
        updateBadge()
    }

    /// Reactively observe tunnel changes via @Observable and update the badge.
    private func observeBadge() {
        observationTask = Task { @MainActor [weak self] in
            guard let self else { return }
            withObservationTracking {
                self.updateBadge()
            } onChange: {
                Task { @MainActor [weak self] in
                    self?.observeBadge()
                }
            }
        }
    }

    private func updateBadge() {
        guard let button = statusItem.button else { return }
        let connected = viewModel.tunnels.filter { $0.status == .connected }.count
        if connected > 0 {
            button.image = NSImage(systemSymbolName: "network", accessibilityDescription: nil)
            button.title = " \(connected)"
        } else {
            button.image = NSImage(systemSymbolName: "network", accessibilityDescription: nil)
            button.title = ""
        }
    }

    // MARK: - Panel

    private func setupPanel() {
        let hostingView = NSHostingView(
            rootView: ContentView(viewModel: viewModel)
                .frame(width: 320, height: 400)
        )
        panel = StatusBarPanel(contentView: hostingView)
        panel.setContentSize(NSSize(width: 320, height: 400))
        panel.onDismiss = { [weak self] in self?.hidePanel() }
    }

    @objc private func togglePanel() {
        if panel.isVisible {
            hidePanel()
        } else {
            showPanel()
        }
    }

    private func showPanel() {
        guard let button = statusItem.button,
              let buttonWindow = button.window else { return }
        let buttonFrame = buttonWindow.convertToScreen(button.convert(button.bounds, to: nil))
        let panelSize = panel.frame.size
        let screen = buttonWindow.screen ?? NSScreen.main ?? NSScreen.screens[0]

        // Center under button, clamped to screen edges
        var x = buttonFrame.midX - panelSize.width / 2
        x = max(screen.visibleFrame.minX + 4, x)
        x = min(screen.visibleFrame.maxX - panelSize.width - 4, x)
        let y = buttonFrame.minY

        panel.setFrameTopLeftPoint(NSPoint(x: x, y: y))

        // Pin the menu bar visible in full-screen
        DistributedNotificationCenter.default().post(
            name: Notification.Name("com.apple.HIToolbox.beginMenuTrackingNotification"),
            object: nil
        )

        panel.makeKeyAndOrderFront(nil)
        addMonitors()

        button.highlight(true)
    }

    private func hidePanel() {
        panel.orderOut(nil)
        removeMonitors()

        // Release the menu bar pin
        DistributedNotificationCenter.default().post(
            name: Notification.Name("com.apple.HIToolbox.endMenuTrackingNotification"),
            object: nil
        )

        statusItem.button?.highlight(false)
    }

    // MARK: - Click-outside-to-dismiss

    private func addMonitors() {
        globalMonitor = NSEvent.addGlobalMonitorForEvents(matching: [.leftMouseDown, .rightMouseDown]) { [weak self] _ in
            self?.hidePanel()
        }
        localMonitor = NSEvent.addLocalMonitorForEvents(matching: [.leftMouseDown, .rightMouseDown]) { [weak self] event in
            if let self, let button = self.statusItem.button,
               event.window == button.window {
                self.hidePanel()
                return nil
            }
            return event
        }
    }

    private func removeMonitors() {
        if let m = globalMonitor { NSEvent.removeMonitor(m); globalMonitor = nil }
        if let m = localMonitor { NSEvent.removeMonitor(m); localMonitor = nil }
    }
}
```

- [ ] **Step 2: Regenerate Xcode project and verify it compiles**

Run:
```bash
cd macos-app && xcodegen generate && xcodebuild -project TunnelMaster.xcodeproj -scheme TunnelMaster build 2>&1 | tail -5
```
Expected: `BUILD SUCCEEDED`

- [ ] **Step 3: Commit**

```bash
git add macos-app/TunnelMaster/StatusBarController.swift
git commit -m "feat: add StatusBarController with menu-tracking notifications for full-screen"
```

---

### Task 3: Wire up `TunnelMasterApp` to use the new panel

**Files:**
- Modify: `macos-app/TunnelMaster/TunnelMasterApp.swift`

- [ ] **Step 1: Replace MenuBarExtra with NSApplicationDelegateAdaptor**

Replace the entire file with:

```swift
import SwiftUI

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var statusBarController: StatusBarController?
    private let viewModel = TunnelViewModel()

    func applicationDidFinishLaunching(_ notification: Notification) {
        viewModel.start()
        statusBarController = StatusBarController(viewModel: viewModel)
    }

    func applicationWillTerminate(_ notification: Notification) {
        viewModel.shutdown()
    }
}

@main
struct TunnelMasterApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        Settings { EmptyView() }
    }
}
```

- [ ] **Step 2: Build and run**

Run:
```bash
cd macos-app && xcodebuild -project TunnelMaster.xcodeproj -scheme TunnelMaster -configuration Debug build 2>&1 | tail -5
```
Expected: `BUILD SUCCEEDED`

- [ ] **Step 3: Manual test checklist**

1. App appears in menu bar with network icon
2. Click opens popup panel below status item
3. Click outside dismisses panel
4. Escape key dismisses panel
5. Connected tunnels show count badge next to icon
6. In full-screen mode: open popup, move mouse away — **menu bar stays visible**
7. Dismiss popup — menu bar resumes normal auto-hide
8. Dialogs (passphrase, password, host key) still work via `.sheet()`
   - If `.sheet()` fails: add `NSApp.activate(ignoringOtherApps: true)` when showing dialog, restore after dismiss

- [ ] **Step 4: Commit**

```bash
git add macos-app/TunnelMaster/TunnelMasterApp.swift
git commit -m "feat: replace MenuBarExtra with custom NSPanel for full-screen menu bar fix"
```
