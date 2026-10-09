import { revealItemInDir } from "@tauri-apps/plugin-opener";
import Alert from "./Alert";
import { createEffect, createResource, createSignal, For, on, onCleanup, onMount, Show } from "solid-js";
import { api, errorMessage, screenshotUrl, type Screenshot } from "../lib/api";
import { formatBytes } from "../lib/format";
import { trapFocus } from "../lib/ui";
import { gameState, lastOutput } from "../lib/games";
import Dialog from "./Dialog";
import { Icon } from "./pixel";

/** Minecraft logs this line each time F2 saves a screenshot. */
const SAVED_LINE = "Saved screenshot as";

const dateTime = (ms: number) => new Date(ms).toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short" });

/** In-game screenshots of an instance, as a grid with a full-size viewer. */
export default function ScreenshotsTab(props: { instanceId: string }) {
  const [error, setError] = createSignal<string | null>(null);
  // Re-read after every run, and live while playing each time F2 saves one.
  const source = () => ({ id: props.instanceId, exit: gameState(props.instanceId).lastExit });
  const [shots, { refetch, mutate }] = createResource(source, ({ id }) => api.listScreenshots(id));
  createEffect(
    on(
      lastOutput,
      (batch) => batch?.instanceId === props.instanceId && batch.lines.some((l) => l.includes(SAVED_LINE)) && void refetch(),
      { defer: true },
    ),
  );
  /** Index in `shots()` of the screenshot shown full size. */
  const [viewing, setViewing] = createSignal<number | null>(null);
  const [deleting, setDeleting] = createSignal<Screenshot | null>(null);

  const list = () => shots() ?? [];
  const totalBytes = () => list().reduce((sum, s) => sum + s.sizeBytes, 0);

  async function attempt(action: () => Promise<unknown>) {
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function confirmDelete() {
    const shot = deleting();
    if (!shot) return;
    setDeleting(null);
    await attempt(async () => {
      await api.deleteScreenshot(props.instanceId, shot.fileName);
      const remaining = list().filter((s) => s.fileName !== shot.fileName);
      mutate(remaining);
      // Stay in the viewer on the next screenshot, or close it after the last one.
      const index = viewing();
      if (index !== null) setViewing(remaining.length === 0 ? null : Math.min(index, remaining.length - 1));
    });
  }

  return (
    <div class="flex h-full flex-col gap-4 overflow-y-auto">
      <div class="flex items-center justify-between gap-3">
        <span class="text-[13px] text-muted">
          {list().length === 0
            ? "Aucune capture."
            : `${list().length} capture${list().length > 1 ? "s" : ""} · ${formatBytes(totalBytes())}`}
        </span>
        <button
          class="btn btn-ghost h-8 px-2.5 text-xs"
          onClick={() => void attempt(() => api.openScreenshotsFolder(props.instanceId))}
        >
          <Icon name="folder" size={10} />
          Ouvrir le dossier
        </button>
      </div>

      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <Show
        when={list().length > 0}
        fallback={
          <div class="panel px-corners-md flex flex-col items-center gap-2 py-12 text-center">
            <p class="text-chalk-2">Aucune capture pour l'instant.</p>
            <p class="text-sm text-muted">
              Appuie sur <kbd class="chip h-5 px-1.5 font-mono text-[11px]">F2</kbd> en jeu : tes captures apparaîtront ici.
            </p>
          </div>
        }
      >
        <ul class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
          <For each={list()}>
            {(shot, index) => (
              <li>
                <button
                  class="group relative block aspect-video w-full overflow-hidden bg-slate-700 shadow-[inset_0_0_0_1px_var(--color-line)] focus-visible:brightness-110"
                  aria-label={`Capture du ${dateTime(shot.takenAt)}`}
                  onClick={() => setViewing(index())}
                >
                  <img
                    src={screenshotUrl(props.instanceId, shot, true)}
                    alt=""
                    loading="lazy"
                    decoding="async"
                    class="size-full object-cover transition-transform duration-150 group-hover:scale-[1.03]"
                  />
                  <span class="absolute inset-x-0 bottom-0 bg-[linear-gradient(180deg,transparent,rgb(0_0_0/0.75))] px-2.5 pt-5 pb-1.5 text-left text-xs text-chalk-2 opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100">
                    {dateTime(shot.takenAt)}
                  </span>
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>

      <Show when={viewing() !== null && list()[viewing()!]}>
        {(shot) => (
          <Viewer
            instanceId={props.instanceId}
            shot={shot()}
            position={`${viewing()! + 1} / ${list().length}`}
            hasPrev={viewing()! > 0}
            hasNext={viewing()! < list().length - 1}
            onMove={(step) => setViewing(viewing()! + step)}
            onClose={() => setViewing(null)}
            onReveal={() => void attempt(() => revealItemInDir(shot().path))}
            onDelete={() => setDeleting(shot())}
            dialogOpen={deleting() !== null}
          />
        )}
      </Show>

      <Show when={deleting()}>
        {(shot) => (
          <Dialog title="Supprimer cette capture ?" onClose={() => setDeleting(null)}>
            <p class="text-chalk-2">
              La capture du {dateTime(shot().takenAt)} sera supprimée définitivement de ton disque.
            </p>
            <div class="flex justify-end gap-2">
              <button class="btn btn-ghost" onClick={() => setDeleting(null)}>
                Annuler
              </button>
              <button class="btn btn-danger px-corners" onClick={() => void confirmDelete()}>
                Supprimer
              </button>
            </div>
          </Dialog>
        )}
      </Show>
    </div>
  );
}

function Viewer(props: {
  instanceId: string;
  shot: Screenshot;
  position: string;
  hasPrev: boolean;
  hasNext: boolean;
  /** Keyboard is left to the confirmation dialog while it is open. */
  dialogOpen: boolean;
  onMove: (step: -1 | 1) => void;
  onClose: () => void;
  onReveal: () => void;
  onDelete: () => void;
}) {
  let root: HTMLDivElement | undefined;
  trapFocus(() => root);
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (props.dialogOpen) return;
      if (e.key === "Escape") props.onClose();
      else if (e.key === "ArrowLeft" && props.hasPrev) props.onMove(-1);
      else if (e.key === "ArrowRight" && props.hasNext) props.onMove(1);
      else if (e.key === "Delete") props.onDelete();
    };
    window.addEventListener("keydown", onKey);
    onCleanup(() => window.removeEventListener("keydown", onKey));
  });

  return (
    <div
      ref={root}
      tabIndex={-1}
      role="dialog"
      aria-modal="true"
      aria-label="Capture d'écran"
      class="fixed inset-0 z-20 flex flex-col bg-black/90"
      onClick={(e) => e.target === e.currentTarget && props.onClose()}
    >
      <div class="relative flex min-h-0 flex-1 items-center justify-center px-16 pt-12 pb-4" onClick={(e) => e.target === e.currentTarget && props.onClose()}>
        <img
          src={screenshotUrl(props.instanceId, props.shot)}
          alt={`Capture du ${dateTime(props.shot.takenAt)}`}
          class="max-h-full max-w-full object-contain shadow-2xl"
        />
        <Show when={props.hasPrev}>
          <button
            class="btn absolute top-1/2 left-4 size-10 -translate-y-1/2 px-0"
            aria-label="Capture précédente"
            onClick={() => props.onMove(-1)}
          >
            <Icon name="arrow" size={14} class="-scale-x-100" />
          </button>
        </Show>
        <Show when={props.hasNext}>
          <button
            class="btn absolute top-1/2 right-4 size-10 -translate-y-1/2 px-0"
            aria-label="Capture suivante"
            onClick={() => props.onMove(1)}
          >
            <Icon name="arrow" size={14} />
          </button>
        </Show>
      </div>
      <div class="flex shrink-0 items-center gap-3 border-t border-line bg-slate-800 px-5 py-3">
        <div class="flex min-w-0 flex-1 flex-col">
          <span class="truncate text-sm">{dateTime(props.shot.takenAt)}</span>
          <span class="truncate text-xs text-muted">
            {props.position} · {formatBytes(props.shot.sizeBytes)} · {props.shot.fileName}
          </span>
        </div>
        <button class="btn btn-ghost h-8 px-2.5 text-xs" onClick={props.onReveal}>
          <Icon name="folder" size={10} />
          Afficher dans le dossier
        </button>
        <button class="btn btn-danger h-8 px-2.5 text-xs" onClick={props.onDelete}>
          <Icon name="trash" size={10} />
          Supprimer
        </button>
        <button class="btn btn-ghost h-8 w-8 px-0" aria-label="Fermer" onClick={props.onClose}>
          <Icon name="close" size={12} />
        </button>
      </div>
    </div>
  );
}
