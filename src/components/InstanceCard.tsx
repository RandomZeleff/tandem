import { Show } from "solid-js";
import type { Instance } from "../lib/api";
import { formatRelative } from "../lib/format";
import { gameState } from "../lib/games";
import { blockLook } from "../lib/look";
import { navigate } from "../lib/store";
import InstanceSlot from "./InstanceSlot";
import { LoaderTag, XpBar } from "./pixel";
import PlayButton from "./PlayButton";

export default function InstanceCard(props: { instance: Instance }) {
  const look = () => blockLook(props.instance.id);
  const state = () => gameState(props.instance.id);
  const ratio = () => {
    const p = state().progress;
    return p && p.stage === "downloading" && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : 0;
  };

  return (
    <article
      class="panel px-corners-md group flex cursor-pointer flex-col transition-[filter] hover:brightness-110"
      onClick={() => navigate({ page: "instance", id: props.instance.id })}
    >
      <div class="relative h-[70px]">
        <svg
          class="block h-full w-full"
          viewBox="0 0 60 14"
          preserveAspectRatio="none"
          shape-rendering="crispEdges"
          aria-hidden="true"
        >
          <rect width="60" height="14" fill={look().sky[0]} />
          <rect y="5" width="60" height="4" fill={look().sky[1]} />
          <path d="M0 10h8v-2h10v1h9v-3h11v2h9v-1h13V14H0z" fill={look().hill} />
        </svg>
        <InstanceSlot
          instance={props.instance}
          size={52}
          style={{
            position: "absolute",
            left: "14px",
            bottom: "-22px",
            "box-shadow": "inset 2px 2px 0 #373737, inset -2px -2px 0 #fff, 0 0 0 3px var(--color-slate-700)",
          }}
        />
      </div>
      <div class="flex flex-col gap-3 px-3.5 pt-[30px] pb-3.5">
        <div class="flex min-w-0 flex-col gap-0.5">
          <h3 class="truncate text-[15px] font-semibold">{props.instance.name}</h3>
          <span class="text-xs text-muted">
            {props.instance.gameVersion} · <LoaderTag loader={props.instance.loader} />
          </span>
        </div>
        <Show
          when={state().status === "preparing"}
          fallback={
            <div class="flex items-center justify-between gap-2">
              <span class="text-xs" classList={{ "text-xp-text": state().status === "running", "text-muted": state().status !== "running" }}>
                {state().status === "running" ? "En cours" : formatRelative(props.instance.lastPlayedAt)}
              </span>
              <PlayButton id={props.instance.id} size="icon" name={props.instance.name} />
            </div>
          }
        >
          <div class="flex flex-col gap-1.5">
            <XpBar value={ratio()} label={`Installation de ${props.instance.name}`} />
            <div class="flex justify-between font-mono text-xs">
              <span class="text-xp-text">Installation</span>
              <span class="text-muted">{Math.round(ratio() * 100)} %</span>
            </div>
          </div>
        </Show>
      </div>
    </article>
  );
}
