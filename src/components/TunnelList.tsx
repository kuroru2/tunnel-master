import type { TunnelInfo } from "../types";
import { TunnelItem } from "./TunnelItem";

interface TunnelListProps {
  tunnels: TunnelInfo[];
  onConnect: (id: string) => void;
  onDisconnect: (id: string) => void;
}

export function TunnelList({ tunnels, onConnect, onDisconnect }: TunnelListProps) {
  if (tunnels.length === 0) {
    return (
      <div className="py-8 text-center">
        <p className="text-gray-400 text-sm">No tunnels configured</p>
        <p className="text-gray-500 text-xs mt-1">
          Edit ~/.tunnel-master/config.json to add tunnels
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-0.5">
      {tunnels.map((tunnel) => (
        <TunnelItem
          key={tunnel.id}
          tunnel={tunnel}
          onConnect={onConnect}
          onDisconnect={onDisconnect}
        />
      ))}
    </div>
  );
}
