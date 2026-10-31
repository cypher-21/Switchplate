import { type Component } from "solid-js";

interface SwitchplateIconProps {
  size?: number;
  color?: string;
  class?: string;
}

/**
 * Geometric 3-slat mechanical icon for SWITCHPLATE.
 * Precision-machined industrial switch plate aesthetic.
 */
export const SwitchplateIcon: Component<SwitchplateIconProps> = (props) => {
  const size = () => props.size ?? 18;
  const color = () => props.color ?? "#E8E1D5";

  return (
    <svg
      width={size()}
      height={size()}
      viewBox="0 0 24 24"
      fill="none"
      class={props.class}
      style={{ display: "inline-block", "vertical-align": "middle", "flex-shrink": "0" }}
    >
      {/* Top slat - stepped right */}
      <rect x="7" y="5" width="12" height="2.5" rx="1.25" fill={color()} />
      {/* Middle slat - full width mechanical bar */}
      <rect x="4" y="10.5" width="16" height="2.5" rx="1.25" fill={color()} />
      {/* Bottom slat - stepped left */}
      <rect x="6" y="16" width="13" height="2.5" rx="1.25" fill={color()} />
    </svg>
  );
};

export default SwitchplateIcon;
