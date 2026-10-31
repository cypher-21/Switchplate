import { type Component, createSignal, onMount, onCleanup } from "solid-js";
import type { Config, NotchState } from "../lib/types";
import SwitchplateIcon from "./SwitchplateIcon";

interface SettingsWindowProps {
  config: Config;
  state: NotchState;
  onSaveConfig: (updated: Config) => void;
  onClose: () => void;
}

type TabKey = "opencode" | "appearance" | "behaviour" | "shortcuts" | "about";

const SettingsWindow: Component<SettingsWindowProps> = (props) => {
  const [activeTab, setActiveTab] = createSignal<TabKey>("opencode");

  // Form signals initialized from config
  const [serverUrl, setServerUrl] = createSignal(props.config.opencode.server_url || "http://127.0.0.1:49374");
  const [timeoutSec, setTimeoutSec] = createSignal(props.config.opencode.request_timeout || 30);
  const [autoReconnect, setAutoReconnect] = createSignal(props.config.opencode.auto_reconnect ?? true);
  const [position, setPosition] = createSignal(props.config.appearance.position || "top-center");
  const [offsetY, setOffsetY] = createSignal(props.config.appearance.offset_y ?? 12);
  const [theme, setTheme] = createSignal(props.config.appearance.theme || "industrial-swiss");
  const [reducedMotion, setReducedMotion] = createSignal(props.config.appearance.reduced_motion ?? false);
  const [onlyOnRequest, setOnlyOnRequest] = createSignal(props.config.behavior.only_show_on_request ?? true);
  const [pauseMonitoring, setPauseMonitoring] = createSignal(props.config.behavior.pause_monitoring ?? false);
  const [soundEnabled, setSoundEnabled] = createSignal(props.config.behavior.sound_enabled ?? false);
  const [savedNotice, setSavedNotice] = createSignal(false);

  const isConnected = () => props.state.connection.status === "connected";

  // Handle Escape key to close settings
  onMount(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        props.onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => window.removeEventListener("keydown", handleKeyDown));
  });

  const handleApply = () => {
    const updated: Config = {
      ...props.config,
      opencode: {
        ...props.config.opencode,
        server_url: serverUrl(),
        request_timeout: timeoutSec(),
        auto_reconnect: autoReconnect(),
      },
      appearance: {
        ...props.config.appearance,
        position: position(),
        offset_y: offsetY(),
        theme: theme(),
        reduced_motion: reducedMotion(),
      },
      behavior: {
        ...props.config.behavior,
        only_show_on_request: onlyOnRequest(),
        pause_monitoring: pauseMonitoring(),
        sound_enabled: soundEnabled(),
      },
    };
    props.onSaveConfig(updated);
    setSavedNotice(true);
    window.setTimeout(() => setSavedNotice(false), 2500);
  };

  return (
    <div class="settings-window">
      {/* Main Container */}
      <div class="settings-window__main">
        {/* Left Sidebar */}
        <nav class="settings-sidebar">
          <div class="settings-sidebar__brand">
            <SwitchplateIcon size={18} color="#B58B53" />
            <span class="settings-sidebar__brand-title">SWITCHPLATE</span>
          </div>

          <button
            class={`settings-nav-item ${activeTab() === "opencode" ? "active" : ""}`}
            onClick={() => setActiveTab("opencode")}
            type="button"
          >
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
              <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
            </svg>
            <span>OpenCode</span>
          </button>

          <button
            class={`settings-nav-item ${activeTab() === "appearance" ? "active" : ""}`}
            onClick={() => setActiveTab("appearance")}
            type="button"
          >
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="2" y="3" width="20" height="14" rx="2" />
              <line x1="8" y1="21" x2="16" y2="21" />
              <line x1="12" y1="17" x2="12" y2="21" />
            </svg>
            <span>Appearance</span>
          </button>

          <button
            class={`settings-nav-item ${activeTab() === "behaviour" ? "active" : ""}`}
            onClick={() => setActiveTab("behaviour")}
            type="button"
          >
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="4" y1="21" x2="4" y2="14" />
              <line x1="4" y1="10" x2="4" y2="3" />
              <line x1="12" y1="21" x2="12" y2="12" />
              <line x1="12" y1="8" x2="12" y2="3" />
              <line x1="20" y1="21" x2="20" y2="16" />
              <line x1="20" y1="12" x2="20" y2="3" />
              <line x1="1" y1="14" x2="7" y2="14" />
              <line x1="9" y1="8" x2="15" y2="8" />
              <line x1="17" y1="16" x2="23" y2="16" />
            </svg>
            <span>Behaviour</span>
          </button>

          <button
            class={`settings-nav-item ${activeTab() === "shortcuts" ? "active" : ""}`}
            onClick={() => setActiveTab("shortcuts")}
            type="button"
          >
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="2" y="4" width="20" height="16" rx="2" />
              <line x1="6" y1="8" x2="6" y2="8" />
              <line x1="10" y1="8" x2="10" y2="8" />
              <line x1="14" y1="8" x2="14" y2="8" />
              <line x1="18" y1="8" x2="18" y2="8" />
              <line x1="6" y1="12" x2="6" y2="12" />
              <line x1="18" y1="12" x2="18" y2="12" />
              <line x1="8" y1="16" x2="16" y2="16" />
            </svg>
            <span>Shortcuts</span>
          </button>

          <button
            class={`settings-nav-item ${activeTab() === "about" ? "active" : ""}`}
            onClick={() => setActiveTab("about")}
            type="button"
          >
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="16" x2="12" y2="12" />
              <line x1="12" y1="8" x2="12.01" y2="8" />
            </svg>
            <span>About</span>
          </button>
        </nav>

        {/* Right Content Area */}
        <div class="settings-content">
          {/* TAB 1: OPENCODE */}
          {activeTab() === "opencode" && (
            <div class="settings-tab-pane">
              <div class="settings-pane-header">
                <h2 class="settings-pane-title">OpenCode connection</h2>
                <p class="settings-pane-subtitle">Configure how Switchplate connects to OpenCode.</p>
              </div>

              {/* Status Card matching screenshot */}
              <div class="connection-status-card">
                <div class="connection-status-card__icon">
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#B58B53" stroke-width="2">
                    <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
                    <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
                  </svg>
                </div>
                <div class="connection-status-card__info">
                  <span class="connection-status-card__headline">
                    {isConnected() ? "CONNECTED" : "OFFLINE"}
                  </span>
                  <span class="connection-status-card__subtext">
                    {isConnected()
                      ? "OpenCode is running and ready."
                      : "OpenCode server not detected. Waiting for service..."}
                  </span>
                </div>
              </div>

              {/* Form Controls */}
              <div class="settings-form-row">
                <label class="settings-form-label">Endpoint URL</label>
                <div class="settings-input-group">
                  <span class="input-icon">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
                      <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
                    </svg>
                  </span>
                  <input
                    class="settings-input"
                    type="text"
                    value={serverUrl()}
                    onInput={(e) => setServerUrl(e.currentTarget.value)}
                    placeholder="http://127.0.0.1:49374"
                  />
                </div>
              </div>

              <div class="settings-form-row">
                <label class="settings-form-label">Request timeout</label>
                <div class="settings-select-group">
                  <span class="input-icon">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="12" cy="12" r="10" />
                      <polyline points="12 6 12 12 16 14" />
                    </svg>
                  </span>
                  <select
                    class="settings-select"
                    value={timeoutSec()}
                    onChange={(e) => setTimeoutSec(parseInt(e.currentTarget.value, 10))}
                  >
                    <option value="15">15 s</option>
                    <option value="30">30 s</option>
                    <option value="60">60 s</option>
                    <option value="120">120 s</option>
                  </select>
                </div>
              </div>

              <div class="settings-toggle-row">
                <div class="settings-toggle-info">
                  <div class="settings-toggle-title">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <polyline points="23 4 23 10 17 10" />
                      <polyline points="1 20 1 14 7 14" />
                      <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
                    </svg>
                    <span>Auto-reconnect</span>
                  </div>
                  <span class="settings-toggle-subtext">Reconnect automatically if the connection is lost.</span>
                </div>
                <label class="switch">
                  <input
                    type="checkbox"
                    checked={autoReconnect()}
                    onChange={(e) => setAutoReconnect(e.currentTarget.checked)}
                  />
                  <span class="slider" />
                </label>
              </div>
            </div>
          )}

          {/* TAB 2: APPEARANCE */}
          {activeTab() === "appearance" && (
            <div class="settings-tab-pane">
              <div class="settings-pane-header">
                <h2 class="settings-pane-title">Appearance & Geometry</h2>
                <p class="settings-pane-subtitle">Configure desktop placement and spacing.</p>
              </div>

              <div class="settings-form-row">
                <label class="settings-form-label">Screen Position</label>
                <div class="segmented-control">
                  <button
                    class={`segmented-btn ${position() === "top-center" ? "active" : ""}`}
                    onClick={() => setPosition("top-center")}
                    type="button"
                  >
                    Top Center
                  </button>
                  <button
                    class={`segmented-btn ${position() === "top-right" ? "active" : ""}`}
                    onClick={() => setPosition("top-right")}
                    type="button"
                  >
                    Top Right
                  </button>
                  <button
                    class={`segmented-btn ${position() === "top-left" ? "active" : ""}`}
                    onClick={() => setPosition("top-left")}
                    type="button"
                  >
                    Top Left
                  </button>
                </div>
              </div>

              <div class="settings-form-row">
                <div class="slider-label-group">
                  <label class="settings-form-label">Top Edge Offset</label>
                  <span class="slider-readout">{offsetY()} px</span>
                </div>
                <input
                  class="range-slider"
                  type="range"
                  min="0"
                  max="120"
                  step="4"
                  value={offsetY()}
                  onInput={(e) => setOffsetY(parseInt(e.currentTarget.value, 10))}
                />
              </div>

              <div class="settings-toggle-row">
                <div class="settings-toggle-info">
                  <span class="settings-toggle-title">Reduced Motion</span>
                  <span class="settings-toggle-subtext">Disable mechanical transition animations.</span>
                </div>
                <label class="switch">
                  <input
                    type="checkbox"
                    checked={reducedMotion()}
                    onChange={(e) => setReducedMotion(e.currentTarget.checked)}
                  />
                  <span class="slider" />
                </label>
              </div>
            </div>
          )}

          {/* TAB 3: BEHAVIOUR */}
          {activeTab() === "behaviour" && (
            <div class="settings-tab-pane">
              <div class="settings-pane-header">
                <h2 class="settings-pane-title">Runtime Behaviour</h2>
                <p class="settings-pane-subtitle">Manage monitoring and presence.</p>
              </div>

              <div class="settings-toggle-row">
                <div class="settings-toggle-info">
                  <span class="settings-toggle-title">Auto-hide when idle</span>
                  <span class="settings-toggle-subtext">Only show when OpenCode requests approval.</span>
                </div>
                <label class="switch">
                  <input
                    type="checkbox"
                    checked={onlyOnRequest()}
                    onChange={(e) => setOnlyOnRequest(e.currentTarget.checked)}
                  />
                  <span class="slider" />
                </label>
              </div>

              <div class="settings-toggle-row">
                <div class="settings-toggle-info">
                  <span class="settings-toggle-title">Pause Monitoring</span>
                  <span class="settings-toggle-subtext">Temporarily stop intercepting OpenCode events.</span>
                </div>
                <label class="switch">
                  <input
                    type="checkbox"
                    checked={pauseMonitoring()}
                    onChange={(e) => setPauseMonitoring(e.currentTarget.checked)}
                  />
                  <span class="slider" />
                </label>
              </div>

              <div class="settings-toggle-row">
                <div class="settings-toggle-info">
                  <span class="settings-toggle-title">Audio notification</span>
                  <span class="settings-toggle-subtext">Play subtle hardware click on incoming approval requests.</span>
                </div>
                <label class="switch">
                  <input
                    type="checkbox"
                    checked={soundEnabled()}
                    onChange={(e) => setSoundEnabled(e.currentTarget.checked)}
                  />
                  <span class="slider" />
                </label>
              </div>
            </div>
          )}

          {/* TAB 4: SHORTCUTS */}
          {activeTab() === "shortcuts" && (
            <div class="settings-tab-pane">
              <div class="settings-pane-header">
                <h2 class="settings-pane-title">Keyboard Shortcuts</h2>
                <p class="settings-pane-subtitle">Global hardware keybindings registered with your desktop.</p>
              </div>

              <div class="shortcut-list">
                <div class="shortcut-item">
                  <div class="shortcut-desc">
                    <span class="shortcut-name">Accept Request</span>
                    <span class="shortcut-subtext">Approve current tool execution</span>
                  </div>
                  <kbd class="keycap">CTRL + SHIFT + A</kbd>
                </div>

                <div class="shortcut-item">
                  <div class="shortcut-desc">
                    <span class="shortcut-name">Reject Request</span>
                    <span class="shortcut-subtext">Deny permission and halt tool</span>
                  </div>
                  <kbd class="keycap">CTRL + SHIFT + D</kbd>
                </div>

                <div class="shortcut-item">
                  <div class="shortcut-desc">
                    <span class="shortcut-name">Always Allow</span>
                    <span class="shortcut-subtext">Authorize tool command for the entire session</span>
                  </div>
                  <kbd class="keycap">CTRL + SHIFT + W</kbd>
                </div>

                <div class="shortcut-item">
                  <div class="shortcut-desc">
                    <span class="shortcut-name">Open / Toggle Switchplate</span>
                    <span class="shortcut-subtext">Focus or toggle the control surface</span>
                  </div>
                  <kbd class="keycap">CTRL + SHIFT + O</kbd>
                </div>

                <div class="shortcut-item">
                  <div class="shortcut-desc">
                    <span class="shortcut-name">Open Settings</span>
                    <span class="shortcut-subtext">Open preferences panel</span>
                  </div>
                  <kbd class="keycap">CTRL + SHIFT + S</kbd>
                </div>
              </div>
            </div>
          )}

          {/* TAB 5: ABOUT */}
          {activeTab() === "about" && (
            <div class="settings-tab-pane">
              <div class="about-hero">
                <div class="about-hero__logo">
                  <SwitchplateIcon size={38} color="#B58B53" />
                </div>
                <h1 class="about-hero__title">SWITCHPLATE</h1>
                <p class="about-hero__subtitle">OpenCode Desktop Companion</p>
                <p class="about-hero__tagline">Your control surface for OpenCode.</p>
                <span class="about-version">Version 1.0.0 — Precision Instrument Interface</span>
              </div>

              <div class="diagnostics-card">
                <span class="diagnostics-title">System Diagnostics</span>
                <div class="diagnostics-grid">
                  <div class="diagnostics-item">
                    <span class="diag-label">Runtime</span>
                    <span class="diag-value">Tauri 2.12 Native Engine</span>
                  </div>
                  <div class="diagnostics-item">
                    <span class="diag-label">Display Server</span>
                    <span class="diag-value">Linux XWayland Bridge</span>
                  </div>
                  <div class="diagnostics-item">
                    <span class="diag-label">OpenCode Protocol</span>
                    <span class="diag-value">v2.2 Server-Sent Events</span>
                  </div>
                  <div class="diagnostics-item">
                    <span class="diag-label">Active State</span>
                    <span class="diag-value">{isConnected() ? "Connected (Live SSE Stream)" : "Disconnected"}</span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Footer */}
          <div class="settings-content__footer">
            <button class="settings-cancel-btn" onClick={props.onClose} type="button">
              Cancel
            </button>
            <span class="settings-status-note">
              {savedNotice() ? "Changes applied and saved." : "Saved to ~/.config/switchplate/config.toml"}
            </span>
            <button class="settings-apply-btn" onClick={handleApply} type="button">
              Apply & Save
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SettingsWindow;
