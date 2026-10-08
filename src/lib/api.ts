import { invoke } from "@tauri-apps/api/core";

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

export interface GameOutput {
  instanceId: string;
  stream: "stdout" | "stderr";
  line: string;
}

export interface GameExited {
  instanceId: string;
  code: number | null;
  stopped: boolean;
  crashReport: string | null;
}

export const EVENTS = {
  log: "log://entry",
  progress: "install://progress",
  output: "game://output",
  started: "game://started",
  exited: "game://exited",
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
  openInstanceFolder: (id: string) => invoke<void>("open_instance_folder", { id }),
  launchInstance: (id: string) => invoke<void>("launch_instance", { id }),
  stopInstance: (id: string) => invoke<boolean>("stop_instance", { id }),
  runningInstances: () => invoke<string[]>("running_instances"),

  searchContent: (query: string, kind: ProjectType, instanceId: string | null, offset = 0) =>
    invoke<SearchResults>("search_content", { query, kind, instanceId, offset }),
  listContent: (instanceId: string) => invoke<InstalledContent[]>("list_content", { instanceId }),
  installContent: (instanceId: string, projectId: string) =>
    invoke<InstalledContent[]>("install_content", { instanceId, projectId }),
  removeContent: (instanceId: string, projectId: string) =>
    invoke<void>("remove_content", { instanceId, projectId }),
  checkContentUpdates: (instanceId: string) => invoke<ContentUpdate[]>("check_content_updates", { instanceId }),
  /** Every outdated project when `projectIds` is omitted. */
  updateContent: (instanceId: string, projectIds?: string[]) =>
    invoke<InstalledContent[]>("update_content", { instanceId, projectIds: projectIds ?? null }),
  setContentEnabled: (instanceId: string, projectId: string, enabled: boolean) =>
    invoke<InstalledContent>("set_content_enabled", { instanceId, projectId, enabled }),

  /** Resolves once the whole pack is installed; progress arrives as install events. */
  installModpack: (projectId: string) => invoke<Instance>("install_modpack", { projectId }),
  importModpack: (path: string) => invoke<Instance>("import_modpack", { path }),
  exportModpack: (instanceId: string, path: string, version: string) =>
    invoke<void>("export_modpack", { instanceId, path, version }),

  listAccounts: () => invoke<Account[]>("list_accounts"),
  addOfflineAccount: (username: string) => invoke<Account>("add_offline_account", { username }),
  setActiveAccount: (id: string) => invoke<Account>("set_active_account", { id }),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
};

/** Tauri rejects with the serialized CommandError string; normalise anything else. */
export function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return String(err);
}
