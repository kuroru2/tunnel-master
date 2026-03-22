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
                Text("Edit List — coming in Phase 3")
            case .editForm:
                Text("Edit Form — coming in Phase 3")
            }
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
