import { type Component, createEffect, createMemo, createSignal, onMount, onCleanup, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Chime, NotchState, Decision, Config, PermissionRequest } from "./lib/types";
import {
  getState,
  respondPermission,
  getConfig,
  saveConfig,
  openSettingsWindow,
  closeSettingsWindow,
  closeIsland,
  dismissResponseNotice,
  testNotificationSound,
  toggleIsland,
} from "./lib/tauri-bridge";
import IslandPill from "./components/IslandPill";
import ApprovalCard from "./components/ApprovalCard";
import SettingsWindow from "./components/SettingsWindow";
import ResponseNotice from "./components/ResponseNotice";
import QuestionCard from "./components/QuestionCard";
import "./styles/approval-card.css";

const DEFAULT_STATE: NotchState = {
  connection: { status: "disconnected" },
  pending_requests: [],
  pending_forms: [],
  response_notices: [],
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
    auto_approval: "manual",
    sound_enabled: true,
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
  const [configReady, setConfigReady] = createSignal(false);
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
  const currentForm = createMemo(
    () => state().pending_forms[0],
    undefined,
    { equals: (previous, next) => previous?.id === next?.id && previous?.session_id === next?.session_id },
  );
  const hasInput = () => hasRealPending() || Boolean(currentForm());
  const isCardVisible = () => hasRealPending() || (!currentForm() && previewMode());
  const visibleNotice = () => hasInput() ? undefined : state().response_notices[0];
  const visibleNoticeId = createMemo(() => visibleNotice()?.id);

  const dismissNotice = (id: number) => {
    void dismissResponseNotice(id).catch((error) => console.warn("Could not dismiss response notice:", error));
  };

  if (!isSettingsWindow) {
    createEffect(() => {
      const id = visibleNoticeId();
      if (id !== undefined) {
        const timer = window.setTimeout(() => dismissNotice(id), 5000);
        onCleanup(() => window.clearTimeout(timer));
      }
    });
  }

  const playPreviewChime = (chime: Chime) => {
    if (config().behavior.sound_enabled) {
      void testNotificationSound(chime).catch((error) => console.warn("Could not play preview chime:", error));
    }
  };

  const hideQuestion = () => {
    void toggleIsland().catch((error) => console.warn("Could not hide question:", error));
  };

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
      playPreviewChime(decision === "deny" || decision === "reject" ? "reject" : "approve");
      setPreviewMode(false);
      closeIsland();
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
    await saveConfig(updated);
    setConfig(updated);
  };

  onMount(() => {
    let mounted = true;
    let stateUpdated = false;
    let configUpdated = false;
    const unlisteners: (() => void)[] = [];
    const subscriptions = [
      listen<NotchState>("state-changed", ({ payload }) => {
        stateUpdated = true;
        if (payload.pending_requests.length || payload.pending_forms.length || payload.response_notices.length) {
          setPreviewMode(false);
        }
        setState(payload);
      }),
      listen<Config>("config-changed", ({ payload }) => {
        configUpdated = true;
        setConfig(payload);
      }),
    ];
    if (!isSettingsWindow) {
      subscriptions.push(
        listen("open-island", () => {
          if (hasInput() || visibleNotice()) return;
          setPreviewMode(true);
          playPreviewChime("request");
        }),
        listen("island-closed", () => setPreviewMode(false)),
        listen("toggle-preview", () => {
          if (hasInput() || visibleNotice()) return;
          const next = !previewMode();
          setPreviewMode(next);
          if (next) {
            playPreviewChime("request");
          } else if (!hasRealPending()) {
            void closeIsland();
          }
        }),
      );
    }

    void (async () => {
      try {
        await Promise.all(subscriptions.map(async (subscription) => {
          const unlisten = await subscription;
          if (mounted) unlisteners.push(unlisten);
          else unlisten();
        }));
        const [initialState, initialConfig] = await Promise.all([getState(), getConfig()]);
        if (mounted && !stateUpdated) setState(initialState);
        if (mounted && !configUpdated) setConfig(initialConfig);
      } catch (error) {
        console.warn("Could not initialize Switchplate:", error);
      } finally {
        if (mounted) setConfigReady(true);
      }
    })();

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (isSettingsWindow) {
          closeSettingsWindow();
        } else if (visibleNoticeId() !== undefined) {
          dismissNotice(visibleNoticeId()!);
        } else if (currentForm() && !hasRealPending()) {
          hideQuestion();
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
      mounted = false;
      unlisteners.forEach((unlisten) => unlisten());
      window.removeEventListener("keydown", handleKeyDown);
    });
  });

  if (isSettingsWindow) {
    return (
      <div class="settings-app-container">
        <Show when={configReady()}>
          <SettingsWindow
            config={config()}
            state={state()}
            onSaveConfig={handleSaveConfig}
            onClose={closeSettingsWindow}
          />
        </Show>
      </div>
    );
  }

  return (
    <div class="island-app-container">
      <Show when={currentForm()} keyed>
        {(form) => (
          <div class="question-shell" hidden={hasRealPending()}>
            <QuestionCard form={form} queueCount={state().pending_forms.length - 1} onHide={hideQuestion} />
          </div>
        )}
      </Show>
      <Show
        when={visibleNotice()}
        fallback={
          <Show
            when={isCardVisible()}
            fallback={
              <Show when={!currentForm() && !config().behavior.only_show_on_request}>
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
        }
      >
        {(notice) => (
          <ResponseNotice outcome={notice().outcome} onDismiss={() => dismissNotice(notice().id)} />
        )}
      </Show>
    </div>
  );
};

export default App;
