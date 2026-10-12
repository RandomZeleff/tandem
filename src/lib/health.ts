import { listen } from "@tauri-apps/api/event";
import { createStore, produce } from "solid-js/store";
import { EVENTS, type HealthSignal } from "./api";

interface Report {
  mspt: number;
  max: number;
  players: number;
  at: number;
}

interface WorldHealthState {
  reports: Report[];
  behindAt: number | null;
}

/** Health of each instance's shared world, from `world://health`. */
const [health, setHealth] = createStore<Record<string, WorldHealthState | undefined>>({});

let started = false;

export async function startHealthEvents() {
  if (started) return;
  started = true;
  await listen<{ instanceId: string; signal: HealthSignal }>(EVENTS.health, ({ payload }) => {
    const { instanceId, signal } = payload;
    setHealth(
      produce((all) => {
        const state = (all[instanceId] ??= { reports: [], behindAt: null });
        if (signal.type === "report") {
          state.reports.push({ mspt: signal.mspt, max: signal.max, players: signal.players, at: Date.now() });
          if (state.reports.length > 3) state.reports.shift();
        } else {
          state.behindAt = Date.now();
        }
      }),
    );
  });
}

export type Level = "ok" | "warn" | "bad" | "unknown";

export interface Reading {
  level: Level;
  label: string;
  detail: string;
}

/** Ticks over 50 ms make the world late; reports come every ~5 s. */
export function worldReading(instanceId: string, since: number, now: number): Reading {
  const state = health[instanceId];
  const recent = (state?.reports ?? []).filter((r) => now - r.at < 30_000);
  const behind = state?.behindAt != null && now - state.behindAt < 60_000;
  if (recent.length > 0) {
    const mspt = recent.reduce((a, r) => a + r.mspt, 0) / recent.length;
    const detail = `${Math.round(mspt)} ms par tick (50 au maximum)`;
    if (behind || mspt >= 45) return { level: "bad", label: "En retard", detail };
    if (mspt >= 30) return { level: "warn", label: "Chargé", detail };
    return { level: "ok", label: "Fluide", detail };
  }
  if (behind) return { level: "bad", label: "En retard", detail: "le monde signale du retard" };
  if (now - since > 30_000) return { level: "ok", label: "Fluide", detail: "aucun retard signalé" };
  return { level: "unknown", label: "Mesure…", detail: "premier relevé dans quelques secondes" };
}

export function memoryReading(freeBytes: number | null): Reading {
  if (freeBytes === null) return { level: "unknown", label: "Mesure…", detail: "" };
  const gb = freeBytes / 1024 ** 3;
  const detail = `${gb.toLocaleString("fr-FR", { maximumFractionDigits: 1 })} Go libres`;
  if (gb < 1) return { level: "bad", label: "Presque pleine", detail };
  if (gb < 2) return { level: "warn", label: "Juste", detail };
  return { level: "ok", label: "Suffisante", detail };
}

/** One tip for the worst problem, or null when everything keeps up. */
export function advice(world: Reading, memory: Reading): string | null {
  if (memory.level === "bad")
    return "Il reste très peu de mémoire : ferme ce qui ne sert pas pendant la partie (navigateur, Discord…).";
  if (world.level === "bad")
    return "Le monde n'arrive plus à suivre : restez moins éloignés les uns des autres, et préparez le monde (onglet Mondes de l'instance) avant la prochaine partie.";
  if (world.level === "warn") return "Le monde est chargé : de nouvelles arrivées ou beaucoup d'exploration pourraient le ralentir.";
  if (memory.level === "warn") return "La mémoire est juste : évite d'ouvrir d'autres programmes lourds pendant la partie.";
  return null;
}
