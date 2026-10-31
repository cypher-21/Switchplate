export type ConnectionStatus =
  | { status: "disconnected" }
  | { status: "connecting" }
  | { status: "connected"; session_count: number };

export interface PermissionRequest {
  id: string;
  tool: string;
  args: unknown;
  session_id: string;
  timestamp: string;
}

export interface NotchState {
  connection: ConnectionStatus;
  pending_requests: PermissionRequest[];
}

export type Decision = "allow" | "deny" | "always" | "approve" | "reject" | "always_approve";

export interface Config {
  opencode: {
    server_url: string;
    password_env: string;
    request_timeout: number;
    auto_reconnect: boolean;
  };
  appearance: {
    position: string;
    card_width: number;
    card_height: number;
    collapsed_width: number;
    collapsed_height: number;
    offset_x: number;
    offset_y: number;
    theme: string;
    reduced_motion: boolean;
  };
  behavior: {
    only_show_on_request: boolean;
    pause_monitoring: boolean;
    sound_enabled: boolean;
    autostart: boolean;
  };
  shortcuts: {
    open_island: string;
    accept: string;
    deny: string;
    always: string;
    open_settings: string;
  };
}
