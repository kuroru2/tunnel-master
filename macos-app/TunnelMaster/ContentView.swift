import SwiftUI

enum ViewMode {
    case list
    case editList
    case editForm(tunnelId: String?)
}

struct ContentView: View {
    @Bindable var viewModel: TunnelViewModel

    var body: some View {
        VStack(spacing: 0) {
            switch viewModel.currentView {
            case .list:
                TunnelListView(viewModel: viewModel)
            case .editList:
                EditListView(viewModel: viewModel)
            case .editForm(let tunnelId):
                EditFormView(viewModel: viewModel, tunnelId: tunnelId)
            }
        }
        .background(Color(nsColor: .windowBackgroundColor))
        .sheet(item: $viewModel.activeDialog) { dialog in
            switch dialog {
            case .passphrase(let tid, let keyPath):
                PassphraseDialog(tunnelId: tid, keyPath: keyPath,
                                onSubmit: viewModel.submitPassphrase, onCancel: viewModel.cancelDialog)
            case .password(let tid):
                PasswordDialog(tunnelId: tid,
                              onSubmit: viewModel.submitPassword, onCancel: viewModel.cancelDialog)
            case .hostKey(let tid, let host, let port, let keyType, let fingerprint):
                HostKeyDialog(tunnelId: tid, host: host, port: port, keyType: keyType, fingerprint: fingerprint,
                             onAccept: viewModel.acceptHostKey, onCancel: viewModel.cancelDialog)
            case .keyboardInteractive(let tid, let name, let instructions, let prompts):
                KeyboardInteractiveDialog(tunnelId: tid, name: name, instructions: instructions, prompts: prompts,
                                         onSubmit: viewModel.respondKeyboardInteractive, onCancel: viewModel.cancelDialog)
            }
        }
    }
}
