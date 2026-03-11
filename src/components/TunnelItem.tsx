import type { TunnelInfo, TunnelStatus } from "../types";

interface TunnelItemProps {
  tunnel: TunnelInfo;
  onConnect: (id: string) => void;
  onDisconnect: (id: string) => void;
}

const STATUS_COLORS: Record<TunnelStatus, string> = {
  disconnected: "bg-gray-500",
  connecting: "bg-yellow-500 animate-pulse",
  connected: "bg-green-500",
  error: "bg-red-500",
  disconnecting: "bg-yellow-500",
};

const STATUS_LABELS: Record<TunnelStatus, string> = {
  disconnected: "Disconnected",
  connecting: "Connecting...",
  connected: "Connected",
  error: "Error",
  disconnecting: "Disconnecting...",
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
    <div className="flex items-center justify-between px-3 py-2.5 hover:bg-white/5 rounded-lg transition-colors">
      <div className="flex items-center gap-3 min-w-0">
        <div className={`w-2 h-2 rounded-full shrink-0 ${STATUS_COLORS[tunnel.status]}`} />
        <div className="min-w-0">
          <div className="text-sm font-medium text-white truncate">{tunnel.name}</div>
          <div className="text-xs text-gray-400 truncate">
            :{tunnel.localPort} &rarr; {tunnel.remoteHost}:{tunnel.remotePort}
          </div>
          {tunnel.errorMessage && (
            <div className="text-xs text-red-400 truncate mt-0.5">{tunnel.errorMessage}</div>
          )}
        </div>
      </div>

      <button
        onClick={handleToggle}
        disabled={isBusy}
        className={`
          shrink-0 ml-3 px-3 py-1 rounded-md text-xs font-medium transition-colors
          ${isBusy ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}
          ${isConnected
            ? "bg-red-500/20 text-red-400 hover:bg-red-500/30"
            : "bg-green-500/20 text-green-400 hover:bg-green-500/30"
          }
        `}
      >
        {isBusy
          ? STATUS_LABELS[tunnel.status]
          : isConnected
          ? "Stop"
          : "Start"}
      </button>
    </div>
  );
}
