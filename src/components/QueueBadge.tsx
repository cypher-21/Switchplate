import { type Component, Show } from "solid-js";

interface QueueBadgeProps {
  count: number;
}

const QueueBadge: Component<QueueBadgeProps> = (props) => {
  return (
    <Show when={props.count > 0}>
      <div class="queue-badge">+{props.count} more</div>
    </Show>
  );
};

export default QueueBadge;
