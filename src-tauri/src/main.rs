#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod opencode;
mod state;
mod window;

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use commands::{AppState, ClientState, ConfigState};
use config::Config;
use opencode::client::OpenCodeClient;
use opencode::events::{Decision, OpenCodeEvent};
use state::{ConnectionStatus, NotchState};
use window::resize_and_position;

use futures_util::StreamExt;
use tauri::{Emitter, Manager};
use tracing::{error, info, warn};

fn main() {
    // On Linux running Wayland, re-exec with GDK_BACKEND=x11 so Xwayland handles the window.
    // This allows the borderless notch panel to float at any position and stay on top reliably.
    #[cfg(target_os = "linux")]
    {
        if std::env::var("GDK_BACKEND").map(|v| v != "x11").unwrap_or(true)
            && std::env::var("DISPLAY").is_ok()
        {
            use std::os::unix::process::CommandExt;
            if let Ok(current_exe) = std::env::current_exe() {
                let args: Vec<String> = std::env::args().skip(1).collect();
                let _ = std::process::Command::new(current_exe)
                    .args(args)
                    .env("GDK_BACKEND", "x11")
                    .exec();
            }
        }
    }

    tracing_subscriber::fmt::init();

    let config = Config::load();
    let app_state: AppState = Arc::new(Mutex::new(NotchState::new()));
    let client_state: ClientState = Arc::new(Mutex::new(None));
    let config_state: ConfigState = Arc::new(RwLock::new(config.clone()));

    let config_for_setup = config.clone();
    let config_state_for_setup = config_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state.clone())
        .manage(client_state.clone())
        .manage(config_state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::respond_permission,
            commands::get_config,
            commands::save_config,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            let should_open_settings =
                std::env::args().any(|arg| arg == "--settings" || arg == "--preview");

            // Initialize notch window: hide if only_show_on_request, or show preview/pill
            if let Some(window) = app_handle.get_webview_window("notch") {
                if should_open_settings {
                    resize_and_position(&window, &config_for_setup, true);
                    let _ = window.show();
                    let _ = window.set_always_on_top(true);
                    let handle_clone = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        let _ = handle_clone.emit("open-settings", ());
                    });
                } else {
                    resize_and_position(&window, &config_for_setup, false);
                }
            } else {
                warn!("Notch window not found during setup");
            }

            // Spawn the OpenCode connection and event loop
            let state_for_task = app_state.clone();
            let client_for_task = client_state.clone();
            let config_for_task = config_state_for_setup.clone();
            let handle_for_task = app_handle.clone();

            tauri::async_runtime::spawn(async move {
                run_connection_loop(
                    state_for_task,
                    client_for_task,
                    config_for_task,
                    handle_for_task,
                )
                .await;
            });

            // Register global shortcuts
            let state_for_shortcuts = app_state.clone();
            let client_for_shortcuts = client_state.clone();
            let config_for_shortcuts = config_state_for_setup.clone();
            let handle_for_shortcuts = app_handle.clone();

            register_global_shortcuts(
                &app_handle,
                state_for_shortcuts,
                client_for_shortcuts,
                config_for_shortcuts,
                handle_for_shortcuts,
            );

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running OpenCode Notch");
}

/// Continuously attempt to connect to the OpenCode server and process SSE events.
/// Reconnects automatically on disconnection with a backoff delay.
async fn run_connection_loop(
    state: AppState,
    client_state: ClientState,
    config_state: ConfigState,
    app_handle: tauri::AppHandle,
) {
    loop {
        // Update state to connecting
        {
            let mut s = state.lock().await;
            s.connection = ConnectionStatus::Connecting;
        }
        emit_state_changed(&app_handle, &state).await;

        let cfg = {
            let guard = config_state.read().await;
            guard.clone()
        };

        info!("Attempting to connect to OpenCode server...");

        match OpenCodeClient::connect(&cfg).await {
            Ok(client) => {
                info!("Connected to OpenCode server at {}", client.base_url());

                // Store the client
                {
                    let mut c = client_state.lock().await;
                    *c = Some(client.clone());
                }

                // Get initial session count
                let session_count = client
                    .list_sessions()
                    .await
                    .map(|s| s.len())
                    .unwrap_or(0);

                // Fetch any currently pending permissions that were already asked
                let initial_requests = client.fetch_pending_permissions().await.unwrap_or_default();

                {
                    let mut s = state.lock().await;
                    s.connection = ConnectionStatus::Connected { session_count };
                    for req in initial_requests {
                        s.add_request(req);
                    }
                }
                emit_state_changed(&app_handle, &state).await;
                update_window_size(&app_handle, &state, &config_state).await;

                // Subscribe to events
                match client.subscribe_events().await {
                    Ok(event_stream) => {
                        info!("Subscribed to SSE event stream");
                        process_event_stream(
                            event_stream,
                            &state,
                            &app_handle,
                            &config_state,
                        )
                        .await;
                        // If we get here, the stream ended
                        warn!("SSE event stream ended, will reconnect");
                    }
                    Err(e) => {
                        error!("Failed to subscribe to SSE events: {}", e);
                    }
                }

                // Clear the client on disconnect
                {
                    let mut c = client_state.lock().await;
                    *c = None;
                }
            }
            Err(e) => {
                error!("Failed to connect to OpenCode server: {}", e);
            }
        }

        // Update state to disconnected
        {
            let mut s = state.lock().await;
            s.connection = ConnectionStatus::Disconnected;
            s.pending_requests.clear();
        }
        emit_state_changed(&app_handle, &state).await;
        update_window_size(&app_handle, &state, &config_state).await;

        // Wait before reconnecting
        info!("Reconnecting in 5 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

/// Process incoming SSE events from the OpenCode server.
async fn process_event_stream(
    event_stream: impl futures_util::Stream<Item = OpenCodeEvent>,
    state: &AppState,
    app_handle: &tauri::AppHandle,
    config_state: &ConfigState,
) {
    futures_util::pin_mut!(event_stream);

    while let Some(event) = event_stream.next().await {
        match event {
            OpenCodeEvent::PermissionAsked(payload) => {
                if let Some(req) = payload.into_permission_request() {
                    info!("Permission requested: {} (tool: {})", req.id, req.tool);
                    {
                        let mut s = state.lock().await;
                        s.add_request(req);
                    }
                    emit_state_changed(app_handle, state).await;
                    update_window_size(app_handle, state, config_state).await;
                }
            }
            OpenCodeEvent::PermissionReplied(payload) => {
                let req_id = payload.request_id();
                let decision = payload.decision_str();
                info!("Permission replied: {} -> {}", req_id, decision);
                {
                    let mut s = state.lock().await;
                    s.remove_request(&req_id);
                }
                emit_state_changed(app_handle, state).await;
                update_window_size(app_handle, state, config_state).await;
            }
            OpenCodeEvent::SessionCreated(session) => {
                info!("Session created: {}", session.session_id());
                {
                    let mut s = state.lock().await;
                    if let ConnectionStatus::Connected {
                        ref mut session_count,
                    } = s.connection
                    {
                        *session_count += 1;
                    }
                }
                emit_state_changed(app_handle, state).await;
            }
            OpenCodeEvent::SessionEnded(session) | OpenCodeEvent::SessionDeleted(session) => {
                info!("Session ended: {}", session.session_id());
                {
                    let mut s = state.lock().await;
                    if let ConnectionStatus::Connected {
                        ref mut session_count,
                    } = s.connection
                    {
                        *session_count = session_count.saturating_sub(1);
                    }
                }
                emit_state_changed(app_handle, state).await;
            }
            OpenCodeEvent::Unknown => {
                // Ignore unknown events
            }
        }
    }
}

/// Emit a "state-changed" event to the frontend with the current state.
async fn emit_state_changed(app_handle: &tauri::AppHandle, state: &AppState) {
    let current_state = {
        let s = state.lock().await;
        s.clone()
    };
    if let Err(e) = app_handle.emit("state-changed", &current_state) {
        error!("Failed to emit state-changed event: {}", e);
    }
}

/// Update the notch window size and visibility based on pending requests.
async fn update_window_size(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    config_state: &ConfigState,
) {
    let has_pending = {
        let s = state.lock().await;
        s.pending_count() > 0
    };
    let cfg = config_state.read().await;
    if let Some(window) = app_handle.get_webview_window("notch") {
        resize_and_position(&window, &cfg, has_pending);
    }
}

/// Register global keyboard shortcuts for quick permission responses.
fn register_global_shortcuts(
    app_handle: &tauri::AppHandle,
    state: AppState,
    client_state: ClientState,
    config_state: ConfigState,
    handle: tauri::AppHandle,
) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let state_a = state.clone();
    let client_a = client_state.clone();
    let config_a = config_state.clone();
    let handle_a = handle.clone();

    let state_d = state.clone();
    let client_d = client_state.clone();
    let config_d = config_state.clone();
    let handle_d = handle.clone();

    let state_w = state.clone();
    let client_w = client_state.clone();
    let config_w = config_state.clone();
    let handle_w = handle.clone();

    let config_t = config_state.clone();
    let handle_t = handle.clone();

    // Accept (Allow)
    let accept_key = "ctrl+shift+a";
    if let Err(e) = app_handle.global_shortcut().on_shortcut(accept_key, move |_app, _shortcut, event| {
        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            let state = state_a.clone();
            let client = client_a.clone();
            let config = config_a.clone();
            let handle = handle_a.clone();
            tauri::async_runtime::spawn(async move {
                handle_shortcut_decision(state, client, handle, config, Decision::Allow).await;
            });
        }
    }) {
        warn!("Failed to register shortcut '{}': {}", accept_key, e);
    }

    // Deny (Reject)
    let deny_key = "ctrl+shift+d";
    if let Err(e) = app_handle.global_shortcut().on_shortcut(deny_key, move |_app, _shortcut, event| {
        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            let state = state_d.clone();
            let client = client_d.clone();
            let config = config_d.clone();
            let handle = handle_d.clone();
            tauri::async_runtime::spawn(async move {
                handle_shortcut_decision(state, client, handle, config, Decision::Deny).await;
            });
        }
    }) {
        warn!("Failed to register shortcut '{}': {}", deny_key, e);
    }

    // Always allow
    let always_key = "ctrl+shift+w";
    if let Err(e) = app_handle.global_shortcut().on_shortcut(always_key, move |_app, _shortcut, event| {
        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            let state = state_w.clone();
            let client = client_w.clone();
            let config = config_w.clone();
            let handle = handle_w.clone();
            tauri::async_runtime::spawn(async move {
                handle_shortcut_decision(state, client, handle, config, Decision::Always).await;
            });
        }
    }) {
        warn!("Failed to register shortcut '{}': {}", always_key, e);
    }

    // Toggle visibility / Open settings
    let toggle_key = "ctrl+shift+n";
    if let Err(e) = app_handle.global_shortcut().on_shortcut(toggle_key, move |_app, _shortcut, event| {
        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            if let Some(window) = handle_t.get_webview_window("notch") {
                let is_visible = window.is_visible().unwrap_or(false);
                if is_visible {
                    let _ = window.hide();
                } else {
                    let cfg_state = config_t.clone();
                    let win = window.clone();
                    let h = handle_t.clone();
                    tauri::async_runtime::spawn(async move {
                        let cfg = cfg_state.read().await;
                        resize_and_position(&win, &cfg, true);
                        let _ = win.show();
                        let _ = win.set_always_on_top(true);
                        let _ = h.emit("open-settings", ());
                    });
                }
            }
        }
    }) {
        warn!("Failed to register shortcut '{}': {}", toggle_key, e);
    }
}

/// Handle a keyboard shortcut decision by responding to the current pending permission request.
async fn handle_shortcut_decision(
    state: AppState,
    client_state: ClientState,
    app_handle: tauri::AppHandle,
    config_state: ConfigState,
    decision: Decision,
) {
    let (request_id, session_id) = {
        let s = state.lock().await;
        match s.current_request() {
            Some(r) => (r.id.clone(), Some(r.session_id.clone())),
            None => return,
        }
    };

    let client_guard = client_state.lock().await;
    let Some(ref client) = *client_guard else {
        warn!("Cannot respond: not connected to OpenCode server");
        return;
    };

    match client.respond_permission(&request_id, session_id.as_deref(), decision).await {
        Ok(()) => {
            info!("Permission {} responded via shortcut", request_id);
            drop(client_guard);
            {
                let mut s = state.lock().await;
                s.remove_request(&request_id);
            }
            emit_state_changed(&app_handle, &state).await;
            update_window_size(&app_handle, &state, &config_state).await;
        }
        Err(e) => {
            error!("Failed to respond to permission {}: {}", request_id, e);
        }
    }
}
