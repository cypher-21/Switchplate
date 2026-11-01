import { type Component } from "solid-js";
import type { PermissionRequest, Decision, Config } from "../lib/types";
import ActionButtons from "./ActionButtons";
import QueueBadge from "./QueueBadge";
import { playApproveChime, playRejectChime } from "../lib/sound";

interface ApprovalCardProps {
  request: PermissionRequest;
  queueCount: number;
  config: Config;
  isPreview?: boolean;
  onRespond: (decision: Decision) => void;
  onOpenSettings?: () => void;
  onDismiss?: () => void;
}

function formatCommand(args: unknown): string {
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
  const toolName = () => {
    const t = props.request.tool.toUpperCase();
    if (t === "BASH") return "SHELL";
    return t;
  };

  const handleAction = (decision: Decision) => {
    if (props.config.behavior.sound_enabled) {
      if (decision === "deny" || decision === "reject") {
        playRejectChime();
      } else {
        playApproveChime();
      }
    }
    props.onRespond(decision);
  };

  return (
    <div class="approval-card">
      {/* Header */}
      <div class="approval-card__header">
        <div class="approval-card__title-group">
          <div class="approval-card__brass-indicator" />
          <div class="approval-card__text-block">
            <span class="approval-card__title">APPROVAL REQUEST</span>
            <span class="approval-card__subtitle">ACTION REQUIRING CONFIRMATION</span>
          </div>
        </div>

        <div class="approval-card__header-right">
          <QueueBadge count={props.queueCount} />
          <span class="tool-tag">{toolName()}</span>
          {props.onOpenSettings && (
            <button
              class="header-action-btn"
              onClick={props.onOpenSettings}
              title="Open Settings"
              type="button"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
            </button>
          )}
          {props.onDismiss && (
            <button
              class="header-action-btn"
              onClick={props.onDismiss}
              title="Close window"
              type="button"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          )}
        </div>
      </div>

      {/* Monospace Command Area */}
      <div class="command-panel">
        <span class="command-prompt">$ </span>
        <span class="command-text">{formatCommand(props.request.args)}</span>
      </div>

      {/* Action Buttons Row */}
      <ActionButtons onRespond={handleAction} />
    </div>
  );
};

export default ApprovalCard;
