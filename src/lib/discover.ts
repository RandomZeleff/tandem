import { api, type ProjectType, type SearchNarrowing, type SearchResults, type SearchSort } from "./api";

/** Modrinth categories offered as filters, per kind, most useful first. */
export const FILTER_CATEGORIES: Record<ProjectType, string[]> = {
  mod: [
    "adventure",
    "technology",
    "magic",
    "worldgen",
    "mobs",
    "decoration",
    "equipment",
    "food",
    "storage",
    "transportation",
    "game-mechanics",
    "utility",
    "optimization",
    "management",
    "economy",
    "minigame",
    "cursed",
  ],
  modpack: ["adventure", "technology", "magic", "quests", "challenging", "combat", "kitchen-sink", "lightweight", "optimization", "multiplayer"],
  resourcepack: ["vanilla-like", "realistic", "simplistic", "themed", "tweaks", "decoration", "combat", "utility", "modded", "cursed"],
  shader: ["vanilla-like", "semi-realistic", "realistic", "fantasy", "cartoon", "potato", "low", "medium", "high"],
  datapack: ["adventure", "worldgen", "mobs", "magic", "technology", "decoration", "equipment", "food", "game-mechanics", "utility", "cursed"],
};

export const SORT_LABELS: Record<SearchSort, string> = {
  relevance: "Pertinence",
  downloads: "Les plus téléchargés",
  follows: "Les plus suivis",
  newest: "Nouveautés",
  updated: "Mis à jour récemment",
};

/** The shelves shown above the full list when the player is just browsing. */
export const SHELVES: { sort: SearchSort; title: string }[] = [
  { sort: "newest", title: "Nouveautés" },
  { sort: "updated", title: "Mis à jour récemment" },
  { sort: "follows", title: "Les plus suivis" },
];

const SHELF_TTL_MS = 10 * 60 * 1000;
const shelfCache = new Map<string, { at: number; results: Promise<SearchResults> }>();

/** Shelf contents, kept 10 minutes: going back to Discover or switching tabs does not refetch. */
export function shelfResults(kind: ProjectType, instanceId: string | null, narrow: SearchNarrowing): Promise<SearchResults> {
  const key = JSON.stringify([kind, instanceId, narrow]);
  const cached = shelfCache.get(key);
  if (cached && Date.now() - cached.at < SHELF_TTL_MS) return cached.results;
  const results = api.searchContent("", kind, instanceId, 0, narrow);
  shelfCache.set(key, { at: Date.now(), results });
  // A failure is not kept: the next visit retries.
  results.catch(() => shelfCache.delete(key));
  return results;
}
