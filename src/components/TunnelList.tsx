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
      <div className="py-12 text-center">
        <div className="text-3xl opacity-20 mb-2">⇌</div>
        <p className="text-[#999] dark:text-[#666] text-sm">No tunnels configured</p>
        <p className="text-[#bbb] dark:text-[#555] text-xs mt-1">
          Click ✎ to add your first tunnel
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
