import { Match, Show, Switch } from "solid-js";
import type { Instance, InstallProgress } from "../lib/api";
import { formatBytes, formatRelative } from "../lib/format";
import { gameState, launch, setConsoleInstance, stop } from "../lib/games";

interface Props {
  instance: Instance;
  onDelete: () => void;
  onOpenFolder: () => void;
}

function progressLabel(p: InstallProgress | undefined): string {
  if (!p || p.stage === "metadata") return "Préparation…";
  if (p.stage === "finalizing") return "Finalisation…";
  if (p.totalFiles === 0) return "Vérification…";
  return `${formatBytes(p.doneBytes)} / ${formatBytes(p.totalBytes)}`;
}

function progressRatio(p: InstallProgress | undefined): number {
  if (!p || p.stage !== "downloading" || p.totalBytes === 0) return 0;
  return Math.min(1, p.doneBytes / p.totalBytes);
}

export default function InstanceCard(props: Props) {
  const state = () => gameState(props.instance.id);

  return (
    <article class="group flex flex-col gap-3 rounded-xl border border-neutral-800 bg-neutral-900/60 p-4 transition-colors hover:border-neutral-700">
      <header class="flex items-start justify-between gap-2">
        <div class="min-w-0">
          <h3 class="truncate font-medium text-neutral-100" title={props.instance.name}>
            {props.instance.name}
          </h3>
          <p class="text-xs text-neutral-500">
            {props.instance.gameVersion} · {props.instance.loader}
          </p>
        </div>
        <div class="flex gap-1 opacity-0 transition-opacity group-hover:opacity-100">
          <button
            class="rounded px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
            onClick={props.onOpenFolder}
            title="Ouvrir le dossier"
          >
            Dossier
          </button>
          <button
            class="rounded px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-red-400 disabled:opacity-30"
            onClick={props.onDelete}
            disabled={state().status !== "idle"}
            title="Supprimer l'instance"
          >
            Supprimer
          </button>
        </div>
      </header>

      <p class="text-xs text-neutral-500">{formatRelative(props.instance.lastPlayedAt)}</p>

      <Switch>
        <Match when={state().status === "preparing"}>
          <div class="space-y-1.5">
            <div class="h-1.5 overflow-hidden rounded-full bg-neutral-800">
              <div
                class="h-full rounded-full bg-emerald-500 transition-[width] duration-150"
                classList={{ "animate-pulse w-full opacity-40": progressRatio(state().progress) === 0 }}
                style={
                  progressRatio(state().progress) > 0
                    ? { width: `${progressRatio(state().progress) * 100}%` }
                    : undefined
                }
              />
            </div>
            <p class="text-xs text-neutral-400">{progressLabel(state().progress)}</p>
          </div>
        </Match>
        <Match when={state().status === "running"}>
          <div class="flex gap-2">
            <button
              class="flex-1 rounded-md bg-neutral-800 py-2 text-sm text-neutral-200 hover:bg-neutral-700"
              onClick={() => setConsoleInstance(props.instance.id)}
            >
              En cours · console
            </button>
            <button
              class="rounded-md bg-red-600/80 px-3 py-2 text-sm text-white hover:bg-red-500"
              onClick={() => stop(props.instance.id)}
            >
              Arrêter
            </button>
          </div>
        </Match>
        <Match when={state().status === "idle"}>
          <button
            class="rounded-md bg-emerald-600 py-2 text-sm font-medium text-white hover:bg-emerald-500"
            onClick={() => launch(props.instance.id)}
          >
            Jouer
          </button>
        </Match>
      </Switch>

      <Show when={state().error}>
        <p class="text-xs text-red-400">{state().error}</p>
      </Show>
      <Show when={state().lastExit && !state().lastExit!.stopped && state().lastExit!.code !== 0}>
        <p class="text-xs text-amber-400">
          Le jeu s'est arrêté (code {state().lastExit!.code ?? "?"}).
          <Show when={state().lastExit!.crashReport}> Rapport de crash dans crash-reports.</Show>
        </p>
      </Show>
    </article>
  );
}
