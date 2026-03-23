import SwiftUI

struct TunnelRow: View {
    let tunnel: TunnelInfo
    let samples: [TrafficSample]
    let onToggle: () -> Void
    let onOpenTerminal: () -> Void

    var body: some View {
        ZStack {
            if tunnel.status == .connected && tunnel.showTrafficChart {
                TrafficSparkline(samples: samples)
            }

            HStack(spacing: 8) {
                Circle()
                    .fill(statusColor)
                    .frame(width: 8, height: 8)

                VStack(alignment: .leading, spacing: 2) {
                    Text(tunnel.name)
                        .font(.body)
                        .lineLimit(1)
                    Text("localhost:\(tunnel.localPort) → \(tunnel.remoteHost):\(tunnel.remotePort)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                    if let msg = tunnel.errorMessage, !msg.isEmpty {
                        HStack(spacing: 4) {
                            let iconName = tunnel.status == .connecting
                                ? "arrow.trianglehead.2.counterclockwise"
                                : "exclamationmark.triangle.fill"
                            Image(systemName: iconName)
                                .font(.caption2)
                                .foregroundStyle(tunnel.status == .connecting ? .yellow : .orange)
                            Text(msg)
                                .font(.caption2)
                                .foregroundColor(tunnel.status == .connecting ? .secondary : .red)
                                .lineLimit(2)
                        }
                    }
                }

                Spacer()

                Button {
                    onOpenTerminal()
                } label: {
                    Image(systemName: "terminal")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
                .help("Open SSH terminal")

                // Fixed-width container for toggle/spinner to prevent layout jitter
                ZStack {
                    if tunnel.status == .connecting || tunnel.status == .disconnecting {
                        ProgressView()
                            .controlSize(.small)
                    } else {
                        Toggle("", isOn: Binding(
                            get: { tunnel.status == .connected },
                            set: { _ in onToggle() }
                        ))
                        .toggleStyle(.switch)
                        .controlSize(.small)
                        .labelsHidden()
                    }
                }
                .frame(width: 36)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
        }
        .contentShape(Rectangle())
    }

    private var statusColor: Color {
        switch tunnel.status {
        case .connected: return .green
        case .connecting, .disconnecting: return .yellow
        case .error: return .red
        case .disconnected:
            if tunnel.errorMessage != nil && !tunnel.errorMessage!.isEmpty {
                return .red
            }
            return .gray
        }
    }
}
