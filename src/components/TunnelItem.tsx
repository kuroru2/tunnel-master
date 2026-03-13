import { useRef, useState, useEffect } from "react";
import type { TunnelInfo, TunnelStatus } from "../types";

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
  const isBusy = tunnel.status === "connecting" || tunnel.status === "disconnecting";

  // -- Toggle feedback state --
  const prevStatusRef = useRef<TunnelStatus>(tunnel.status);
  const connectStartRef = useRef<number>(0);
  const failTimerRef = useRef<ReturnType<typeof setTimeout>>();
  const minVisTimerRef = useRef<ReturnType<typeof setTimeout>>();
  const [recentlyFailed, setRecentlyFailed] = useState(false);
  const [showConnecting, setShowConnecting] = useState(false);

  useEffect(() => {
    const prev = prevStatusRef.current;
    const curr = tunnel.status;

    // Entering connecting state — record timestamp
    if (curr === "connecting" && prev !== "connecting") {
      connectStartRef.current = Date.now();
      setShowConnecting(true);
    }

    // Leaving connecting state
    if (prev === "connecting" && curr !== "connecting") {
      const elapsed = Date.now() - connectStartRef.current;
      const remaining = Math.max(0, 400 - elapsed);

      if (curr === "error" || curr === "disconnected") {
        // Failure: show connecting for remaining min-visible, then trigger fail animation
        if (remaining > 0) {
          minVisTimerRef.current = setTimeout(() => {
            setShowConnecting(false);
            setRecentlyFailed(true);
            failTimerRef.current = setTimeout(() => setRecentlyFailed(false), 600);
          }, remaining);
        } else {
          setShowConnecting(false);
          setRecentlyFailed(true);
          failTimerRef.current = setTimeout(() => setRecentlyFailed(false), 600);
        }
      } else {
        // Success or other transition: just clear after min-visible
        if (remaining > 0) {
          minVisTimerRef.current = setTimeout(() => setShowConnecting(false), remaining);
        } else {
          setShowConnecting(false);
        }
      }
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
    if (isBusy || visuallyBusy || recentlyFailed) return;
    if (isConnected) {
      onDisconnect(tunnel.id);
    } else {
      onConnect(tunnel.id);
    }
  };

  return (
    <div className="flex items-center justify-between px-3 py-2.5 hover:bg-[rgba(0,0,0,0.03)] dark:hover:bg-[rgba(255,255,255,0.04)] rounded-lg transition-colors">
      <div className="flex items-center gap-3 min-w-0">
        <div
          className={`w-1.5 h-1.5 rounded-full shrink-0 ${STATUS_DOT[tunnel.status]}`}
          style={isConnected ? { boxShadow: "var(--glow-green)" } : undefined}
        />
        <div className="min-w-0">
          <div className="text-sm font-medium truncate">{tunnel.name}</div>
          <div className="text-xs text-[#999] dark:text-[#555] truncate" style={{ fontFamily: "var(--font-mono)" }}>
            :{tunnel.localPort} &rarr; {tunnel.remoteHost}:{tunnel.remotePort}
          </div>
          {tunnel.jumpHostName && (
            <div className="text-xs text-[#999] dark:text-[#555] truncate">
              via {tunnel.jumpHostName}
            </div>
          )}
        </div>
      </div>

      {tunnel.errorMessage && (
        <div className="shrink-0 ml-auto text-[#ef4444]" title={tunnel.errorMessage}>
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-3.5 h-3.5">
            <path fillRule="evenodd" d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.168 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495zM10 6a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 6zm0 9a1 1 0 100-2 1 1 0 000 2z" clipRule="evenodd"/>
          </svg>
        </div>
      )}

      <button
        onClick={handleToggle}
        disabled={visuallyBusy || recentlyFailed}
        className={`shrink-0 ml-3 w-7 h-4 rounded-full relative transition-colors ${
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
