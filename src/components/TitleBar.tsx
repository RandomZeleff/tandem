import { createMemo, Show } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { games } from "../lib/games";
import { canGoBack, canGoForward, goBack, goForward, instances, navigate } from "../lib/store";
import { Icon, XpBar } from "./pixel";

// macOS draws its own traffic lights over the bar (titleBarStyle: Overlay).
const isMac = navigator.userAgent.includes("Mac");

function Logo() {
  return (
    <svg width="24" height="16" viewBox="0 0 12 8" shape-rendering="crispEdges" aria-hidden="true">
      <rect x="0" y="2" width="6" height="6" fill="#2E6B1E" />
      <rect x="0" y="2" width="6" height="2" fill="#5DBB3F" />
      <rect x="1" y="2" width="1" height="1" fill="var(--color-xp)" />
      <rect x="6" y="0" width="6" height="6" fill="#7A5420" />
      <rect x="6" y="0" width="6" height="2" fill="#F2C744" />
      <rect x="10" y="0" width="1" height="1" fill="#FFE38A" />
      <rect x="5" y="2" width="1" height="4" fill="#0B0C0E" />
    </svg>
  );
}

export default function TitleBar() {
  const appWindow = getCurrentWindow();

  // First instance currently installing, shown as a compact progress indicator.
  const installing = createMemo(() => {
    for (const [id, state] of Object.entries(games)) {
      if (state.status !== "preparing") continue;
      const p = state.progress;
      const ratio =
        p && p.stage === "downloading" && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : 0;
      const name = instances().find((i) => i.id === id)?.name ?? id;
      return { id, name, ratio };
    }
    return null;
  });

  const control =
    "flex h-full w-[46px] items-center justify-center text-muted hover:bg-slate-600 hover:text-chalk";

  return (
    <header
      data-tauri-drag-region
      class="flex h-10 shrink-0 items-center bg-slate-900 shadow-[inset_0_-1px_0_var(--color-divider)]"
    >
      <Show when={!isMac}>
        <div
          data-tauri-drag-region
          class="flex h-full w-60 shrink-0 items-center gap-2.5 px-[22px] shadow-[inset_-1px_0_0_var(--color-divider)]"
        >
          <Logo />
          <span
            data-tauri-drag-region
            class="font-pixel text-lg font-bold tracking-[0.5px] [text-shadow:2px_2px_0_rgb(0_0_0/0.5)]"
          >
            Tandem
          </span>
        </div>
      </Show>

      <div
        data-tauri-drag-region
        class="flex h-full items-center gap-0.5 px-2"
        classList={{ "pl-[84px]": isMac }}
      >
        <button
          class="flex size-7 items-center justify-center text-muted hover:bg-slate-700 hover:text-chalk focus-visible:bg-slate-700 focus-visible:text-chalk disabled:pointer-events-none disabled:opacity-30"
          aria-label="Page précédente"
          title={isMac ? "Page précédente (⌘[)" : "Page précédente (Alt+←)"}
          disabled={!canGoBack()}
          onClick={goBack}
        >
          <Icon name="arrow" size={12} class="-scale-x-100" />
        </button>
        <button
          class="flex size-7 items-center justify-center text-muted hover:bg-slate-700 hover:text-chalk focus-visible:bg-slate-700 focus-visible:text-chalk disabled:pointer-events-none disabled:opacity-30"
          aria-label="Page suivante"
          title={isMac ? "Page suivante (⌘])" : "Page suivante (Alt+→)"}
          disabled={!canGoForward()}
          onClick={goForward}
        >
          <Icon name="arrow" size={12} />
        </button>
      </div>

      <div data-tauri-drag-region class="flex h-full flex-1 items-center justify-end gap-2 px-3">
        <Show when={installing()}>
          {(job) => (
            <button
              class="flex h-7 items-center gap-2.5 bg-slate-750 px-2.5 text-xs shadow-[inset_0_0_0_1px_var(--color-line)] hover:bg-slate-700"
              onClick={() => navigate({ page: "instance", id: job().id })}
            >
              <Icon name="download" size={12} color="var(--color-xp)" />
              <span class="max-w-40 truncate">{job().name}</span>
              <span class="w-16">
                <XpBar value={job().ratio} segments={8} height={8} label={`Installation de ${job().name}`} />
              </span>
              <span class="font-mono text-xp-text">{Math.round(job().ratio * 100)} %</span>
            </button>
          )}
        </Show>
      </div>

      <Show when={!isMac}>
        <div class="flex h-full">
          <button class={control} aria-label="Réduire" onClick={() => appWindow.minimize()}>
            <svg width="10" height="10" viewBox="0 0 10 10" shape-rendering="crispEdges" aria-hidden="true">
              <path d="M0 5h10v1H0z" fill="currentColor" />
            </svg>
          </button>
          <button class={control} aria-label="Agrandir" onClick={() => appWindow.toggleMaximize()}>
            <svg width="10" height="10" viewBox="0 0 10 10" shape-rendering="crispEdges" fill-rule="evenodd" aria-hidden="true">
              <path d="M0 0h10v10H0zM1 1v8h8V1z" fill="currentColor" />
            </svg>
          </button>
          <button
            class={`${control} hover:!bg-redstone hover:!text-white`}
            aria-label="Fermer"
            onClick={() => appWindow.close()}
          >
            <Icon name="close" size={10} />
          </button>
        </div>
      </Show>
    </header>
  );
}
