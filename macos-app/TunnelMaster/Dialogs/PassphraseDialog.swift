import SwiftUI

struct PassphraseDialog: View {
    let tunnelId: String
    let keyPath: String
    let onSubmit: (String, String) -> Void
    let onCancel: () -> Void
    @State private var passphrase = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Passphrase Required").font(.headline)
            Text("Enter the passphrase for the SSH key. It will be stored securely.")
                .font(.caption).foregroundStyle(.secondary)
            SecureField("SSH key passphrase", text: $passphrase)
                .textFieldStyle(.roundedBorder)
                .onSubmit { submit() }
            HStack {
                Spacer()
                Button("Cancel") { onCancel() }.keyboardShortcut(.cancelAction)
                Button("Unlock") { submit() }.keyboardShortcut(.defaultAction).disabled(passphrase.isEmpty)
            }
        }.padding().frame(width: 300)
    }

    private func submit() {
        guard !passphrase.isEmpty else { return }
        onSubmit(passphrase, tunnelId)
    }
}
