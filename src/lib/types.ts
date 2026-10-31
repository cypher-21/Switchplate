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
  };
  appearance: {
    position: "top-center" | "top-right" | "top-left" | "bottom-right" | "center" | string;
    card_width: number;
    card_height: number;
    collapsed_width: number;
    collapsed_height: number;
    offset_x: number;
    offset_y: number;
    theme: "ai-dark" | "ai-violet" | "ai-emerald" | "ai-cyber" | string;
    accent_glow: boolean;
  };
  behavior: {
    only_show_on_request: boolean;
    sound_enabled: boolean;
  };
  shortcuts: {
    accept: string;
    deny: string;
    always: string;
    toggle: string;
  };
}
