import { createResource, createSignal, For, Show } from "solid-js";
import { api, type Instance } from "../lib/api";
import { installContent, installedContent, isBusy } from "../lib/content";
import { Icon } from "./pixel";
import ProjectIcon from "./ProjectIcon";

/** What each catalog mod brings, by Modrinth project id. */
const PURPOSE: Record<string, string> = {
  AANobbMI: "Moteur de rendu bien plus rapide : souvent deux à trois fois plus de FPS.",
  sk9rgfiA: "Le moteur de rendu de Sodium, adapté à Forge.",
  gvQqBUqZ: "Optimise la logique du jeu (mobs, blocs, physique) sans toucher au gameplay.",
  uXXizFIs: "Réduit nettement la mémoire utilisée.",
  nmDcB62a: "Démarrage plus rapide et moins de mémoire, surtout avec beaucoup de mods.",
  "5ZwdcRci": "Accélère l'affichage de l'interface, du texte et des entités.",
  NNAgCjsB: "N'affiche plus les entités et les coffres cachés derrière les murs.",
  LQ3K71Q1: "Baisse les FPS quand le jeu est en arrière-plan : moins de chauffe et de batterie.",
};

const hiddenKey = (id: string) => `perf_suggestions_hidden.${id}`;

/** Performance mods the instance lacks, installable in one click. Errors (offline) hide the panel. */
export default function PerfSuggestions(props: {
  instance: Instance;
  locked: boolean;
  onError: (message: string | null) => void;
}) {
  const id = () => props.instance.id;
  const [hidden, { mutate: setHidden }] = createResource(id, async (i) => (await api.getSetting<boolean>(hiddenKey(i))) ?? false);
  // Refetched whenever the installed list changes, so installed mods drop out.
  const [suggestions] = createResource(
    () => (props.instance.loader === "vanilla" ? null : { id: id(), count: installedContent(id()).length }),
    ({ id }) => api.perfSuggestions(id).catch(() => []),
  );
  const [installingAll, setInstallingAll] = createSignal(false);
  const list = () => suggestions.latest ?? [];

  async function install(projectId: string) {
    props.onError(await installContent(id(), projectId));
  }

  async function installAll() {
    setInstallingAll(true);
    for (const s of list()) {
      const error = await installContent(id(), s.projectId);
      if (error) {
        props.onError(error);
        break;
      }
    }
    setInstallingAll(false);
  }

  async function hide() {
    setHidden(true);
    await api.setSetting(hiddenKey(id()), true);
  }

  return (
    <Show when={hidden() === false && list().length > 0}>
      <section class="panel px-corners-md flex flex-col gap-3 p-4 shadow-[inset_0_0_0_1px_#4E9A1E]">
        <div class="flex items-start justify-between gap-4">
          <div class="flex flex-col gap-1">
            <h2 class="flex items-center gap-2 text-sm font-medium">
              <Icon name="sparkle" size={12} class="text-xp-text" />
              Optimisations recommandées
            </h2>
            <p class="text-xs text-muted">Mods reconnus pour gagner des FPS et de la mémoire, compatibles avec cette instance.</p>
          </div>
          <div class="flex shrink-0 gap-2">
            <button class="btn btn-ghost h-8 px-2.5 text-xs" onClick={() => void hide()}>
              Masquer
            </button>
            <button
              class="btn btn-primary px-corners h-8 px-3 text-xs"
              disabled={props.locked || installingAll()}
              onClick={() => void installAll()}
            >
              <Icon name="download" size={10} />
              {installingAll() ? "Installation…" : "Tout installer"}
            </button>
          </div>
        </div>
        <ul class="flex flex-col divide-y divide-line">
          <For each={list()}>
            {(s) => (
              <li class="flex items-center gap-3 py-2">
                <ProjectIcon url={s.iconUrl} size={32} />
                <div class="flex min-w-0 flex-1 flex-col">
                  <span class="truncate text-sm font-medium">{s.title}</span>
                  <span class="truncate text-xs text-muted">{PURPOSE[s.projectId]}</span>
                </div>
                <button
                  class="btn px-corners h-8 shrink-0 px-2.5 text-xs"
                  disabled={props.locked || installingAll() || isBusy(id(), s.projectId)}
                  onClick={() => void install(s.projectId)}
                >
                  {isBusy(id(), s.projectId) ? "Installation…" : "Installer"}
                </button>
              </li>
            )}
          </For>
        </ul>
      </section>
    </Show>
  );
}
