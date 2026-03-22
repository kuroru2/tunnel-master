import SwiftUI

struct TrafficSparkline: View {
    let samples: [TrafficSample]
    private let maxPoints = 60

    var body: some View {
        if samples.count >= 2 {
            Canvas { context, size in
                let maxVal = max(UInt64(100), samples.map { max($0.bytesIn, $0.bytesOut) }.max() ?? 100)

                let downloadPath = buildPath(samples: samples, size: size, maxVal: maxVal) { $0.bytesIn }
                context.stroke(downloadPath, with: .color(.green.opacity(0.5)), lineWidth: 1.5)

                let uploadPath = buildPath(samples: samples, size: size, maxVal: maxVal) { $0.bytesOut }
                context.stroke(uploadPath, with: .color(.blue.opacity(0.4)),
                              style: StrokeStyle(lineWidth: 1, dash: [3, 2]))
            }
            .allowsHitTesting(false)
            .opacity(0.25)
        }
    }

    private func buildPath(samples: [TrafficSample], size: CGSize, maxVal: UInt64,
                          getValue: (TrafficSample) -> UInt64) -> Path {
        Path { path in
            let step = size.width / CGFloat(maxPoints - 1)
            let startIndex = max(0, maxPoints - samples.count)
            for (i, sample) in samples.enumerated() {
                let x = CGFloat(startIndex + i) * step
                let y = size.height - (CGFloat(getValue(sample)) / CGFloat(maxVal)) * (size.height - 4)
                if i == 0 { path.move(to: CGPoint(x: x, y: y)) }
                else { path.addLine(to: CGPoint(x: x, y: y)) }
            }
        }
    }
}
