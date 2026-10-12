import { createSignal, For, Show } from "solid-js";
import Alert from "../components/Alert";
import InstanceCard from "../components/InstanceCard";
import LauncherImportDialog from "../components/LauncherImportDialog";
import Tabs, { tabPanel } from "../components/Tabs";
import { Icon } from "../components/pixel";
import { errorMessage } from "../lib/api";
import { importModpackFile } from "../lib/modpacks";
import { instances, remembered, setNewInstanceDialog } from "../lib/store";

type Filter = "all" | "modded" | "vanilla";

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "Toutes" },
  { id: "modded", label: "Moddées" },
  { id: "vanilla", label: "Vanilla" },
];

export default function Instances() {
  const [filter, setFilter] = remembered<Filter>("filter", "all");
  const [importing, setImporting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [launcherImport, setLauncherImport] = createSignal(false);

  async function importFile() {
    setImporting(true);
    setError(null);
    try {
      await importModpackFile();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setImporting(false);
    }
  }
  const visible = () =>
    instances().filter((i) =>
      filter() === "all" ? true : filter() === "vanilla" ? i.loader === "vanilla" : i.loader !== "vanilla",
    );

  return (
    <div class="mx-auto flex max-w-[1400px] flex-col gap-5">
      <div class="flex flex-wrap items-end justify-between gap-x-4 gap-y-3">
        <div class="flex min-w-0 flex-col gap-1.5">
          <h1 class="title-display text-[44px] leading-[0.9]">Instances</h1>
          <span class="text-base text-chalk-2">Chaque instance a son propre dossier, ses mondes et ses réglages.</span>
        </div>
        <div class="flex gap-2 whitespace-nowrap">
          <button class="btn px-corners h-10 px-4" onClick={() => setLauncherImport(true)} title="Modrinth App, Prism, app CurseForge, launcher officiel… (fichiers déjà sur ce PC)">
            <Icon name="arrow" size={12} />
            Depuis un autre launcher
          </button>
          <button
            class="btn px-corners h-10 px-4"
            disabled={importing()}
            onClick={() => void importFile()}
            title="Fichier .mrpack (Modrinth)"
          >
            <Icon name="folder" size={12} />
            {importing() ? "Import…" : "Importer un modpack"}
          </button>
          <button class="btn btn-primary px-corners h-10 px-4" onClick={() => setNewInstanceDialog({})}>
            <Icon name="plus" size={12} />
            Nouvelle instance
          </button>
        </div>
      </div>

      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <Show when={launcherImport()}>
        <LauncherImportDialog onClose={() => setLauncherImport(false)} />
      </Show>

      <Tabs label="Filtrer" idPrefix="instances-filter" variant="segmented" tabClass="h-7" items={FILTERS} value={filter()} onChange={setFilter} />

      <Show
        when={visible().length > 0}
        fallback={
          <div class="flex flex-col items-center gap-3 py-20 text-center">
            <p class="text-chalk-2">Aucune instance ici.</p>
            <button class="btn btn-primary px-corners" onClick={() => setNewInstanceDialog({})}>
              Créer une instance
            </button>
          </div>
        }
      >
        <div {...tabPanel("instances-filter", filter())} class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4">
          <For each={visible()}>{(instance) => <InstanceCard instance={instance} />}</For>
        </div>
      </Show>
    </div>
  );
}
