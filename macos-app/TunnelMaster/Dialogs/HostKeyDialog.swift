import SwiftUI

struct HostKeyDialog: View {
    let tunnelId: String
    let host: String
    let port: UInt16
    let keyType: String
    let fingerprint: String
    let onAccept: (String, UInt16) -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Unknown Host").font(.headline)
            Text("The authenticity of \(host):\(port) can't be established.")
                .font(.caption).foregroundStyle(.secondary)
            GroupBox {
                VStack(alignment: .leading, spacing: 4) {
                    Text("\(keyType) fingerprint:").font(.caption).foregroundStyle(.secondary)
                    Text("SHA256:\(fingerprint)")
                        .font(.system(.caption, design: .monospaced))
                        .textSelection(.enabled)
                }.frame(maxWidth: .infinity, alignment: .leading)
            }
            Text("Are you sure you want to continue connecting?")
                .font(.caption).foregroundStyle(.secondary)
            HStack {
                Spacer()
                Button("Cancel") { onCancel() }.keyboardShortcut(.cancelAction)
                Button("Trust & Connect") { onAccept(host, port) }.keyboardShortcut(.defaultAction)
            }
        }.padding().frame(width: 340)
    }
}
