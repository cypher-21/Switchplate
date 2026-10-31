import { type Component, createSignal } from "solid-js";
import type { Config } from "../lib/types";

interface SettingsModalProps {
  config: Config;
  isPreview?: boolean;
  onTogglePreview?: () => void;
  onSave: (updated: Config) => void;
  onClose: () => void;
}

const SettingsModal: Component<SettingsModalProps> = (props) => {
  const [position, setPosition] = createSignal(props.config.appearance.position || "top-right");
  const [offsetY, setOffsetY] = createSignal(props.config.appearance.offset_y ?? 24);
  const [offsetX, setOffsetX] = createSignal(props.config.appearance.offset_x ?? 0);
  const [onlyOnRequest, setOnlyOnRequest] = createSignal(
    props.config.behavior.only_show_on_request ?? true
  );
  const [theme, setTheme] = createSignal(props.config.appearance.theme || "ai-dark");

  const handleSave = () => {
    const updated: Config = {
      ...props.config,
      appearance: {
        ...props.config.appearance,
        position: position(),
        offset_y: offsetY(),
        offset_x: offsetX(),
        theme: theme(),
      },
      behavior: {
        ...props.config.behavior,
        only_show_on_request: onlyOnRequest(),
      },
    };
    props.onSave(updated);
  };

  return (
    <div class="settings-panel">
      <div class="settings-header">
        <span class="settings-title">Notch Preferences</span>
        <button class="settings-close-btn" onClick={props.onClose} title="Close Settings">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>

      <div class="settings-body">
        {/* Visibility Setting */}
        <div class="settings-row">
          <div class="settings-label-group">
            <span class="settings-label">Auto-hide when idle</span>
            <span class="settings-subtext">Only pop up when OpenCode CLI needs approval</span>
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

        {/* Position Setting */}
        <div class="settings-row">
          <div class="settings-label-group">
            <span class="settings-label">Screen Position</span>
            <span class="settings-subtext">Anchor position on your display</span>
          </div>
          <div class="settings-btn-group">
            <button
              class={`settings-toggle-btn ${position() === "top-right" ? "active" : ""}`}
              onClick={() => setPosition("top-right")}
              title="Top Right Corner (Standard notification area)"
            >
              Top Right
            </button>
            <button
              class={`settings-toggle-btn ${position() === "top-center" ? "active" : ""}`}
              onClick={() => setPosition("top-center")}
              title="Top Center Island"
            >
              Top Center
            </button>
            <button
              class={`settings-toggle-btn ${position() === "top-left" ? "active" : ""}`}
              onClick={() => setPosition("top-left")}
              title="Top Left Corner"
            >
              Top Left
            </button>
            <button
              class={`settings-toggle-btn ${position() === "bottom-right" ? "active" : ""}`}
              onClick={() => setPosition("bottom-right")}
              title="Bottom Right Toast"
            >
              Bottom Right
            </button>
          </div>
        </div>

        {/* Vertical & Horizontal Offset */}
        <div class="settings-row">
          <div class="settings-label-group">
            <span class="settings-label">Vertical Offset</span>
            <span class="settings-subtext">Distance from screen edge ({offsetY()}px)</span>
          </div>
          <input
            type="range"
            min="0"
            max="120"
            step="4"
            value={offsetY()}
            class="settings-slider"
            onInput={(e) => setOffsetY(parseInt(e.currentTarget.value, 10))}
          />
        </div>

        {/* AI Theme Palette */}
        <div class="settings-row">
          <div class="settings-label-group">
            <span class="settings-label">AI Color Palette</span>
            <span class="settings-subtext">Holographic gradient aesthetic</span>
          </div>
          <div class="settings-btn-group">
            <button
              class={`settings-toggle-btn ${theme() === "ai-dark" ? "active" : ""}`}
              onClick={() => setTheme("ai-dark")}
            >
              Obsidian
            </button>
            <button
              class={`settings-toggle-btn ${theme() === "ai-violet" ? "active" : ""}`}
              onClick={() => setTheme("ai-violet")}
            >
              Violet
            </button>
            <button
              class={`settings-toggle-btn ${theme() === "ai-emerald" ? "active" : ""}`}
              onClick={() => setTheme("ai-emerald")}
            >
              Emerald
            </button>
            <button
              class={`settings-toggle-btn ${theme() === "ai-cyber" ? "active" : ""}`}
              onClick={() => setTheme("ai-cyber")}
            >
              Cyan
            </button>
          </div>
        </div>
      </div>

      <div class="settings-footer">
        {props.onTogglePreview && (
          <button
            class="settings-preview-btn"
            onClick={props.onTogglePreview}
            title="Preview how approval cards will look on screen"
          >
            {props.isPreview ? "Hide Preview" : "Preview Card"}
          </button>
        )}
        <button class="settings-save-btn" onClick={handleSave}>
          Apply & Save
        </button>
      </div>
    </div>
  );
};

export default SettingsModal;
