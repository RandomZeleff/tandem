import { createSignal, Show } from "solid-js";
import { duoActive } from "../lib/duo";
import { formatBytes } from "../lib/format";
import { games } from "../lib/games";
import { installUpdate, update, updateProgress } from "../lib/updates";
import Dialog from "./Dialog";
import { XpBar } from "./pixel";

/** Shows what the new version brings and installs it; Tandem restarts on its own. */
export default function UpdateDialog(props: { onClose: () => void }) {
  const [error, setError] = createSignal<string | null>(null);
  const installing = () => updateProgress() !== null;
  const gameBusy = () => Object.values(games).some((g) => g.status !== "idle");
  const blocker = () =>
    gameBusy()
      ? "Ferme le jeu avant de mettre Tandem à jour."
      : duoActive()
        ? "Termine la partie avec tes amis avant de mettre Tandem à jour."
        : null;

  const install = async () => {
    setError(null);
    setError(await installUpdate());
  };

  const ratio = () => {
    const p = updateProgress();
    return p?.total ? p.downloaded / p.total : 0;
  };

  return (
    <Dialog title={`Tandem ${update()?.version ?? ""}`} onClose={() => !installing() && props.onClose()}>
      <Show when={update()?.notes}>
        {(notes) => (
          <p class="max-h-64 overflow-y-auto text-sm whitespace-pre-line text-chalk-2 select-text">{notes().trim()}</p>
        )}
      </Show>
      <p class="text-sm text-muted">
        Tu as la version {update()?.currentVersion}. Tandem redémarre une fois la mise à jour installée.
      </p>

      <Show when={installing()}>
        <div class="flex items-center gap-3">
          <div class="flex-1">
            <XpBar value={ratio()} label="Téléchargement de la mise à jour" />
          </div>
          <span class="font-mono text-xs text-muted">{formatBytes(updateProgress()?.downloaded ?? 0)}</span>
        </div>
      </Show>

      <Show when={error() ?? (!installing() && blocker())}>
        {(message) => <p class="text-sm text-redstone-text">{message()}</p>}
      </Show>

      <div class="flex justify-end gap-2">
        <button class="btn btn-ghost" disabled={installing()} onClick={props.onClose}>
          Plus tard
        </button>
        <button class="btn btn-primary px-corners px-5" disabled={installing() || !!blocker()} onClick={install}>
          {installing() ? "Mise à jour…" : "Mettre à jour"}
        </button>
      </div>
    </Dialog>
  );
}
