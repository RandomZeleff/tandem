import { createResource, createSignal, For, onMount, Show } from "solid-js";
import AccountMenu from "./components/AccountMenu";
import InstanceCard from "./components/InstanceCard";
import LogPanel from "./components/LogPanel";
import NewInstanceDialog from "./components/NewInstanceDialog";
import { api, errorMessage, type Instance } from "./lib/api";
import { onGamePlayed, startGameEvents } from "./lib/games";
import { startLogStream } from "./lib/logs";

function App() {
  const [info] = createResource(api.appInfo);
  const [instances, { refetch }] = createResource(api.listInstances);
  const [showLogs, setShowLogs] = createSignal(true);
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  onMount(() => {
    void startLogStream();
    void startGameEvents();
    onGamePlayed(() => void refetch());
  });

  async function remove(instance: Instance) {
    if (!confirm(`Supprimer « ${instance.name} » et tout son dossier (mondes compris) ?`)) return;
    try {
      await api.deleteInstance(instance.id);
      await refetch();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function openFolder(instance: Instance) {
    try {
      await api.openInstanceFolder(instance.id);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <div class="flex h-full flex-col">
      <header class="flex h-12 shrink-0 items-center justify-between border-b border-neutral-800 px-4">
        <div class="flex items-baseline gap-2">
          <h1 class="text-lg font-semibold tracking-tight">Tandem</h1>
          <span class="text-xs text-neutral-500">v{info()?.version ?? "…"}</span>
        </div>
        <div class="flex items-center gap-2">
          <button
            class="rounded px-2 py-1 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
            onClick={() => setShowLogs((v) => !v)}
          >
            {showLogs() ? "Masquer la console" : "Afficher la console"}
          </button>
          <AccountMenu />
        </div>
      </header>

      <main class="min-h-0 flex-1 overflow-y-auto p-6">
        <div class="mb-5 flex items-center justify-between">
          <h2 class="text-sm font-medium tracking-wide text-neutral-400 uppercase">Instances</h2>
          <button
            class="rounded-md bg-neutral-800 px-3 py-1.5 text-sm text-neutral-200 hover:bg-neutral-700"
            onClick={() => setCreating(true)}
          >
            Nouvelle instance
          </button>
        </div>

        <Show when={error()}>
          <p class="mb-4 rounded-md border border-red-900 bg-red-950/40 px-3 py-2 text-sm text-red-300">
            {error()}
            <button class="ml-2 text-red-400 underline" onClick={() => setError(null)}>
              OK
            </button>
          </p>
        </Show>

        <Show
          when={(instances()?.length ?? 0) > 0}
          fallback={
            <Show when={!instances.loading}>
              <div class="flex flex-col items-center gap-3 py-20 text-center">
                <p class="text-neutral-300">Aucune instance pour l'instant.</p>
                <button
                  class="rounded-md bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-500"
                  onClick={() => setCreating(true)}
                >
                  Créer ma première instance
                </button>
              </div>
            </Show>
          }
        >
          <div class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4">
            <For each={instances()}>
              {(instance) => (
                <InstanceCard
                  instance={instance}
                  onDelete={() => remove(instance)}
                  onOpenFolder={() => openFolder(instance)}
                />
              )}
            </For>
          </div>
        </Show>
      </main>

      <Show when={showLogs()}>
        <div class="h-56 shrink-0">
          <LogPanel />
        </div>
      </Show>

      <Show when={creating()}>
        <NewInstanceDialog
          onClose={() => setCreating(false)}
          onCreated={() => {
            setCreating(false);
            void refetch();
          }}
        />
      </Show>
    </div>
  );
}

export default App;
