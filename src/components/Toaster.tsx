import { For, Show } from "solid-js";
import { dismissToast, type ToastTone, toasts } from "../lib/toast";
import { Icon, type IconName } from "./pixel";

const TONES: Record<ToastTone, { icon: IconName; color: string; frame: string }> = {
  success: { icon: "check", color: "var(--color-xp)", frame: "shadow-[inset_0_0_0_1px_var(--color-success-line),0_10px_28px_rgb(0_0_0/0.5)]" },
  error: { icon: "close", color: "var(--color-redstone-text)", frame: "shadow-[inset_0_0_0_1px_var(--color-danger-line),0_10px_28px_rgb(0_0_0/0.5)]" },
  info: { icon: "dots", color: "var(--color-muted)", frame: "shadow-[inset_0_0_0_1px_var(--color-line-strong),0_10px_28px_rgb(0_0_0/0.5)]" },
};

/** Stack of toasts in the bottom-right corner, newest at the bottom. */
export default function Toaster() {
  return (
    <div aria-live="polite" class="pointer-events-none fixed right-5 bottom-5 z-40 flex w-[360px] flex-col gap-2">
      <For each={toasts()}>
        {(t) => (
          <div
            role={t.tone === "error" ? "alert" : "status"}
            class={`toast-in px-corners-md pointer-events-auto flex items-center gap-3 bg-slate-750 py-2.5 pr-2.5 pl-3.5 text-sm ${TONES[t.tone].frame}`}
          >
            <Icon name={TONES[t.tone].icon} size={12} color={TONES[t.tone].color} class="shrink-0" />
            <p class="min-w-0 flex-1 break-words text-chalk-2">{t.message}</p>
            <Show when={t.action}>
              {(action) => (
                <button class="btn btn-ghost h-7 shrink-0 px-2 text-xs text-xp-text" onClick={() => action().run()}>
                  {action().label}
                </button>
              )}
            </Show>
            <button
              class="flex size-6 shrink-0 items-center justify-center text-faint hover:text-chalk focus-visible:text-chalk"
              aria-label="Fermer la notification"
              onClick={() => dismissToast(t.id)}
            >
              <Icon name="close" size={9} />
            </button>
          </div>
        )}
      </For>
    </div>
  );
}
