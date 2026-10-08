import { Match, Switch } from "solid-js";
import { gameState, launch, stop } from "../lib/games";
import { Icon } from "./pixel";

/** Play / installing / stop button bound to an instance's game state. */
export default function PlayButton(props: { id: string; size?: "lg" | "md" | "icon"; name?: string }) {
  const state = () => gameState(props.id);
  const size = () => props.size ?? "md";
  const ratio = () => {
    const p = state().progress;
    return p && p.stage === "downloading" && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : 0;
  };

  const sizeClass = () =>
    ({
      lg: "px-corners-md h-[54px] px-7 font-pixel text-[22px] font-bold tracking-[1px]",
      md: "px-corners h-[38px] px-4 font-pixel text-base font-semibold tracking-[0.5px]",
      icon: "px-corners h-8 w-9 px-0",
    })[size()];

  const label = (text: string) => (size() === "icon" ? null : text);

  return (
    <Switch>
      <Match when={state().status === "running"}>
        <button
          class={`btn btn-danger ${sizeClass()}`}
          aria-label={props.name ? `Arrêter ${props.name}` : "Arrêter"}
          onClick={(e) => {
            e.stopPropagation();
            void stop(props.id);
          }}
        >
          <Icon name="stop" size={size() === "lg" ? 16 : 12} />
          {label("ARRÊTER")}
        </button>
      </Match>
      <Match when={state().status === "preparing"}>
        <button class={`btn ${sizeClass()}`} disabled aria-label="Installation en cours">
          <Icon name="download" size={size() === "lg" ? 16 : 12} color="#8BE04E" />
          {size() === "icon" ? null : <span class="font-mono text-[0.75em]">{Math.round(ratio() * 100)} %</span>}
        </button>
      </Match>
      <Match when={state().status === "idle"}>
        <button
          class={`btn btn-primary ${sizeClass()}`}
          aria-label={props.name ? `Jouer à ${props.name}` : "Jouer"}
          onClick={(e) => {
            e.stopPropagation();
            void launch(props.id);
          }}
        >
          <Icon name="play" size={size() === "lg" ? 18 : 12} />
          {label("JOUER")}
        </button>
      </Match>
    </Switch>
  );
}
