import type { GameMode, World } from "./api";

export const GAME_MODES: Record<GameMode, string> = {
  survival: "Survie",
  creative: "Créatif",
  adventure: "Aventure",
  spectator: "Spectateur",
  hardcore: "Hardcore",
};

/**
 * Whether the game can start straight into a world (`--quickPlaySingleplayer`, from 1.20
 * and its 23w snapshots). Older versions open on the title screen.
 */
export function opensWorld(gameVersion: string): boolean {
  const release = /^(\d+)\.(\d+)/.exec(gameVersion);
  if (release) return Number(release[1]) > 1 || Number(release[2]) >= 20;
  const snapshot = /^(\d\d)w\d\d/.exec(gameVersion);
  return !!snapshot && Number(snapshot[1]) >= 23;
}

/** The world played last, the one "Continuer" opens. */
export function lastWorld(worlds: World[]): World | undefined {
  return worlds.reduce<World | undefined>((best, w) => ((w.lastPlayed ?? 0) > (best?.lastPlayed ?? -1) ? w : best), undefined);
}

export const worldPlayedAt = (world: World) => (world.lastPlayed === null ? null : new Date(world.lastPlayed).toISOString());
