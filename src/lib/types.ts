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
  pending_forms: OpenCodeForm[];
  response_notices: ResponseNotice[];
}

export interface FormId {
  id: string;
  session_id: string;
}

export interface OpenCodeForm extends FormId {
  title: string;
  fields: FormField[];
}

export type FormScalar = string | boolean | number;
export type FormValue = FormScalar | string[];
export type FormAnswer = Record<string, FormValue>;

export interface FormOption {
  value: string;
  label: string;
  description?: string;
}

export interface FormField {
  key: string;
  type: string;
  title?: string;
  description?: string;
  required?: boolean;
  hidden?: boolean;
  when?: { key: string; op: "eq" | "neq"; value: FormScalar }[];
  default?: FormValue;
  options?: FormOption[];
  custom?: boolean;
  placeholder?: string;
  minItems?: number;
  maxItems?: number;
  minLength?: number;
  maxLength?: number;
  minimum?: number | string;
  maximum?: number | string;
}

export type ResponseOutcome = "ready" | "failed" | "interrupted";

export interface ResponseNotice {
  id: number;
  outcome: ResponseOutcome;
}

export type Chime = "request" | "completion" | "approve" | "reject";

export type Decision = "allow" | "deny" | "always" | "approve" | "reject" | "always_approve";
export type AutoApproval = "manual" | "allow_once" | "always_allow";

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
    auto_approval: AutoApproval;
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
