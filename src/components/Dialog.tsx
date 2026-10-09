import { onCleanup, onMount, type JSX } from "solid-js";
import { trapFocus } from "../lib/ui";

/** Modal frame: backdrop click and Escape close it. */
export default function Dialog(props: { title: string; onClose: () => void; children: JSX.Element; width?: number }) {
  let panel: HTMLDivElement | undefined;
  trapFocus(() => panel);
  onMount(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && props.onClose();
    window.addEventListener("keydown", onKey);
    onCleanup(() => window.removeEventListener("keydown", onKey));
  });

  return (
    <div
      class="fixed inset-0 z-30 flex items-center justify-center bg-black/65 p-4"
      onClick={(e) => e.target === e.currentTarget && props.onClose()}
    >
      <div
        ref={panel}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-label={props.title}
        class="panel px-corners-lg flex w-full flex-col gap-5 bg-slate-750 p-6 shadow-2xl"
        style={{ "max-width": `${props.width ?? 460}px` }}
      >
        <h2 class="pixel-shadow font-pixel text-2xl font-bold">{props.title}</h2>
        {props.children}
      </div>
    </div>
  );
}
