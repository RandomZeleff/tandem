import { createResource } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [version] = createResource(() => invoke<string>("core_version"));

  return (
    <main class="flex h-full flex-col items-center justify-center gap-2">
      <h1 class="text-3xl font-semibold tracking-tight">Tandem</h1>
      <p class="text-sm text-neutral-400">core v{version() ?? "…"}</p>
    </main>
  );
}

export default App;
