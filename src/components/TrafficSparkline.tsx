import type { TrafficSample } from "../types";

interface TrafficSparklineProps {
  samples: TrafficSample[];
}

export function TrafficSparkline({ samples }: TrafficSparklineProps) {
  if (samples.length < 2) return null;

  const maxPoints = 60;
  const viewWidth = 320;
  const viewHeight = 56;

  // Find max value for Y scaling (minimum floor of 100 to avoid jitter)
  const maxVal = Math.max(
    100,
    ...samples.map((s) => Math.max(s.bytesIn, s.bytesOut))
  );

  const toPoints = (getValue: (s: TrafficSample) => number): string => {
    const step = viewWidth / (maxPoints - 1);
    const startIndex = Math.max(0, maxPoints - samples.length);
    return samples
      .map((s, i) => {
        const x = (startIndex + i) * step;
        const y = viewHeight - (getValue(s) / maxVal) * (viewHeight - 4);
        return `${x},${y}`;
      })
      .join(" ");
  };

  const downloadPoints = toPoints((s) => s.bytesIn);
  const uploadPoints = toPoints((s) => s.bytesOut);

  return (
    <svg
      viewBox={`0 0 ${viewWidth} ${viewHeight}`}
      preserveAspectRatio="none"
      style={{
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        opacity: 0.25,
        pointerEvents: "none",
      }}
    >
      <polyline
        points={downloadPoints}
        fill="none"
        stroke="#4ade80"
        strokeWidth="1.5"
      />
      <polyline
        points={uploadPoints}
        fill="none"
        stroke="#60a5fa"
        strokeWidth="1"
        strokeDasharray="3,2"
      />
    </svg>
  );
}
