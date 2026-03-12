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

  const handleToggle = () => {
    if (isBusy) return;
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
          {tunnel.errorMessage && (
            <div className="text-xs text-[#dc2626] dark:text-[#f87171] truncate mt-0.5" style={{ fontFamily: "var(--font-mono)" }}>
              {tunnel.errorMessage}
            </div>
          )}
        </div>
      </div>

      {isBusy ? (
        <span className="shrink-0 ml-3 text-xs text-[#fbbf24]">
          {tunnel.status === "connecting" ? "connecting" : "disconnecting"}
        </span>
      ) : (
        <button
          onClick={handleToggle}
          className="shrink-0 ml-3 p-1 cursor-pointer"
          title={isConnected ? "Disconnect" : "Connect"}
        >
          <div
            className={`w-2 h-2 rounded-full ${
              isConnected ? "bg-[#f87171]" : "bg-[#4ade80]"
            }`}
          />
        </button>
      )}
    </div>
  );
}
