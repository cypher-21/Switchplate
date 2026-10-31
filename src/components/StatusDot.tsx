import { type Component } from "solid-js";

interface StatusDotProps {
  status: "connected" | "disconnected" | "connecting";
  onClick?: () => void;
}

const STATUS_COLORS: Record<StatusDotProps["status"], string> = {
  connected: "#22c55e",
  disconnected: "#ef4444",
  connecting: "#eab308",
};

const STATUS_LABELS: Record<StatusDotProps["status"], string> = {
  connected: "OpenCode",
  disconnected: "Offline",
  connecting: "Connecting",
};

const StatusDot: Component<StatusDotProps> = (props) => {
  const color = () => STATUS_COLORS[props.status] || STATUS_COLORS.disconnected;
  const label = () => STATUS_LABELS[props.status] || "OpenCode";
  const isConnecting = () => props.status === "connecting";

  return (
    <div class="notch-pill-wrapper" onClick={props.onClick} title="Click to configure Notch">
      <div class="notch-pill">
        <div
          class="notch-dot"
          classList={{ "notch-dot--pulse": isConnecting() }}
          style={{
            background: color(),
            "box-shadow": `0 0 10px 2px ${color()}`,
          }}
        />
        <span class="notch-label">{label()}</span>
      </div>
    </div>
  );
};

export default StatusDot;
