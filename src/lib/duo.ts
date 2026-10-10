import { listen } from "@tauri-apps/api/event";
import { createSignal } from "solid-js";
import { createStore, produce } from "solid-js/store";
import {
  api,
  type DuoGuest,
  type DuoGuestEvent,
  type DuoHostEvent,
  type DuoHostView,
  type DuoJoinView,
  type InstanceDiff,
  type LinkStatus,
} from "./api";
import { gameState, launch } from "./games";
import { toast } from "./toast";

export interface HostingState extends DuoHostView {
  guests: (DuoGuest & { link?: LinkStatus })[];
}

interface DuoStore {
  host: HostingState | null;
  guest: DuoJoinView | null;
}

const [duo, setDuo] = createStore<DuoStore>({ host: null, guest: null });
export { duo };

/** A session is running (for the sidebar's live dot). */
export const duoActive = () => !!duo.host || !!duo.guest;

export const diffMatches = (d: InstanceDiff) => d.sameGame && d.sameLoader && d.missing.length === 0 && d.extra.length === 0;

/** Setting: the guest's game starts by itself once the host's world is open. */
const AUTO_LAUNCH_SETTING = "duo_auto_launch";
const [autoLaunch, setAutoLaunchSignal] = createSignal(true);
export { autoLaunch };
export function setAutoLaunch(on: boolean) {
  setAutoLaunchSignal(on);
  void api.setSetting(AUTO_LAUNCH_SETTING, on).catch(() => {});
  if (on) maybeAutoLaunch();
}

/** The game was started for this joined session already: never twice on its own. */
let autoLaunched = false;

/** Starts the guest's game once everything is ready: world open, matching instance. */
function maybeAutoLaunch() {
  const guest = duo.guest;
  if (!guest || autoLaunched || !autoLaunch() || !guest.world || !guest.instanceId) return;
  if (guest.diff && !diffMatches(guest.diff)) return;
  if (gameState(guest.instanceId).status !== "idle") return;
  autoLaunched = true;
  playWithFriend();
}

let started = false;

/** Listens to session events and loads the session already running, once. */
export async function startDuoEvents() {
  if (started) return;
  started = true;
  await listen<DuoHostEvent>("duo://host", ({ payload }) => {
    if (!duo.host) return;
    switch (payload.type) {
      case "world":
        setDuo("host", "world", payload.world);
        break;
      case "guestJoined":
        setDuo("host", "guests", (list) => [...list.filter((g) => g.id !== payload.guest.id), payload.guest]);
        toast(`${payload.guest.player} a rejoint ta partie`);
        break;
      case "guestInstance":
        setDuo(
          "host",
          "guests",
          produce((list) => {
            const guest = list.find((g) => g.id === payload.id);
            if (guest) guest.diff = payload.diff;
          }),
        );
        break;
      case "guestLeft": {
        const left = duo.host.guests.find((g) => g.id === payload.id);
        setDuo("host", "guests", (list) => list.filter((g) => g.id !== payload.id));
        if (left) toast(`${left.player} est parti`, { tone: "info" });
        break;
      }
      case "link":
        setDuo(
          "host",
          "guests",
          produce((list) => {
            const guest = list.find((g) => g.id === payload.id);
            if (guest) guest.link = payload.link;
          }),
        );
        break;
    }
  });
  await listen<DuoGuestEvent>("duo://guest", ({ payload }) => {
    if (!duo.guest) return;
    switch (payload.type) {
      case "world":
        setDuo("guest", "world", payload.world);
        if (payload.world) toast(`${duo.guest.hostPlayer} a ouvert son monde`);
        maybeAutoLaunch();
        break;
      case "link":
        setDuo("guest", "link", payload.link);
        break;
      case "closed":
        toast(payload.reason, { tone: "info", durationMs: 8000 });
        setDuo("guest", null);
        break;
    }
  });
  setAutoLaunchSignal((await api.getSetting<boolean>(AUTO_LAUNCH_SETTING).catch(() => null)) ?? true);
  const current = await api.duoState().catch(() => null);
  if (current) setDuo({ host: current.host ? { ...current.host, guests: [] } : null, guest: current.guest });
}

/** Creates the invitation, then starts the game (straight into `world` on 1.20+). */
export async function startHosting(instanceId: string | null, world?: string) {
  const view = await api.duoHost(instanceId);
  setDuo("host", { ...view, guests: [] });
  if (instanceId && gameState(instanceId).status === "idle") {
    void launch(instanceId, () => api.launchInstance(instanceId, world));
  }
}

export async function stopHosting() {
  await api.duoStopHost();
  setDuo("host", null);
}

export const kickGuest = (id: string) => api.duoKick(id);

/** Joins with the instance that best matches the host's; plays at once when possible. */
export async function joinFriend(code: string) {
  autoLaunched = false;
  setDuo("guest", await api.duoJoin(code));
  maybeAutoLaunch();
}

/** Plays with another instance than the one picked. */
export async function changeInstance(instanceId: string) {
  setDuo("guest", await api.duoSetInstance(instanceId));
  maybeAutoLaunch();
}

export async function leaveFriend() {
  await api.duoLeave();
  setDuo("guest", null);
}

/** Starts the joined instance straight into the host's world. */
export function playWithFriend() {
  const id = duo.guest?.instanceId;
  if (id) void launch(id, api.duoPlay);
}

/** `XXXX-XXXX` as the player types: uppercase, dash after 4 characters. */
export function formatCode(input: string): string {
  const raw = input
    .toUpperCase()
    .replace(/[^0-9A-Z]/g, "")
    .slice(0, 8);
  return raw.length > 4 ? `${raw.slice(0, 4)}-${raw.slice(4)}` : raw;
}
