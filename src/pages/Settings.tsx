import { createResource } from "solid-js";
import LogView from "../components/LogView";
import { api } from "../lib/api";

export default function Settings() {
  const [info] = createResource(api.appInfo);

  return (
    <div class="flex h-full flex-col gap-5">
      <h1 class="pixel-shadow font-pixel text-3xl font-bold">Réglages</h1>

      <section class="panel px-corners-md flex flex-col gap-3 p-4">
        <h2 class="panel-title">Launcher</h2>
        <dl class="grid grid-cols-[auto_1fr] gap-x-8 gap-y-2 text-sm">
          <dt class="text-muted">Version</dt>
          <dd class="font-mono">{info()?.version ?? "…"}</dd>
          <dt class="text-muted">Dossier des données</dt>
          <dd class="font-mono text-xs break-all select-text">{info()?.dataDir ?? "…"}</dd>
        </dl>
      </section>

      <section class="panel px-corners-md flex min-h-0 flex-1 flex-col gap-3 p-4">
        <h2 class="panel-title">Journal du launcher</h2>
        <div class="min-h-0 flex-1">
          <LogView />
        </div>
      </section>
    </div>
  );
}
