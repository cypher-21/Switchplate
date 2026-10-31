use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::error;

use crate::config::Config;
use crate::opencode::client::OpenCodeClient;
use crate::opencode::events::Decision;
use crate::state::NotchState;
use tauri::{Emitter, Manager};

pub type AppState = Arc<Mutex<NotchState>>;
pub type ClientState = Arc<Mutex<Option<OpenCodeClient>>>;
pub type ConfigState = Arc<RwLock<Config>>;

/// IPC command: get current state.
#[tauri::command]
pub async fn get_state(state: tauri::State<'_, AppState>) -> Result<NotchState, String> {
    let guard = state.lock().await;
    Ok(guard.clone())
}

/// IPC command: respond to an approval request.
#[tauri::command]
pub async fn respond_permission(
    request_id: String,
    decision: String,
    state: tauri::State<'_, AppState>,
    client: tauri::State<'_, ClientState>,
    config: tauri::State<'_, ConfigState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let parsed_decision = Decision::from_str_loose(&decision)
        .ok_or_else(|| format!("Invalid decision '{}': expected allow, deny, or always", decision))?;

    let session_id = {
        let state_guard = state.lock().await;
        state_guard
            .pending_requests
            .iter()
            .find(|r| r.id == request_id)
            .map(|r| r.session_id.clone())
    };

    let client_guard = client.lock().await;
    let opencode_client = client_guard
        .as_ref()
        .ok_or_else(|| "Not connected to OpenCode server".to_string())?;

    opencode_client
        .respond_permission(&request_id, session_id.as_deref(), parsed_decision)
        .await?;

    drop(client_guard);

    // Remove the request from local state immediately
    let (has_pending, current_state) = {
        let mut state_guard = state.lock().await;
        if state_guard.remove_request(&request_id).is_none() {
            error!("Permission request {} was not found in pending queue", request_id);
        }
        (state_guard.pending_count() > 0, state_guard.clone())
    };

    // Emit state update immediately to frontend
    let _ = app_handle.emit("state-changed", &current_state);

    // Immediately update island window size/visibility
    let cfg_guard = config.read().await;
    if let Some(window) = app_handle.get_webview_window("island") {
        crate::window::resize_and_position(&window, &cfg_guard, has_pending);
    }

    Ok(())
}

/// IPC command: get the loaded configuration.
#[tauri::command]
pub async fn get_config(config: tauri::State<'_, ConfigState>) -> Result<Config, String> {
    let guard = config.read().await;
    Ok(guard.clone())
}

/// IPC command: save new configuration and apply window updates.
#[tauri::command]
pub async fn save_config(
    new_config: Config,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    config: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    new_config.save()?;

    {
        let mut guard = config.write().await;
        *guard = new_config.clone();
    }

    let has_pending = {
        let guard = state.lock().await;
        guard.pending_count() > 0
    };

    if let Some(window) = app_handle.get_webview_window("island") {
        crate::window::resize_and_position(&window, &new_config, has_pending);
    }

    let _ = app_handle.emit("config-changed", &new_config);

    Ok(())
}

/// IPC command: open the dedicated settings window.
#[tauri::command]
pub async fn open_settings_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}

/// IPC command: close/hide settings window.
#[tauri::command]
pub async fn close_settings_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("settings") {
        let _ = window.hide();
    }
    Ok(())
}

/// IPC command: close island if no pending requests.
#[tauri::command]
pub async fn close_island(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let has_pending = {
        let guard = state.lock().await;
        guard.pending_count() > 0
    };
    if !has_pending {
        if let Some(window) = app_handle.get_webview_window("island") {
            let _ = window.hide();
        }
    }
    Ok(())
}

/// IPC command: toggle island visibility.
#[tauri::command]
pub async fn toggle_island(
    app_handle: tauri::AppHandle,
    config: tauri::State<'_, ConfigState>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("island") {
        let is_visible = window.is_visible().unwrap_or(false);
        if is_visible {
            let _ = window.hide();
        } else {
            let cfg = config.read().await;
            let has_pending = {
                let guard = state.lock().await;
                guard.pending_count() > 0
            };
            crate::window::resize_and_position(&window, &cfg, has_pending);
            let _ = window.show();
            let _ = window.set_always_on_top(true);
        }
    }
    Ok(())
}

/// IPC command: toggle pause monitoring.
#[tauri::command]
pub async fn toggle_monitoring(
    config: tauri::State<'_, ConfigState>,
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    let mut guard = config.write().await;
    guard.behavior.pause_monitoring = !guard.behavior.pause_monitoring;
    let is_paused = guard.behavior.pause_monitoring;
    let new_cfg = guard.clone();
    drop(guard);

    let _ = new_cfg.save();
    let _ = app_handle.emit("config-changed", &new_cfg);

    Ok(is_paused)
}
