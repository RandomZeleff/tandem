import { listen } from "@tauri-apps/api/event";
import { createStore, produce } from "solid-js/store";
import {
  api,
  type DuoGuest,
  type DuoGuestEvent,
  type DuoHostEvent,
  type DuoHostView,
  type DuoJoinView,
  type LinkStatus,
} from "./api";
import { launch } from "./games";
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
  const current = await api.duoState().catch(() => null);
  if (current) setDuo({ host: current.host ? { ...current.host, guests: [] } : null, guest: current.guest });
}

export async function startHosting(instanceId: string | null) {
  const view = await api.duoHost(instanceId);
  setDuo("host", { ...view, guests: [] });
}

export async function stopHosting() {
  await api.duoStopHost();
  setDuo("host", null);
}

export const kickGuest = (id: string) => api.duoKick(id);

export async function joinFriend(code: string, instanceId: string) {
  setDuo("guest", await api.duoJoin(code, instanceId));
}

export async function leaveFriend() {
  await api.duoLeave();
  setDuo("guest", null);
}

/** Starts the joined instance straight into the host's world. */
export function playWithFriend() {
  const guest = duo.guest;
  if (guest) void launch(guest.instanceId, api.duoPlay);
}

/** `XXXX-XXXX` as the player types: uppercase, dash after 4 characters. */
export function formatCode(input: string): string {
  const raw = input
    .toUpperCase()
    .replace(/[^0-9A-Z]/g, "")
    .slice(0, 8);
  return raw.length > 4 ? `${raw.slice(0, 4)}-${raw.slice(4)}` : raw;
}
