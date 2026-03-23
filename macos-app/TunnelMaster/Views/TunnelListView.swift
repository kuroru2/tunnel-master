import SwiftUI

struct TunnelListView: View {
    @Bindable var viewModel: TunnelViewModel
    @State private var searchText = ""

    private var filteredTunnels: [TunnelInfo] {
        if searchText.isEmpty {
            return viewModel.tunnels
        }
        let query = searchText.lowercased()
        return viewModel.tunnels.filter {
            $0.name.lowercased().contains(query) ||
            $0.remoteHost.lowercased().contains(query) ||
            String($0.localPort).contains(query)
        }
    }

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

            // Search
            if viewModel.tunnels.count > 3 {
                HStack(spacing: 6) {
                    Image(systemName: "magnifyingglass")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                    TextField("Filter...", text: $searchText)
                        .textFieldStyle(.plain)
                        .font(.caption)
                    if !searchText.isEmpty {
                        Button {
                            searchText = ""
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .font(.caption)
                                .foregroundStyle(.tertiary)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 4)
                .background(Color(nsColor: .controlBackgroundColor).opacity(0.5))
            }

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
            } else if filteredTunnels.isEmpty {
                VStack {
                    Spacer()
                    Text("No matches for \"\(searchText)\"")
                        .foregroundStyle(.secondary)
                        .font(.caption)
                    Spacer()
                }
                .frame(maxHeight: .infinity)
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(filteredTunnels, id: \.id) { tunnel in
                            TunnelRow(tunnel: tunnel, samples: viewModel.trafficHistory[tunnel.id] ?? [],
                                     onToggle: { viewModel.toggleConnection(id: tunnel.id) },
                                     onOpenTerminal: { viewModel.openTerminal(id: tunnel.id) })
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
        .task {
            viewModel.start()
        }
    }
}
