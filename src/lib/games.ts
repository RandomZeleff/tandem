import { createSignal } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  api,
  errorMessage,
  EVENTS,
  ROSETTA_MISSING,
  type GameExited,
  type GameStats,
  type GameOutput,
  type InstallProgress,
} from "./api";

const MAX_OUTPUT_LINES = 5000;
/** Two minutes of samples at one every 2 s. */
const MAX_STATS_SAMPLES = 60;

export type GameStatus = "idle" | "preparing" | "running";

export interface GameState {
  status: GameStatus;
  progress?: InstallProgress;
  error?: string;
  lastExit?: GameExited;
}

export interface OutputLine {
  stream: GameOutput["stream"];
  line: string;
}

const [games, setGames] = createStore<Record<string, GameState>>({});
const [output, setOutput] = createStore<Record<string, OutputLine[]>>({});
const [stats, setStats] = createStore<Record<string, GameStats[]>>({});
/** Instance whose console the bottom panel shows (the last one launched). */
const [consoleInstance, setConsoleInstance] = createSignal<string | null>(null);
/** Instance whose launch is waiting for Rosetta to be installed. */
const [rosettaPrompt, setRosettaPrompt] = createSignal<string | null>(null);

/** The latest batch of game output, for views that react to specific lines. */
const [lastOutput, setLastOutput] = createSignal<GameOutput | null>(null, { equals: false });

export { games, output, lastOutput, stats, consoleInstance, setConsoleInstance, rosettaPrompt, setRosettaPrompt };

export function gameState(id: string): GameState {
  return games[id] ?? { status: "idle" };
}

/** Setting: what the launcher does when a game starts. */
export const ON_GAME_START_SETTING = "on_game_start";
export type OnGameStart = "keep" | "minimize";

/** Games whose start minimized the launcher: it comes back when the last one exits. */
const minimizedFor = new Set<string>();

async function onGameStarted(id: string) {
  const choice = (await api.getSetting<OnGameStart>(ON_GAME_START_SETTING).catch(() => null)) ?? "keep";
  if (choice !== "minimize") return;
  minimizedFor.add(id);
  await getCurrentWindow().minimize();
}

async function onGameExited(id: string) {
  if (!minimizedFor.delete(id) || minimizedFor.size > 0) return;
  const window = getCurrentWindow();
  await window.unminimize();
  await window.setFocus();
}

let onPlayedCallback: () => void = () => {};
/** Called when a game starts or instances change in the background, so the list can refresh. */
export function onGamePlayed(cb: () => void) {
  onPlayedCallback = cb;
}

let started = false;
export async function startGameEvents() {
  if (started) return;
  started = true;

  await listen<InstallProgress>(EVENTS.progress, ({ payload }) => {
    setGames(payload.instanceId, { status: "preparing", progress: payload, error: undefined });
  });
  await listen<string>(EVENTS.started, ({ payload: id }) => {
    setGames(id, { status: "running", progress: undefined });
    onPlayedCallback();
    void onGameStarted(id).catch(() => {});
  });
  // Modpack installs reuse the progress events but end without a game.
  await listen<string>(EVENTS.installFinished, ({ payload: id }) => {
    setGames(id, { status: "idle", progress: undefined });
  });
  await listen(EVENTS.instancesChanged, () => onPlayedCallback());
  await listen<GameOutput>(EVENTS.output, ({ payload }) => {
    const fresh = payload.lines.slice(-MAX_OUTPUT_LINES).map((line) => ({ stream: payload.stream, line }));
    setOutput(
      produce((all) => {
        const lines = (all[payload.instanceId] ??= []);
        const overflow = lines.length + fresh.length - MAX_OUTPUT_LINES;
        if (overflow > 0) lines.splice(0, overflow);
        lines.push(...fresh);
      }),
    );
    setLastOutput(payload);
  });
  await listen<GameStats>(EVENTS.stats, ({ payload }) => {
    setStats(
      produce((all) => {
        const samples = (all[payload.instanceId] ??= []);
        samples.push(payload);
        if (samples.length > MAX_STATS_SAMPLES) samples.splice(0, samples.length - MAX_STATS_SAMPLES);
      }),
    );
  });
  await listen<GameExited>(EVENTS.exited, ({ payload }) => {
    setGames(payload.instanceId, { status: "idle", lastExit: payload, progress: undefined });
    setStats(payload.instanceId, []);
    void onGameExited(payload.instanceId).catch(() => {});
  });

  for (const id of await api.runningInstances()) {
    setGames(id, { status: "running" });
  }
}

export async function launch(id: string) {
  setGames(id, { status: "preparing", error: undefined, lastExit: undefined, progress: undefined });
  setOutput(id, []);
  setStats(id, []);
  setConsoleInstance(id);
  try {
    await api.launchInstance(id);
  } catch (err) {
    const message = errorMessage(err);
    if (message === ROSETTA_MISSING) {
      setGames(id, { status: "idle", progress: undefined });
      setRosettaPrompt(id);
      return;
    }
    setGames(id, { status: "idle", error: message, progress: undefined });
  }
}

export async function stop(id: string) {
  await api.stopInstance(id);
}
