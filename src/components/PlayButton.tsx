import { api, type InstallProgress, isTransferStage } from "../lib/api";
import { Match, Show, Switch } from "solid-js";
import { gameState, launch, stop } from "../lib/games";
import { Icon } from "./pixel";

/** What the game is doing while the Play button waits, in a few words. */
function stepLabel(p: InstallProgress | undefined): string {
  switch (p?.stage) {
    case "downloading":
      return "Téléchargement";
    case "copying":
      return "Copie";
    case "metadata":
      return "Vérification…";
    case "processing":
      return "Préparation du loader…";
    case "finalizing":
      return "Finalisation…";
    default:
      return "Lancement…";
  }
}

/**
 * Play button bound to an instance's game state. Pressed, it becomes a track that fills
 * while the game installs, sweeps while it starts, then shows "in game" next to a stop
 * button, so stopping is never one stray click on the main button.
 */
export default function PlayButton(props: {
  id: string;
  size?: "lg" | "md" | "icon";
  name?: string;
  /** World folder to open at once (1.20+): the button then reads "Continuer". */
  world?: string | null;
  /** Shown name of that world, for screen readers. */
  worldName?: string;
}) {
  const state = () => gameState(props.id);
  const size = () => props.size ?? "md";
  const transfer = () => {
    const p = state().progress;
    return p && isTransferStage(p.stage) && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : null;
  };

  const box = () =>
    ({
      lg: "px-corners-md h-[54px] min-w-[236px] px-6",
      md: "px-corners h-[38px] min-w-[156px] px-4",
      icon: "px-corners h-8 w-9 px-0",
    })[size()];
  const stopBox = () =>
    ({
      lg: "px-corners-md size-[54px]",
      md: "px-corners size-[38px]",
      icon: "px-corners h-8 w-9",
    })[size()];
  /** Text size of the waiting states (the idle lg label uses the display face). */
  const text = () => ({ lg: "text-base", md: "text-sm", icon: "" })[size()];
  const label = (text: string) => (size() === "icon" ? null : <span>{text}</span>);
  const name = () => props.name ?? "le jeu";

  return (
    <Switch>
      <Match when={state().status === "idle"}>
        <button
          class={`btn btn-primary ${box()}`}
          classList={{ "title-display text-[28px] uppercase [text-shadow:0_2px_0_rgb(0_0_0/0.3)]": size() === "lg", "text-sm": size() === "md" }}
          aria-label={
            props.world
              ? `Continuer « ${props.worldName ?? props.world} »`
              : props.name
                ? `Jouer à ${props.name}`
                : "Jouer"
          }
          onClick={(e) => {
            e.stopPropagation();
            const world = props.world;
            void launch(props.id, world ? () => api.launchInstance(props.id, world) : undefined);
          }}
        >
          <Icon name="play" size={size() === "lg" ? 24 : 12} />
          {label(props.world ? "Continuer" : "Jouer")}
        </button>
      </Match>

      <Match when={state().status === "preparing"}>
        <div
          role="progressbar"
          aria-label={`Préparation de ${name()}`}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={transfer() === null ? undefined : Math.round(transfer()! * 100)}
          title={stepLabel(state().progress)}
          class={`launch ${box()} ${text()}`}
          onClick={(e) => e.stopPropagation()}
        >
          <Show when={transfer() !== null} fallback={<span class="launch-sweep" />}>
            <span class="launch-fill" style={{ transform: `scaleX(${transfer()!.toFixed(3)})` }} />
          </Show>
          <Show when={size() !== "icon"}>
            <span>{stepLabel(state().progress)}</span>
            <Show when={transfer() !== null}>
              <span class="font-mono text-[0.85em] text-chalk-2">{Math.round(transfer()! * 100)} %</span>
            </Show>
          </Show>
        </div>
      </Match>

      <Match when={state().status === "running"}>
        <div class="flex items-stretch gap-2" onClick={(e) => e.stopPropagation()}>
          <Show when={size() !== "icon"}>
            <div role="status" class={`launch ${box()} ${text()}`} classList={{ "is-playing": !!state().loaded }}>
              <Show when={state().loaded} fallback={<span class="launch-sweep" />}>
                <span class="launch-fill" />
              </Show>
              <Show
                when={state().loaded}
                fallback={label("Démarrage du jeu…")}
              >
                <span class="step-pulse size-2 rounded-[1px] bg-xp" aria-hidden="true" />
                {label("En jeu")}
              </Show>
            </div>
          </Show>
          <button
            class={`btn px-0 text-redstone-text ${stopBox()}`}
            aria-label={`Arrêter ${name()}`}
            title="Arrêter le jeu"
            onClick={() => void stop(props.id)}
          >
            <Icon name="stop" size={size() === "lg" ? 24 : 12} />
          </button>
        </div>
      </Match>
    </Switch>
  );
}
