export type DeviceKind = "mouse" | "keyboard" | "headset" | "mousepad" | "other";
export type ConnectionStatus = "online" | "offline" | "error";

export interface DeviceInfo {
  id: string;
  name: string;
  vendor: string;
  kind: DeviceKind;
  vendor_id: number;
  product_id: number;
  path?: string | null;
}

export interface DeviceStatus {
  id: string;
  status: ConnectionStatus;
  battery?: number | null;
  details: Record<string, unknown>;
}

export type Route = "home" | "engine" | "sonar";
