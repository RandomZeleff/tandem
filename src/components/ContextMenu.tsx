import { createEffect, createSignal, For, type JSX, Match, onCleanup, Show, Switch } from "solid-js";
import { Portal } from "solid-js/web";
import { Icon, type IconName } from "./pixel";

export type MenuItem =
  | { kind?: "action"; label: string; icon?: IconName; danger?: boolean; disabled?: boolean; hint?: string; run: () => void }
  | { kind: "header"; label: JSX.Element }
  | { kind: "separator" };

const [menu, setMenu] = createSignal<{ x: number; y: number; items: MenuItem[] } | null>(null);
const MENU_WIDTH = 232;

export const closeContextMenu = () => setMenu(null);

/**
 * Opens the menu where the player right-clicked (or under the element, from the keyboard:
 * the Menu key or Shift+F10).
 */
export function openContextMenu(e: MouseEvent | KeyboardEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  if (e instanceof MouseEvent && e.type === "contextmenu" && e.clientX + e.clientY > 0) {
    setMenu({ x: e.clientX, y: e.clientY, items });
  } else {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setMenu({ x: r.left + 8, y: r.bottom, items });
  }
}

/** Opens the menu from the keyboard keys that mean "context menu". */
export function isMenuKey(e: KeyboardEvent) {
  return e.key === "ContextMenu" || (e.shiftKey && e.key === "F10");
}

/** Mounted once in App: the open menu, in a portal above everything. */
export function ContextMenuHost() {
  let panel: HTMLDivElement | undefined;
  const [place, setPlace] = createSignal({ left: 0, top: 0 });

  const actions = () =>
    Array.from(panel?.querySelectorAll<HTMLButtonElement>("[role=menuitem]:not(:disabled)") ?? []);

  createEffect(() => {
    const open = menu();
    if (!open) return;
    // Kept inside the window, flipped up or left near the edges.
    queueMicrotask(() => {
      const h = panel?.offsetHeight ?? 0;
      const left = Math.min(open.x, window.innerWidth - MENU_WIDTH - 8);
      const top = open.y + h > window.innerHeight - 8 ? Math.max(8, open.y - h) : open.y;
      setPlace({ left: Math.max(8, left), top });
      actions()[0]?.focus();
    });
    const close = () => setMenu(null);
    const onDown = (e: PointerEvent) => {
      if (panel && !panel.contains(e.target as Node)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      const list = actions();
      const index = list.indexOf(document.activeElement as HTMLButtonElement);
      if (e.key === "Escape") {
        e.preventDefault();
        close();
      } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const step = e.key === "ArrowDown" ? 1 : -1;
        list[(index + step + list.length) % list.length]?.focus();
      } else if (e.key === "Home" || e.key === "End") {
        e.preventDefault();
        list[e.key === "Home" ? 0 : list.length - 1]?.focus();
      } else if (e.key === "Tab") {
        close();
      }
    };
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", close);
    window.addEventListener("resize", close);
    document.addEventListener("scroll", close, true);
    onCleanup(() => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", close);
      window.removeEventListener("resize", close);
      document.removeEventListener("scroll", close, true);
    });
  });

  return (
    <Show when={menu()}>
      {(open) => (
        <Portal>
          <div
            ref={panel}
            role="menu"
            class="panel px-corners-md pop-in fixed z-50 flex flex-col bg-slate-750 p-1.5 shadow-[0_12px_32px_rgb(0_0_0/0.55)]"
            style={{ left: `${place().left}px`, top: `${place().top}px`, width: `${MENU_WIDTH}px` }}
            onContextMenu={(e) => e.preventDefault()}
          >
            <For each={open().items}>
              {(item) => (
                <Switch>
                  <Match when={item.kind === "separator"}>
                    <div class="mx-1 my-1.5 h-px bg-line" role="separator" />
                  </Match>
                  <Match when={item.kind === "header" && item}>
                    {(header) => <div class="px-2 pt-1 pb-1.5 text-xs text-faint">{(header() as { label: JSX.Element }).label}</div>}
                  </Match>
                  <Match when={(item.kind ?? "action") === "action" && (item as Extract<MenuItem, { run: () => void }>)}>
                    {(action) => (
                      <button
                        role="menuitem"
                        class="row flex h-8 w-full items-center gap-2.5 px-2 text-left text-sm disabled:pointer-events-none disabled:opacity-40"
                        classList={{ "text-redstone-text": action().danger }}
                        disabled={action().disabled}
                        title={action().hint}
                        onClick={() => {
                          setMenu(null);
                          action().run();
                        }}
                      >
                        <span class="flex w-4 justify-center">
                          <Show when={action().icon}>{(icon) => <Icon name={icon()} size={12} />}</Show>
                        </span>
                        <span class="flex-1 truncate">{action().label}</span>
                      </button>
                    )}
                  </Match>
                </Switch>
              )}
            </For>
          </div>
        </Portal>
      )}
    </Show>
  );
}
