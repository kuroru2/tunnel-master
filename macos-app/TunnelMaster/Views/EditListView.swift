import SwiftUI

struct EditListView: View {
    @Bindable var viewModel: TunnelViewModel
    @State private var confirmingDelete: String? = nil

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button("+ Add") { viewModel.currentView = .editForm(tunnelId: nil) }
                    .buttonStyle(.plain).foregroundStyle(.secondary)
                Spacer()
                Text("Edit Tunnels").font(.headline)
                Spacer()
                Button("Done") { viewModel.currentView = .list }
                    .buttonStyle(.plain).fontWeight(.semibold)
            }
            .padding(.horizontal, 12).padding(.vertical, 8)

            Divider()

            if viewModel.tunnels.isEmpty {
                VStack {
                    Spacer()
                    Text("No tunnels configured").foregroundStyle(.secondary)
                    Button("Add your first tunnel") { viewModel.currentView = .editForm(tunnelId: nil) }
                        .buttonStyle(.plain).foregroundStyle(.secondary).font(.caption)
                    Spacer()
                }.frame(maxHeight: .infinity)
            } else {
                List {
                    ForEach(viewModel.tunnels, id: \.id) { tunnel in
                        HStack {
                            Button {
                                if confirmingDelete == tunnel.id {
                                    viewModel.deleteTunnel(id: tunnel.id)
                                    confirmingDelete = nil
                                } else {
                                    confirmingDelete = tunnel.id
                                }
                            } label: {
                                Image(systemName: confirmingDelete == tunnel.id ? "trash.fill" : "minus.circle.fill")
                                    .foregroundStyle(.red)
                            }.buttonStyle(.plain)

                            Button { viewModel.currentView = .editForm(tunnelId: tunnel.id) } label: {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(tunnel.name).font(.body).lineLimit(1)
                                    Text("localhost:\(tunnel.localPort) \u{2192} \(tunnel.remoteHost):\(tunnel.remotePort)")
                                        .font(.caption).foregroundStyle(.secondary).lineLimit(1)
                                }
                            }.buttonStyle(.plain)

                            Spacer()
                            Image(systemName: "chevron.right").font(.caption).foregroundStyle(.tertiary)
                        }
                    }
                    .onMove { from, to in
                        var ids = viewModel.tunnels.map(\.id)
                        ids.move(fromOffsets: from, toOffset: to)
                        viewModel.reorderTunnels(ids: ids)
                    }
                }.listStyle(.plain)
            }
        }
    }
}
