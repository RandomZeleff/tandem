import { createSignal } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { listen } from "@tauri-apps/api/event";
import {
  api,
  errorMessage,
  EVENTS,
  ROSETTA_MISSING,
  type GameExited,
  type GameOutput,
  type InstallProgress,
} from "./api";

const MAX_OUTPUT_LINES = 5000;

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
/** Instance whose console the bottom panel shows (the last one launched). */
const [consoleInstance, setConsoleInstance] = createSignal<string | null>(null);
/** Instance whose launch is waiting for Rosetta to be installed. */
const [rosettaPrompt, setRosettaPrompt] = createSignal<string | null>(null);

export { games, output, consoleInstance, setConsoleInstance, rosettaPrompt, setRosettaPrompt };

export function gameState(id: string): GameState {
  return games[id] ?? { status: "idle" };
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
  });
  // Modpack installs reuse the progress events but end without a game.
  await listen<string>(EVENTS.installFinished, ({ payload: id }) => {
    setGames(id, { status: "idle", progress: undefined });
  });
  await listen(EVENTS.instancesChanged, () => onPlayedCallback());
  await listen<GameOutput>(EVENTS.output, ({ payload }) => {
    setOutput(
      produce((all) => {
        const lines = (all[payload.instanceId] ??= []);
        lines.push({ stream: payload.stream, line: payload.line });
        if (lines.length > MAX_OUTPUT_LINES) lines.splice(0, lines.length - MAX_OUTPUT_LINES);
      }),
    );
  });
  await listen<GameExited>(EVENTS.exited, ({ payload }) => {
    setGames(payload.instanceId, { status: "idle", lastExit: payload, progress: undefined });
  });

  for (const id of await api.runningInstances()) {
    setGames(id, { status: "running" });
  }
}

export async function launch(id: string) {
  setGames(id, { status: "preparing", error: undefined, lastExit: undefined, progress: undefined });
  setOutput(id, []);
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
