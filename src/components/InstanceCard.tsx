import { Show } from "solid-js";
import { isTransferStage, type Instance } from "../lib/api";
import { formatRelative } from "../lib/format";
import { gameState } from "../lib/games";
import { navigate } from "../lib/store";
import InstanceSlot from "./InstanceSlot";
import InstanceVisual from "./InstanceVisual";
import { LoaderTag, XpBar } from "./pixel";
import PlayButton from "./PlayButton";

export default function InstanceCard(props: { instance: Instance }) {
  const state = () => gameState(props.instance.id);
  const ratio = () => {
    const p = state().progress;
    return p && isTransferStage(p.stage) && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : 0;
  };
  const task = () => (state().progress?.stage === "copying" ? "Copie" : "Installation");

  return (
    <article
      tabIndex={0}
      aria-label={`Ouvrir ${props.instance.name}`}
      class="panel group flex cursor-pointer flex-col overflow-hidden transition-colors duration-100 hover:bg-slate-650 focus-visible:bg-slate-650"
      onClick={() => navigate({ page: "instance", id: props.instance.id })}
      onKeyDown={(e) => {
        if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          navigate({ page: "instance", id: props.instance.id });
        }
      }}
    >
      <div class="relative aspect-[16/8] overflow-hidden bg-slate-900">
        <InstanceVisual instance={props.instance} small />
        <div class="absolute inset-0 bg-[linear-gradient(0deg,rgb(16_17_19/0.55),transparent_45%)]" />
        <span class="absolute bottom-2.5 left-2.5 rounded-sm shadow-[0_0_0_2px_rgb(16_17_19/0.6)]">
          <InstanceSlot instance={props.instance} size={40} />
        </span>
      </div>
      <div class="flex flex-col gap-3 px-3.5 pt-3 pb-3.5">
        <div class="flex min-w-0 flex-col gap-0.5">
          <h3 class="truncate text-[15px] font-semibold">{props.instance.name}</h3>
          <span class="text-[13px] text-muted">
            {props.instance.gameVersion} · <LoaderTag loader={props.instance.loader} />
          </span>
        </div>
        <Show
          when={state().status === "preparing"}
          fallback={
            <div class="flex items-center justify-between gap-2">
              <span
                class="text-[13px]"
                classList={{ "font-medium text-xp-text": state().status === "running", "text-faint": state().status !== "running" }}
              >
                {state().status === "running" ? "En cours" : formatRelative(props.instance.lastPlayedAt)}
              </span>
              <PlayButton id={props.instance.id} size="icon" name={props.instance.name} />
            </div>
          }
        >
          <div class="flex flex-col gap-1.5">
            <XpBar value={ratio()} label={`${task()} de ${props.instance.name}`} />
            <div class="flex justify-between text-xs">
              <span class="font-medium text-xp-text">{task()}</span>
              <span class="font-mono text-muted">{Math.round(ratio() * 100)} %</span>
            </div>
          </div>
        </Show>
      </div>
    </article>
  );
}
