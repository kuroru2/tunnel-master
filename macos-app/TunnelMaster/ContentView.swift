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
                VStack {
                    HStack {
                        Button {
                            viewModel.currentView = .list
                        } label: {
                            Image(systemName: "chevron.left")
                            Text("Back")
                        }
                        .buttonStyle(.plain)
                        Spacer()
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    Spacer()
                    Text("Edit List — coming in Phase 3")
                        .foregroundStyle(.secondary)
                    Spacer()
                }
            case .editForm:
                VStack {
                    HStack {
                        Button {
                            viewModel.currentView = .editList
                        } label: {
                            Image(systemName: "chevron.left")
                            Text("Back")
                        }
                        .buttonStyle(.plain)
                        Spacer()
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    Spacer()
                    Text("Edit Form — coming in Phase 3")
                        .foregroundStyle(.secondary)
                    Spacer()
                }
            }
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
