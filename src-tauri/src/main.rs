#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod opencode;
mod sound;
mod state;
mod window;

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;

use commands::{AppState, ClientState, ConfigState};
use config::{Config, OpenCodeConfig};
use opencode::client::{OpenCodeClient, PendingInputs};
use opencode::events::{Decision, OpenCodeEvent, PermissionRequest, ResponseOutcome, ResponseTracker};
use sound::Chime;
use state::{ConnectionStatus, IslandContent, NotchState};
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
    let config_state = ConfigState::new(config.clone());

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
            commands::test_response_notification,
            commands::test_notification_sound,
            commands::dismiss_response_notice,
            commands::reply_to_form,
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

                resize_and_position(&island_win, &config_for_setup, IslandContent::Idle, false, false);
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
                        let state = state_for_menu.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Some(win) = h.get_webview_window("island") {
                                let cfg = cfg_state.borrow().clone();
                                let content = state.lock().await.island_content();
                                resize_and_position(&win, &cfg, content, true, true);
                                let _ = win.unminimize();
                                let _ = win.show();
                                let _ = win.set_always_on_top(true);
                                let _ = win.set_focus();
                                let _ = h.emit("open-island", ());
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
                        info!("Quit requested via tray menu. Stopping service and exiting Switchplate.");
                        let _ = std::process::Command::new("systemctl")
                            .args(["--user", "stop", "switchplate.service"])
                            .status();
                        let _ = std::process::Command::new("systemctl")
                            .args(["--user", "stop", "opencode-notch.service"])
                            .status();
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
                        if let Some(win) = app.get_webview_window("island") {
                            let is_visible = win.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = win.hide();
                                let _ = app.emit("island-closed", ());
                            } else {
                                let _ = win.unminimize();
                                let _ = win.show();
                                let _ = win.set_always_on_top(true);
                                let _ = win.set_focus();
                                let _ = app.emit("open-island", ());
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
    let mut config_changes = config_state.subscribe();
    loop {
        let cfg = config_changes.borrow_and_update().clone();
        if cfg.behavior.pause_monitoring {
            let _ = status_item.set_text("OpenCode: Paused");
            let _ = config_changes.changed().await;
            continue;
        }

        // Update state to connecting
        {
            let mut s = state.lock().await;
            s.connection = ConnectionStatus::Connecting;
        }
        let _ = status_item.set_text("OpenCode: Connecting...");
        emit_state_changed(&app_handle, &state).await;

        info!("Attempting to connect to OpenCode server...");

        let mut reconnect_now = false;
        match OpenCodeClient::connect(&cfg).await {
            Ok(client) => {
                info!("Connected to OpenCode server at {}", client.base_url());
                let _ = status_item.set_text("OpenCode: Connected");

                {
                    let mut c = client_state.lock().await;
                    *c = Some(client.clone());
                }

                let session_count = client.list_sessions().await.map(|s| s.len()).unwrap_or(0);
                match client.subscribe_events().await {
                    Ok(event_stream) => {
                        info!("Subscribed to OpenCode event stream");
                        // Subscribe before taking the snapshot so requests arriving
                        // during startup are available through at least one path.
                        let initial_inputs = client.fetch_pending_inputs().await;
                        state.lock().await.connection =
                            ConnectionStatus::Connected { session_count };
                        emit_state_changed(&app_handle, &state).await;
                        reconnect_now = process_event_stream(
                            event_stream,
                            initial_inputs,
                            &state,
                            &client_state,
                            &app_handle,
                            &config_state,
                            &cfg.opencode,
                        )
                        .await;
                        if !reconnect_now {
                            warn!("OpenCode event stream ended, will reconnect");
                        }
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
            s.pending_forms.clear();
        }
        let _ = status_item.set_text("OpenCode: Offline");
        emit_state_changed(&app_handle, &state).await;
        update_window_size(&app_handle, &state, &config_state).await;

        if !reconnect_now {
            let delay = if config_state.borrow().opencode.auto_reconnect {
                5
            } else {
                30
            };
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(delay)) => {},
                _ = config_changes.wait_for(|config| config.behavior.pause_monitoring || config.opencode != cfg.opencode) => {},
            }
        }
    }
}

/// Process incoming SSE events from the OpenCode server.
async fn process_event_stream(
    event_stream: impl futures_util::Stream<Item = OpenCodeEvent>,
    initial_inputs: PendingInputs,
    state: &AppState,
    client: &ClientState,
    app_handle: &tauri::AppHandle,
    config_state: &ConfigState,
    connected_config: &OpenCodeConfig,
) -> bool {
    futures_util::pin_mut!(event_stream);
    let mut config_changes = config_state.subscribe();
    let config = config_changes.borrow().clone();
    if config.behavior.pause_monitoring || &config.opencode != connected_config {
        return true;
    }
    let mut approval_mode = config.behavior.auto_approval;
    let mut recent_requests = VecDeque::new();
    let mut seen_forms = HashSet::new();
    let mut responses = ResponseTracker::default();

    for request in initial_inputs.permissions {
        handle_permission_request(request, &mut recent_requests, state, client, config_state).await;
    }
    {
        let mut state = state.lock().await;
        for form in initial_inputs.forms {
            seen_forms.insert(form.key.clone());
            state.add_form(form);
        }
    }
    emit_state_changed(app_handle, state).await;
    update_window_size(app_handle, state, config_state).await;
    if config_state.borrow().behavior.sound_enabled {
        let state = state.lock().await;
        if state.current_request().is_some() {
            sound::play(Chime::Request);
        } else if !state.pending_forms.is_empty() {
            sound::play(Chime::Question);
        }
    }

    loop {
        let cfg = config_changes.borrow_and_update().clone();
        if cfg.behavior.pause_monitoring || &cfg.opencode != connected_config {
            return true;
        }

        if cfg.behavior.auto_approval != approval_mode {
            approval_mode = cfg.behavior.auto_approval;
            if cfg.behavior.automatic_decision().is_some() {
                approve_pending_requests(state, client, app_handle, config_state).await;
                continue;
            }
        }

        let event = tokio::select! {
            biased;
            _ = config_changes.changed() => continue,
            event = event_stream.next() => match event {
                Some(event) => event,
                None => return false,
            },
        };

        if let Some(outcome) = responses.observe(&event) {
            info!("OpenCode response finished: {:?}", outcome);
            let play_sound = config_state.borrow().behavior.sound_enabled;
            show_response_notice(app_handle, state, config_state, outcome, play_sound).await;
        }

        match event {
            OpenCodeEvent::PermissionAsked(payload) => {
                if let Some(req) = payload.into_permission_request() {
                    if handle_permission_request(
                        req,
                        &mut recent_requests,
                        state,
                        client,
                        config_state,
                    )
                    .await
                    {
                        emit_state_changed(app_handle, state).await;
                        update_window_size(app_handle, state, config_state).await;
                        if config_state.borrow().behavior.sound_enabled {
                            sound::play(Chime::Request);
                        }
                    }
                }
            }
            OpenCodeEvent::PermissionReplied(payload) => {
                let req_id = payload.request_id();
                let decision = payload.decision_str();
                info!("Permission replied: {} -> {}", req_id, decision);
                if state.lock().await.remove_request(&req_id).is_some() {
                    emit_state_changed(app_handle, state).await;
                    update_window_size(app_handle, state, config_state).await;
                }
            }
            OpenCodeEvent::FormCreated(event) => {
                let form = event.data.form;
                if seen_forms.insert(form.key.clone()) && state.lock().await.add_form(form) {
                    emit_state_changed(app_handle, state).await;
                    update_window_size(app_handle, state, config_state).await;
                    if config_state.borrow().behavior.sound_enabled {
                        sound::play(Chime::Question);
                    }
                }
            }
            OpenCodeEvent::FormSettled(event) => {
                if state.lock().await.remove_form(&event.data) {
                    emit_state_changed(app_handle, state).await;
                    update_window_size(app_handle, state, config_state).await;
                }
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
                let session_id = session.session_id();
                info!("Session ended: {session_id}");
                let forms_removed = {
                    let mut s = state.lock().await;
                    if let ConnectionStatus::Connected {
                        ref mut session_count,
                    } = s.connection
                    {
                        *session_count = session_count.saturating_sub(1);
                    }
                    let previous_count = s.pending_forms.len();
                    s.pending_forms.retain(|form| form.key.session_id != session_id);
                    s.pending_forms.len() != previous_count
                };
                emit_state_changed(app_handle, state).await;
                if forms_removed {
                    update_window_size(app_handle, state, config_state).await;
                }
            }
            _ => {}
        }
    }
}

/// Route both snapshot and streamed requests through the saved approval mode.
async fn handle_permission_request(
    request: PermissionRequest,
    recent_requests: &mut VecDeque<String>,
    state: &AppState,
    client: &ClientState,
    config: &ConfigState,
) -> bool {
    if request.id.is_empty() || recent_requests.contains(&request.id) {
        return false;
    }
    // An ask can appear both in the startup snapshot and on the live stream.
    recent_requests.push_back(request.id.clone());
    if recent_requests.len() > 1024 {
        recent_requests.pop_front();
    }

    info!(
        "Permission requested: {} (tool: {})",
        request.id, request.tool
    );
    if config.borrow().behavior.automatic_decision().is_some() {
        let client = client.lock().await;
        if let Some(client) = client.as_ref() {
            let cfg = config.borrow().clone();
            match client.auto_approve(&request, &cfg).await {
                Ok(true) => return false,
                Ok(false) => {}
                Err(error) => warn!("Automatic reply failed for {}: {error}", request.id),
            }
        }
    }
    state.lock().await.add_request(request)
}

/// Apply a newly saved automatic mode to requests already waiting for review.
async fn approve_pending_requests(
    state: &AppState,
    client: &ClientState,
    app_handle: &tauri::AppHandle,
    config: &ConfigState,
) {
    let ids: Vec<_> = state
        .lock()
        .await
        .pending_requests
        .iter()
        .map(|request| request.id.clone())
        .collect();
    for id in ids {
        match reply_to_pending_request(state, client, config, &id, None).await {
            Ok(true) => {
                emit_state_changed(app_handle, state).await;
                update_window_size(app_handle, state, config).await;
            }
            Ok(false) => {}
            Err(error) => warn!("Automatic reply failed for {id}: {error}"),
        }
    }
}

/// Serialize local replies so manual and automatic actions cannot answer the same request.
async fn reply_to_pending_request(
    state: &AppState,
    client_state: &ClientState,
    config: &ConfigState,
    request_id: &str,
    manual_decision: Option<Decision>,
) -> Result<bool, String> {
    let client = client_state.lock().await;
    let Some(request) = state
        .lock()
        .await
        .pending_requests
        .iter()
        .find(|request| request.id == request_id)
        .cloned()
    else {
        return Ok(false);
    };
    let client = client.as_ref().ok_or("Not connected to OpenCode server")?;
    if let Some(decision) = manual_decision {
        client.respond_permission(&request, decision).await?;
    } else {
        let cfg = config.borrow().clone();
        if !client.auto_approve(&request, &cfg).await? {
            return Ok(false);
        }
    }
    state.lock().await.remove_request(request_id);
    if let Some(decision) = manual_decision.filter(|_| config.borrow().behavior.sound_enabled) {
        sound::play(if decision == Decision::Deny { Chime::Reject } else { Chime::Approve });
    }
    Ok(true)
}

async fn show_response_notice(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    config: &ConfigState,
    outcome: ResponseOutcome,
    play_sound: bool,
) {
    state.lock().await.add_response_notice(outcome);
    emit_state_changed(app_handle, state).await;
    update_window_size(app_handle, state, config).await;
    if play_sound {
        sound::play(if outcome == ResponseOutcome::Ready { Chime::Completion } else { Chime::Reject });
    }
}

/// Emit "state-changed" event to frontend with current state.
async fn emit_state_changed(app_handle: &tauri::AppHandle, state: &AppState) {
    let current_state = state.lock().await;
    if let Err(e) = app_handle.emit("state-changed", &*current_state) {
        error!("Failed to emit state-changed event: {}", e);
    }
}

/// Update island window size and visibility based on pending requests and config.
async fn update_window_size(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    config_state: &ConfigState,
) {
    let state = state.lock().await;
    let cfg = config_state.borrow().clone();
    if let Some(window) = app_handle.get_webview_window("island") {
        resize_and_position(&window, &cfg, state.island_content(), false, state.current_request().is_some());
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
    let handle_o = handle.clone();
    let state_o = state.clone();

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
                let state = state_o.clone();
                let win = window.clone();
                let h = handle_o.clone();
                tauri::async_runtime::spawn(async move {
                    let is_vis = win.is_visible().unwrap_or(false);
                    if is_vis {
                        let _ = win.hide();
                        let _ = h.emit("island-closed", ());
                    } else {
                        let cfg = cfg_state.borrow().clone();
                        let content = state.lock().await.island_content();
                        resize_and_position(&win, &cfg, content, true, true);
                        let _ = win.unminimize();
                        let _ = win.show();
                        let _ = win.set_always_on_top(true);
                        let _ = win.set_focus();
                        let _ = h.emit("open-island", ());
                    }
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
    let request_id = {
        let s = state.lock().await;
        match s.current_request() {
            Some(r) => r.id.clone(),
            None => return,
        }
    };

    match reply_to_pending_request(
        &state,
        &client_state,
        &config_state,
        &request_id,
        Some(decision),
    )
    .await
    {
        Ok(true) => {
            info!("Permission {} responded via shortcut", request_id);
            emit_state_changed(&app_handle, &state).await;
            update_window_size(&app_handle, &state, &config_state).await;
        }
        Ok(false) => {}
        Err(e) => {
            error!("Failed to respond to permission {}: {}", request_id, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::AutoApproval;

    #[tokio::test]
    async fn unavailable_automatic_reply_queues_one_copy_for_manual_review() {
        let state = Arc::new(Mutex::new(NotchState::new()));
        let client = Arc::new(Mutex::new(None));
        let mut config = Config::default();
        config.behavior.auto_approval = AutoApproval::AllowOnce;
        let config = ConfigState::new(config);
        let mut recent = VecDeque::new();
        let request = PermissionRequest {
            id: "per_example".into(),
            tool: "shell".into(),
            args: serde_json::json!(["git status"]),
            session_id: "ses_example".into(),
            timestamp: String::new(),
        };

        assert!(
            handle_permission_request(request.clone(), &mut recent, &state, &client, &config).await
        );
        assert!(!handle_permission_request(request, &mut recent, &state, &client, &config).await);
        assert_eq!(state.lock().await.pending_requests.len(), 1);
    }
}
