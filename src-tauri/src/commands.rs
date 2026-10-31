use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::error;

use crate::config::Config;
use crate::opencode::client::OpenCodeClient;
use crate::opencode::events::Decision;
use crate::state::NotchState;
use tauri::{Emitter, Manager};

/// Type alias for the shared application state managed by Tauri.
pub type AppState = Arc<Mutex<NotchState>>;

/// Type alias for the shared OpenCode client managed by Tauri.
pub type ClientState = Arc<Mutex<Option<OpenCodeClient>>>;

/// Type alias for the thread-safe configuration managed by Tauri.
pub type ConfigState = Arc<RwLock<Config>>;

/// IPC command: get the current notch state for the frontend.
#[tauri::command]
pub async fn get_state(state: tauri::State<'_, AppState>) -> Result<NotchState, String> {
    let guard = state.lock().await;
    Ok(guard.clone())
}

/// IPC command: respond to a pending permission request.
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

    // Look up session_id from pending requests
    let session_id = {
        let state_guard = state.lock().await;
        state_guard
            .pending_requests
            .iter()
            .find(|r| r.id == request_id)
            .map(|r| r.session_id.clone())
    };

    // Send the response to the OpenCode server
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

    // Immediately update window size/visibility (hides window when only_show_on_request is true)
    let cfg_guard = config.read().await;
    if let Some(window) = app_handle.get_webview_window("notch") {
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
    // Save to disk at ~/.config/opencode-notch/config.toml
    new_config.save()?;

    // Update in-memory state
    {
        let mut guard = config.write().await;
        *guard = new_config.clone();
    }

    let has_pending = {
        let guard = state.lock().await;
        guard.pending_count() > 0
    };

    if let Some(window) = app_handle.get_webview_window("notch") {
        crate::window::resize_and_position(&window, &new_config, has_pending);
    }

    // Broadcast config change to frontend
    let _ = app_handle.emit("config-changed", &new_config);

    Ok(())
}
