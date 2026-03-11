export type TunnelStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "error"
  | "disconnecting";

export interface TunnelInfo {
  id: string;
  name: string;
  status: TunnelStatus;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  errorMessage: string | null;
}

export interface TunnelStatusEvent {
  id: string;
  status: TunnelStatus;
  timestamp: number;
}

export interface TunnelErrorEvent {
  id: string;
  message: string;
  code: string;
}

export interface TunnelInput {
  name: string;
  host: string;
  port: number;
  user: string;
  keyPath: string;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  autoConnect: boolean;
}

export interface TunnelConfig {
  id: string;
  name: string;
  host: string;
  port: number;
  user: string;
  keyPath: string;
  type: "local" | "reverse" | "dynamic";
  localPort: number;
  remoteHost: string;
  remotePort: number;
  autoConnect: boolean;
}
