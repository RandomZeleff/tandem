import { createSignal, For, Show } from "solid-js";
import Alert from "../components/Alert";
import InstanceCard from "../components/InstanceCard";
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
    <div class="flex flex-col gap-5">
      <div class="flex items-end justify-between gap-4">
        <div class="flex flex-col gap-1">
          <h1 class="pixel-shadow font-pixel text-3xl font-bold">Instances</h1>
          <span class="text-[13px] text-muted">Chaque instance a son propre dossier, ses mondes et ses réglages.</span>
        </div>
        <div class="flex gap-2">
          <button class="btn px-corners h-10 px-4" disabled={importing()} onClick={() => void importFile()}>
            <Icon name="folder" size={12} />
            {importing() ? "Import…" : "Importer un .mrpack"}
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

      <Tabs label="Filtrer" idPrefix="instances-filter" variant="segmented" tabClass="h-7" items={FILTERS} value={filter()} onChange={setFilter} />

      <Show
        when={visible().length > 0}
        fallback={
          <div class="flex flex-col items-center gap-3 py-20 text-center">
            <p class="text-chalk-2">Aucune instance ici.</p>
            <button class="btn px-corners" onClick={() => setNewInstanceDialog({})}>
              Créer une instance
            </button>
          </div>
        }
      >
        <div {...tabPanel("instances-filter", filter())} class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3.5">
          <For each={visible()}>{(instance) => <InstanceCard instance={instance} />}</For>
        </div>
      </Show>
    </div>
  );
}
