import { createEffect, For, on, Show } from "solid-js";
import { output } from "../lib/games";

export default function GameConsole(props: { instanceId: string }) {
  const lines = () => output[props.instanceId] ?? [];

  let scroller: HTMLDivElement | undefined;
  let stickToBottom = true;
  createEffect(
    on(
      () => lines().length,
      () => {
        if (scroller && stickToBottom) scroller.scrollTop = scroller.scrollHeight;
      },
    ),
  );

  return (
    <div
      ref={scroller}
      class="h-full overflow-y-auto bg-slate-850 px-4 py-3 font-mono text-xs leading-5 select-text shadow-[inset_0_0_0_1px_#23272C,inset_0_3px_0_rgb(0_0_0/0.4)]"
      onScroll={(e) => {
        const el = e.currentTarget;
        stickToBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
      }}
    >
      <Show
        when={lines().length > 0}
        fallback={<p class="text-faint">La sortie du jeu s'affichera ici au prochain lancement.</p>}
      >
        <For each={lines()}>
          {(l) => (
            <div
              class="whitespace-pre-wrap"
              classList={{ "text-chalk-2": l.stream === "stdout", "text-redstone-text": l.stream === "stderr" }}
            >
              {l.line}
            </div>
          )}
        </For>
      </Show>
    </div>
  );
}
