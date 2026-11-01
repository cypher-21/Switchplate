import { type Component } from "solid-js";

interface ResponseNoticeProps {
  onDismiss: () => void;
}

const ResponseNotice: Component<ResponseNoticeProps> = (props) => {
  return (
    <div class="response-notice-card">
      <div class="response-notice-card__header">
        <div class="response-notice-card__title-group">
          <div class="response-notice-card__brass-indicator" />
          <div class="response-notice-card__text-block">
            <span class="response-notice-card__title">RESPONSE GENERATED</span>
            <span class="response-notice-card__subtitle">TASK COMPLETED</span>
          </div>
        </div>

        <button
          class="header-action-btn"
          onClick={props.onDismiss}
          title="Dismiss notification"
          type="button"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>

      <div class="response-notice-card__body">
        <div class="response-notice-card__icon-box">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#B58B53" stroke-width="2.2">
            <polyline points="20 6 9 17 4 12" />
          </svg>
        </div>
        <div class="response-notice-card__desc">
          <span class="response-notice-card__main-text">Output generation complete.</span>
          <span class="response-notice-card__sub-text">Check your terminal session for results.</span>
        </div>
      </div>

      <div class="response-notice-card__actions">
        <button
          class="response-notice-btn"
          onClick={props.onDismiss}
          type="button"
        >
          Dismiss
        </button>
      </div>
    </div>
  );
};

export default ResponseNotice;
