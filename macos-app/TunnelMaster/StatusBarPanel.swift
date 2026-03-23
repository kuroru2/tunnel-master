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