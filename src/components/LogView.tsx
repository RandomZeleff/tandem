import { createEffect, createMemo, createSignal, For, on } from "solid-js";
import type { LogLevel } from "../lib/api";
import { logEntries } from "../lib/logs";

const LEVEL_STYLE: Record<LogLevel, string> = {
  TRACE: "text-faint",
  DEBUG: "text-diamond",
  INFO: "text-xp-text",
  WARN: "text-gold",
  ERROR: "text-redstone-text",
};

const LEVEL_RANK: Record<LogLevel, number> = { TRACE: 0, DEBUG: 1, INFO: 2, WARN: 3, ERROR: 4 };

const timeFormat = new Intl.DateTimeFormat(undefined, {
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
});

/** Launcher log stream with a minimum-level filter. */
export default function LogView() {
  const [minLevel, setMinLevel] = createSignal<LogLevel>("INFO");
  const visible = createMemo(() =>
    logEntries().filter((e) => LEVEL_RANK[e.level] >= LEVEL_RANK[minLevel()]),
  );

  let scroller: HTMLDivElement | undefined;
  let stickToBottom = true;
  createEffect(
    on(visible, () => {
      if (scroller && stickToBottom) scroller.scrollTop = scroller.scrollHeight;
    }),
  );

  return (
    <div class="flex h-full flex-col gap-2">
      <div class="flex items-center justify-end gap-2 text-xs text-muted">
        <label for="log-level">Niveau minimum</label>
        <select
          id="log-level"
          class="field h-8 px-2 text-xs"
          value={minLevel()}
          onChange={(e) => setMinLevel(e.currentTarget.value as LogLevel)}
        >
          <For each={Object.keys(LEVEL_RANK) as LogLevel[]}>
            {(level) => <option value={level}>{level}</option>}
          </For>
        </select>
      </div>
      <div
        ref={scroller}
        class="min-h-0 flex-1 overflow-y-auto bg-slate-850 px-4 py-3 font-mono text-xs leading-5 select-text shadow-[inset_0_0_0_1px_#23272C,inset_0_3px_0_rgb(0_0_0/0.4)]"
        onScroll={(e) => {
          const el = e.currentTarget;
          stickToBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
        }}
      >
        <For each={visible()}>
          {(entry) => (
            <div class="flex gap-2 whitespace-pre-wrap">
              <span class="shrink-0 text-faint">{timeFormat.format(entry.timestampMs)}</span>
              <span class={`w-11 shrink-0 ${LEVEL_STYLE[entry.level]}`}>{entry.level}</span>
              <span class="shrink-0 text-faint">{entry.target}</span>
              <span class="text-chalk-2">{entry.message}</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
