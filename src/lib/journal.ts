import type { Journal } from "./api";

/** Vanilla big steps (`journal.rs`), as the player would name them. */
export const MILESTONES: Record<string, string> = {
  "story/smelt_iron": "Premier lingot de fer",
  "story/mine_diamond": "Des diamants",
  "story/enter_the_nether": "Le Nether",
  "nether/find_fortress": "Une forteresse du Nether",
  "story/follow_ender_eye": "Le fort de l'End",
  "story/enter_the_end": "L'End",
  "end/kill_dragon": "Le dragon de l'Ender vaincu",
  "nether/summon_wither": "Le Wither invoqué",
  "end/elytra": "Des élytres",
};

const number = new Intl.NumberFormat("fr-FR");
const decimal = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 1 });
const day = new Intl.DateTimeFormat("fr-FR", { day: "numeric", month: "short", year: "numeric" });

export const formatNumber = (n: number) => number.format(n);

/** `42 min`, `3 h 20`, `112 h`. */
export function formatPlayTime(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours >= 10) return `${number.format(hours)} h`;
  const rest = minutes % 60;
  return rest ? `${hours} h ${String(rest).padStart(2, "0")}` : `${hours} h`;
}

/** `850 m`, `34,2 km`. */
export function formatDistance(meters: number): string {
  return meters < 1000 ? `${meters} m` : `${decimal.format(meters / 1000)} km`;
}

/** When a step was reached; empty for steps the game did not date. */
export function formatStepDate(ms: number | null): string {
  return ms ? day.format(new Date(ms)) : "";
}

/** The first big step not reached yet. */
export function nextMilestone(journal: Journal): string | undefined {
  const next = journal.milestones.find((m) => m.doneAt === null);
  return next && MILESTONES[next.id];
}

/** Nothing recorded yet: the world was never really played. */
export function isEmptyJournal(journal: Journal): boolean {
  return !journal.stats && !journal.milestones.some((m) => m.doneAt !== null) && !journal.quests?.done;
}
