import { type Component, createSignal, onMount, onCleanup, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { NotchState, Decision, Config, PermissionRequest } from "./lib/types";
import {
  getState,
  respondPermission,
  getConfig,
  saveConfig,
  openSettingsWindow,
  closeSettingsWindow,
  closeIsland,
} from "./lib/tauri-bridge";
import IslandPill from "./components/IslandPill";
import ApprovalCard from "./components/ApprovalCard";
import SettingsWindow from "./components/SettingsWindow";
import "./styles/approval-card.css";

const DEFAULT_STATE: NotchState = {
  connection: { status: "disconnected" },
  pending_requests: [],
};

const DEFAULT_CONFIG: Config = {
  opencode: {
    server_url: "http://127.0.0.1:49374",
    password_env: "OPENCODE_API_PASSWORD",
    request_timeout: 30,
    auto_reconnect: true,
  },
  appearance: {
    position: "top-center",
    card_width: 480,
    card_height: 190,
    collapsed_width: 280,
    collapsed_height: 38,
    offset_x: 0,
    offset_y: 12,
    theme: "industrial-swiss",
    reduced_motion: false,
  },
  behavior: {
    only_show_on_request: true,
    pause_monitoring: false,
    sound_enabled: false,
    autostart: true,
  },
  shortcuts: {
    open_island: "ctrl+shift+o",
    accept: "ctrl+shift+a",
    deny: "ctrl+shift+d",
    always: "ctrl+shift+w",
    open_settings: "ctrl+shift+s",
  },
};

const MOCK_PREVIEW_REQUEST: PermissionRequest = {
  id: "preview-req-sample",
  session_id: "preview-session",
  tool: "shell",
  args: { command: "du -sh /tmp /var/tmp 2>/dev/null" },
  timestamp: new Date().toISOString(),
};

const App: Component = () => {
  const [state, setState] = createSignal<NotchState>(DEFAULT_STATE);
  const [config, setConfig] = createSignal<Config>(DEFAULT_CONFIG);
  const [previewMode, setPreviewMode] = createSignal(false);

  let isSettingsWindow = false;
  try {
    const currentWin = getCurrentWebviewWindow();
    if (currentWin && currentWin.label === "settings") {
      isSettingsWindow = true;
    }
  } catch {
    isSettingsWindow = false;
  }

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
      if (event.payload.pending_requests.length > 0) {
        setPreviewMode(false);
      }
    });

    const unlistenConfig = await listen<Config>("config-changed", (event) => {
      setConfig(event.payload);
    });

    const unlistenPreview = await listen("toggle-preview", () => {
      setPreviewMode((prev) => !prev);
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (isSettingsWindow) {
          closeSettingsWindow();
        } else if (previewMode()) {
          setPreviewMode(false);
          if (!hasRealPending()) {
            closeIsland();
          }
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    onCleanup(() => {
      unlistenState();
      unlistenConfig();
      unlistenPreview();
      window.removeEventListener("keydown", handleKeyDown);
    });
  });

  if (isSettingsWindow) {
    return (
      <div class="settings-app-container">
        <SettingsWindow
          config={config()}
          state={state()}
          onSaveConfig={handleSaveConfig}
          onClose={closeSettingsWindow}
        />
      </div>
    );
  }

  return (
    <div class="island-app-container">
      <Show
        when={isCardVisible()}
        fallback={
          <Show when={!config().behavior.only_show_on_request}>
            <IslandPill
              status={connectionStatus()}
              onClick={() => setPreviewMode(!previewMode())}
              onOpenSettings={openSettingsWindow}
            />
          </Show>
        }
      >
        <ApprovalCard
          request={currentRequest()!}
          queueCount={queueCount()}
          config={config()}
          isPreview={previewMode() && !hasRealPending()}
          onRespond={handleRespond}
          onOpenSettings={openSettingsWindow}
          onDismiss={() => {
            setPreviewMode(false);
            if (!hasRealPending()) {
              closeIsland();
            }
          }}
        />
      </Show>
    </div>
  );
};

export default App;
