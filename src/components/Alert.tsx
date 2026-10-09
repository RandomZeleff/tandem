import { type JSX, Show } from "solid-js";
import { Icon } from "./pixel";

export type AlertTone = "error" | "warning" | "success";

const TONES: Record<AlertTone, string> = {
  error: "bg-danger text-redstone-text shadow-[inset_0_0_0_1px_var(--color-danger-line)]",
  warning: "bg-warning text-gold shadow-[inset_0_0_0_1px_var(--color-gold-deep)]",
  success: "bg-success text-xp-text shadow-[inset_0_0_0_1px_var(--color-success-line)]",
};

/** Inline message under a toolbar or a form; closable when `onClose` is given. */
export default function Alert(props: { tone?: AlertTone; children: JSX.Element; onClose?: () => void; class?: string }) {
  const tone = () => props.tone ?? "error";
  return (
    <div
      role={tone() === "error" ? "alert" : "status"}
      class={`flex items-start gap-3 px-3 py-2 text-sm ${TONES[tone()]} ${props.class ?? ""}`}
    >
      <p class="min-w-0 flex-1 break-words select-text">{props.children}</p>
      <Show when={props.onClose}>
        <button
          type="button"
          class="-mr-1 flex size-5 shrink-0 items-center justify-center opacity-70 hover:opacity-100 focus-visible:opacity-100"
          aria-label="Fermer le message"
          onClick={() => props.onClose?.()}
        >
          <Icon name="close" size={9} />
        </button>
      </Show>
    </div>
  );
}
