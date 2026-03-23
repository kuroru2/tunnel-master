import SwiftUI

struct TunnelListView: View {
    @Bindable var viewModel: TunnelViewModel
    @State private var searchText = ""
    @State private var collapsedGroups: Set<String> = []

    private var filteredTunnels: [TunnelInfo] {
        if searchText.isEmpty {
            return viewModel.tunnels
        }
        let query = searchText.lowercased()
        return viewModel.tunnels.filter {
            $0.name.lowercased().contains(query) ||
            $0.remoteHost.lowercased().contains(query) ||
            String($0.localPort).contains(query) ||
            ($0.group?.lowercased().contains(query) ?? false)
        }
    }

    /// Group names in order of first appearance, plus nil for ungrouped
    private var groupOrder: [String?] {
        var seen = Set<String>()
        var order: [String?] = []
        for tunnel in filteredTunnels {
            let key = tunnel.group ?? ""
            if seen.insert(key).inserted {
                order.append(tunnel.group)
            }
        }
        return order
    }

    private func tunnelsInGroup(_ group: String?) -> [TunnelInfo] {
        filteredTunnels.filter { $0.group == group }
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
                let groups = groupOrder
                let hasGroups = groups.contains(where: { $0 != nil })

                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(groups, id: \.self) { group in
                            let tunnels = tunnelsInGroup(group)

                            if hasGroups {
                                GroupHeader(
                                    name: group ?? "Ungrouped",
                                    tunnels: tunnels,
                                    isCollapsed: collapsedGroups.contains(group ?? ""),
                                    isGrouped: group != nil,
                                    onToggleCollapse: {
                                        let key = group ?? ""
                                        if collapsedGroups.contains(key) {
                                            collapsedGroups.remove(key)
                                        } else {
                                            collapsedGroups.insert(key)
                                        }
                                    },
                                    onToggleGroup: {
                                        if let groupName = group {
                                            viewModel.toggleGroup(groupName)
                                        }
                                    }
                                )
                            }

                            if !collapsedGroups.contains(group ?? "") {
                                ForEach(tunnels, id: \.id) { tunnel in
                                    TunnelRow(
                                        tunnel: tunnel,
                                        samples: viewModel.trafficHistory[tunnel.id] ?? [],
                                        onToggle: { viewModel.toggleConnection(id: tunnel.id) },
                                        onOpenTerminal: { viewModel.openTerminal(id: tunnel.id) }
                                    )
                                    Divider()
                                }
                            }
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

struct GroupHeader: View {
    let name: String
    let tunnels: [TunnelInfo]
    let isCollapsed: Bool
    let isGrouped: Bool
    let onToggleCollapse: () -> Void
    let onToggleGroup: () -> Void

    private var connectedCount: Int {
        tunnels.filter { $0.status == .connected }.count
    }

    var body: some View {
        HStack(spacing: 6) {
            Button {
                onToggleCollapse()
            } label: {
                Image(systemName: isCollapsed ? "chevron.right" : "chevron.down")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .frame(width: 12)
            }
            .buttonStyle(.plain)

            Text(name.uppercased())
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.secondary)
                .lineLimit(1)

            Text("\(connectedCount)/\(tunnels.count)")
                .font(.system(size: 9))
                .foregroundStyle(.tertiary)

            Spacer()

            if isGrouped {
                Button {
                    onToggleGroup()
                } label: {
                    Text(connectedCount == tunnels.count ? "Stop All" : "Start All")
                        .font(.system(size: 9))
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 4)
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.3))
    }
}
