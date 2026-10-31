import { type Component } from "solid-js";
import type { Decision } from "../lib/types";

interface ActionButtonsProps {
  onRespond: (decision: Decision) => void;
}

const ActionButtons: Component<ActionButtonsProps> = (props) => {
  return (
    <div class="action-buttons-row">
      {/* 1. Accept Button (Warm Bone #E8E1D5 with dark text and clean checkmark) */}
      <button
        class="action-btn action-btn--accept"
        onClick={() => props.onRespond("allow")}
        title="Allow this action (Ctrl+Shift+A)"
        type="button"
      >
        <svg
          width="15"
          height="15"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polyline points="20 6 9 17 4 12" />
        </svg>
        <span class="action-btn__label">Accept</span>
      </button>

      {/* 2. Reject Button (Dark charcoal #22211E with subtle neutral outline and clean X) */}
      <button
        class="action-btn action-btn--reject"
        onClick={() => props.onRespond("deny")}
        title="Reject this action (Ctrl+Shift+D)"
        type="button"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
        <span class="action-btn__label">Reject</span>
      </button>

      {/* 3. Always Allow Button (Restrained Brass #B58B53 with geometric layered icon) */}
      <button
        class="action-btn action-btn--always"
        onClick={() => props.onRespond("always")}
        title="Always allow this command (Ctrl+Shift+W)"
        type="button"
      >
        <svg
          width="15"
          height="15"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.4"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polygon points="12 2 2 7 12 12 22 7 12 2" />
          <polyline points="2 17 12 22 22 17" />
          <polyline points="2 12 12 17 22 12" />
        </svg>
        <span class="action-btn__label">Always Allow</span>
      </button>
    </div>
  );
};

export default ActionButtons;
