use std::sync::Arc;
use tokio::sync::{watch, Mutex};

use crate::config::Config;
use crate::opencode::client::{FormReply, OpenCodeClient};
use crate::opencode::events::{Decision, ResponseOutcome};
use crate::opencode::forms::{FormAnswer, FormId};
use crate::sound::{self, Chime};
use crate::state::NotchState;
use tauri::{Emitter, Manager};

pub type AppState = Arc<Mutex<NotchState>>;
pub type ClientState = Arc<Mutex<Option<OpenCodeClient>>>;
pub type ConfigState = watch::Sender<Config>;

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
    let parsed_decision = Decision::from_str_loose(&decision).ok_or_else(|| {
        format!(
            "Invalid decision '{}': expected allow, deny, or always",
            decision
        )
    })?;

    if crate::reply_to_pending_request(&state, &client, &config, &request_id, Some(parsed_decision))
        .await?
    {
        crate::emit_state_changed(&app_handle, &state).await;
        crate::update_window_size(&app_handle, &state, &config).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn reply_to_form(
    form: FormId,
    answer: FormAnswer,
    state: tauri::State<'_, AppState>,
    client: tauri::State<'_, ClientState>,
    config: tauri::State<'_, ConfigState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let client = client.lock().await;
    if !state
        .lock()
        .await
        .pending_forms
        .iter()
        .any(|pending| pending.key == form)
    {
        return Err("This question has already closed.".into());
    }
    let server = client
        .as_ref()
        .ok_or("OpenCode is disconnected. Try again after reconnecting.")?;
    let reply = server.reply_to_form(&form, answer).await?;
    let removed = state.lock().await.remove_form(&form);
    drop(client);

    if removed {
        crate::emit_state_changed(&app_handle, &state).await;
        crate::update_window_size(&app_handle, &state, &config).await;
    }
    match reply {
        FormReply::Sent => Ok(()),
        FormReply::AlreadyClosed => {
            Err("This question was already answered or closed in OpenCode.".into())
        }
    }
}

/// IPC command: get the loaded configuration.
#[tauri::command]
pub async fn get_config(config: tauri::State<'_, ConfigState>) -> Result<Config, String> {
    Ok(config.borrow().clone())
}

/// IPC command: save new configuration and apply window updates.
#[tauri::command]
pub async fn save_config(
    new_config: Config,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    config: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    persist_config(&config, |_| new_config.clone())?;

    crate::update_window_size(&app_handle, &state, &config).await;
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

/// IPC command: close the preview when no notification is waiting.
#[tauri::command]
pub async fn close_island(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().await;
    if !state.has_card() {
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
            let content = state.lock().await.island_content();
            let cfg = config.borrow().clone();
            crate::window::resize_and_position(&window, &cfg, content, true, false);
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
    let new_cfg = persist_config(&config, |current| {
        let mut updated = current.clone();
        updated.behavior.pause_monitoring = !updated.behavior.pause_monitoring;
        updated
    })?;
    let _ = app_handle.emit("config-changed", &new_cfg);

    Ok(new_cfg.behavior.pause_monitoring)
}

fn persist_config(
    config: &ConfigState,
    update: impl FnOnce(&Config) -> Config,
) -> Result<Config, String> {
    let mut result = Ok(None);
    config.send_if_modified(|current| {
        let updated = update(current);
        match updated.save() {
            Ok(()) => {
                *current = updated.clone();
                result = Ok(Some(updated));
                true
            }
            Err(error) => {
                result = Err(error);
                false
            }
        }
    });
    Ok(result?.expect("config update must produce a result"))
}

/// IPC command: test/trigger a response completed popup.
#[tauri::command]
pub async fn test_response_notification(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    config: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    crate::show_response_notice(&app_handle, &state, &config, ResponseOutcome::Ready, true).await;
    Ok(())
}

#[tauri::command]
pub async fn test_notification_sound(chime: Chime) -> Result<(), String> {
    sound::play_checked(chime).await
}

#[tauri::command]
pub async fn dismiss_response_notice(
    id: u64,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    config: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    if state.lock().await.dismiss_response_notice(id) {
        crate::emit_state_changed(&app_handle, &state).await;
        crate::update_window_size(&app_handle, &state, &config).await;
    }
    Ok(())
}
