import { type Component, createSignal, Show } from "solid-js";
import type { PermissionRequest, Decision, Config } from "../lib/types";
import ActionButtons from "./ActionButtons";
import QueueBadge from "./QueueBadge";
import SettingsModal from "./SettingsModal";
import "../styles/approval-card.css";

interface ApprovalCardProps {
  request: PermissionRequest;
  queueCount: number;
  config: Config;
  isPreview?: boolean;
  onRespond: (decision: Decision) => void;
  onSaveConfig: (updated: Config) => void;
  onTogglePreview?: () => void;
}

const TOOL_BADGE_CLASS: Record<string, string> = {
  bash: "tool-badge--orange",
  shell: "tool-badge--orange",
  exec: "tool-badge--orange",
  edit: "tool-badge--blue",
  write: "tool-badge--blue",
  read: "tool-badge--green",
  glob: "tool-badge--green",
  grep: "tool-badge--green",
  browser: "tool-badge--pink",
};

function formatArgs(args: unknown): string {
  if (typeof args === "string") {
    return args;
  }
  if (Array.isArray(args)) {
    return args.map((x) => (typeof x === "string" ? x : JSON.stringify(x))).join(" ");
  }
  if (args && typeof args === "object") {
    const obj = args as Record<string, unknown>;
    if ("command" in obj && typeof obj.command === "string") {
      return obj.command;
    }
    if ("file_path" in obj && typeof obj.file_path === "string") {
      return obj.file_path;
    }
    return JSON.stringify(args, null, 2);
  }
  return String(args);
}

const ApprovalCard: Component<ApprovalCardProps> = (props) => {
  const [showSettings, setShowSettings] = createSignal(false);

  const badgeClass = () => {
    const tool = props.request.tool.toLowerCase();
    return TOOL_BADGE_CLASS[tool] ?? "tool-badge--gray";
  };

  const themeClass = () => {
    return `theme-${props.config.appearance.theme || "ai-dark"}`;
  };

  return (
    <div class={`approval-card ${themeClass()}`}>
      <Show
        when={showSettings()}
        fallback={
          <>
            <div class="card-header">
              <div class="header-left">
                <div class="ai-spark-dot" />
                <span class="card-title">OpenCode Request</span>
                {props.isPreview && <span class="preview-tag">Preview</span>}
                <span class={`tool-badge ${badgeClass()}`}>{props.request.tool}</span>
              </div>
              <div class="header-right">
                <QueueBadge count={props.queueCount} />
                <button
                  class="icon-btn settings-btn"
                  onClick={() => setShowSettings(true)}
                  title="Customize Notch Settings"
                >
                  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="3" />
                    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
                  </svg>
                </button>
              </div>
            </div>

            <div class="command-box">
              <pre class="command-preview">{formatArgs(props.request.args)}</pre>
            </div>

            <ActionButtons onRespond={props.onRespond} />
          </>
        }
      >
        <SettingsModal
          config={props.config}
          isPreview={props.isPreview}
          onTogglePreview={props.onTogglePreview}
          onSave={(updated) => {
            props.onSaveConfig(updated);
            setShowSettings(false);
          }}
          onClose={() => setShowSettings(false)}
        />
      </Show>
    </div>
  );
};

export default ApprovalCard;
