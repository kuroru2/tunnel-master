import SwiftUI

struct TunnelRow: View {
    let tunnel: TunnelInfo
    let samples: [TrafficSample]
    let onToggle: () -> Void

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
                    if tunnel.status == .error, let msg = tunnel.errorMessage {
                        HStack(spacing: 4) {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .font(.caption2)
                                .foregroundStyle(.orange)
                            Text(msg)
                                .font(.caption2)
                                .foregroundStyle(.red)
                                .lineLimit(2)
                        }
                    }
                }

                Spacer()

                if tunnel.status == .connecting || tunnel.status == .disconnecting {
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 24)
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
        case .disconnected: return .gray
        }
    }
}
