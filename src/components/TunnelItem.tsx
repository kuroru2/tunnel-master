import { useRef, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TunnelInfo, TunnelStatus, TrafficSample, TrafficEvent } from "../types";
import { TrafficSparkline } from "./TrafficSparkline";

interface TunnelItemProps {
  tunnel: TunnelInfo;
  onConnect: (id: string) => void;
  onDisconnect: (id: string) => void;
}

const STATUS_DOT: Record<TunnelStatus, string> = {
  disconnected: "bg-[#d4d4d4] dark:bg-[#333]",
  connecting: "bg-[#fbbf24] animate-pulse",
  connected: "bg-[#4ade80]",
  error: "bg-[#ef4444]",
  disconnecting: "bg-[#fbbf24]",
};

export function TunnelItem({ tunnel, onConnect, onDisconnect }: TunnelItemProps) {
  const isConnected = tunnel.status === "connected";

  // -- Toggle feedback state --
  const prevStatusRef = useRef<TunnelStatus>(tunnel.status);
  const connectStartRef = useRef<number>(0);
  const failTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);
  const minVisTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);
  const [recentlyFailed, setRecentlyFailed] = useState(false);
  const [showConnecting, setShowConnecting] = useState(false);
  const [errorExpanded, setErrorExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    const prev = prevStatusRef.current;
    const curr = tunnel.status;

    // Clear any pending timers before scheduling new ones
    clearTimeout(failTimerRef.current);
    clearTimeout(minVisTimerRef.current);

    // Entering connecting state — record timestamp
    // showConnecting extends the visual beyond the actual "connecting" status;
    // while status === "connecting", visuallyConnecting is already true via the prop check
    if (curr === "connecting" && prev !== "connecting") {
      connectStartRef.current = Date.now();
      minVisTimerRef.current = setTimeout(() => setShowConnecting(true), 0);
    }

    // Leaving connecting state — schedule visual transitions via timers
    // (all setState calls are async via setTimeout to avoid synchronous cascading renders)
    if (prev === "connecting" && curr !== "connecting") {
      const elapsed = Date.now() - connectStartRef.current;
      const delay = Math.max(0, 400 - elapsed);
      const isFail = curr === "error" || curr === "disconnected";

      minVisTimerRef.current = setTimeout(() => {
        setShowConnecting(false);
        if (isFail) {
          setRecentlyFailed(true);
          failTimerRef.current = setTimeout(() => setRecentlyFailed(false), 600);
        }
      }, delay);
    }

    prevStatusRef.current = curr;
  }, [tunnel.status]);

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      clearTimeout(failTimerRef.current);
      clearTimeout(minVisTimerRef.current);
    };
  }, []);

  const visuallyConnecting = showConnecting || tunnel.status === "connecting";
  const visuallyBusy = visuallyConnecting || tunnel.status === "disconnecting";

  const handleToggle = () => {
    if (visuallyBusy || recentlyFailed) return;
    if (isConnected) {
      onDisconnect(tunnel.id);
    } else {
      onConnect(tunnel.id);
    }
  };

  // Reset error expand state when the error message changes
  useEffect(() => {
    setErrorExpanded(false);
    setCopied(false);
  }, [tunnel.errorMessage]);

  const handleCopy = () => {
    if (!tunnel.errorMessage || !navigator.clipboard) return;
    navigator.clipboard.writeText(tunnel.errorMessage).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }).catch(() => {});
  };

  // -- Traffic monitoring --
  const [trafficSamples, setTrafficSamples] = useState<TrafficSample[]>([]);
  const showChart = tunnel.showTrafficChart;

  // Fetch history when tunnel is connected and chart is enabled
  useEffect(() => {
    if (tunnel.status !== "connected" || !showChart) {
      setTimeout(() => setTrafficSamples([]), 0);
      return;
    }
    invoke<TrafficSample[]>("get_traffic_history", { id: tunnel.id })
      .then(setTrafficSamples)
      .catch(() => {}); // ignore errors
  }, [tunnel.id, tunnel.status, showChart]);

  // Listen for real-time traffic events
  useEffect(() => {
    if (tunnel.status !== "connected" || !showChart) return;

    const unlisten = listen<TrafficEvent>("tunnel-traffic", (event) => {
      if (event.payload.id !== tunnel.id) return;
      const sample: TrafficSample = {
        bytesIn: event.payload.bytesIn,
        bytesOut: event.payload.bytesOut,
        timestamp: Date.now(),
      };
      setTrafficSamples((prev) => {
        const next = [...prev, sample];
        return next.length > 60 ? next.slice(-60) : next;
      });
    });

    return () => { unlisten.then((fn) => fn()); };
  }, [tunnel.id, tunnel.status, showChart]);

  // Format bytes/s for display
  const formatRate = (bytes: number): string => {
    if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB/s`;
    if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB/s`;
    if (bytes >= 1_000) return `${(bytes / 1_000).toFixed(1)} KB/s`;
    return `${bytes} B/s`;
  };

  const lastSample = trafficSamples.length > 0 ? trafficSamples[trafficSamples.length - 1] : null;
  const isIdle = lastSample ? lastSample.bytesIn + lastSample.bytesOut < 10 : true;
  const showTraffic = showChart && tunnel.status === "connected" && trafficSamples.length > 0;

  return (
    <div className="relative overflow-hidden flex items-center justify-between px-3 py-2.5 hover:bg-[rgba(0,0,0,0.03)] dark:hover:bg-[rgba(255,255,255,0.04)] rounded-lg transition-colors">
      {showTraffic && <TrafficSparkline samples={trafficSamples} />}
      <div className="flex items-center gap-3 min-w-0 z-10">
        <div
          className={`w-1.5 h-1.5 rounded-full shrink-0 ${STATUS_DOT[tunnel.status]}`}
          style={isConnected ? { boxShadow: "var(--glow-green)" } : undefined}
        />
        <div className="min-w-0">
          <div className="text-sm font-medium truncate">{tunnel.name}</div>
          {tunnel.errorMessage ? (
            <div
              className={`text-xs text-[#dc2626] dark:text-[#f87171] cursor-pointer ${
                errorExpanded ? "leading-relaxed [overflow-wrap:anywhere] select-text" : "truncate"
              }`}
              style={{ fontFamily: "var(--font-mono)" }}
              role="button"
              aria-expanded={errorExpanded}
              onClick={() => setErrorExpanded(!errorExpanded)}
            >
              {tunnel.errorMessage}
              {errorExpanded && (
                <div className="mt-1.5 select-none">
                  <button
                    className="text-[10px] px-1.5 py-0.5 border border-current rounded opacity-40 hover:opacity-70"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleCopy();
                    }}
                  >
                    {copied ? "Copied!" : "Copy"}
                  </button>
                </div>
              )}
            </div>
          ) : (
            <>
              <div className="text-xs text-[#999] dark:text-[#555] truncate" style={{ fontFamily: "var(--font-mono)" }}>
                :{tunnel.localPort} &rarr; {tunnel.remoteHost}:{tunnel.remotePort}
              </div>
              {tunnel.jumpHostName && (
                <div className="text-xs text-[#999] dark:text-[#555] truncate">
                  via {tunnel.jumpHostName}
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {showTraffic && (
        <div className="shrink-0 ml-auto text-right z-10" style={{ fontFamily: "var(--font-mono)" }}>
          {isIdle ? (
            <div className="text-[10px] text-[#6b7280]">idle</div>
          ) : (
            <>
              <div className="text-[10px] text-[#4ade80]">↓ {formatRate(lastSample!.bytesIn)}</div>
              <div className="text-[10px] text-[#60a5fa]">↑ {formatRate(lastSample!.bytesOut)}</div>
            </>
          )}
        </div>
      )}

      <button
        onClick={handleToggle}
        disabled={visuallyBusy || recentlyFailed}
        className={`shrink-0 ml-3 w-7 h-4 rounded-full relative transition-colors z-10 ${
          visuallyBusy || recentlyFailed ? "cursor-not-allowed" : "cursor-pointer"
        } ${
          recentlyFailed
            ? ""
            : isConnected || tunnel.status === "disconnecting"
            ? "bg-[#4ade80]"
            : visuallyConnecting
            ? "bg-[#fbbf24]"
            : "bg-[#ccc] dark:bg-[#333]"
        }`}
        style={recentlyFailed ? { animation: "flash-red 0.6s ease-out forwards, shake 0.4s ease-in-out" } : undefined}
        title={isConnected ? "Disconnect" : "Connect"}
      >
        <div
          className={`w-3 h-3 rounded-full absolute top-[2px] transition-transform ${
            isConnected || tunnel.status === "disconnecting"
              ? "translate-x-[14px] bg-white"
              : visuallyConnecting
              ? "translate-x-[14px] bg-white animate-pulse"
              : "translate-x-[2px] bg-white dark:bg-[#888]"
          }`}
        />
      </button>
    </div>
  );
}
