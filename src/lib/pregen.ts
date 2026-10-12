import { listen } from "@tauri-apps/api/event";
import { createStore } from "solid-js/store";
import { api, EVENTS, type PregenView } from "./api";
import { toast } from "./toast";

/** Radii offered (same as `tandem_core::pregen::RADII`), with what to expect. */
export const PREGEN_SIZES = [
  // Time: 35 to 77 chunks/s measured (TEST-DUO); disk: about 11 KB per chunk.
  { radius: 500, label: "Petite zone", hint: "1 000 × 1 000 blocs", minutes: "1 à 2 min", disk: "50 Mo" },
  { radius: 1000, label: "Zone moyenne", hint: "2 000 × 2 000 blocs", minutes: "4 à 8 min", disk: "180 Mo" },
  { radius: 2000, label: "Grande zone", hint: "4 000 × 4 000 blocs", minutes: "15 à 30 min", disk: "700 Mo" },
] as const;

/** Preparation of each instance, as the backend last reported it. */
const [pregens, setPregens] = createStore<Record<string, PregenView | undefined>>({});
export { pregens };

let started = false;

/** Listens to progress, once; loads the preparation already running for `instanceId`. */
export async function watchPregen(instanceId: string) {
  if (!started) {
    started = true;
    await listen<PregenView>(EVENTS.pregenProgress, ({ payload }) => {
      const before = pregens[payload.instanceId];
      setPregens(payload.instanceId, payload);
      if (before?.status === payload.status) return;
      if (payload.status === "done") toast(`« ${payload.world} » est prêt à être exploré à plusieurs`, { tone: "success" });
      if (payload.status === "cancelled") toast("Préparation arrêtée : le terrain déjà généré est gardé.");
      if (payload.status === "failed") toast(payload.error ?? "La préparation a échoué", { tone: "error" });
    });
  }
  const current = await api.pregenStatus(instanceId);
  if (current) setPregens(instanceId, current);
}

export const isPregenActive = (view: PregenView | undefined) =>
  view !== undefined && (view.status === "starting" || view.status === "running" || view.status === "stopping");

/** "3 min 20 s" from seconds. */
export function formatEta(secs: number): string {
  if (secs < 60) return `${Math.max(1, Math.round(secs))} s`;
  const m = Math.floor(secs / 60);
  if (m < 60) return `${m} min ${Math.round(secs % 60)} s`;
  return `${Math.floor(m / 60)} h ${m % 60} min`;
}
