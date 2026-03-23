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
        var originX = buttonFrame.midX - panelSize.width / 2
        originX = max(screen.visibleFrame.minX + 4, originX)
        originX = min(screen.visibleFrame.maxX - panelSize.width - 4, originX)
        let originY = buttonFrame.minY

        panel.setFrameTopLeftPoint(NSPoint(x: originX, y: originY))

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
        if let monitor = globalMonitor { NSEvent.removeMonitor(monitor); globalMonitor = nil }
        if let monitor = localMonitor { NSEvent.removeMonitor(monitor); localMonitor = nil }
    }
}