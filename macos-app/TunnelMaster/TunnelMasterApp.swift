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
        SwiftUI.Settings { EmptyView() }
    }
}