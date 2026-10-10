import { openUrl } from "@tauri-apps/plugin-opener";
import type { ProjectDetails, ProjectSide } from "./api";
import { navigate } from "./store";

/** Opens a project page; `instanceId` preselects where content would be installed. */
export function openProject(id: string, instanceId?: string) {
  navigate(instanceId ? { page: "project", id, instanceId } : { page: "project", id });
}

/** Opens a web page in the player's browser. */
export function openExternal(url: string) {
  void openUrl(url).catch(() => window.open(url, "_blank", "noopener"));
}

const MODRINTH_PROJECT = /^https?:\/\/(?:www\.)?modrinth\.com\/(?:mod|modpack|resourcepack|shader|datapack|plugin|project)\/([^/?#]+)/i;

/** Slug or id of a Modrinth project page URL, so the link opens in Tandem. */
export function modrinthProject(url: string): string | null {
  return MODRINTH_PROJECT.exec(url)?.[1] ?? null;
}

/** Click handler for rendered descriptions: Modrinth projects open in Tandem, the rest in the browser. */
export function followLink(e: MouseEvent, instanceId?: string) {
  const link = (e.target as HTMLElement).closest("a");
  const href = link?.getAttribute("href");
  if (!href) return;
  e.preventDefault();
  const project = modrinthProject(href);
  if (project) openProject(project, instanceId);
  else if (/^(https?|mailto):/i.test(href)) openExternal(href);
}

const TYPES: Record<string, string> = {
  mod: "Mod",
  modpack: "Modpack",
  resourcepack: "Pack de textures",
  shader: "Shader",
  datapack: "Datapack",
  plugin: "Plugin",
};

export function projectTypeLabel(type: string): string {
  return TYPES[type] ?? type;
}

/** Modrinth's category slugs, in French. Unknown ones show as is. */
const CATEGORIES: Record<string, string> = {
  adventure: "Aventure",
  datapack: "Datapack",
  cursed: "Insolite",
  decoration: "Décoration",
  economy: "Économie",
  equipment: "Équipement",
  food: "Nourriture",
  "game-mechanics": "Mécaniques de jeu",
  library: "Bibliothèque",
  magic: "Magie",
  management: "Gestion",
  minigame: "Mini-jeu",
  mobs: "Créatures",
  optimization: "Optimisation",
  social: "Social",
  storage: "Stockage",
  technology: "Technologie",
  transportation: "Transport",
  utility: "Utilitaire",
  worldgen: "Génération du monde",
  challenging: "Exigeant",
  combat: "Combat",
  "kitchen-sink": "Fourre-tout",
  lightweight: "Léger",
  multiplayer: "Multijoueur",
  quests: "Quêtes",
  audio: "Sons",
  blocks: "Blocs",
  "core-shaders": "Core shaders",
  entities: "Entités",
  environment: "Environnement",
  fonts: "Polices",
  gui: "Interface",
  items: "Objets",
  locale: "Langues",
  models: "Modèles",
  modded: "Pour les mods",
  realistic: "Réaliste",
  simplistic: "Épuré",
  themed: "Thématique",
  tweaks: "Retouches",
  "vanilla-like": "Fidèle au jeu",
  atmosphere: "Atmosphère",
  cartoon: "Cartoon",
  "colored-lighting": "Lumières colorées",
  fantasy: "Fantaisie",
  foliage: "Végétation",
  "path-tracing": "Path tracing",
  pbr: "PBR",
  reflections: "Reflets",
  "semi-realistic": "Semi-réaliste",
  shadows: "Ombres",
  potato: "Très léger",
  low: "Peu gourmand",
  medium: "Moyennement gourmand",
  high: "Gourmand",
  screenshot: "Pour les captures",
};

export function categoryLabel(slug: string): string {
  return CATEGORIES[slug] ?? slug;
}

/** Loaders shown as categories by Modrinth; listed separately on project pages. */
const LOADER_SLUGS = new Set(["fabric", "quilt", "forge", "neoforge", "liteloader", "rift", "modloader", "minecraft"]);

export function isLoaderCategory(slug: string): boolean {
  return LOADER_SLUGS.has(slug);
}

const isRelease = (v: string) => /^\d+(\.\d+)+$/.test(v);

/** `1.16.5 – 26.3` from a list of game versions, releases only; snapshots alone are listed. */
export function versionRange(versions: string[]): string {
  const releases = versions.filter(isRelease);
  if (releases.length === 0) return versions.slice(0, 2).join(", ");
  const sorted = [...releases].sort(compareVersions);
  return sorted.length === 1 ? sorted[0] : `${sorted[0]} – ${sorted[sorted.length - 1]}`;
}

/** Game versions newest first, releases only. */
export function releasesNewestFirst(versions: string[]): string[] {
  return versions.filter(isRelease).sort(compareVersions).reverse();
}

export function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const diff = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

const needed = (side: ProjectSide) => side === "required" || side === "optional";

/** Where the project has to be installed, in the player's words. */
export function sideLabel(details: Pick<ProjectDetails, "clientSide" | "serverSide">): string | null {
  const { clientSide: client, serverSide: server } = details;
  if (client === "unknown" && server === "unknown") return null;
  if (needed(client) && server === "unsupported") return "Côté client uniquement";
  if (client === "unsupported" && needed(server)) return "Côté serveur uniquement";
  if (client === "required" && server === "required") return "Client et serveur";
  if (client === "optional" && server === "required") return "Serveur (facultatif côté client)";
  if (client === "required" && server === "optional") return "Client (facultatif côté serveur)";
  return "Client ou serveur";
}

const VERSION_TYPES: Record<string, string> = { release: "Stable", beta: "Bêta", alpha: "Alpha" };

export function versionTypeLabel(type: string): string {
  return VERSION_TYPES[type] ?? type;
}

const longDate = new Intl.DateTimeFormat("fr-FR", { dateStyle: "long" });

export function formatDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? "" : longDate.format(date);
}
