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
                let connectedCount = viewModel.tunnels.filter { $0.status == .connected }.count
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
        .onAppear {
            viewModel.start()
        }
    }
}
