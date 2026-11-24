use crate::config::Config;
use crate::state::IslandContent;
use tauri::WebviewWindow;
use tracing::{debug, warn};

/// Position and resize the Switchplate window based on the active card.
///
/// Questions get enough room for scrollable answers and a fixed submit button.
pub fn resize_and_position(
    window: &WebviewWindow,
    config: &Config,
    content: IslandContent,
    force_show: bool,
    focus: bool,
) {
    let has_card = content != IslandContent::Idle;
    if !has_card && !force_show && config.behavior.only_show_on_request {
        debug!("No pending requests and only_show_on_request is enabled: hiding window");
        let _ = window.hide();
        return;
    }

    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let (monitor_width, monitor_height, monitor_x, monitor_y) = get_monitor_dimensions(window);

    let (logical_width, logical_height) = if content == IslandContent::Form {
        (
            f64::from(config.appearance.card_width)
                .max(400.0)
                .min((monitor_width - 48.0).max(300.0)),
            f64::from(config.appearance.card_height)
                .max(520.0)
                .min((monitor_height - 100.0).max(240.0)),
        )
    } else if has_card || force_show {
        (
            config.appearance.card_width as f64,
            config.appearance.card_height as f64,
        )
    } else {
        (
            config.appearance.collapsed_width as f64,
            config.appearance.collapsed_height as f64,
        )
    };

    // Resize window
    if let Ok(current_size) = window.inner_size() {
        let current_logical = current_size.to_logical::<f64>(scale_factor);
        if (current_logical.width - logical_width).abs() >= 1.0
            || (current_logical.height - logical_height).abs() >= 1.0
        {
            debug!(
                "Resizing Switchplate to {}x{} (scale: {})",
                logical_width, logical_height, scale_factor
            );
            if let Err(e) = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                logical_width,
                logical_height,
            ))) {
                warn!("Failed to resize window: {}", e);
            }
        }
    }

    let (x, y) = calculate_coordinates(
        &config.appearance.position,
        monitor_width,
        monitor_height,
        monitor_x,
        monitor_y,
        logical_width,
        logical_height,
        config.appearance.offset_x as f64,
        config.appearance.offset_y as f64,
    );

    debug!(
        "Positioning Switchplate at logical ({}, {}) for anchor '{}'",
        x, y, config.appearance.position
    );

    if let Err(e) = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)))
    {
        debug!("Position adjustment error: {}", e);
    }

    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    if focus {
        let _ = window.set_focus();
    }
}

/// Calculate screen coordinates from position anchor and offsets.
fn calculate_coordinates(
    position: &str,
    monitor_width: f64,
    monitor_height: f64,
    monitor_x: f64,
    monitor_y: f64,
    width: f64,
    height: f64,
    offset_x: f64,
    offset_y: f64,
) -> (f64, f64) {
    // GNOME Top Panel is ~38px. top_margin ensures the notch floats cleanly below it.
    let top_margin = 42.0;

    match position.to_lowercase().as_str() {
        "top-right" => (
            monitor_x + monitor_width - width - 24.0 + offset_x,
            monitor_y + top_margin + offset_y,
        ),
        "top-left" => (
            monitor_x + 24.0 + offset_x,
            monitor_y + top_margin + offset_y,
        ),
        "bottom-right" => (
            monitor_x + monitor_width - width - 24.0 + offset_x,
            monitor_y + monitor_height - height - 24.0 + offset_y,
        ),
        "center" => (
            monitor_x + (monitor_width - width) / 2.0 + offset_x,
            monitor_y + (monitor_height - height) / 2.0 + offset_y,
        ),
        // Default "top-center"
        _ => (
            monitor_x + (monitor_width - width) / 2.0 + offset_x,
            monitor_y + top_margin + offset_y,
        ),
    }
}

/// Returns (width, height, x, y) in logical coordinates for the primary monitor.
fn get_monitor_dimensions(window: &WebviewWindow) -> (f64, f64, f64, f64) {
    let scale_factor = window.scale_factor().unwrap_or(1.0);

    let monitor = window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten());

    match monitor {
        Some(m) => {
            let logical_size = m.size().to_logical::<f64>(scale_factor);
            let logical_pos = m.position().to_logical::<f64>(scale_factor);
            (
                logical_size.width,
                logical_size.height,
                logical_pos.x,
                logical_pos.y,
            )
        }
        None => (1920.0, 1080.0, 0.0, 0.0),
    }
}
