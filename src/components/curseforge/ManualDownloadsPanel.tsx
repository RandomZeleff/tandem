import { createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { api, errorMessage } from "../../lib/api";
import { loadContent } from "../../lib/content";
import { formatBytes } from "../../lib/format";
import { gameState } from "../../lib/games";
import { openExternal } from "../../lib/projects";
import { toast } from "../../lib/toast";
import { Icon } from "../pixel";

/** How often the Downloads folder is checked while files are awaited. */
const POLL_MS = 3000;

/**
 * Files of a CurseForge pack that only the player may download: one link per file, and the
 * Downloads folder is watched so each one lands in the instance as soon as it is there.
 */
export default function ManualDownloadsPanel(props: { instanceId: string }) {
  // Refetched when an install ends (the list is written at the end of the import).
  const [files, { mutate }] = createResource(
    () => `${props.instanceId}:${gameState(props.instanceId).status === "preparing"}`,
    () => api.manualDownloads(props.instanceId).catch(() => []),
  );
  const [checking, setChecking] = createSignal(false);

  async function collect(manual: boolean) {
    const before = files()?.length ?? 0;
    if (before === 0 || checking() || gameState(props.instanceId).status === "preparing") return;
    setChecking(true);
    try {
      const missing = await api.collectManualDownloads(props.instanceId);
      mutate(missing);
      if (missing.length < before) {
        void loadContent(props.instanceId).catch(() => {});
        toast(
          missing.length === 0
            ? "Tous les fichiers sont là : le modpack est complet"
            : `${before - missing.length} fichier${before - missing.length > 1 ? "s" : ""} récupéré${before - missing.length > 1 ? "s" : ""}`,
        );
      } else if (manual) {
        toast("Aucun des fichiers attendus dans Téléchargements pour l'instant", { tone: "info" });
      }
    } catch (err) {
      if (manual) toast(errorMessage(err), { tone: "error" });
    } finally {
      setChecking(false);
    }
  }

  onMount(() => {
    const timer = setInterval(() => void collect(false), POLL_MS);
    const onFocus = () => void collect(false);
    window.addEventListener("focus", onFocus);
    onCleanup(() => {
      clearInterval(timer);
      window.removeEventListener("focus", onFocus);
    });
  });

  return (
    <Show when={(files()?.length ?? 0) > 0}>
      <div class="panel px-corners-md flex flex-col gap-3 p-4 shadow-[inset_0_0_0_1px_var(--color-gold-deep)]">
        <div class="flex items-start justify-between gap-4">
          <div class="flex flex-col gap-1">
            <span class="text-sm font-medium text-gold">
              {files()!.length === 1 ? "1 fichier à télécharger à la main" : `${files()!.length} fichiers à télécharger à la main`}
            </span>
            <span class="text-sm text-chalk-2">
              Leurs auteurs interdisent aux launchers de les télécharger. Ouvre chaque page et clique sur « Download » :
              Tandem les récupère tout seul dans ton dossier Téléchargements.
            </span>
          </div>
          <button class="btn px-corners h-8 shrink-0 px-2.5 text-xs" disabled={checking()} onClick={() => void collect(true)}>
            {checking() ? "Recherche…" : "Vérifier maintenant"}
          </button>
        </div>
        <ul class="flex max-h-64 flex-col divide-y divide-line overflow-y-auto">
          <For each={files()}>
            {(file) => (
              <li class="flex items-center gap-3 py-1.5">
                <div class="flex min-w-0 flex-1 flex-col">
                  <span class="truncate text-sm font-medium">{file.name}</span>
                  <span class="truncate text-xs text-muted" title={file.fileName}>
                    {file.fileName}
                    <Show when={file.size > 0}> · {formatBytes(file.size)}</Show>
                  </span>
                </div>
                <button class="btn px-corners h-8 shrink-0 px-2.5 text-xs" onClick={() => openExternal(file.url)}>
                  <Icon name="external" size={10} />
                  Ouvrir la page
                </button>
              </li>
            )}
          </For>
        </ul>
      </div>
    </Show>
  );
}
