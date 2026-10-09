import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  version: string;
  dataDir: string;
}

export type LogLevel = "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";

export interface LogEntry {
  seq: number;
  timestampMs: number;
  level: LogLevel;
  target: string;
  message: string;
}

export interface VersionEntry {
  id: string;
  type: "release" | "snapshot" | "old_beta" | "old_alpha";
  url: string;
  releaseTime: string;
  sha1: string;
}

export interface VersionList {
  latest: { release: string; snapshot: string };
  versions: VersionEntry[];
}

export type Loader = "vanilla" | "fabric" | "quilt" | "forge" | "neoforge";

export interface LoaderVersion {
  version: string;
  stable: boolean;
  /** Picked by the loader's team for this game version (Forge). */
  recommended: boolean;
}

export interface NewInstance {
  name: string;
  gameVersion: string;
  loader?: Loader;
  /** Latest stable when omitted. */
  loaderVersion?: string;
}

export interface Instance {
  id: string;
  name: string;
  gameVersion: string;
  loader: Loader;
  loaderVersion: string | null;
  javaPath: string | null;
  memoryMb: number | null;
  jvmArgs: string | null;
  icon: string | null;
  createdAt: string;
  lastPlayedAt: string | null;
  /** Modrinth modpack the instance was created from. */
  packProjectId: string | null;
  packVersionId: string | null;
  packVersion: string | null;
}

export type ContentKind = "mod" | "resourcepack" | "shader";

/** What the Discover page can search: instance content or modpacks. */
export type ProjectType = ContentKind | "modpack";

export interface SearchHit {
  projectId: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  downloads: number;
  follows: number;
  iconUrl: string | null;
  displayCategories: string[];
  dateModified: string;
}

export interface SearchResults {
  hits: SearchHit[];
  offset: number;
  limit: number;
  totalHits: number;
}

export interface InstalledContent {
  projectId: string;
  versionId: string;
  kind: ContentKind;
  title: string;
  versionNumber: string;
  fileName: string;
  sha1: string;
  iconUrl: string | null;
  isDependency: boolean;
  enabled: boolean;
  installedAt: string;
}

export interface ContentUpdate {
  projectId: string;
  title: string;
  currentVersion: string;
  newVersion: string;
}

export interface Account {
  id: string;
  kind: "microsoft" | "offline";
  username: string;
  mcUuid: string;
  isActive: boolean;
}

export type InstallStage = "metadata" | "downloading" | "finalizing" | "processing";

export interface InstallProgress {
  instanceId: string;
  stage: InstallStage;
  doneFiles: number;
  totalFiles: number;
  doneBytes: number;
  totalBytes: number;
}

/** A modpack version Tandem can install. */
export interface PackVersion {
  id: string;
  versionNumber: string;
  versionType: "release" | "beta" | "alpha" | string;
  gameVersions: string[];
  loaders: string[];
  datePublished: string;
  size: number;
  /** Installed when the player does not choose. */
  recommended: boolean;
}

export interface ProjectLink {
  kind: "source" | "issues" | "wiki" | "discord" | "donation";
  /** Donation platform, empty otherwise. */
  label: string;
  url: string;
}

export interface GalleryImage {
  thumbUrl: string;
  url: string;
  title: string | null;
  description: string | null;
}

export interface ProjectAuthor {
  name: string;
  avatarUrl: string | null;
  role: string;
  url: string;
}

export type ProjectSide = "required" | "optional" | "unsupported" | "unknown";

/** Everything a project page shows. */
export interface ProjectDetails {
  id: string;
  slug: string;
  projectType: ProjectType | "datapack" | "plugin";
  title: string;
  summary: string;
  /** Sanitized by the backend. */
  bodyHtml: string;
  iconUrl: string | null;
  color: number | null;
  downloads: number;
  followers: number;
  published: string;
  updated: string;
  categories: string[];
  loaders: string[];
  /** Oldest first, snapshots included. */
  gameVersions: string[];
  clientSide: ProjectSide;
  serverSide: ProjectSide;
  license: { id: string; name: string; url: string | null } | null;
  links: ProjectLink[];
  gallery: GalleryImage[];
  organization: { name: string; iconUrl: string | null; url: string } | null;
  authors: ProjectAuthor[];
  url: string;
}

export interface VersionSummary {
  id: string;
  name: string;
  versionNumber: string;
  versionType: "release" | "beta" | "alpha" | string;
  gameVersions: string[];
  loaders: string[];
  datePublished: string;
  downloads: number;
  fileName: string | null;
  size: number;
  hasChangelog: boolean;
  /** Installable: in the given instance for content, by Tandem for a modpack. */
  compatible: boolean;
}

export interface ProjectVersions {
  versions: VersionSummary[];
  /** What "Install" picks. */
  recommended: string | null;
}

export interface ProjectSummary {
  id: string;
  slug: string;
  title: string;
  summary: string;
  projectType: string;
  iconUrl: string | null;
  downloads: number;
}

export interface DependencyItem {
  /** `embedded`: shipped inside (the mods of a modpack). */
  kind: "required" | "optional" | "incompatible" | "embedded";
  project: ProjectSummary;
}

/** An enabled mod that needs another one to start. */
export interface Dependent {
  name: string;
  fileName: string;
}

/** The jar behind a mod id. `fileName` is without `.disabled`. */
export interface ModProvider {
  modId: string;
  name: string;
  fileName: string;
  enabled: boolean;
}

/** A batch of lines from one stream of a running game. */
export interface GameOutput {
  instanceId: string;
  stream: "stdout" | "stderr";
  lines: string[];
}

export interface GameExited {
  instanceId: string;
  code: number | null;
  stopped: boolean;
  crashReport: string | null;
  /** Present when the game crashed. */
  analysis: CrashAnalysis | null;
}

export type SuspectReason = "namedByLoader" | "mixin" | "incompatible" | "stackTrace";
export type CrashHint = "outOfMemory" | "wrongJava" | "graphics" | "incompatibleMods" | "nativeCrash";

export interface CrashAnalysis {
  description: string | null;
  exception: string | null;
  suspects: { name: string; fileName: string | null; reason: SuspectReason }[];
  hint: CrashHint | null;
  /** File the analysis was read from (crash report, JVM crash log or latest.log). */
  source: string | null;
}

/** Sampled every 2 s while a game runs. */
export interface GameStats {
  instanceId: string;
  /** Resident memory of the Java process (heap and native). */
  memoryBytes: number;
  /** Share of the whole machine, 0–100. */
  cpuPercent: number;
}

export type GameMode = "survival" | "creative" | "adventure" | "spectator" | "hardcore";

export interface World {
  /** Folder in saves/, which identifies the world. */
  folder: string;
  name: string;
  /** Unix milliseconds. */
  lastPlayed: number | null;
  gameMode: GameMode;
  version: string | null;
  sizeBytes: number;
  /** icon.png as a data URL. */
  icon: string | null;
}

export type BackupKind = "manual" | "auto" | "beforeRestore";

export interface WorldBackup {
  world: string;
  fileName: string;
  createdAt: number;
  kind: BackupKind;
  sizeBytes: number;
}

export interface Screenshot {
  /** File in screenshots/, which identifies the screenshot. */
  fileName: string;
  path: string;
  /** Unix milliseconds. */
  takenAt: number;
  sizeBytes: number;
}

/** Image URL served by the backend's `tandem-shot` protocol; `thumb` asks for a small JPEG. */
export function screenshotUrl(instanceId: string, shot: Screenshot, thumb = false): string {
  return `${convertFileSrc(`${instanceId}/${shot.fileName}`, "tandem-shot")}?v=${shot.takenAt}${thumb ? "&thumb" : ""}`;
}

export interface PerfSuggestion {
  projectId: string;
  title: string;
  iconUrl: string | null;
}

export interface MemoryInfo {
  /** What "automatic" gives this instance right now (depends on its mods). */
  autoMb: number;
  totalMb: number;
}

export const EVENTS = {
  log: "log://entry",
  progress: "install://progress",
  output: "game://output",
  started: "game://started",
  exited: "game://exited",
  stats: "game://stats",
  installFinished: "install://finished",
  instancesChanged: "instances://changed",
} as const;

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  getLogs: () => invoke<LogEntry[]>("get_logs"),
  getSetting: <T>(key: string) => invoke<T | null>("get_setting", { key }),
  setSetting: (key: string, value: unknown) => invoke<void>("set_setting", { key, value }),

  listVersions: () => invoke<VersionList>("list_versions"),
  listLoaderVersions: (loader: Loader, gameVersion: string) =>
    invoke<LoaderVersion[]>("list_loader_versions", { loader, gameVersion }),

  listInstances: () => invoke<Instance[]>("list_instances"),
  createInstance: (instance: NewInstance) => invoke<Instance>("create_instance", { instance }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  memoryInfo: (id: string) => invoke<MemoryInfo>("memory_info", { id }),
  /** `null` goes back to automatic memory. */
  setInstanceMemory: (id: string, memoryMb: number | null) =>
    invoke<void>("set_instance_memory", { id, memoryMb }),
  openDataFolder: () => invoke<void>("open_data_folder"),
  openInstanceFolder: (id: string) => invoke<void>("open_instance_folder", { id }),
  launchInstance: (id: string) => invoke<void>("launch_instance", { id }),
  stopInstance: (id: string) => invoke<boolean>("stop_instance", { id }),
  /** `false` when the user dismisses macOS's password prompt. */
  installRosetta: () => invoke<boolean>("install_rosetta"),
  runningInstances: () => invoke<string[]>("running_instances"),

  searchContent: (query: string, kind: ProjectType, instanceId: string | null, offset = 0) =>
    invoke<SearchResults>("search_content", { query, kind, instanceId, offset }),
  listContent: (instanceId: string) => invoke<InstalledContent[]>("list_content", { instanceId }),
  /** The best compatible version when `versionId` is omitted. */
  installContent: (instanceId: string, projectId: string, versionId?: string) =>
    invoke<InstalledContent[]>("install_content", { instanceId, projectId, versionId: versionId ?? null }),
  projectDetails: (id: string) => invoke<ProjectDetails>("project_details", { id }),
  projectVersions: (projectId: string, instanceId: string | null) =>
    invoke<ProjectVersions>("project_versions", { projectId, instanceId }),
  versionChangelog: (projectId: string, versionId: string) =>
    invoke<string>("version_changelog", { projectId, versionId }),
  versionDependencies: (projectId: string, versionId: string) =>
    invoke<DependencyItem[]>("version_dependencies", { projectId, versionId }),
  removeContent: (instanceId: string, projectId: string) =>
    invoke<void>("remove_content", { instanceId, projectId }),
  listWorlds: (instanceId: string) => invoke<World[]>("list_worlds", { instanceId }),
  listWorldBackups: (instanceId: string) => invoke<WorldBackup[]>("list_world_backups", { instanceId }),
  backupWorld: (instanceId: string, world: string) => invoke<WorldBackup>("backup_world", { instanceId, world }),
  restoreWorldBackup: (instanceId: string, world: string, fileName: string) =>
    invoke<void>("restore_world_backup", { instanceId, world, fileName }),
  openWorldBackups: (instanceId: string, world: string) => invoke<void>("open_world_backups", { instanceId, world }),
  listScreenshots: (instanceId: string) => invoke<Screenshot[]>("list_screenshots", { instanceId }),
  deleteScreenshot: (instanceId: string, fileName: string) => invoke<void>("delete_screenshot", { instanceId, fileName }),
  openScreenshotsFolder: (instanceId: string) => invoke<void>("open_screenshots_folder", { instanceId }),
  warmModDependencies: (instanceId: string) => invoke<void>("warm_mod_dependencies", { instanceId }),
  contentDependents: (instanceId: string, projectId: string) =>
    invoke<Dependent[]>("content_dependents", { instanceId, projectId }),
  modProviders: (instanceId: string, modIds: string[]) => invoke<ModProvider[]>("mod_providers", { instanceId, modIds }),
  perfSuggestions: (instanceId: string) => invoke<PerfSuggestion[]>("perf_suggestions", { instanceId }),
  checkContentUpdates: (instanceId: string) => invoke<ContentUpdate[]>("check_content_updates", { instanceId }),
  /** Every outdated project when `projectIds` is omitted. */
  updateContent: (instanceId: string, projectIds?: string[]) =>
    invoke<InstalledContent[]>("update_content", { instanceId, projectIds: projectIds ?? null }),
  setContentEnabled: (instanceId: string, projectId: string, enabled: boolean) =>
    invoke<InstalledContent>("set_content_enabled", { instanceId, projectId, enabled }),

  /** Resolves once the whole pack is installed; progress arrives as install events. */
  /** The recommended version when `versionId` is omitted. */
  installModpack: (projectId: string, versionId?: string) =>
    invoke<Instance>("install_modpack", { projectId, versionId: versionId ?? null }),
  modpackVersions: (projectId: string) => invoke<PackVersion[]>("modpack_versions", { projectId }),
  importModpack: (path: string) => invoke<Instance>("import_modpack", { path }),
  exportModpack: (instanceId: string, path: string, version: string) =>
    invoke<void>("export_modpack", { instanceId, path, version }),

  listAccounts: () => invoke<Account[]>("list_accounts"),
  addOfflineAccount: (username: string) => invoke<Account>("add_offline_account", { username }),
  setActiveAccount: (id: string) => invoke<Account>("set_active_account", { id }),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
};

/** Launch error meaning the version needs Rosetta 2, which is not installed. */
export const ROSETTA_MISSING = "rosetta-missing";

/** Error meaning Modrinth does not know the project. */
export const PROJECT_NOT_FOUND = "project-not-found";

/** Tauri rejects with the serialized CommandError string; normalise anything else. */
export function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return String(err);
}
