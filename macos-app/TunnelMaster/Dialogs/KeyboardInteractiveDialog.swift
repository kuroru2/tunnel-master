import SwiftUI

struct KeyboardInteractiveDialog: View {
    let tunnelId: String
    let name: String
    let instructions: String
    let prompts: [KiPromptEntry]
    let onSubmit: ([String], String) -> Void
    let onCancel: () -> Void
    @State private var responses: [String]

    init(tunnelId: String, name: String, instructions: String, prompts: [KiPromptEntry],
         onSubmit: @escaping ([String], String) -> Void, onCancel: @escaping () -> Void) {
        self.tunnelId = tunnelId
        self.name = name
        self.instructions = instructions
        self.prompts = prompts
        self.onSubmit = onSubmit
        self.onCancel = onCancel
        self._responses = State(initialValue: Array(repeating: "", count: prompts.count))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(name.isEmpty ? "Authentication Required" : name).font(.headline)
            if !instructions.isEmpty {
                Text(instructions).font(.caption).foregroundStyle(.secondary)
            }
            ForEach(Array(prompts.enumerated()), id: \.offset) { index, prompt in
                VStack(alignment: .leading, spacing: 4) {
                    Text(prompt.text).font(.caption).foregroundStyle(.secondary)
                    if prompt.echo {
                        TextField("", text: $responses[index]).textFieldStyle(.roundedBorder)
                    } else {
                        SecureField("", text: $responses[index]).textFieldStyle(.roundedBorder)
                    }
                }
            }
            HStack {
                Spacer()
                Button("Cancel") { onCancel() }.keyboardShortcut(.cancelAction)
                Button("Submit") { onSubmit(responses, tunnelId) }.keyboardShortcut(.defaultAction)
            }
        }.padding().frame(width: 300)
    }
}
