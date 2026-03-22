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
