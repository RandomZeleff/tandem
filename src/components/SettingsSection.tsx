import { type JSX, Show } from "solid-js";

/** One setting: what it does on the left, its control on the right. */
export function Row(props: { title: string; description?: string; control: JSX.Element; for?: string }) {
  return (
    <div class="flex items-center justify-between gap-6 py-3.5">
      <div class="flex min-w-0 flex-col gap-0.5">
        <label class="text-sm font-medium" for={props.for}>
          {props.title}
        </label>
        <Show when={props.description}>
          <p class="text-meta text-muted">{props.description}</p>
        </Show>
      </div>
      <div class="shrink-0">{props.control}</div>
    </div>
  );
}

/** A panel of settings rows under a title. */
export function Section(props: { title: string; children: JSX.Element; class?: string }) {
  return (
    <section class={`panel px-corners-md flex flex-col px-5 pt-4 pb-1.5 ${props.class ?? ""}`}>
      <h2 class="panel-title">{props.title}</h2>
      <div class="flex flex-col divide-y divide-line">{props.children}</div>
    </section>
  );
}
