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
    440
}

fn default_card_height() -> u32 {
    240
}

fn default_collapsed_width() -> u32 {
    140
}

fn default_collapsed_height() -> u32 {
    34
}

fn default_position() -> String {
    String::from("top-right")
}

fn default_theme() -> String {
    String::from("ai-dark")
}

fn default_offset_y() -> i32 {
    // Sleek 24px float from top edge (safely below GNOME/KDE panel)
    24
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

fn default_shortcut_accept() -> String {
    String::from("ctrl+shift+a")
}

fn default_shortcut_deny() -> String {
    String::from("ctrl+shift+d")
}

fn default_shortcut_always() -> String {
    String::from("ctrl+shift+w")
}

fn default_shortcut_toggle() -> String {
    String::from("ctrl+shift+n")
}

/// Root configuration loaded from ~/.config/opencode-notch/config.toml.
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
}

/// Visual appearance settings.
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
    #[serde(default = "default_true")]
    pub accent_glow: bool,
}

/// Behavioral settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    /// If true, the notch is completely hidden when idle, and only appears
    /// when OpenCode requests permission.
    #[serde(default = "default_true")]
    pub only_show_on_request: bool,
    #[serde(default = "default_false")]
    pub sound_enabled: bool,
}

/// Global keyboard shortcuts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    #[serde(default = "default_shortcut_accept")]
    pub accept: String,
    #[serde(default = "default_shortcut_deny")]
    pub deny: String,
    #[serde(default = "default_shortcut_always")]
    pub always: String,
    #[serde(default = "default_shortcut_toggle")]
    pub toggle: String,
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
            accent_glow: default_true(),
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            only_show_on_request: default_true(),
            sound_enabled: default_false(),
        }
    }
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            accept: default_shortcut_accept(),
            deny: default_shortcut_deny(),
            always: default_shortcut_always(),
            toggle: default_shortcut_toggle(),
        }
    }
}

impl Config {
    /// Load configuration from ~/.config/opencode-notch/config.toml.
    /// Creates the file with annotated defaults if it doesn't exist.
    pub fn load() -> Self {
        let config_path = Self::config_path();

        let Some(path) = config_path else {
            info!("No config directory found, using default configuration");
            return Self::default();
        };

        if !path.exists() {
            info!("Creating default configuration file at {}", path.display());
            let default_cfg = Self::default();
            default_cfg.save_to_path(&path);
            return default_cfg;
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(config) => {
                    info!("Loaded configuration from {}", path.display());
                    config
                }
                Err(e) => {
                    warn!(
                        "Malformed config file at {}: {}. Using defaults",
                        path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(e) => {
                warn!(
                    "Could not read config file at {}: {}. Using defaults",
                    path.display(),
                    e
                );
                Self::default()
            }
        }
    }

    /// Save configuration to ~/.config/opencode-notch/config.toml.
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
r#"# OpenCode Notch Configuration
# Location: ~/.config/opencode-notch/config.toml

[behavior]
# When true, the notch stays completely hidden when idle.
# It only pops up when OpenCode CLI needs your approval (Allow / Deny / Always).
only_show_on_request = {only_show_on_request}

# Enable subtle notification sounds on incoming requests
sound_enabled = {sound_enabled}

[appearance]
# Position on screen: "top-center", "top-right", "top-left", "bottom-right", "center"
position = "{position}"

# Offset in pixels from the anchor edge
offset_x = {offset_x}
offset_y = {offset_y}

# Card dimensions when expanded for permission requests
card_width = {card_width}
card_height = {card_height}

# Pill dimensions if only_show_on_request is false
collapsed_width = {collapsed_width}
collapsed_height = {collapsed_height}

# AI Theme: "ai-dark", "ai-violet", "ai-emerald", "ai-cyber"
theme = "{theme}"

# Enable AI holographic gradient border glow
accent_glow = {accent_glow}

[shortcuts]
accept = "{accept}"
deny = "{deny}"
always = "{always}"
toggle = "{toggle}"

[opencode]
# OpenCode service is automatically discovered from ~/.local/state/opencode/service.json.
# Override only if you run a custom server address.
server_url = "{server_url}"
password_env = "{password_env}"
"#,
            only_show_on_request = self.behavior.only_show_on_request,
            sound_enabled = self.behavior.sound_enabled,
            position = self.appearance.position,
            offset_x = self.appearance.offset_x,
            offset_y = self.appearance.offset_y,
            card_width = self.appearance.card_width,
            card_height = self.appearance.card_height,
            collapsed_width = self.appearance.collapsed_width,
            collapsed_height = self.appearance.collapsed_height,
            theme = self.appearance.theme,
            accent_glow = self.appearance.accent_glow,
            accept = self.shortcuts.accept,
            deny = self.shortcuts.deny,
            always = self.shortcuts.always,
            toggle = self.shortcuts.toggle,
            server_url = self.opencode.server_url,
            password_env = self.opencode.password_env,
        );

        if let Err(e) = std::fs::write(path, toml_content) {
            warn!("Failed to write config file to {}: {}", path.display(), e);
        }
    }

    /// Returns the expected path to the configuration file.
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("opencode-notch").join("config.toml"))
    }

    /// Resolve the API password from the environment variable specified in config.
    pub fn resolve_password(&self) -> String {
        std::env::var(&self.opencode.password_env)
            .or_else(|_| std::env::var("OPENCODE_SERVER_PASSWORD"))
            .unwrap_or_default()
    }
}
