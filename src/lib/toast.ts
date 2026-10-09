import { createSignal } from "solid-js";

export type ToastTone = "success" | "error" | "info";

export interface Toast {
  id: number;
  tone: ToastTone;
  message: string;
  action?: { label: string; run: () => void };
}

const DURATION_MS = 4000;
const MAX_TOASTS = 4;

const [toasts, setToasts] = createSignal<Toast[]>([]);
export { toasts };

let nextId = 1;
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export function dismissToast(id: number) {
  clearTimeout(timers.get(id));
  timers.delete(id);
  setToasts((list) => list.filter((t) => t.id !== id));
}

/** Short message in the corner, gone after a few seconds (longer when it offers an action). */
export function toast(message: string, options: { tone?: ToastTone; action?: Toast["action"]; durationMs?: number } = {}) {
  const id = nextId++;
  setToasts((list) => [...list, { id, tone: options.tone ?? "success", message, action: options.action }].slice(-MAX_TOASTS));
  timers.set(
    id,
    setTimeout(() => dismissToast(id), options.durationMs ?? (options.action ? DURATION_MS * 1.5 : DURATION_MS)),
  );
  return id;
}

/**
 * Hides something at once and really removes it a few seconds later, unless the player
 * clicks "Annuler" in the toast. `hide`/`show` toggle it in the UI, `commit` does the work.
 */
export function removeWithUndo(options: {
  message: string;
  hide: () => void;
  show: () => void;
  commit: () => Promise<string | null | void>;
}) {
  options.hide();
  let undone = false;
  const durationMs = DURATION_MS * 1.5;
  const id = toast(options.message, {
    tone: "info",
    durationMs,
    action: {
      label: "Annuler",
      run: () => {
        undone = true;
        options.show();
        dismissToast(id);
      },
    },
  });
  setTimeout(async () => {
    if (undone) return;
    const error = await options.commit().catch((err: unknown) => String(err));
    if (error) {
      options.show();
      toast(error, { tone: "error" });
    }
  }, durationMs);
}
