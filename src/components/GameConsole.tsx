import { createEffect, For, on, Show } from "solid-js";
import { consoleInstance, output } from "../lib/games";

export default function GameConsole() {
  const lines = () => {
    const id = consoleInstance();
    return id ? (output[id] ?? []) : [];
  };

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
      class="h-full overflow-y-auto px-3 pb-2 font-mono text-xs leading-5 select-text"
      onScroll={(e) => {
        const el = e.currentTarget;
        stickToBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
      }}
    >
      <Show
        when={lines().length > 0}
        fallback={<p class="pt-2 text-neutral-500">Lance une instance pour voir la sortie du jeu.</p>}
      >
        <For each={lines()}>
          {(l) => (
            <div
              class="whitespace-pre-wrap"
              classList={{
                "text-neutral-300": l.stream === "stdout",
                "text-red-300": l.stream === "stderr",
              }}
            >
              {l.line}
            </div>
          )}
        </For>
      </Show>
    </div>
  );
}
