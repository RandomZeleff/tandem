import { listen } from "@tauri-apps/api/event";
import { createResource, createRoot } from "solid-js";
import { createStore } from "solid-js/store";
import {
  api,
  errorMessage,
  EVENTS,
  type ProviderConfig,
  type TranslationProgress,
  type TranslationReport,
  type TranslationSourceKind,
} from "./api";

export interface RunState {
  running: boolean;
  progress?: TranslationProgress;
  report?: TranslationReport;
  error?: string;
}

/** Translation runs by instance: they go on while the player browses elsewhere. */
const [runs, setRuns] = createStore<Record<string, RunState>>({});

export const runState = (instanceId: string): RunState => runs[instanceId] ?? { running: false };

/** Translation settings shared by the settings page, instance tabs and project pages. */
export const { translationSettings, refetchTranslationSettings } = createRoot(() => {
  const [translationSettings, { refetch }] = createResource(api.translationSettings);
  return { translationSettings, refetchTranslationSettings: refetch };
});

/** The configured provider, if it can be used (key present when needed). */
export function readyProvider(): ProviderConfig | null {
  const settings = translationSettings();
  const config = settings?.provider.config;
  if (!settings || !config || !config.model) return null;
  const preset = settings.presets.find((p) => p.id === config.preset);
  if (preset?.needsKey && !settings.provider.hasKey) return null;
  return config;
}

export function isLocalProvider(config: ProviderConfig | null): boolean {
  return !!config && !!translationSettings()?.presets.find((p) => p.id === config.preset)?.local;
}

export function providerLabel(config: ProviderConfig): string {
  const preset = translationSettings()?.presets.find((p) => p.id === config.preset);
  return `${preset?.label ?? config.preset} · ${config.model}`;
}

let listening = false;
export async function startTranslationEvents() {
  if (listening) return;
  listening = true;
  await listen<{ instanceId: string; progress: TranslationProgress }>(EVENTS.translateProgress, (e) => {
    setRuns(e.payload.instanceId, (r) => ({ ...(r ?? { running: true }), progress: e.payload.progress }));
  });
}

/** Starts translating an instance; resolves with the report when the run ends. */
export async function startTranslation(instanceId: string, locale: string): Promise<TranslationReport | null> {
  setRuns(instanceId, { running: true });
  try {
    const report = await api.translateInstance(instanceId, locale);
    setRuns(instanceId, { running: false, progress: report.progress, report });
    if (report.progress.elapsedMs > 5000 && report.progress.completionTokens > 0) {
      rememberSpeed(report.progress.completionTokens / (report.progress.elapsedMs / 1000));
    }
    return report;
  } catch (err) {
    setRuns(instanceId, { running: false, error: errorMessage(err) });
    return null;
  }
}

export function dismissReport(instanceId: string) {
  setRuns(instanceId, { running: false });
}

const SPEED_KEY = "translation.speed";

/** Output tokens per second measured on the last run with the current provider. */
function rememberSpeed(tokensPerSecond: number) {
  const config = translationSettings()?.provider.config;
  if (!config) return;
  try {
    localStorage.setItem(SPEED_KEY, JSON.stringify({ id: `${config.preset}:${config.model}`, tokensPerSecond }));
  } catch {
    // Storage unavailable: estimates fall back to defaults.
  }
}

/** Expected output speed: measured last time, else a cautious default. */
export function expectedSpeed(config: ProviderConfig): { tokensPerSecond: number; measured: boolean } {
  try {
    const saved = JSON.parse(localStorage.getItem(SPEED_KEY) ?? "null");
    if (saved?.id === `${config.preset}:${config.model}` && saved.tokensPerSecond > 0) {
      return { tokensPerSecond: saved.tokensPerSecond, measured: true };
    }
  } catch {
    // Ignore unreadable storage.
  }
  return { tokensPerSecond: isLocalProvider(config) ? 35 : 120 * Math.max(1, config.concurrency), measured: false };
}

/** `2 h 15`, `12 min`, `moins d'une minute`. */
export function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 60) return "moins d'une minute";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest ? `${hours} h ${String(rest).padStart(2, "0")}` : `${hours} h`;
}

export const SOURCE_KINDS: Record<TranslationSourceKind, string> = {
  mod: "Mod",
  resourcePack: "Pack de ressources",
  kubeJs: "KubeJS",
  quests: "Quêtes",
  book: "Livre",
};
