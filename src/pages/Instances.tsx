import { createSignal, For, Show } from "solid-js";
import InstanceCard from "../components/InstanceCard";
import { Icon } from "../components/pixel";
import { instances, setNewInstanceDialog } from "../lib/store";

type Filter = "all" | "modded" | "vanilla";

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "Toutes" },
  { id: "modded", label: "Moddées" },
  { id: "vanilla", label: "Vanilla" },
];

export default function Instances() {
  const [filter, setFilter] = createSignal<Filter>("all");
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
        <button class="btn btn-primary px-corners h-10 px-4" onClick={() => setNewInstanceDialog({})}>
          <Icon name="plus" size={12} />
          Nouvelle instance
        </button>
      </div>

      <div role="tablist" aria-label="Filtrer" class="flex w-fit gap-0.5 bg-slate-900 p-[3px] shadow-[inset_0_0_0_1px_var(--color-line)]">
        <For each={FILTERS}>
          {(f) => (
            <button
              role="tab"
              aria-selected={filter() === f.id}
              class="h-7 px-3 text-[13px]"
              classList={{
                "bg-slate-600 font-medium text-chalk shadow-[inset_0_1px_0_rgb(255_255_255/0.06)]": filter() === f.id,
                "text-muted hover:text-chalk": filter() !== f.id,
              }}
              onClick={() => setFilter(f.id)}
            >
              {f.label}
            </button>
          )}
        </For>
      </div>

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
        <div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3.5">
          <For each={visible()}>{(instance) => <InstanceCard instance={instance} />}</For>
        </div>
      </Show>
    </div>
  );
}
