export type TunnelStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "error"
  | "disconnecting";

export type AuthMethod = "key" | "password" | "agent" | "keyboard-interactive";

export interface TunnelInfo {
  id: string;
  name: string;
  status: TunnelStatus;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  errorMessage: string | null;
  authMethod: AuthMethod;
  jumpHostName: string | null;
  showTrafficChart: boolean;
}

export interface TunnelStatusEvent {
  id: string;
  status: TunnelStatus;
  timestamp: number;
}

export interface TunnelInput {
  name: string;
  host: string;
  port: number;
  user: string;
  keyPath: string;
  authMethod: AuthMethod;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  autoConnect: boolean;
  jumpHost: string | null;
  showTrafficChart: boolean;
}

export interface TunnelConfig {
  id: string;
  name: string;
  host: string;
  port: number;
  user: string;
  keyPath: string;
  authMethod: AuthMethod;
  type: "local" | "reverse" | "dynamic";
  localPort: number;
  remoteHost: string;
  remotePort: number;
  autoConnect: boolean;
  jumpHost: string | null;
  showTrafficChart: boolean;
}

export interface TrafficSample {
  bytesIn: number;
  bytesOut: number;
  timestamp: number;
}

export interface TrafficEvent {
  id: string;
  bytesIn: number;
  bytesOut: number;
}