import { type Component, createSignal, onMount, onCleanup, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import type { NotchState, Decision, Config, PermissionRequest } from "./lib/types";
import { getState, respondPermission, getConfig, saveConfig } from "./lib/tauri-bridge";
import StatusDot from "./components/StatusDot";
import ApprovalCard from "./components/ApprovalCard";
import SettingsModal from "./components/SettingsModal";

const DEFAULT_STATE: NotchState = {
  connection: { status: "disconnected" },
  pending_requests: [],
};

const DEFAULT_CONFIG: Config = {
  opencode: {
    server_url: "http://localhost:4096",
    password_env: "OPENCODE_API_PASSWORD",
  },
  appearance: {
    position: "top-right",
    card_width: 440,
    card_height: 240,
    collapsed_width: 140,
    collapsed_height: 34,
    offset_x: 0,
    offset_y: 24,
    theme: "ai-dark",
    accent_glow: true,
  },
  behavior: {
    only_show_on_request: true,
    sound_enabled: false,
  },
  shortcuts: {
    accept: "ctrl+shift+a",
    deny: "ctrl+shift+d",
    always: "ctrl+shift+w",
    toggle: "ctrl+shift+n",
  },
};

const MOCK_PREVIEW_REQUEST: PermissionRequest = {
  id: "preview-id-sample",
  session_id: "preview-session",
  tool: "bash",
  args: { command: "cargo test --release && opencode run" },
  timestamp: new Date().toISOString(),
};

const App: Component = () => {
  const [state, setState] = createSignal<NotchState>(DEFAULT_STATE);
  const [config, setConfig] = createSignal<Config>(DEFAULT_CONFIG);
  const [showSettingsStandalone, setShowSettingsStandalone] = createSignal(false);
  const [previewMode, setPreviewMode] = createSignal(false);

  const connectionStatus = () => state().connection.status;
  const pendingRequests = () => state().pending_requests;
  const hasRealPending = () => pendingRequests().length > 0;
  const isCardVisible = () => hasRealPending() || previewMode();

  const currentRequest = (): PermissionRequest | undefined => {
    if (hasRealPending()) {
      return pendingRequests()[0];
    }
    if (previewMode()) {
      return MOCK_PREVIEW_REQUEST;
    }
    return undefined;
  };

  const queueCount = () => (hasRealPending() ? Math.max(0, pendingRequests().length - 1) : 0);

  const handleRespond = async (decision: Decision) => {
    if (!hasRealPending() && previewMode()) {
      setPreviewMode(false);
      return;
    }
    const req = currentRequest();
    if (!req) return;
    try {
      await respondPermission(req.id, decision);
    } catch (err) {
      console.error("Failed to respond to permission request:", err);
    }
  };

  const handleSaveConfig = async (updated: Config) => {
    try {
      await saveConfig(updated);
      setConfig(updated);
      setShowSettingsStandalone(false);
    } catch (err) {
      console.error("Failed to save configuration:", err);
    }
  };

  onMount(async () => {
    try {
      const [initialState, initialConfig] = await Promise.all([
        getState(),
        getConfig(),
      ]);
      setState(initialState);
      setConfig(initialConfig);
    } catch (err) {
      console.warn("Could not fetch initial state or config:", err);
    }

    const unlistenState = await listen<NotchState>("state-changed", (event) => {
      setState(event.payload);
      // If a real request arrives, close preview mode
      if (event.payload.pending_requests.length > 0) {
        setPreviewMode(false);
      }
    });

    const unlistenConfig = await listen<Config>("config-changed", (event) => {
      setConfig(event.payload);
    });

    const unlistenSettings = await listen("open-settings", () => {
      setShowSettingsStandalone(true);
    });

    onCleanup(() => {
      unlistenState();
      unlistenConfig();
      unlistenSettings();
    });
  });

  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        "align-items": "center",
        "justify-content": "center",
        padding: isCardVisible() || showSettingsStandalone() ? "8px" : "0",
        "box-sizing": "border-box",
      }}
    >
      <Show
        when={!showSettingsStandalone()}
        fallback={
          <div class={`approval-card theme-${config().appearance.theme || "ai-dark"}`}>
            <SettingsModal
              config={config()}
              isPreview={previewMode()}
              onTogglePreview={() => setPreviewMode(!previewMode())}
              onSave={handleSaveConfig}
              onClose={() => setShowSettingsStandalone(false)}
            />
          </div>
        }
      >
        <Show
          when={isCardVisible()}
          fallback={
            <Show when={!config().behavior.only_show_on_request}>
              <StatusDot
                status={connectionStatus()}
                onClick={() => setShowSettingsStandalone(true)}
              />
            </Show>
          }
        >
          <ApprovalCard
            request={currentRequest()!}
            queueCount={queueCount()}
            config={config()}
            isPreview={previewMode() && !hasRealPending()}
            onTogglePreview={() => setPreviewMode(!previewMode())}
            onRespond={handleRespond}
            onSaveConfig={handleSaveConfig}
          />
        </Show>
      </Show>
    </div>
  );
};

export default App;
