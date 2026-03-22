import SwiftUI
import AppKit

struct EditFormView: View {
    @Bindable var viewModel: TunnelViewModel
    let tunnelId: String?

    @State private var name = ""
    @State private var host = ""
    @State private var port: UInt16 = 22
    @State private var user = ""
    @State private var authMethod: AuthMethod = .key
    @State private var keyPath = ""
    @State private var jumpHost: String? = nil
    @State private var localPort: UInt16 = 0
    @State private var remoteHost = ""
    @State private var remotePort: UInt16 = 0
    @State private var autoConnect = false
    @State private var showTrafficChart = true
    @State private var error: String? = nil

    var isValid: Bool {
        !name.trimmingCharacters(in: .whitespaces).isEmpty &&
        !host.trimmingCharacters(in: .whitespaces).isEmpty &&
        !user.trimmingCharacters(in: .whitespaces).isEmpty &&
        localPort > 0 &&
        !remoteHost.trimmingCharacters(in: .whitespaces).isEmpty &&
        remotePort > 0
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Button("\u{2039} Back") { viewModel.currentView = .editList }
                    .buttonStyle(.plain).foregroundStyle(.secondary)
                Spacer()
                Text(tunnelId != nil ? "Edit Tunnel" : "New Tunnel").font(.headline)
                Spacer()
                Button("Save") { save() }
                    .buttonStyle(.plain).fontWeight(.semibold).disabled(!isValid)
            }
            .padding(.horizontal, 12).padding(.vertical, 8)

            Divider()

            if let error {
                Text(error).font(.caption).foregroundStyle(.red)
                    .padding(.horizontal, 12).padding(.top, 8)
            }

            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    // Connection
                    GroupBox("Connection") {
                        VStack(spacing: 8) {
                            formField("Name", text: $name)
                            formField("Host", text: $host)
                            formFieldUInt16("Port", value: $port)
                            formField("Username", text: $user)

                            HStack {
                                Text("Auth").font(.caption).foregroundStyle(.secondary)
                                    .frame(width: 70, alignment: .leading)
                                Picker("", selection: $authMethod) {
                                    Text("Key").tag(AuthMethod.key)
                                    Text("Password").tag(AuthMethod.password)
                                    Text("Agent").tag(AuthMethod.agent)
                                    Text("2FA").tag(AuthMethod.keyboardInteractive)
                                }.pickerStyle(.segmented).labelsHidden()
                            }

                            if authMethod == .key {
                                HStack {
                                    Text("Key").font(.caption).foregroundStyle(.secondary)
                                        .frame(width: 70, alignment: .leading)
                                    TextField("~/.ssh/id_rsa", text: $keyPath)
                                        .textFieldStyle(.roundedBorder)
                                        .font(.system(.body, design: .monospaced))
                                    Button { pickKeyFile() } label: {
                                        Image(systemName: "folder")
                                    }
                                }
                            }

                            HStack {
                                Text("Jump").font(.caption).foregroundStyle(.secondary)
                                    .frame(width: 70, alignment: .leading)
                                Picker("None", selection: Binding(
                                    get: { jumpHost ?? "" },
                                    set: { jumpHost = $0.isEmpty ? nil : $0 }
                                )) {
                                    Text("None").tag("")
                                    ForEach(viewModel.tunnels.filter { $0.id != tunnelId }, id: \.id) { t in
                                        Text(t.name).tag(t.id)
                                    }
                                }
                            }
                        }
                    }

                    GroupBox("Port Forwarding") {
                        VStack(spacing: 8) {
                            formFieldUInt16("Local Port", value: $localPort)
                            formField("Remote Host", text: $remoteHost)
                            formFieldUInt16("Remote Port", value: $remotePort)
                        }
                    }

                    GroupBox("Options") {
                        VStack(spacing: 8) {
                            Toggle("Auto Connect", isOn: $autoConnect)
                            Toggle("Traffic Chart", isOn: $showTrafficChart)
                        }
                    }
                }.padding(12)
            }
        }
        .onAppear { loadExisting() }
    }

    private func loadExisting() {
        guard let id = tunnelId, let config = viewModel.getTunnelConfig(id: id) else { return }
        name = config.name; host = config.host; port = config.port; user = config.user
        authMethod = config.authMethod; keyPath = config.keyPath
        jumpHost = config.jumpHost; localPort = config.localPort
        remoteHost = config.remoteHost; remotePort = config.remotePort
        autoConnect = config.autoConnect; showTrafficChart = config.showTrafficChart
    }

    private func save() {
        let id = tunnelId ?? generateId(from: name)
        let config = TunnelConfig(
            id: id, name: name, host: host, port: port, user: user,
            authMethod: authMethod, keyPath: keyPath, tunnelType: .local,
            localPort: localPort, remoteHost: remoteHost, remotePort: remotePort,
            autoConnect: autoConnect, jumpHost: jumpHost, showTrafficChart: showTrafficChart
        )
        if tunnelId != nil {
            viewModel.updateTunnel(id: id, config: config)
        } else {
            viewModel.addTunnel(config: config)
        }
    }

    private func generateId(from name: String) -> String {
        let slug = name.lowercased()
            .replacingOccurrences(of: "[^a-z0-9]", with: "-", options: .regularExpression)
            .replacingOccurrences(of: "-+", with: "-", options: .regularExpression)
            .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        let existingIds = Set(viewModel.tunnels.map(\.id))
        if !existingIds.contains(slug) && !slug.isEmpty { return slug }
        var n = 2
        while existingIds.contains("\(slug)-\(n)") { n += 1 }
        return slug.isEmpty ? "tunnel-\(n)" : "\(slug)-\(n)"
    }

    private func pickKeyFile() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.directoryURL = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".ssh")
        if panel.runModal() == .OK, let url = panel.url { keyPath = url.path }
    }

    private func formField(_ label: String, text: Binding<String>) -> some View {
        HStack {
            Text(label).font(.caption).foregroundStyle(.secondary).frame(width: 70, alignment: .leading)
            TextField("", text: text).textFieldStyle(.roundedBorder)
        }
    }

    private func formFieldUInt16(_ label: String, value: Binding<UInt16>) -> some View {
        HStack {
            Text(label).font(.caption).foregroundStyle(.secondary).frame(width: 70, alignment: .leading)
            TextField("", text: Binding(
                get: { value.wrappedValue == 0 ? "" : String(value.wrappedValue) },
                set: { value.wrappedValue = UInt16($0) ?? 0 }
            )).textFieldStyle(.roundedBorder)
        }
    }
}
