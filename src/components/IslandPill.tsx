import { type Component } from "solid-js";
import SwitchplateIcon from "./SwitchplateIcon";

interface IslandPillProps {
  status: "connected" | "disconnected" | "connecting";
  isWorking?: boolean;
  workingTool?: string;
  onClick?: () => void;
  onOpenSettings?: () => void;
}

const IslandPill: Component<IslandPillProps> = (props) => {
  const statusLabel = () => {
    if (props.isWorking) {
      return props.workingTool ? `WORKING: ${props.workingTool.toUpperCase()}` : "WORKING";
    }
    switch (props.status) {
      case "connected":
        return "CONNECTED";
      case "connecting":
        return "CONNECTING";
      case "disconnected":
        return "OFFLINE";
    }
  };

  const statusColor = () => {
    if (props.status === "connected") return "#B58B53"; // Brass
    return "#746B5D";
  };

  return (
    <div
      class="island-pill"
      onClick={props.onClick}
      title="Switchplate — Click to preview or open controls"
    >
      <div class="island-pill__brand">
        <SwitchplateIcon size={14} color="#E8E1D5" />
        <span class="island-pill__title">SWITCHPLATE</span>
      </div>

      <div class="island-pill__divider" />

      <div class="island-pill__status">
        <svg
          width="13"
          height="13"
          viewBox="0 0 24 24"
          fill="none"
          stroke={statusColor()}
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
          <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
        </svg>
        <span class="island-pill__status-text">{statusLabel()}</span>
      </div>

      <div class="island-pill__chevron">
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="#746B5D"
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </div>
    </div>
  );
};

export default IslandPill;
