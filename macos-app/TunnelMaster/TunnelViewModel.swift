import Foundation
import Observation
import AppKit
import os.log

private let logger = Logger(subsystem: "com.kuroru2.tunnel-master", category: "ViewModel")

private let logQueue = DispatchQueue(label: "com.kuroru2.tunnel-master.log", qos: .utility)

private func tmLog(_ msg: String) {
    logger.info("\(msg)")
    let line = "\(Date()): \(msg)\n"
    logQueue.async {
        let logFile = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".tunnel-master/swiftui.log")
        guard let data = line.data(using: .utf8) else { return }
        if FileManager.default.fileExists(atPath: logFile.path) {
            if let handle = try? FileHandle(forWritingTo: logFile) {
                handle.seekToEndOfFile()
                handle.write(data)
                handle.closeFile()
            }
        } else {
            try? data.write(to: logFile)
        }
    }
}

@Observable
final class TunnelViewModel: TunnelEventHandler {
    var tunnels: [TunnelInfo] = []
    var currentView: ViewMode = .list
    var activeDialog: DialogState?
    var trafficHistory: [String: [TrafficSample]] = [:]

    private var core: TunnelCore?

    enum DialogState: Identifiable {
        case passphrase(tunnelId: String, keyPath: String)
        case password(tunnelId: String)
        case hostKey(tunnelId: String, host: String, port: UInt16, keyType: String, fingerprint: String)
        case keyboardInteractive(tunnelId: String, name: String, instructions: String, prompts: [KiPromptEntry])

        var id: String {
            switch self {
            case .passphrase(let tid, _): return "passphrase-\(tid)"
            case .password(let tid): return "password-\(tid)"
            case .hostKey(let tid, _, _, _, _): return "hostkey-\(tid)"
            case .keyboardInteractive(let tid, _, _, _): return "ki-\(tid)"
            }
        }
    }

    func start() {
        tmLog("[TM] start() called, core is \(core == nil ? "nil" : "set")")
        guard core == nil else {
            refreshTunnels()
            return
        }
        tmLog("[TM] Creating TunnelCore...")
        core = TunnelCore(eventHandler: self)
        tmLog("[TM] TunnelCore created, refreshing tunnels")
        refreshTunnels()
        autoConnectTunnels()
    }

    private func autoConnectTunnels() {
        guard let core else { return }
        // Use list directly from core (tunnels array may not be populated yet)
        let allTunnels = core.listTunnels()
        for tunnel in allTunnels {
            if let config = core.getTunnelConfig(id: tunnel.id),
               config.autoConnect {
                tmLog("[TM] Auto-connecting tunnel: \(tunnel.name)")
                core.connect(id: tunnel.id)
            }
        }
    }

    func shutdown() {
        core?.shutdown()
        core = nil
    }

    // MARK: - Public API

    func refreshTunnels() {
        guard let core else { return }
        let list = core.listTunnels()
        tmLog("[TM] refreshTunnels: \(list.map { "\($0.id):\($0.status) err=\($0.errorMessage ?? "nil")" })")
        Task { @MainActor in
            self.tunnels = list
        }
    }

    func toggleConnection(id: String) {
        guard let core else { return }
        if let tunnel = tunnels.first(where: { $0.id == id }) {
            switch tunnel.status {
            case .connected, .connecting:
                core.disconnect(id: id)
            case .disconnected, .error:
                core.connect(id: id)
            case .disconnecting:
                break
            }
        }
    }

    // MARK: - Group actions

    func toggleGroup(_ group: String) {
        let groupTunnels = tunnels.filter { $0.group == group }
        let allConnected = groupTunnels.allSatisfy { $0.status == .connected }
        for tunnel in groupTunnels {
            if allConnected {
                core?.disconnect(id: tunnel.id)
            } else if tunnel.status == .disconnected || tunnel.status == .error {
                core?.connect(id: tunnel.id)
            }
        }
    }

    // MARK: - SSH Terminal

    func openTerminal(id: String) {
        guard let core, let config = core.getTunnelConfig(id: id) else { return }
        let port = config.port == 22 ? "" : " -p \(config.port)"
        let sshCommand = "ssh \(config.user)@\(config.host)\(port)"
        tmLog("[TM] Opening terminal: \(sshCommand)")

        // Create a temp shell script and open it with Terminal
        let tmpDir = FileManager.default.temporaryDirectory
        let scriptFile = tmpDir.appendingPathComponent("tm-ssh-\(id).command")
        let scriptContent = "#!/bin/bash\n\(sshCommand)\n"
        try? scriptContent.write(to: scriptFile, atomically: true, encoding: .utf8)
        try? FileManager.default.setAttributes(
            [.posixPermissions: 0o755], ofItemAtPath: scriptFile.path)
        NSWorkspace.shared.open(scriptFile)
    }

    // MARK: - CRUD operations

    func deleteTunnel(id: String) {
        core?.deleteTunnel(id: id)
        refreshTunnels()
    }

    func reorderTunnels(ids: [String]) {
        core?.reorderTunnels(ids: ids)
        refreshTunnels()
    }

    func getTunnelConfig(id: String) -> TunnelConfig? {
        return core?.getTunnelConfig(id: id)
    }

    func addTunnel(config: TunnelConfig) {
        core?.addTunnel(config: config)
        refreshTunnels()
        currentView = .editList
    }

    func updateTunnel(id: String, config: TunnelConfig) {
        core?.updateTunnel(id: id, config: config)
        refreshTunnels()
        currentView = .editList
    }

    // MARK: - Dialog actions

    func submitPassphrase(_ passphrase: String, tunnelId: String) {
        core?.submitPassphrase(id: tunnelId, passphrase: passphrase)
        activeDialog = nil
    }

    func submitPassword(_ password: String, tunnelId: String) {
        core?.submitPassword(id: tunnelId, password: password)
        activeDialog = nil
    }

    func acceptHostKey(host: String, port: UInt16) {
        // Save tunnelId before clearing dialog
        var tunnelIdToReconnect: String?
        if case .hostKey(let tid, _, _, _, _) = activeDialog {
            tunnelIdToReconnect = tid
        }
        core?.acceptHostKey(host: host, port: port)
        activeDialog = nil
        if let tid = tunnelIdToReconnect {
            core?.connect(id: tid)
        }
    }

    func respondKeyboardInteractive(_ responses: [String], tunnelId: String) {
        core?.respondKeyboardInteractive(id: tunnelId, responses: responses)
        activeDialog = nil
    }

    func cancelDialog() {
        if case .keyboardInteractive(let tid, _, _, _) = activeDialog {
            core?.cancelAuth(id: tid)
        }
        activeDialog = nil
    }

    // MARK: - TunnelEventHandler (called by Rust on background thread)

    func onTunnelStateChanged(id: String, status: TunnelStatus, errorMessage: String?) {
        tmLog("[TM] onTunnelStateChanged id=\(id) status=\(status) error=\(errorMessage ?? "nil")")
        Task { @MainActor in
            if let idx = self.tunnels.firstIndex(where: { $0.id == id }) {
                let old = self.tunnels[idx]
                let resolvedError: String?
                if status == .disconnected && errorMessage == nil && old.errorMessage != nil {
                    resolvedError = old.errorMessage
                } else if status == .connected {
                    resolvedError = nil
                } else if status == .connecting && errorMessage == nil {
                    resolvedError = nil
                } else {
                    resolvedError = errorMessage
                }
                self.updateTunnelAt(idx, status: status, errorMessage: resolvedError)
            } else {
                self.refreshTunnels()
            }
        }
    }

    func onPassphraseRequested(id: String, keyPath: String) {
        Task { @MainActor in
            self.activeDialog = .passphrase(tunnelId: id, keyPath: keyPath)
        }
    }

    func onPasswordRequested(id: String) {
        Task { @MainActor in
            self.activeDialog = .password(tunnelId: id)
        }
    }

    func onHostKeyVerification(id: String, host: String, port: UInt16, keyType: String, fingerprint: String) {
        Task { @MainActor in
            self.activeDialog = .hostKey(
                tunnelId: id, host: host, port: port, keyType: keyType, fingerprint: fingerprint
            )
        }
    }

    func onKeyboardInteractive(id: String, name: String, instructions: String, prompts: [KiPromptEntry]) {
        Task { @MainActor in
            self.activeDialog = .keyboardInteractive(
                tunnelId: id, name: name, instructions: instructions, prompts: prompts
            )
        }
    }

    func onTrafficUpdate(id: String, sample: TrafficSample) {
        Task { @MainActor in
            var history = self.trafficHistory[id] ?? []
            history.append(sample)
            if history.count > 60 { history.removeFirst(history.count - 60) }
            self.trafficHistory[id] = history
        }
    }

    func onError(id: String, message: String) {
        tmLog("[TM] onError id=\(id) message=\(message)")
        Task { @MainActor in
            if let idx = self.tunnels.firstIndex(where: { $0.id == id }) {
                self.updateTunnelAt(idx, status: .error, errorMessage: message)
            }
        }
    }

    // MARK: - Private helpers

    /// Update a tunnel's status and error in-place, preserving all other fields.
    private func updateTunnelAt(_ idx: Int, status: TunnelStatus, errorMessage: String?) {
        let old = tunnels[idx]
        tunnels[idx] = TunnelInfo(
            id: old.id, name: old.name, status: status,
            localPort: old.localPort, remoteHost: old.remoteHost,
            remotePort: old.remotePort, errorMessage: errorMessage,
            authMethod: old.authMethod, jumpHostName: old.jumpHostName,
            showTrafficChart: old.showTrafficChart, group: old.group
        )
    }
}
