import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage, type ModProvider } from "../lib/api";
import { installedContent, loadContent, setContentEnabled } from "../lib/content";
import { gameState, launch, output, stop } from "../lib/games";
import { parseMissingDeps } from "../lib/missingDeps";
import { navigate } from "../lib/store";
import { toast } from "../lib/toast";
import { Icon } from "./pixel";

/** Waits until the game has stopped (it was asked to). */
async function stopped(instanceId: string) {
  for (let i = 0; i < 100 && gameState(instanceId).status !== "idle"; i++) {
    await new Promise((r) => setTimeout(r, 100));
  }
}

/**
 * Forge and NeoForge keep the game open on an error screen when a required mod is missing,
 * so no crash happens: read their report from the output and offer the fix.
 */
export default function MissingDepsPanel(props: { instanceId: string }) {
  const missing = createMemo(() => parseMissingDeps((output[props.instanceId] ?? []).map((l) => l.line)), undefined, {
    equals: (a, b) => JSON.stringify(a) === JSON.stringify(b),
  });
  const [providers] = createResource(
    () => (missing().length > 0 ? missing().map((m) => m.modId) : null),
    async (ids) => {
      await loadContent(props.instanceId).catch(() => {});
      return api.modProviders(props.instanceId, ids).catch(() => [] as ModProvider[]);
    },
  );
  const [busy, setBusy] = createSignal<string | null>(null);

  const providerOf = (modId: string) => providers()?.find((p) => p.modId === modId);
  /** Tandem-managed content behind a disabled provider, which can be switched back on. */
  const contentOf = (p: ModProvider | undefined) =>
    p && !p.enabled ? installedContent(props.instanceId).find((c) => c.fileName === p.fileName) : undefined;

  async function reenable(modId: string) {
    const content = contentOf(providerOf(modId));
    if (!content) return;
    setBusy(modId);
    try {
      if (gameState(props.instanceId).status !== "idle") {
        await stop(props.instanceId);
        await stopped(props.instanceId);
      }
      const error = await setContentEnabled(props.instanceId, content.projectId, true);
      if (error) throw error;
      toast(`${content.title} réactivé, relance du jeu…`);
      await launch(props.instanceId);
    } catch (err) {
      toast(errorMessage(err), { tone: "error" });
    } finally {
      setBusy(null);
    }
  }

  return (
    <Show when={missing().length > 0}>
      <div class="panel px-corners-md flex flex-col gap-3 p-4 shadow-[inset_0_0_0_1px_var(--color-gold-deep)]">
        <div class="flex flex-col gap-1">
          <span class="text-sm font-medium text-gold">Le jeu ne peut pas démarrer : il manque des mods</span>
          <span class="text-sm text-chalk-2">
            {gameState(props.instanceId).status === "running"
              ? "Le jeu affiche un écran d'erreur. Ces mods sont demandés par d'autres :"
              : "Au dernier lancement, ces mods étaient demandés par d'autres :"}
          </span>
        </div>
        <ul class="flex flex-col divide-y divide-line">
          <For each={missing()}>
            {(dep) => {
              const provider = () => providerOf(dep.modId);
              const content = () => contentOf(provider());
              return (
                <li class="flex items-center gap-3 py-1.5">
                  <div class="flex min-w-0 flex-1 flex-col">
                    <span class="truncate text-sm font-medium">{provider()?.name ?? dep.modId}</span>
                    <span class="truncate text-xs text-muted">
                      <Show
                        when={dep.installed}
                        fallback={provider() ? "désactivé" : "pas installé"}
                      >
                        version {dep.installed} non compatible
                      </Show>{" "}
                      · demandé par {dep.requestedBy.join(", ")}
                    </span>
                  </div>
                  <Show
                    when={content()}
                    fallback={
                      <Show when={!provider() && !dep.installed}>
                        <button
                          class="btn px-corners h-8 shrink-0 px-2.5 text-xs"
                          onClick={() => navigate({ page: "discover", instanceId: props.instanceId, query: dep.modId })}
                        >
                          <Icon name="search" size={10} />
                          Chercher sur Modrinth
                        </button>
                      </Show>
                    }
                  >
                    <button
                      class="btn btn-primary px-corners h-8 shrink-0 px-2.5 text-xs"
                      disabled={busy() !== null}
                      onClick={() => void reenable(dep.modId)}
                    >
                      {busy() === dep.modId ? "Réactivation…" : "Réactiver et relancer"}
                    </button>
                  </Show>
                </li>
              );
            }}
          </For>
        </ul>
      </div>
    </Show>
  );
}
