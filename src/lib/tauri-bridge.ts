import { invoke } from "@tauri-apps/api/core";
import type { NotchState, Decision, Config } from "./types";

export async function getState(): Promise<NotchState> {
  return await invoke<NotchState>("get_state");
}

export async function respondPermission(
  requestId: string,
  decision: Decision
): Promise<void> {
  await invoke("respond_permission", {
    requestId,
    decision,
  });
}

export async function getConfig(): Promise<Config> {
  return await invoke<Config>("get_config");
}

export async function saveConfig(config: Config): Promise<void> {
  await invoke("save_config", {
    newConfig: config,
  });
}

export async function openSettingsWindow(): Promise<void> {
  await invoke("open_settings_window");
}

export async function closeSettingsWindow(): Promise<void> {
  await invoke("close_settings_window");
}

export async function toggleIsland(): Promise<void> {
  await invoke("toggle_island");
}

export async function toggleMonitoring(): Promise<boolean> {
  return await invoke<boolean>("toggle_monitoring");
}
