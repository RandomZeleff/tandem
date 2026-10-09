import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { For, onMount, Show } from "solid-js";
import { errorMessage, type CrashHint, type GameExited, type SuspectReason } from "../lib/api";
import { installedContent, isBusy, loadContent, setContentEnabled } from "../lib/content";
import { Icon } from "./pixel";

const HINTS: Record<CrashHint, string> = {
  outOfMemory: "Le jeu a manqué de mémoire. Donne-lui-en plus dans l'onglet Informations.",
  wrongJava: "Un mod demande une version de Java plus récente que celle prévue pour cette version de Minecraft.",
  graphics: "La fenêtre du jeu n'a pas pu s'ouvrir : souvent un souci de pilote graphique ou d'OpenGL.",
  incompatibleMods: "Des mods ne sont pas compatibles avec cette version du jeu, ou entre eux.",
  nativeCrash: "Java lui-même a planté, souvent à cause d'un pilote graphique ou d'un mod qui utilise du code natif.",
};

const REASONS: Record<SuspectReason, string> = {
  namedByLoader: "désigné par le loader",
  mixin: "une de ses modifications du jeu a échoué",
  incompatible: "incompatible",
  stackTrace: "apparaît dans l'erreur",
};

/** What went wrong in the last run, the mods to suspect, and one-click fixes. */
export default function CrashPanel(props: {
  instanceId: string;
  exit: GameExited;
  locked: boolean;
  onShowMemory: () => void;
  onError: (message: string | null) => void;
}) {
  const analysis = () => props.exit.analysis;
  onMount(() => void loadContent(props.instanceId).catch(() => {}));

  /** Tandem-managed content behind a suspect jar, so it can be switched off. */
  const contentFor = (fileName: string | null) =>
    fileName ? installedContent(props.instanceId).find((c) => c.fileName === fileName && c.enabled) : undefined;

  async function reveal(path: string) {
    try {
      await revealItemInDir(path);
    } catch (err) {
      props.onError(errorMessage(err));
    }
  }

  return (
    <div class="panel px-corners-md flex flex-col gap-3 p-4 shadow-[inset_0_0_0_1px_var(--color-gold-deep)]">
      <div class="flex items-start justify-between gap-4">
        <div class="flex flex-col gap-1">
          <span class="text-sm font-medium text-gold">Le jeu a planté (code {props.exit.code ?? "?"})</span>
          <Show when={analysis()?.hint} fallback={<span class="text-sm text-chalk-2">{analysis()?.description ?? "Cause inconnue."}</span>}>
            {(hint) => <span class="text-sm text-chalk-2">{HINTS[hint()]}</span>}
          </Show>
        </div>
        <div class="flex shrink-0 gap-2">
          <Show when={analysis()?.hint === "outOfMemory"}>
            <button class="btn px-corners h-8 px-2.5 text-xs" onClick={props.onShowMemory}>
              Régler la mémoire
            </button>
          </Show>
          <Show when={analysis()?.source}>
            {(source) => (
              <button class="btn btn-ghost h-8 px-2.5 text-xs" onClick={() => void reveal(source())}>
                <Icon name="folder" size={11} />
                Voir le rapport
              </button>
            )}
          </Show>
        </div>
      </div>

      <Show when={analysis()?.exception}>
        {(exception) => <code class="block truncate bg-slate-850 px-2.5 py-1.5 font-mono text-xs text-muted" title={exception()}>{exception()}</code>}
      </Show>

      <Show when={(analysis()?.suspects.length ?? 0) > 0}>
        <div class="flex flex-col gap-1.5">
          <span class="text-xs text-muted">
            {analysis()!.suspects.length === 1 ? "Mod probablement en cause" : "Mods probablement en cause"}
          </span>
          <ul class="flex flex-col divide-y divide-line">
            <For each={analysis()!.suspects}>
              {(suspect) => {
                const content = () => contentFor(suspect.fileName);
                return (
                  <li class="flex items-center gap-3 py-1.5">
                    <div class="flex min-w-0 flex-1 flex-col">
                      <span class="truncate text-sm font-medium">{suspect.name}</span>
                      <span class="truncate text-xs text-muted">
                        {REASONS[suspect.reason]}
                        <Show when={suspect.fileName}> · {suspect.fileName}</Show>
                      </span>
                    </div>
                    <Show when={content()}>
                      {(c) => (
                        <button
                          class="btn px-corners h-8 shrink-0 px-2.5 text-xs"
                          disabled={props.locked || isBusy(props.instanceId, c().projectId)}
                          onClick={async () => props.onError(await setContentEnabled(props.instanceId, c().projectId, false))}
                        >
                          Désactiver
                        </button>
                      )}
                    </Show>
                  </li>
                );
              }}
            </For>
          </ul>
        </div>
      </Show>
    </div>
  );
}
