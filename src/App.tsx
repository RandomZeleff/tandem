import { createResource, createSignal, onMount, Show } from "solid-js";
import LogPanel from "./components/LogPanel";
import { api } from "./lib/api";
import { startLogStream } from "./lib/logs";

function App() {
  const [info] = createResource(api.appInfo);
  const [showLogs, setShowLogs] = createSignal(true);

  onMount(() => void startLogStream());

  return (
    <div class="flex h-full flex-col">
      <header class="flex h-12 shrink-0 items-center justify-between border-b border-neutral-800 px-4">
        <div class="flex items-baseline gap-2">
          <h1 class="text-lg font-semibold tracking-tight">Tandem</h1>
          <span class="text-xs text-neutral-500">v{info()?.version ?? "…"}</span>
        </div>
        <button
          class="rounded px-2 py-1 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200"
          onClick={() => setShowLogs((v) => !v)}
        >
          {showLogs() ? "Masquer les logs" : "Afficher les logs"}
        </button>
      </header>

      <main class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
        <p class="text-neutral-300">Aucune instance pour l'instant.</p>
        <p class="text-xs text-neutral-500" title={info()?.dataDir}>
          Données : {info()?.dataDir ?? "…"}
        </p>
      </main>

      <Show when={showLogs()}>
        <div class="h-56 shrink-0">
          <LogPanel />
        </div>
      </Show>
    </div>
  );
}

export default App;
