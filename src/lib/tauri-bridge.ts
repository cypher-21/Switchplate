import { invoke } from "@tauri-apps/api/core";
import type { NotchState, Decision, Config } from "./types";

export async function getState(): Promise<NotchState> {
  return invoke("get_state");
}

export async function respondPermission(
  requestId: string,
  decision: Decision
): Promise<void> {
  return invoke("respond_permission", { requestId, decision });
}

export async function getConfig(): Promise<Config> {
  return invoke("get_config");
}

export async function saveConfig(newConfig: Config): Promise<void> {
  return invoke("save_config", { newConfig });
}
