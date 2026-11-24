use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, warn};

use crate::opencode::events::Decision;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub auto_approval: AutoApproval,
    #[serde(default = "default_true")]
    pub sound_enabled: bool,
    #[serde(default = "default_true")]
    pub autostart: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoApproval {
    #[default]
    Manual,
    AllowOnce,
    AlwaysAllow,
}

impl BehaviorConfig {
    pub fn automatic_decision(&self) -> Option<Decision> {
        if self.pause_monitoring {
            return None;
        }

        match self.auto_approval {
            AutoApproval::Manual => None,
            AutoApproval::AllowOnce => Some(Decision::Allow),
            AutoApproval::AlwaysAllow => Some(Decision::Always),
        }
    }
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
            auto_approval: AutoApproval::default(),
            sound_enabled: default_true(),
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
                match std::fs::read_to_string(path) {
                    Ok(contents) => {
                        match toml::from_str::<Config>(&contents) {
                            Ok(config) => {
                                info!("Loaded Switchplate configuration from {}", path.display());
                                return config;
                            }
                            Err(parse_err) => {
                                // File exists but is malformed — back it up, don't overwrite.
                                error!(
                                    "Failed to parse config at {}: {}. Using defaults in memory.",
                                    path.display(),
                                    parse_err
                                );
                                let backup_path = path.with_extension("toml.bak");
                                match std::fs::copy(path, &backup_path) {
                                    Ok(_) => warn!(
                                        "Backed up malformed config to {}",
                                        backup_path.display()
                                    ),
                                    Err(e) => warn!(
                                        "Could not back up malformed config to {}: {}",
                                        backup_path.display(),
                                        e
                                    ),
                                }
                                // Return defaults WITHOUT saving — preserve the (backed-up) file.
                                return Self::default();
                            }
                        }
                    }
                    Err(read_err) => {
                        error!(
                            "Could not read config file at {}: {}. Using defaults.",
                            path.display(),
                            read_err
                        );
                        return Self::default();
                    }
                }
            }
        }

        // Backward compatibility migration: check old paths
        if let Some(legacy_path) = Self::legacy_config_path() {
            if legacy_path.exists() {
                info!("Found legacy config at {}. Migrating...", legacy_path.display());
                match std::fs::read_to_string(&legacy_path) {
                    Ok(contents) => {
                        match toml::from_str::<Config>(&contents) {
                            Ok(mut config) => {
                                config.appearance.position = "top-center".to_string();
                                config.appearance.theme = "industrial-swiss".to_string();
                                let _ = config.save();
                                return config;
                            }
                            Err(parse_err) => {
                                error!(
                                    "Failed to parse legacy config at {}: {}. Using defaults.",
                                    legacy_path.display(),
                                    parse_err
                                );
                                let backup_path = legacy_path.with_extension("toml.bak");
                                match std::fs::copy(&legacy_path, &backup_path) {
                                    Ok(_) => warn!(
                                        "Backed up malformed legacy config to {}",
                                        backup_path.display()
                                    ),
                                    Err(e) => warn!(
                                        "Could not back up malformed legacy config: {}",
                                        e
                                    ),
                                }
                                // Fall through to create fresh defaults.
                            }
                        }
                    }
                    Err(read_err) => {
                        warn!(
                            "Could not read legacy config at {}: {}",
                            legacy_path.display(),
                            read_err
                        );
                    }
                }
            }
        }

        // No config file exists at all — create new default config and save it.
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    /// Save configuration to ~/.config/switchplate/config.toml.
    pub fn save(&self) -> Result<(), String> {
        let path =
            Self::config_path().ok_or_else(|| "Could not determine config path".to_string())?;
        let contents = toml::to_string_pretty(self)
            .map_err(|e| format!("Could not serialize configuration: {e}"))?;
        let contents = format!("# SWITCHPLATE — Configuration\n\n{contents}");
        let write_config = || -> std::io::Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let temporary_path = path.with_extension(format!("{}.tmp", std::process::id()));
            std::fs::write(&temporary_path, contents)?;
            std::fs::rename(&temporary_path, &path)
        };

        write_config()
            .map_err(|e| format!("Could not save configuration to {}: {e}", path.display()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_configs_require_manual_approval() {
        let config: Config = toml::from_str("[behavior]\nsound_enabled = false").unwrap();
        assert_eq!(config.behavior.auto_approval, AutoApproval::Manual);
        assert_eq!(config.behavior.automatic_decision(), None);
    }

    #[test]
    fn automatic_modes_round_trip_and_respect_pause() {
        for (mode, decision) in [
            (AutoApproval::AllowOnce, Decision::Allow),
            (AutoApproval::AlwaysAllow, Decision::Always),
        ] {
            let mut config = Config::default();
            config.behavior.auto_approval = mode;
            let mut restored: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            assert_eq!(restored.behavior.automatic_decision(), Some(decision));
            restored.behavior.pause_monitoring = true;
            assert_eq!(restored.behavior.automatic_decision(), None);
        }
    }
}
