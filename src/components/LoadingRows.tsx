import { For } from "solid-js";

/** Placeholder rows shown while a list loads, so "empty" is never claimed too early. */
export default function LoadingRows(props: { count?: number; height?: number; label?: string }) {
  return (
    <div role="status" aria-label={props.label ?? "Chargement…"} class="flex flex-col gap-2">
      <For each={Array.from({ length: props.count ?? 3 })}>
        {(_, i) => <div class="skeleton px-corners-md" style={{ height: `${props.height ?? 56}px`, "animation-delay": `${i() * 120}ms` }} />}
      </For>
    </div>
  );
}
