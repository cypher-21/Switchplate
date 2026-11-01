use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

fn default_server_url() -> String {
    String::from("http://localhost:4096")
}

fn default_password_env() -> String {
    String::from("OPENCODE_API_PASSWORD")
}

fn default_card_width() -> u32 {
    460
}

fn default_card_height() -> u32 {
    240
}

fn default_collapsed_width() -> u32 {
    280
}

fn default_collapsed_height() -> u32 {
    38
}

fn default_position() -> String {
    String::from("top-center")
}

fn default_theme() -> String {
    String::from("industrial-swiss")
}

fn default_offset_y() -> i32 {
    12
}

fn default_offset_x() -> i32 {
    0
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_timeout() -> u32 {
    30
}

fn default_shortcut_open_island() -> String {
    String::from("ctrl+shift+o")
}

fn default_shortcut_accept() -> String {
    String::from("ctrl+shift+a")
}

fn default_shortcut_deny() -> String {
    String::from("ctrl+shift+d")
}

fn default_shortcut_always() -> String {
    String::from("ctrl+shift+w")
}

fn default_shortcut_settings() -> String {
    String::from("ctrl+shift+s")
}

/// Root configuration loaded from ~/.config/switchplate/config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub opencode: OpenCodeConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,
}

/// Configuration for connecting to the OpenCode server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeConfig {
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default = "default_password_env")]
    pub password_env: String,
    #[serde(default = "default_timeout")]
    pub request_timeout: u32,
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
}

/// Visual appearance settings (Industrial Swiss design system).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default = "default_position")]
    pub position: String,
    #[serde(default = "default_card_width")]
    pub card_width: u32,
    #[serde(default = "default_card_height")]
    pub card_height: u32,
    #[serde(default = "default_collapsed_width")]
    pub collapsed_width: u32,
    #[serde(default = "default_collapsed_height")]
    pub collapsed_height: u32,
    #[serde(default = "default_offset_x")]
    pub offset_x: i32,
    #[serde(default = "default_offset_y")]
    pub offset_y: i32,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_false")]
    pub reduced_motion: bool,
}

/// Behavioral settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    /// If true, the island completely hides when idle, only surfacing on approvals.
    #[serde(default = "default_true")]
    pub only_show_on_request: bool,
    #[serde(default = "default_false")]
    pub pause_monitoring: bool,
    #[serde(default = "default_false")]
    pub sound_enabled: bool,
    #[serde(default = "default_true")]
    pub autostart: bool,
}

/// Global keyboard shortcuts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    #[serde(default = "default_shortcut_open_island")]
    pub open_island: String,
    #[serde(default = "default_shortcut_accept")]
    pub accept: String,
    #[serde(default = "default_shortcut_deny")]
    pub deny: String,
    #[serde(default = "default_shortcut_always")]
    pub always: String,
    #[serde(default = "default_shortcut_settings")]
    pub open_settings: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            opencode: OpenCodeConfig::default(),
            appearance: AppearanceConfig::default(),
            behavior: BehaviorConfig::default(),
            shortcuts: ShortcutsConfig::default(),
        }
    }
}

impl Default for OpenCodeConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            password_env: default_password_env(),
            request_timeout: default_timeout(),
            auto_reconnect: default_true(),
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            position: default_position(),
            card_width: default_card_width(),
            card_height: default_card_height(),
            collapsed_width: default_collapsed_width(),
            collapsed_height: default_collapsed_height(),
            offset_x: default_offset_x(),
            offset_y: default_offset_y(),
            theme: default_theme(),
            reduced_motion: default_false(),
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            only_show_on_request: default_true(),
            pause_monitoring: default_false(),
            sound_enabled: default_false(),
            autostart: default_true(),
        }
    }
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            open_island: default_shortcut_open_island(),
            accept: default_shortcut_accept(),
            deny: default_shortcut_deny(),
            always: default_shortcut_always(),
            open_settings: default_shortcut_settings(),
        }
    }
}

impl Config {
    /// Load configuration from ~/.config/switchplate/config.toml.
    /// Checks for backward-compatibility with ~/.config/opencode-notch/config.toml.
    pub fn load() -> Self {
        let primary_path = Self::config_path();

        if let Some(ref path) = primary_path {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    if let Ok(config) = toml::from_str::<Config>(&contents) {
                        info!("Loaded Switchplate configuration from {}", path.display());
                        return config;
                    }
                }
            }
        }

        // Backward compatibility migration: check old paths
        if let Some(legacy_path) = Self::legacy_config_path() {
            if legacy_path.exists() {
                info!("Found legacy config at {}. Migrating...", legacy_path.display());
                if let Ok(contents) = std::fs::read_to_string(&legacy_path) {
                    if let Ok(mut config) = toml::from_str::<Config>(&contents) {
                        config.appearance.position = "top-center".to_string();
                        config.appearance.theme = "industrial-swiss".to_string();
                        let _ = config.save();
                        return config;
                    }
                }
            }
        }

        // Create new default config
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    /// Save configuration to ~/.config/switchplate/config.toml.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or_else(|| "Could not determine config path".to_string())?;
        self.save_to_path(&path);
        Ok(())
    }

    fn save_to_path(&self, path: &PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let toml_content = format!(
r#"# SWITCHPLATE — Configuration
# Your control surface for OpenCode.
# Location: ~/.config/switchplate/config.toml

[opencode]
server_url = "{server_url}"
password_env = "{password_env}"
request_timeout = {request_timeout}
auto_reconnect = {auto_reconnect}

[appearance]
position = "{position}"
offset_x = {offset_x}
offset_y = {offset_y}
card_width = {card_width}
card_height = {card_height}
collapsed_width = {collapsed_width}
collapsed_height = {collapsed_height}
theme = "{theme}"
reduced_motion = {reduced_motion}

[behavior]
only_show_on_request = {only_show_on_request}
pause_monitoring = {pause_monitoring}
sound_enabled = {sound_enabled}
autostart = {autostart}

[shortcuts]
open_island = "{open_island}"
accept = "{accept}"
deny = "{deny}"
always = "{always}"
open_settings = "{open_settings}"
"#,
            server_url = self.opencode.server_url,
            password_env = self.opencode.password_env,
            request_timeout = self.opencode.request_timeout,
            auto_reconnect = self.opencode.auto_reconnect,
            position = self.appearance.position,
            offset_x = self.appearance.offset_x,
            offset_y = self.appearance.offset_y,
            card_width = self.appearance.card_width,
            card_height = self.appearance.card_height,
            collapsed_width = self.appearance.collapsed_width,
            collapsed_height = self.appearance.collapsed_height,
            theme = self.appearance.theme,
            reduced_motion = self.appearance.reduced_motion,
            only_show_on_request = self.behavior.only_show_on_request,
            pause_monitoring = self.behavior.pause_monitoring,
            sound_enabled = self.behavior.sound_enabled,
            autostart = self.behavior.autostart,
            open_island = self.shortcuts.open_island,
            accept = self.shortcuts.accept,
            deny = self.shortcuts.deny,
            always = self.shortcuts.always,
            open_settings = self.shortcuts.open_settings,
        );

        if let Err(e) = std::fs::write(path, toml_content) {
            warn!("Failed to write config file to {}: {}", path.display(), e);
        }
    }

    /// Primary config path: ~/.config/switchplate/config.toml
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("switchplate").join("config.toml"))
    }

    /// Legacy config paths for migration: ~/.config/opencode-notch/config.toml
    pub fn legacy_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("opencode-notch").join("config.toml"))
    }

    /// Resolve the API password from the environment variable specified in config.
    pub fn resolve_password(&self) -> String {
        std::env::var(&self.opencode.password_env)
            .or_else(|_| std::env::var("OPENCODE_SERVER_PASSWORD"))
            .unwrap_or_default()
    }
}
