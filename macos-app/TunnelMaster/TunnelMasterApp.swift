import SwiftUI

@main
struct TunnelMasterApp: App {
    @State private var viewModel = TunnelViewModel()

    var body: some Scene {
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
    }
}
