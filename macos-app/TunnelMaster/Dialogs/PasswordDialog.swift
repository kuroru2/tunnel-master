import SwiftUI

struct PasswordDialog: View {
    let tunnelId: String
    let onSubmit: (String, String) -> Void
    let onCancel: () -> Void
    @State private var password = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Password Required").font(.headline)
            Text("Enter the SSH password. It will be stored securely.")
                .font(.caption).foregroundStyle(.secondary)
            SecureField("SSH password", text: $password)
                .textFieldStyle(.roundedBorder)
                .onSubmit { submit() }
            HStack {
                Spacer()
                Button("Cancel") { onCancel() }.keyboardShortcut(.cancelAction)
                Button("Connect") { submit() }.keyboardShortcut(.defaultAction).disabled(password.isEmpty)
            }
        }.padding().frame(width: 300)
    }

    private func submit() {
        guard !password.isEmpty else { return }
        onSubmit(password, tunnelId)
    }
}
