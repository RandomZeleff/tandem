import { createEffect, createMemo, createSignal, For, on, Show } from "solid-js";
import type { LogLevel } from "../lib/api";
import { consoleInstance } from "../lib/games";
import { logEntries } from "../lib/logs";
import GameConsole from "./GameConsole";

const LEVEL_STYLE: Record<LogLevel, string> = {
  TRACE: "text-neutral-500",
  DEBUG: "text-sky-400",
  INFO: "text-emerald-400",
  WARN: "text-amber-400",
  ERROR: "text-red-400",
};

const LEVEL_RANK: Record<LogLevel, number> = { TRACE: 0, DEBUG: 1, INFO: 2, WARN: 3, ERROR: 4 };

const timeFormat = new Intl.DateTimeFormat(undefined, {
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
});

type Tab = "game" | "launcher";

function LauncherLogs(props: { minLevel: LogLevel }) {
  const visible = createMemo(() =>
    logEntries().filter((e) => LEVEL_RANK[e.level] >= LEVEL_RANK[props.minLevel]),
  );

  let scroller: HTMLDivElement | undefined;
  let stickToBottom = true;
  createEffect(
    on(visible, () => {
      if (scroller && stickToBottom) scroller.scrollTop = scroller.scrollHeight;
    }),
  );

  return (
    <div
      ref={scroller}
      class="h-full overflow-y-auto px-3 pb-2 font-mono text-xs leading-5 select-text"
      onScroll={(e) => {
        const el = e.currentTarget;
        stickToBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
      }}
    >
      <For each={visible()}>
        {(entry) => (
          <div class="flex gap-2 whitespace-pre-wrap">
            <span class="shrink-0 text-neutral-500">{timeFormat.format(entry.timestampMs)}</span>
            <span class={`w-11 shrink-0 ${LEVEL_STYLE[entry.level]}`}>{entry.level}</span>
            <span class="shrink-0 text-neutral-500">{entry.target}</span>
            <span class="text-neutral-200">{entry.message}</span>
          </div>
        )}
      </For>
    </div>
  );
}

export default function LogPanel() {
  const [tab, setTab] = createSignal<Tab>("launcher");
  const [minLevel, setMinLevel] = createSignal<LogLevel>("INFO");

  // Jump to the game console whenever a new instance is launched.
  createEffect(on(consoleInstance, (id) => id && setTab("game"), { defer: true }));

  const tabClass = (t: Tab) =>
    tab() === t ? "text-neutral-100" : "text-neutral-500 hover:text-neutral-300";

  return (
    <section class="flex h-full flex-col border-t border-neutral-800 bg-neutral-900/60">
      <header class="flex items-center justify-between px-3 py-1.5 text-xs">
        <nav class="flex gap-3 font-medium tracking-wide uppercase">
          <button class={tabClass("launcher")} onClick={() => setTab("launcher")}>
            Launcher
          </button>
          <button class={tabClass("game")} onClick={() => setTab("game")}>
            Jeu
            <Show when={consoleInstance()}>
              <span class="ml-1 normal-case text-neutral-500">({consoleInstance()})</span>
            </Show>
          </button>
        </nav>
        <Show when={tab() === "launcher"}>
          <select
            class="rounded border border-neutral-700 bg-neutral-900 px-1.5 py-0.5 text-neutral-300"
            value={minLevel()}
            onChange={(e) => setMinLevel(e.currentTarget.value as LogLevel)}
          >
            <For each={Object.keys(LEVEL_RANK) as LogLevel[]}>
              {(level) => <option value={level}>{level}</option>}
            </For>
          </select>
        </Show>
      </header>
      <div class="min-h-0 flex-1">
        <Show when={tab() === "launcher"} fallback={<GameConsole />}>
          <LauncherLogs minLevel={minLevel()} />
        </Show>
      </div>
    </section>
  );
}
