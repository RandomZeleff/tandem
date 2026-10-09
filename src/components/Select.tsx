import { createEffect, createSignal, createUniqueId, For, type JSX, on, onCleanup, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { onDismiss } from "../lib/ui";
import { Icon } from "./pixel";

export interface SelectOption {
  value: string;
  label: string;
  /** Secondary text, right-aligned (e.g. "recommandée"). */
  hint?: string;
  icon?: JSX.Element;
}

const MAX_LIST_HEIGHT = 288;

/**
 * Dropdown in the launcher's style, replacing the native `<select>`: listbox in a portal
 * (never clipped by a scrolling panel), arrows, Home/End, type-ahead, Enter, Escape.
 */
export default function Select(props: {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  id?: string;
  label?: string;
  placeholder?: string;
  disabled?: boolean;
  /** Classes for the trigger (width, height, text size). */
  class?: string;
}) {
  const listId = createUniqueId();
  const [open, setOpen] = createSignal(false);
  const [active, setActive] = createSignal(0);
  const [place, setPlace] = createSignal<{ left: number; width: number; top?: number; bottom?: number; maxHeight: number }>();
  let trigger: HTMLButtonElement | undefined;
  let list: HTMLUListElement | undefined;

  const selected = () => props.options.find((o) => o.value === props.value);
  const selectedIndex = () => Math.max(0, props.options.findIndex((o) => o.value === props.value));

  function position() {
    if (!trigger) return;
    const r = trigger.getBoundingClientRect();
    const below = window.innerHeight - r.bottom - 8;
    const above = r.top - 8;
    // Opens downward unless the space below is short and the space above is larger.
    const down = below >= Math.min(MAX_LIST_HEIGHT, 160) || below >= above;
    setPlace({
      left: r.left,
      width: r.width,
      ...(down ? { top: r.bottom + 4 } : { bottom: window.innerHeight - r.top + 4 }),
      maxHeight: Math.min(MAX_LIST_HEIGHT, down ? below : above),
    });
  }

  function show() {
    if (props.disabled || props.options.length === 0) return;
    position();
    setActive(selectedIndex());
    setOpen(true);
  }

  function choose(index: number) {
    const option = props.options[index];
    setOpen(false);
    trigger?.focus();
    if (option && option.value !== props.value) props.onChange(option.value);
  }

  let typed = "";
  let typedAt = 0;
  /** Jumps to the next option starting with what was typed in the last half second. */
  function typeAhead(key: string) {
    const now = Date.now();
    typed = now - typedAt > 500 ? key : typed + key;
    typedAt = now;
    const start = typed.length === 1 ? active() + 1 : active();
    const n = props.options.length;
    for (let i = 0; i < n; i++) {
      const index = (start + i) % n;
      if (props.options[index].label.toLowerCase().startsWith(typed.toLowerCase())) {
        if (open()) setActive(index);
        else choose(index);
        return;
      }
    }
  }

  function onKeyDown(e: KeyboardEvent) {
    const last = props.options.length - 1;
    if (!open()) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
        e.preventDefault();
        show();
      } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
        typeAhead(e.key);
      }
      return;
    }
    const moves: Record<string, () => number> = {
      ArrowDown: () => Math.min(last, active() + 1),
      ArrowUp: () => Math.max(0, active() - 1),
      Home: () => 0,
      End: () => last,
      PageDown: () => Math.min(last, active() + 8),
      PageUp: () => Math.max(0, active() - 8),
    };
    if (moves[e.key]) {
      e.preventDefault();
      setActive(moves[e.key]());
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      choose(active());
    } else if (e.key === "Tab") {
      setOpen(false);
    } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      typeAhead(e.key);
    }
  }

  // Keep the highlighted option in view while moving with the keyboard.
  createEffect(
    on(active, (index) => {
      if (!open() || !list) return;
      list.querySelector<HTMLElement>(`[data-index="${index}"]`)?.scrollIntoView({ block: "nearest" });
    }),
  );

  return (
    <>
      <button
        ref={trigger}
        id={props.id}
        type="button"
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={open()}
        aria-controls={listId}
        aria-activedescendant={open() ? `${listId}-${active()}` : undefined}
        aria-label={props.label}
        disabled={props.disabled}
        class={`field flex items-center gap-2 text-left disabled:cursor-default disabled:opacity-60 focus-visible:shadow-[inset_0_0_0_1px_var(--color-xp),inset_0_3px_0_rgb(0_0_0/0.4)] ${props.class ?? "text-sm"}`}
        classList={{ "shadow-[inset_0_0_0_1px_var(--color-xp),inset_0_3px_0_rgb(0_0_0/0.4)]": open() }}
        onClick={() => (open() ? setOpen(false) : show())}
        onKeyDown={onKeyDown}
      >
        <Show when={selected()?.icon}>
          <span class="flex shrink-0">{selected()!.icon}</span>
        </Show>
        <span class="min-w-0 flex-1 truncate" classList={{ "text-faint": !selected() }}>
          {selected()?.label ?? props.placeholder ?? ""}
        </span>
        <Icon name="caret" size={10} class={`shrink-0 text-muted transition-transform duration-100 ${open() ? "rotate-180" : ""}`} />
      </button>

      <Show when={open() && place()}>
        {(p) => (
          <Portal>
            <SelectList
              ref={(el) => (list = el)}
              id={listId}
              label={props.label}
              place={p()}
              options={props.options}
              value={props.value}
              active={active()}
              onHover={setActive}
              onChoose={choose}
              onClose={() => setOpen(false)}
              trigger={() => trigger}
            />
          </Portal>
        )}
      </Show>
    </>
  );
}

function SelectList(props: {
  ref: (el: HTMLUListElement) => void;
  id: string;
  label?: string;
  place: { left: number; width: number; top?: number; bottom?: number; maxHeight: number };
  options: SelectOption[];
  value: string;
  active: number;
  onHover: (index: number) => void;
  onChoose: (index: number) => void;
  onClose: () => void;
  trigger: () => HTMLButtonElement | undefined;
}) {
  let el: HTMLUListElement | undefined;
  onDismiss(() => [el, props.trigger()], props.onClose);
  onMount(() => {
    el?.querySelector<HTMLElement>(`[data-index="${props.active}"]`)?.scrollIntoView({ block: "nearest" });
    // The list is placed once: close it if anything behind it moves.
    const onScroll = (e: Event) => {
      if (e.target instanceof Node && el?.contains(e.target)) return;
      props.onClose();
    };
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", props.onClose);
    onCleanup(() => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", props.onClose);
    });
  });

  return (
    <ul
      ref={(e) => {
        el = e;
        props.ref(e);
      }}
      id={props.id}
      role="listbox"
      aria-label={props.label}
      class="panel px-corners-md fixed z-50 overflow-y-auto bg-slate-750 p-1 shadow-[0_12px_32px_rgb(0_0_0/0.55)]"
      style={{
        left: `${props.place.left}px`,
        "min-width": `${props.place.width}px`,
        "max-width": `${Math.max(props.place.width, 420)}px`,
        "max-height": `${props.place.maxHeight}px`,
        top: props.place.top !== undefined ? `${props.place.top}px` : undefined,
        bottom: props.place.bottom !== undefined ? `${props.place.bottom}px` : undefined,
      }}
    >
      <For each={props.options}>
        {(option, i) => (
          <li
            id={`${props.id}-${i()}`}
            data-index={i()}
            role="option"
            aria-selected={option.value === props.value}
            class="flex h-8 cursor-pointer items-center gap-2 px-2.5 text-sm"
            classList={{
              "bg-slate-600 text-chalk": i() === props.active,
              "text-chalk-2": i() !== props.active,
            }}
            onPointerMove={() => props.active !== i() && props.onHover(i())}
            onClick={() => props.onChoose(i())}
          >
            <span class="flex w-3 shrink-0 justify-center">
              <Show when={option.value === props.value}>
                <Icon name="check" size={10} color="#8BE04E" />
              </Show>
            </span>
            <Show when={option.icon}>
              <span class="flex shrink-0">{option.icon}</span>
            </Show>
            <span class="min-w-0 flex-1 truncate" classList={{ "font-medium": option.value === props.value }}>
              {option.label}
            </span>
            <Show when={option.hint}>
              <span class="shrink-0 text-xs text-muted">{option.hint}</span>
            </Show>
          </li>
        )}
      </For>
    </ul>
  );
}
