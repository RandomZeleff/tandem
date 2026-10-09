import { createResource, createRoot, createSignal, type Signal } from "solid-js";
import { api } from "./api";

export type Route =
  | { page: "home" }
  | { page: "instances" }
  | { page: "instance"; id: string }
  | { page: "discover"; instanceId?: string; query?: string }
  /** Modrinth project page (id or slug); `instanceId` is where content would go. */
  | { page: "project"; id: string; instanceId?: string }
  | { page: "multi" }
  | { page: "settings" };

interface Entry {
  route: Route;
  /** Page state to restore when coming back (tab, search, filter): see `remembered`. */
  memo: Record<string, unknown>;
  scrollTop: number;
}

const MAX_HISTORY = 50;

const [history, setHistory] = createSignal<{ entries: Entry[]; index: number }>({
  entries: [{ route: { page: "home" }, memo: {}, scrollTop: 0 }],
  index: 0,
});

const current = () => history().entries[history().index];
/** Changes on every navigation, even between two pages of the same kind. */
export const currentEntry = current;
export const route = () => current().route;

/** The scrolling page area, whose position each history entry keeps. */
let scroller: HTMLElement | undefined;
export function setPageScroller(el: HTMLElement) {
  scroller = el;
}

function restoreScroll(top: number) {
  // Solid has already rendered the new page; once more next frame, for lists that grow.
  if (scroller) scroller.scrollTop = top;
  requestAnimationFrame(() => scroller && (scroller.scrollTop = top));
}

function sameRoute(a: Route, b: Route): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** Opens a page, dropping any "forward" history. Opening the current page does nothing. */
export function navigate(to: Route) {
  if (sameRoute(to, route())) return;
  current().scrollTop = scroller?.scrollTop ?? 0;
  setHistory(({ entries, index }) => {
    const kept = [...entries.slice(0, index + 1), { route: to, memo: {}, scrollTop: 0 }].slice(-MAX_HISTORY);
    return { entries: kept, index: kept.length - 1 };
  });
  restoreScroll(0);
}

/** An instance page whose instance was deleted since is skipped. */
function reachable(entry: Entry): boolean {
  return entry.route.page !== "instance" || instances().some((i) => i.id === (entry.route as { id: string }).id);
}

function step(direction: -1 | 1) {
  const { entries, index } = history();
  let target = index + direction;
  while (target >= 0 && target < entries.length && !reachable(entries[target])) target += direction;
  if (target < 0 || target >= entries.length) return;
  current().scrollTop = scroller?.scrollTop ?? 0;
  setHistory({ entries, index: target });
  restoreScroll(entries[target].scrollTop);
}

export const goBack = () => step(-1);
export const goForward = () => step(1);
export const canGoBack = () => history().entries.slice(0, history().index).some(reachable);
export const canGoForward = () => history().entries.slice(history().index + 1).some(reachable);

/**
 * A signal kept in the current history entry: a page created again by Back or Forward
 * starts from the value it had, a fresh visit from `initial`.
 */
export function remembered<T>(key: string, initial: T): Signal<T> {
  const memo = current().memo;
  const [value, setValue] = createSignal<T>(key in memo ? (memo[key] as T) : initial);
  return [
    value,
    ((next: T | ((prev: T) => T)) =>
      setValue((prev) => {
        const v = typeof next === "function" ? (next as (prev: T) => T)(prev) : next;
        memo[key] = v;
        return v;
      })) as Signal<T>[1],
  ];
}

/** App-wide data shared by the sidebar and the pages. */
export const { instances, refetchInstances, accounts, refetchAccounts } = createRoot(() => {
  const [instances, { refetch: refetchInstances }] = createResource(api.listInstances, {
    initialValue: [],
  });
  const [accounts, { refetch: refetchAccounts }] = createResource(api.listAccounts, {
    initialValue: [],
  });
  return { instances, refetchInstances, accounts, refetchAccounts };
});

export const activeAccount = () => accounts().find((a) => a.isActive);

/** Open state of the "new instance" dialog, with an optional preselected version. */
export const [newInstanceDialog, setNewInstanceDialog] = createSignal<{ version?: string } | null>(null);
