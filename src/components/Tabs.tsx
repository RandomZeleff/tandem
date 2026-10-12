import { For, type JSX } from "solid-js";

export interface TabItem<T extends string> {
  id: T;
  label: JSX.Element;
}

/**
 * Tab strip with the keyboard behaviour of native tabs: one Tab stop, arrows, Home/End.
 * `underline` for page sections, `segmented` for compact filters.
 */
export default function Tabs<T extends string>(props: {
  items: TabItem<T>[];
  value: T;
  onChange: (id: T) => void;
  label: string;
  /** Prefix of the tab ids; the panel is `${idPrefix}-panel` (see `tabPanel`). */
  idPrefix: string;
  variant?: "underline" | "segmented";
  class?: string;
  tabClass?: string;
}) {
  let strip: HTMLDivElement | undefined;
  const segmented = () => props.variant === "segmented";

  function onKeyDown(e: KeyboardEvent) {
    const ids = props.items.map((t) => t.id);
    const index = ids.indexOf(props.value);
    const next: Record<string, number> = {
      ArrowRight: (index + 1) % ids.length,
      ArrowLeft: (index - 1 + ids.length) % ids.length,
      Home: 0,
      End: ids.length - 1,
    };
    if (next[e.key] === undefined) return;
    e.preventDefault();
    props.onChange(ids[next[e.key]]);
    strip?.querySelector<HTMLElement>(`#${props.idPrefix}-tab-${ids[next[e.key]]}`)?.focus();
  }

  return (
    <div
      ref={strip}
      role="tablist"
      aria-label={props.label}
      class={props.class}
      classList={{
        "flex w-fit gap-0.5 rounded-md bg-slate-900 p-[3px] shadow-[inset_0_0_0_1px_rgb(0_0_0/0.3)]": segmented(),
        "flex gap-1": !segmented(),
      }}
      onKeyDown={onKeyDown}
    >
      <For each={props.items}>
        {(tab) => {
          const selected = () => props.value === tab.id;
          return (
            <button
              type="button"
              role="tab"
              id={`${props.idPrefix}-tab-${tab.id}`}
              aria-selected={selected()}
              aria-controls={`${props.idPrefix}-panel`}
              tabIndex={selected() ? 0 : -1}
              class={props.tabClass}
              classList={
                segmented()
                  ? {
                      "flex items-center gap-1.5 rounded-sm px-3 text-[13px] whitespace-nowrap transition-colors duration-100": true,
                      "bg-slate-600 font-semibold text-chalk shadow-[0_1px_0_rgb(0_0_0/0.35),inset_0_1px_0_rgb(255_255_255/0.08)]": selected(),
                      "text-muted hover:text-chalk focus-visible:text-chalk": !selected(),
                    }
                  : {
                      "h-11 px-3.5 transition-colors duration-100": true,
                      "font-semibold text-chalk shadow-[inset_0_-3px_0_var(--color-grass-light)]": selected(),
                      "font-medium text-muted hover:text-chalk focus-visible:text-chalk": !selected(),
                    }
              }
              onClick={() => props.onChange(tab.id)}
            >
              {tab.label}
            </button>
          );
        }}
      </For>
    </div>
  );
}

/** Attributes for the panel a `Tabs` strip controls. */
export function tabPanel(idPrefix: string, value: string) {
  return { id: `${idPrefix}-panel`, role: "tabpanel", "aria-labelledby": `${idPrefix}-tab-${value}` } as const;
}
