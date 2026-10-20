import { isTransferStage } from "../lib/api";
import { createMemo, Show } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { games } from "../lib/games";
import { canGoBack, canGoForward, goBack, goForward, instances, navigate } from "../lib/store";
import { sidebarCollapsed, toggleSidebar } from "../lib/ui";
import { setUpdateDialog, update } from "../lib/updates";
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
        p && isTransferStage(p.stage) && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : null;
      const name = instances().find((i) => i.id === id)?.name ?? id;
      return { id, name, ratio };
    }
    return null;
  });

  const control =
    "flex h-full w-[46px] items-center justify-center text-muted hover:bg-white/[0.08] hover:text-chalk";
  const navButton =
    "flex size-7 items-center justify-center rounded-sm text-muted hover:bg-white/[0.08] hover:text-chalk focus-visible:bg-white/[0.08] focus-visible:text-chalk disabled:pointer-events-none disabled:opacity-30";

  return (
    <header
      data-tauri-drag-region
      class="flex h-10 shrink-0 items-center bg-slate-900"
    >
      <Show when={!isMac}>
        <div
          data-tauri-drag-region
          class="flex h-full shrink-0 items-center gap-2.5 overflow-hidden px-[22px] shadow-[inset_-1px_0_0_var(--color-divider)] transition-[width] duration-150 ease-out motion-reduce:transition-none"
          classList={{ "w-60": !sidebarCollapsed(), "w-20 justify-center px-0": sidebarCollapsed() }}
        >
          <Logo />
          <span data-tauri-drag-region class="title-display text-xl leading-none" classList={{ hidden: sidebarCollapsed() }}>
            Tandem
          </span>
        </div>
      </Show>

      {/* Everything right of the brand column sits on the content side of the frame. */}
      <div data-tauri-drag-region class="flex h-full min-w-0 flex-1 items-center shadow-[inset_0_-1px_0_var(--color-divider)]">
      <div
        data-tauri-drag-region
        class="flex h-full items-center gap-0.5 px-2"
        classList={{ "pl-[84px]": isMac }}
      >
        <button
          class={navButton}
          aria-label={sidebarCollapsed() ? "Afficher le menu" : "Réduire le menu"}
          aria-expanded={!sidebarCollapsed()}
          title={`${sidebarCollapsed() ? "Afficher le menu" : "Réduire le menu"} (${isMac ? "⌘B" : "Ctrl+B"})`}
          onClick={toggleSidebar}
        >
          <Icon name="sidebar" size={12} />
        </button>
        <span class="mx-1 h-4 w-px bg-divider" aria-hidden="true" />
        <button
          class={navButton}
          aria-label="Page précédente"
          title={isMac ? "Page précédente (⌘[)" : "Page précédente (Alt+←)"}
          disabled={!canGoBack()}
          onClick={goBack}
        >
          <Icon name="arrow" size={12} class="-scale-x-100" />
        </button>
        <button
          class={navButton}
          aria-label="Page suivante"
          title={isMac ? "Page suivante (⌘])" : "Page suivante (Alt+→)"}
          disabled={!canGoForward()}
          onClick={goForward}
        >
          <Icon name="arrow" size={12} />
        </button>
      </div>

      <div data-tauri-drag-region class="flex h-full flex-1 items-center justify-end gap-2 px-3">
        <Show when={update()}>
          {(found) => (
            <button
              class="flex h-7 items-center gap-2 rounded-sm bg-slate-750 px-2.5 text-xs shadow-[inset_0_0_0_1px_var(--color-success-line)] hover:bg-slate-700 focus-visible:bg-slate-700"
              onClick={() => setUpdateDialog(true)}
            >
              <Icon name="download" size={12} color="var(--color-xp)" />
              Tandem {found().version}
            </button>
          )}
        </Show>
        <Show when={installing()}>
          {(job) => (
            <button
              class="flex h-7 items-center gap-2.5 rounded-sm bg-slate-750 px-2.5 text-xs shadow-[inset_0_0_0_1px_var(--color-line)] hover:bg-slate-700 focus-visible:bg-slate-700"
              onClick={() => navigate({ page: "instance", id: job().id })}
            >
              <Icon name="download" size={12} color="var(--color-xp)" />
              <span class="max-w-40 truncate">{job().name}</span>
              <span class="w-16" classList={{ hidden: job().ratio === null }}>
                <XpBar value={job().ratio ?? 0} segments={8} height={8} label={`Installation de ${job().name}`} />
              </span>
              <Show when={job().ratio !== null} fallback={<span class="text-muted">Préparation…</span>}>
                <span class="font-mono text-xp-text">{Math.round(job().ratio! * 100)} %</span>
              </Show>
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
      </div>
    </header>
  );
}
