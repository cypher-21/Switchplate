import { type Component } from "solid-js";
import type { Decision } from "../lib/types";

interface ActionButtonsProps {
  onRespond: (decision: Decision) => void;
}

const ActionButtons: Component<ActionButtonsProps> = (props) => {
  return (
    <div class="actions">
      <button
        class="action-btn action-btn--accept"
        onClick={() => props.onRespond("allow")}
      >
        <span>Accept</span>
        <span class="action-btn__hint">Ctrl+Shift+A</span>
      </button>
      <button
        class="action-btn action-btn--reject"
        onClick={() => props.onRespond("deny")}
      >
        <span>Reject</span>
        <span class="action-btn__hint">Ctrl+Shift+D</span>
      </button>
      <button
        class="action-btn action-btn--always"
        onClick={() => props.onRespond("always")}
      >
        <span>Always</span>
        <span class="action-btn__hint">Ctrl+Shift+W</span>
      </button>
    </div>
  );
};

export default ActionButtons;
