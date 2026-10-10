import { open } from "@tauri-apps/plugin-dialog";
import { createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage, type FoundInstance, type LauncherSource } from "../lib/api";
import { formatRelative, loaderLabel } from "../lib/format";
import { navigate, refetchInstances } from "../lib/store";
import { toast } from "../lib/toast";
import Dialog from "./Dialog";
import LoadingRows from "./LoadingRows";
import { Icon } from "./pixel";

const SOURCES: Record<LauncherSource, string> = {
  modrinthApp: "Modrinth App",
  prism: "Prism Launcher",
  curseForge: "CurseForge",
  official: "Launcher officiel",
  folder: "Dossier",
};

/** Instances already imported, so the list can say so (they can still be imported again). */
const IMPORTED_SETTING = "imported_launcher_instances";
const keyOf = (f: FoundInstance) => `${f.gameDir}|${f.name}`;

function versionLabel(f: FoundInstance): string {
  if (f.gameVersion === "latest-release") return "Dernière version";
  if (f.gameVersion === "latest-snapshot") return "Dernière snapshot";
  return f.gameVersion;
}

function details(f: FoundInstance): string {
  const parts = [versionLabel(f), loaderLabel(f.loader)];
  if (f.mods > 0) parts.push(`${f.mods} mods`);
  if (f.worlds > 0) parts.push(f.worlds === 1 ? "1 monde" : `${f.worlds} mondes`);
  if (f.lastPlayed) parts.push(`joué ${formatRelative(new Date(f.lastPlayed).toISOString())}`);
  return parts.join(" · ");
}

/**
 * Instances of other launchers found on this computer, plus a folder picker for the rest.
 * Importing copies the instance (the original is left untouched) and opens it.
 */
export default function LauncherImportDialog(props: { onClose: () => void }) {
  const [found] = createResource(() => api.scanOtherLaunchers().catch(() => [] as FoundInstance[]));
  const [imported, { mutate: setImported }] = createResource(
    async () => new Set((await api.getSetting<string[]>(IMPORTED_SETTING).catch(() => null)) ?? []),
  );
  const [picking, setPicking] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  /** Instances of the folder the player picked, shown first. */
  const [picked, setPicked] = createSignal<FoundInstance[]>([]);

  const groups = () => {
    const first = new Set(picked().map(keyOf));
    const list = (found() ?? []).filter((f) => !first.has(keyOf(f)));
    return [
      { title: "Dossier choisi", items: picked() },
      ...(Object.keys(SOURCES) as LauncherSource[]).map((source) => ({
        title: SOURCES[source],
        items: list.filter((f) => f.source === source),
      })),
    ].filter((g) => g.items.length > 0);
  };

  async function pickFolder() {
    const path = await open({ directory: true, multiple: false, title: "Dossier d'une instance ou d'un launcher" });
    if (typeof path !== "string") return;
    setPicking(true);
    setError(null);
    try {
      const more = await api.inspectLauncherFolder(path);
      if (more.length === 0) throw "Aucune instance trouvée dans ce dossier";
      setPicked(more);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPicking(false);
    }
  }

  async function importOne(f: FoundInstance) {
    props.onClose();
    toast(`Import de « ${f.name} »… L'original n'est pas modifié.`, { tone: "info" });
    try {
      const instance = await api.importFromLauncher(f);
      const next = new Set(imported() ?? []).add(keyOf(f));
      setImported(next);
      void api.setSetting(IMPORTED_SETTING, [...next]).catch(() => {});
      await refetchInstances();
      toast(`« ${instance.name} » importée`, {
        action: { label: "Ouvrir", run: () => navigate({ page: "instance", id: instance.id }) },
      });
    } catch (err) {
      toast(`Import de « ${f.name} » impossible : ${errorMessage(err)}`, { tone: "error", durationMs: 10_000 });
    }
  }

  return (
    <Dialog title="Importer depuis un autre launcher" onClose={props.onClose} width={640}>
      <p class="-mt-2 text-sm text-chalk-2">
        L'instance est copiée dans Tandem avec ses mods, mondes et réglages ; l'originale n'est pas modifiée.
      </p>

      <div class="-mx-2 flex max-h-[min(460px,60vh)] flex-col gap-4 overflow-y-auto px-2">
        <Show when={!found.loading} fallback={<LoadingRows count={4} />}>
          <Show
            when={groups().length > 0}
            fallback={
              <p class="py-6 text-center text-sm text-muted">
                Aucun autre launcher trouvé à son emplacement habituel. Choisis le dossier d'une instance ci-dessous.
              </p>
            }
          >
            <For each={groups()}>
              {(group) => (
                <section class="flex flex-col gap-1">
                  <h3 class="panel-title">{group.title}</h3>
                  <ul class="flex flex-col divide-y divide-line">
                    <For each={group.items}>
                      {(f) => (
                        <li class="flex items-center gap-3 py-2">
                          <div class="flex min-w-0 flex-1 flex-col">
                            <span class="flex items-center gap-2">
                              <span class="truncate text-sm font-medium" title={f.gameDir}>
                                {f.name}
                              </span>
                              <Show when={imported()?.has(keyOf(f))}>
                                <span class="shrink-0 text-xs text-xp-text">déjà importée</span>
                              </Show>
                            </span>
                            <span class="truncate text-xs text-muted">
                              <Show when={group.title === "Dossier choisi" && f.source !== "folder"}>{SOURCES[f.source]} · </Show>
                              {details(f)}
                            </span>
                          </div>
                          <button class="btn px-corners h-8 shrink-0 px-3 text-xs" onClick={() => void importOne(f)}>
                            Importer
                          </button>
                        </li>
                      )}
                    </For>
                  </ul>
                </section>
              )}
            </For>
          </Show>
        </Show>
      </div>

      <Show when={error()}>
        <p class="text-sm text-redstone-text">{error()}</p>
      </Show>

      <div class="flex items-center justify-between gap-3">
        <button class="btn px-corners h-9 px-3 text-sm" disabled={picking()} onClick={() => void pickFolder()}>
          <Icon name="folder" size={12} />
          {picking() ? "Lecture du dossier…" : "Choisir un dossier…"}
        </button>
        <button class="btn btn-ghost" onClick={props.onClose}>
          Fermer
        </button>
      </div>
    </Dialog>
  );
}
