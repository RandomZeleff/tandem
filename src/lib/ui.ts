import { onCleanup, onMount } from "solid-js";

const FOCUSABLE =
  'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.offsetParent !== null || el === document.activeElement);
}

/** Open modals, innermost last: only the top one handles Tab. */
const traps: HTMLElement[] = [];

/**
 * Keeps keyboard focus inside a modal while it is open: focuses its `autofocus` element
 * (else its first control), loops Tab within it, and gives focus back on close.
 */
export function trapFocus(root: () => HTMLElement | undefined) {
  const previous = document.activeElement as HTMLElement | null;
  onMount(() => {
    const el = root();
    if (!el) return;
    const initial = el.querySelector<HTMLElement>("[autofocus]") ?? focusables(el)[0] ?? el;
    initial.focus();
    traps.push(el);
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Tab" || traps[traps.length - 1] !== el) return;
      const list = focusables(el);
      if (list.length === 0) {
        e.preventDefault();
        return;
      }
      const first = list[0];
      const last = list[list.length - 1];
      const inside = el.contains(document.activeElement);
      if (e.shiftKey && (document.activeElement === first || !inside)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (document.activeElement === last || !inside)) {
        e.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKey);
    onCleanup(() => {
      document.removeEventListener("keydown", onKey);
      traps.splice(traps.indexOf(el), 1);
    });
  });
  onCleanup(() => {
    if (previous?.isConnected) previous.focus();
  });
}

/** Calls `onDismiss` on Escape or on a pointer press outside every given element. */
export function onDismiss(elements: () => (HTMLElement | undefined)[], onDismiss: () => void) {
  onMount(() => {
    const onPointer = (e: PointerEvent) => {
      const target = e.target as Node;
      if (!elements().some((el) => el?.contains(target))) onDismiss();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onDismiss();
      }
    };
    document.addEventListener("pointerdown", onPointer, true);
    // Capture, so an open menu closes before a dialog behind it sees the Escape.
    window.addEventListener("keydown", onKey, true);
    onCleanup(() => {
      document.removeEventListener("pointerdown", onPointer, true);
      window.removeEventListener("keydown", onKey, true);
    });
  });
}
