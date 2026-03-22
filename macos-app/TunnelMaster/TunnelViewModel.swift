import Foundation
import Observation

@Observable
final class TunnelViewModel: TunnelEventHandler {
    var tunnels: [TunnelInfo] = []
    var currentView: ViewMode = .list
    var activeDialog: DialogState? = nil

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
        core = TunnelCore(eventHandler: self)
        refreshTunnels()
    }

    func shutdown() {
        core?.shutdown()
        core = nil
    }

    // MARK: - Public API

    func refreshTunnels() {
        guard let core else { return }
        let list = core.listTunnels()
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
        var tunnelIdToReconnect: String? = nil
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
        Task { @MainActor in
            if let idx = self.tunnels.firstIndex(where: { $0.id == id }) {
                let old = self.tunnels[idx]
                self.tunnels[idx] = TunnelInfo(
                    id: old.id, name: old.name, status: status,
                    localPort: old.localPort, remoteHost: old.remoteHost,
                    remotePort: old.remotePort, errorMessage: errorMessage,
                    authMethod: old.authMethod, jumpHostName: old.jumpHostName,
                    showTrafficChart: old.showTrafficChart
                )
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
            self.activeDialog = .hostKey(tunnelId: id, host: host, port: port, keyType: keyType, fingerprint: fingerprint)
        }
    }

    func onKeyboardInteractive(id: String, name: String, instructions: String, prompts: [KiPromptEntry]) {
        Task { @MainActor in
            self.activeDialog = .keyboardInteractive(tunnelId: id, name: name, instructions: instructions, prompts: prompts)
        }
    }

    func onTrafficUpdate(id: String, sample: TrafficSample) {
        // Phase 3: traffic sparklines
    }

    func onError(id: String, message: String) {
        Task { @MainActor in
            if let idx = self.tunnels.firstIndex(where: { $0.id == id }) {
                let old = self.tunnels[idx]
                self.tunnels[idx] = TunnelInfo(
                    id: old.id, name: old.name, status: .error,
                    localPort: old.localPort, remoteHost: old.remoteHost,
                    remotePort: old.remotePort, errorMessage: message,
                    authMethod: old.authMethod, jumpHostName: old.jumpHostName,
                    showTrafficChart: old.showTrafficChart
                )
            }
        }
    }
}
