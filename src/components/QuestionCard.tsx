import { type Component, createMemo, createSignal, For, Show } from "solid-js";
import { replyToForm } from "../lib/tauri-bridge";
import type { FormAnswer, FormField, FormOption, FormValue, OpenCodeForm } from "../lib/types";
import QueueBadge from "./QueueBadge";

interface QuestionCardProps {
  form: OpenCodeForm;
  queueCount: number;
  onHide: () => void;
}

interface FieldProps {
  field: FormField;
  value: FormValue | undefined;
  onChange: (value: FormValue | undefined) => void;
}

const supportedTypes = new Set(["string", "multiselect", "number", "integer", "boolean"]);

function isAnswered(value: FormValue | undefined) {
  if (typeof value === "string") return value.trim().length > 0;
  if (Array.isArray(value)) return value.length > 0;
  return value !== undefined;
}

function validAnswer(field: FormField, value: FormValue | undefined) {
  const count = Array.isArray(value) ? value.length : 0;
  if (field.type === "multiselect") {
    const minimum = Math.max(field.minItems ?? 0, field.required ? 1 : 0);
    return (count === 0 && !field.required && value === undefined)
      || (count >= minimum && count <= (field.maxItems ?? Infinity));
  }
  if (!isAnswered(value)) return !field.required;
  if (field.type === "integer") return typeof value === "number" && Number.isInteger(value);
  return true;
}

const Choice: Component<{
  option: FormOption;
  selected: boolean;
  disabled?: boolean;
  onSelect: () => void;
}> = (props) => (
  <button
    type="button"
    class="question-option"
    aria-pressed={props.selected}
    disabled={props.disabled}
    onClick={props.onSelect}
  >
    <span>{props.option.label}</span>
    <Show when={props.option.description}>
      <small>{props.option.description}</small>
    </Show>
  </button>
);

const ChoiceField: Component<FieldProps> = (props) => {
  const field = () => props.field;
  const isMultiple = () => field().type === "multiselect";
  const selected = () => Array.isArray(props.value) ? props.value : [];
  const options = () => field().options ?? [];
  const customSelections = () => selected().filter((value) => !options().some((option) => option.value === value));
  const atLimit = () => selected().length >= (field().maxItems ?? Infinity);
  const [custom, setCustom] = createSignal(
    typeof props.value === "string" && !options().some((option) => option.value === props.value) ? props.value : "",
  );
  let customInput: HTMLInputElement | undefined;

  const toggleChoice = (value: string) => {
    if (isMultiple()) {
      const next = selected().includes(value)
        ? selected().filter((answer) => answer !== value)
        : [...selected(), value];
      props.onChange(next.length ? next : undefined);
    } else {
      setCustom("");
      props.onChange(props.value === value && !field().required ? undefined : value);
    }
  };

  const addCustom = () => {
    const value = custom().trim();
    if (!value) return;
    if (!selected().includes(value)) {
      if (atLimit()) return;
      props.onChange([...selected(), value]);
    }
    setCustom("");
    customInput?.setCustomValidity("");
  };

  return (
    <>
      <For each={options()}>
        {(option) => (
          <Choice
            option={option}
            selected={isMultiple() ? selected().includes(option.value) : props.value === option.value}
            disabled={isMultiple() && atLimit() && !selected().includes(option.value)}
            onSelect={() => toggleChoice(option.value)}
          />
        )}
      </For>
      <For each={customSelections()}>
        {(value) => (
          <Choice option={{ value, label: value }} selected onSelect={() => toggleChoice(value)} />
        )}
      </For>
      <Show when={field().custom}>
        <div class="question-custom">
          <input
            class="question-input"
            ref={customInput}
            type="text"
            aria-label={`Custom answer for ${field().title || field().key}`}
            placeholder={field().placeholder || (isMultiple() ? "Add your own option" : "Type your own answer")}
            value={custom()}
            minLength={field().minLength}
            maxLength={field().maxLength}
            onInput={(event) => {
              const value = event.currentTarget.value;
              setCustom(value);
              if (isMultiple()) {
                event.currentTarget.setCustomValidity(value.trim() ? "Select Add to include this answer." : "");
              } else {
                props.onChange(value.trim() ? value : undefined);
              }
            }}
            onKeyDown={(event) => {
              if (isMultiple() && event.key === "Enter") {
                event.preventDefault();
                addCustom();
              }
            }}
          />
          <Show when={isMultiple()}>
            <button
              type="button"
              class="response-notice-btn"
              disabled={!custom().trim() || (atLimit() && !selected().includes(custom().trim()))}
              onClick={addCustom}
            >
              Add
            </button>
          </Show>
        </div>
      </Show>
    </>
  );
};

const QuestionField: Component<FieldProps> = (props) => {
  const field = () => props.field;
  const title = () => field().title || field().key;
  const numeric = () => field().type === "number" || field().type === "integer";
  return (
    <fieldset class="question-field">
      <legend>{title()}</legend>
      <Show when={field().description}><p>{field().description}</p></Show>
      <Show
        when={field().type === "multiselect" || (field().type === "string" && Boolean(field().options?.length))}
        fallback={
          <Show
            when={field().type === "boolean"}
            fallback={
              <input
                class="question-input"
                aria-label={title()}
                type={numeric() ? "number" : "text"}
                min={typeof field().minimum === "number" ? field().minimum : undefined}
                max={typeof field().maximum === "number" ? field().maximum : undefined}
                step={field().type === "integer" ? "1" : "any"}
                minLength={field().minLength}
                maxLength={field().maxLength}
                placeholder={field().placeholder}
                value={typeof props.value === "string" || typeof props.value === "number" ? props.value : ""}
                required={field().required}
                onInput={(event) => {
                  const input = event.currentTarget;
                  props.onChange(!input.value.trim() ? undefined : numeric() ? input.valueAsNumber : input.value);
                }}
              />
            }
          >
            <For each={[true, false]}>
              {(value) => (
                <Choice
                  option={{ value: String(value), label: value ? "Yes" : "No" }}
                  selected={props.value === value}
                  onSelect={() => props.onChange(props.value === value && !field().required ? undefined : value)}
                />
              )}
            </For>
          </Show>
        }
      >
        <ChoiceField field={field()} value={props.value} onChange={props.onChange} />
      </Show>
    </fieldset>
  );
};

const QuestionCard: Component<QuestionCardProps> = (props) => {
  const [draft, setDraft] = createSignal<FormAnswer>(Object.fromEntries(
    props.form.fields
      .filter((field) => field.default !== undefined)
      .map((field) => [field.key, field.default!]),
  ));
  const [sending, setSending] = createSignal(false);
  const [error, setError] = createSignal("");
  const answerFor = (key: string) => Object.hasOwn(draft(), key) ? draft()[key] : undefined;
  const visibleFields = createMemo(() => props.form.fields.filter((field) => !field.hidden
    && (!field.when || field.when.every((condition) => {
      const equal = answerFor(condition.key) === condition.value;
      return condition.op === "eq" ? equal : !equal;
    }))));
  const needsTerminal = () => visibleFields().some((field) => !supportedTypes.has(field.type));
  const canSend = () => !sending() && !needsTerminal()
    && visibleFields().every((field) => validAnswer(field, answerFor(field.key)));

  const send = async (event: SubmitEvent) => {
    event.preventDefault();
    if (!canSend()) return;
    const { id, session_id } = props.form;
    const answer = Object.fromEntries(visibleFields().flatMap((field) => {
      const value = answerFor(field.key);
      return isAnswered(value) ? [[field.key, value!]] : [];
    }));
    setError("");
    setSending(true);
    try {
      await replyToForm({ id, session_id }, answer);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSending(false);
    }
  };

  return (
    <form class="question-card" onSubmit={send}>
      <header class="question-header">
        <h2>OpenCode question</h2>
        <QueueBadge count={props.queueCount} />
        <button class="response-notice-btn" type="button" onClick={props.onHide}>Hide</button>
      </header>
      <div class="question-fields">
        <Show when={props.form.title && props.form.title !== "Questions"}>
          <p class="question-form-title">{props.form.title}</p>
        </Show>
        <Show when={!needsTerminal()} fallback={<p>Continue in your OpenCode terminal to answer this form.</p>}>
          <For each={visibleFields()}>
            {(field) => (
              <QuestionField
                field={field}
                value={answerFor(field.key)}
                onChange={(value) => {
                  setError("");
                  setDraft((current) => {
                    if (value !== undefined) return { ...current, [field.key]: value };
                    const next = { ...current };
                    delete next[field.key];
                    return next;
                  });
                }}
              />
            )}
          </For>
        </Show>
      </div>
      <Show when={!needsTerminal()}>
        <footer class="question-footer">
          <Show when={error()}><p role="alert">{error()}</p></Show>
          <button class="settings-apply-btn" type="submit" disabled={!canSend()}>
            {sending() ? "Sending…" : "Send answers"}
          </button>
        </footer>
      </Show>
    </form>
  );
};

export default QuestionCard;
