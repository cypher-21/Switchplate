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
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tracing::{error, info, warn};

fn main() {
    // On Linux running Wayland, re-exec with GDK_BACKEND=x11 so Xwayland handles the window.
    // This allows the borderless top-center island to float cleanly without window manager decoration conflicts.
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
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            info!(
                "Another Switchplate instance was launched with args: {:?}. Focusing existing instance.",
                args
            );
            if args.iter().any(|a| a == "--settings" || a == "-s") {
                if let Some(win) = app.get_webview_window("settings") {
                    let _ = win.unminimize();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            } else if args.iter().any(|a| a == "--island" || a == "-i" || a == "--open") {
                if let Some(win) = app.get_webview_window("island") {
                    let _ = win.unminimize();
                    let _ = win.show();
                    let _ = win.set_always_on_top(true);
                    let _ = win.set_focus();
                    let _ = app.emit("toggle-preview", ());
                }
            }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state.clone())
        .manage(client_state.clone())
        .manage(config_state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::respond_permission,
            commands::get_config,
            commands::save_config,
            commands::open_settings_window,
            commands::close_settings_window,
            commands::toggle_island,
            commands::close_island,
            commands::toggle_monitoring,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            let should_open_settings =
                std::env::args().any(|arg| arg == "--settings" || arg == "-s");

            // Use the bundled icon for the window and tray
            let app_icon = app
                .default_window_icon()
                .cloned()
                .expect("Failed to get default icon for tray");

            // Setup Island window
            if let Some(island_win) = app_handle.get_webview_window("island") {
                let _ = island_win.set_icon(app_icon.clone());
                let win_clone = island_win.clone();
                island_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });

                resize_and_position(&island_win, &config_for_setup, false);
            } else {
                warn!("Island window not found during setup");
            }

            // Setup Dedicated Settings window
            if let Some(settings_win) = app_handle.get_webview_window("settings") {
                let _ = settings_win.set_icon(app_icon.clone());
                let win_clone = settings_win.clone();
                settings_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });

                if should_open_settings {
                    let _ = settings_win.show();
                    let _ = settings_win.set_focus();
                }
            } else {
                warn!("Settings window not found during setup");
            }

            // Build Native Linux System Tray Menu
            let open_island_item = MenuItem::with_id(app, "open_island", "Open Switchplate", true, None::<&str>)?;
            let status_item = MenuItem::with_id(app, "status", "OpenCode: Connecting...", false, None::<&str>)?;
            let accept_item = MenuItem::with_id(app, "accept", "Accept Request", true, None::<&str>)?;
            let reject_item = MenuItem::with_id(app, "reject", "Reject Request", true, None::<&str>)?;
            let always_item = MenuItem::with_id(app, "always", "Always Allow", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Switchplate", true, None::<&str>)?;

            let tray_menu = Menu::with_items(
                app,
                &[
                    &open_island_item,
                    &status_item,
                    &accept_item,
                    &reject_item,
                    &always_item,
                    &settings_item,
                    &quit_item,
                ],
            )?;

            let handle_for_menu = app_handle.clone();
            let config_for_menu = config_state_for_setup.clone();
            let state_for_menu = app_state.clone();
            let client_for_menu = client_state.clone();

            let _tray = TrayIconBuilder::with_id("switchplate-tray")
                .icon(app_icon)
                .tooltip("Switchplate — Control surface for OpenCode")
                .menu(&tray_menu)
                .on_menu_event(move |_app, event| match event.id.as_ref() {
                    "open_island" => {
                        let h = handle_for_menu.clone();
                        let cfg_state = config_for_menu.clone();
                        let s_state = state_for_menu.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Some(win) = h.get_webview_window("island") {
                                let cfg = cfg_state.read().await;
                                resize_and_position(&win, &cfg, true);
                                let _ = win.unminimize();
                                let _ = win.show();
                                let _ = win.set_always_on_top(true);
                                let _ = win.set_focus();
                                let _ = h.emit("toggle-preview", ());
                            }
                        });
                    }
                    "accept" => {
                        let s = state_for_menu.clone();
                        let c = client_for_menu.clone();
                        let cfg = config_for_menu.clone();
                        let h = handle_for_menu.clone();
                        tauri::async_runtime::spawn(async move {
                            handle_shortcut_decision(s, c, h, cfg, Decision::Allow).await;
                        });
                    }
                    "reject" => {
                        let s = state_for_menu.clone();
                        let c = client_for_menu.clone();
                        let cfg = config_for_menu.clone();
                        let h = handle_for_menu.clone();
                        tauri::async_runtime::spawn(async move {
                            handle_shortcut_decision(s, c, h, cfg, Decision::Deny).await;
                        });
                    }
                    "always" => {
                        let s = state_for_menu.clone();
                        let c = client_for_menu.clone();
                        let cfg = config_for_menu.clone();
                        let h = handle_for_menu.clone();
                        tauri::async_runtime::spawn(async move {
                            handle_shortcut_decision(s, c, h, cfg, Decision::Always).await;
                        });
                    }
                    "settings" => {
                        if let Some(settings_win) = handle_for_menu.get_webview_window("settings") {
                            let _ = settings_win.show();
                            let _ = settings_win.set_focus();
                        }
                    }
                    "quit" => {
                        info!("Quit requested via tray menu. Exiting Switchplate.");
                        std::process::exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(settings_win) = app.get_webview_window("settings") {
                            let is_visible = settings_win.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = settings_win.hide();
                            } else {
                                let _ = settings_win.show();
                                let _ = settings_win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Spawn the OpenCode connection and event loop
            let state_for_task = app_state.clone();
            let client_for_task = client_state.clone();
            let config_for_task = config_state_for_setup.clone();
            let handle_for_task = app_handle.clone();
            let status_item_for_task = status_item.clone();

            tauri::async_runtime::spawn(async move {
                run_connection_loop(
                    state_for_task,
                    client_for_task,
                    config_for_task,
                    handle_for_task,
                    status_item_for_task,
                )
                .await;
            });

            // Register global keyboard shortcuts
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
        .expect("Error while running Switchplate");
}

/// Continuously attempt to connect to OpenCode server and stream events.
async fn run_connection_loop(
    state: AppState,
    client_state: ClientState,
    config_state: ConfigState,
    app_handle: tauri::AppHandle,
    status_item: tauri::menu::MenuItem<tauri::Wry>,
) {
    loop {
        // Check if monitoring is paused
        let is_paused = {
            let guard = config_state.read().await;
            guard.behavior.pause_monitoring
        };

        if is_paused {
            let _ = status_item.set_text("OpenCode: Paused");
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            continue;
        }

        // Update state to connecting
        {
            let mut s = state.lock().await;
            s.connection = ConnectionStatus::Connecting;
        }
        let _ = status_item.set_text("OpenCode: Connecting...");
        emit_state_changed(&app_handle, &state).await;

        let cfg = {
            let guard = config_state.read().await;
            guard.clone()
        };

        info!("Attempting to connect to OpenCode server...");

        match OpenCodeClient::connect(&cfg).await {
            Ok(client) => {
                info!("Connected to OpenCode server at {}", client.base_url());
                let _ = status_item.set_text("OpenCode: Connected");

                {
                    let mut c = client_state.lock().await;
                    *c = Some(client.clone());
                }

                let session_count = client.list_sessions().await.map(|s| s.len()).unwrap_or(0);
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

                match client.subscribe_events().await {
                    Ok(event_stream) => {
                        info!("Subscribed to OpenCode event stream");
                        process_event_stream(
                            event_stream,
                            &state,
                            &app_handle,
                            &config_state,
                        )
                        .await;
                        warn!("OpenCode event stream ended, will reconnect");
                    }
                    Err(e) => {
                        error!("Failed to subscribe to OpenCode events: {}", e);
                    }
                }

                {
                    let mut c = client_state.lock().await;
                    *c = None;
                }
            }
            Err(e) => {
                error!("Failed to connect to OpenCode server: {}", e);
            }
        }

        {
            let mut s = state.lock().await;
            s.connection = ConnectionStatus::Disconnected;
            s.pending_requests.clear();
        }
        let _ = status_item.set_text("OpenCode: Offline");
        emit_state_changed(&app_handle, &state).await;
        update_window_size(&app_handle, &state, &config_state).await;

        let auto_reconnect = {
            let guard = config_state.read().await;
            guard.opencode.auto_reconnect
        };

        if !auto_reconnect {
            info!("Auto-reconnect disabled. Waiting 30s before retry.");
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        } else {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
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
        let is_paused = {
            let guard = config_state.read().await;
            guard.behavior.pause_monitoring
        };
        if is_paused {
            continue;
        }

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
            OpenCodeEvent::Unknown => {}
        }
    }
}

/// Emit "state-changed" event to frontend with current state.
async fn emit_state_changed(app_handle: &tauri::AppHandle, state: &AppState) {
    let current_state = {
        let s = state.lock().await;
        s.clone()
    };
    if let Err(e) = app_handle.emit("state-changed", &current_state) {
        error!("Failed to emit state-changed event: {}", e);
    }
}

/// Update island window size and visibility based on pending requests and config.
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
    if let Some(window) = app_handle.get_webview_window("island") {
        resize_and_position(&window, &cfg, has_pending);
    }
}

/// Register global keyboard shortcuts for Switchplate.
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

    let config_o = config_state.clone();
    let state_o = state.clone();
    let handle_o = handle.clone();

    let handle_s = handle.clone();

    // Accept (Ctrl+Shift+A)
    let _ = app_handle.global_shortcut().on_shortcut("ctrl+shift+a", move |_app, _shortcut, event| {
        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            let s = state_a.clone();
            let c = client_a.clone();
            let cfg = config_a.clone();
            let h = handle_a.clone();
            tauri::async_runtime::spawn(async move {
                handle_shortcut_decision(s, c, h, cfg, Decision::Allow).await;
            });
        }
    });

    // Deny (Ctrl+Shift+D)
    let _ = app_handle.global_shortcut().on_shortcut("ctrl+shift+d", move |_app, _shortcut, event| {
        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            let s = state_d.clone();
            let c = client_d.clone();
            let cfg = config_d.clone();
            let h = handle_d.clone();
            tauri::async_runtime::spawn(async move {
                handle_shortcut_decision(s, c, h, cfg, Decision::Deny).await;
            });
        }
    });

    // Always Allow (Ctrl+Shift+W)
    let _ = app_handle.global_shortcut().on_shortcut("ctrl+shift+w", move |_app, _shortcut, event| {
        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            let s = state_w.clone();
            let c = client_w.clone();
            let cfg = config_w.clone();
            let h = handle_w.clone();
            tauri::async_runtime::spawn(async move {
                handle_shortcut_decision(s, c, h, cfg, Decision::Always).await;
            });
        }
    });

    // Open/Toggle Switchplate (Ctrl+Shift+O)
    let _ = app_handle.global_shortcut().on_shortcut("ctrl+shift+o", move |_app, _shortcut, event| {
        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            if let Some(window) = handle_o.get_webview_window("island") {
                let cfg_state = config_o.clone();
                let s_state = state_o.clone();
                let win = window.clone();
                let h = handle_o.clone();
                tauri::async_runtime::spawn(async move {
                    let cfg = cfg_state.read().await;
                    resize_and_position(&win, &cfg, true);
                    let _ = win.unminimize();
                    let _ = win.show();
                    let _ = win.set_always_on_top(true);
                    let _ = win.set_focus();
                    let _ = h.emit("toggle-preview", ());
                });
            }
        }
    });

    // Open Settings (Ctrl+Shift+S)
    let _ = app_handle.global_shortcut().on_shortcut("ctrl+shift+s", move |_app, _shortcut, event| {
        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            if let Some(settings_win) = handle_s.get_webview_window("settings") {
                let is_visible = settings_win.is_visible().unwrap_or(false);
                if is_visible {
                    let _ = settings_win.hide();
                } else {
                    let _ = settings_win.show();
                    let _ = settings_win.set_focus();
                }
            }
        }
    });
}

/// Handle shortcut decision by responding to current pending permission request.
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
