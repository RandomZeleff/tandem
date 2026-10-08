/**
 * Browser-only preview: fakes the Tauri backend so the UI can be designed in a
 * regular browser (`pnpm dev`, then open http://localhost:1420). Never bundled
 * in the desktop app: main.tsx only imports it in dev outside Tauri.
 */
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type {
  Account,
  ContentKind,
  InstalledContent,
  Instance,
  Loader,
  LogEntry,
  NewInstance,
  ProjectType,
  SearchHit,
} from "../lib/api";

const now = Date.now();
const iso = (hoursAgo: number) => new Date(now - hoursAgo * 3_600_000).toISOString();

const instances: Instance[] = [
  ["survie-avec-leo", "Survie avec Léo", "26.3", "fabric", "0.19.5", iso(20)],
  ["pack-create", "Pack Create", "1.21.1", "neoforge", "21.1.77", iso(70)],
  ["crea-redstone", "Créa redstone", "26.3", "vanilla", null, iso(100)],
  ["nostalgie-1-12", "Nostalgie 1.12", "1.12.2", "vanilla", null, iso(500)],
].map(([id, name, gameVersion, loader, loaderVersion, lastPlayedAt]) => ({
  id: id!,
  name: name!,
  gameVersion: gameVersion!,
  loader: loader as Loader,
  loaderVersion,
  javaPath: null,
  memoryMb: null,
  jvmArgs: null,
  icon: null as string | null,
  createdAt: iso(900),
  lastPlayedAt,
  packProjectId: null as string | null,
  packVersionId: null as string | null,
  packVersion: null as string | null,
}));

/** Fabric/Quilt-like answers: nothing before 1.14, a beta on top of stable builds. */
function fakeLoaderVersions(loader: Loader, gameVersion: string) {
  const v = (version: string, stable = true, recommended = false) => ({ version, stable, recommended });
  if (loader === "forge") return [v("66.0.9"), v("66.0.4", true, true), v("66.0.1")];
  if (loader === "neoforge") return [v("26.3.0.58-beta", false), v("26.3.0.40-beta", false)];
  if (gameVersion === "1.12.2") return [];
  return loader === "quilt"
    ? [v("0.31.0-beta.2", false), v("0.30.1"), v("0.29.2")]
    : [v("0.19.5"), v("0.19.4"), v("0.18.6")];
}

type Row = [id: string, title: string, author: string, description: string, downloads: number, categories: string[]];

const hit = ([projectId, title, author, description, downloads, displayCategories]: Row): SearchHit => ({
  projectId,
  slug: projectId,
  title,
  description,
  author,
  downloads,
  follows: Math.round(downloads / 1000),
  iconUrl: `https://cdn.modrinth.com/data/${projectId}/icon.png`,
  displayCategories,
  dateModified: iso(48),
});

/** Fake Modrinth catalogue (real project ids, so icons load from the Modrinth CDN). */
const CATALOGUE: Record<ProjectType, SearchHit[]> = {
  modpack: (
    [
      ["1KVo5zza", "Fabulously Optimized", "robotkoer", "Improved performance and graphics, with familiar menus and controls.", 14_000_000, ["fabric", "optimization"]],
      ["BYfVnHa7", "Simply Optimized", "JustAlittleWolf", "The leading Fabric modpack for optimization, without changing the look of the game.", 3_100_000, ["fabric", "optimization"]],
    ] as Row[]
  ).map(hit),
  mod: (
    [
      ["P7dR8mSH", "Fabric API", "modmuss50", "Lightweight and modular API providing common hooks for Fabric mods.", 120_000_000, ["fabric", "library"]],
      ["AANobbMI", "Sodium", "jellysquid3", "The fastest and most compatible rendering optimization mod for Minecraft.", 98_000_000, ["optimization"]],
      ["YL57xq9U", "Iris Shaders", "coderbot", "A modern shader pack loader compatible with existing OptiFine shader packs.", 82_000_000, ["decoration", "optimization"]],
      ["mOgUt4GM", "Mod Menu", "Prospector", "Adds a mod menu to view the list of mods you have installed.", 70_000_000, ["utility"]],
    ] as Row[]
  ).map(hit),
  resourcepack: ([["Bq0hLR4Y", "Faithful 32x", "Faithful", "Faithful to the original textures, at twice the resolution.", 4_200_000, ["32x", "vanilla-like"]]] as Row[]).map(hit),
  shader: ([["HVnmMxH1", "Complementary Shaders - Reimagined", "EminGT", "Enhances your Minecraft experience while staying close to the default art style.", 11_000_000, ["iris", "optifine"]]] as Row[]).map(hit),
};

const make = (h: SearchHit, kind: ContentKind, isDependency: boolean, versionId = "v2"): InstalledContent => ({
  projectId: h.projectId,
  versionId,
  kind,
  title: h.title,
  versionNumber: versionId === "old" ? "1.0.0+1.21.4" : "1.2.0+1.21.4",
  fileName: `${h.projectId}.jar`,
  sha1: "0",
  iconUrl: h.iconUrl,
  isDependency,
  enabled: true,
  installedAt: new Date().toISOString(),
});

/** "survie-avec-leo" starts with content, two items outdated (version id "old"). */
const content: Record<string, InstalledContent[]> = {
  "survie-avec-leo": [
    make(CATALOGUE.mod[1], "mod", false, "old"),
    make(CATALOGUE.mod[3], "mod", false),
    make(CATALOGUE.mod[0], "mod", true, "old"),
    { ...make(CATALOGUE.shader[0], "shader", false), enabled: false },
  ],
};

function fakeInstall(instanceId: string, projectId: string): InstalledContent[] {
  const found = Object.entries(CATALOGUE)
    .flatMap(([kind, hits]) => hits.map((h) => ({ kind: kind as ContentKind, h })))
    .find(({ h }) => h.projectId === projectId || h.title.toLowerCase().startsWith(projectId));
  if (!found) throw new Error(`unknown project ${projectId}`);
  const list = (content[instanceId] ??= []);
  const added = [make(found.h, found.kind, false)];
  // Mod Menu pulls Fabric API, like the real dependency resolution.
  if (found.h.projectId === "mOgUt4GM" && !list.some((c) => c.projectId === "P7dR8mSH")) {
    added.push(make(CATALOGUE.mod[0], "mod", true));
  }
  list.push(...added);
  return added;
}

let accounts: Account[] = [
  { id: "a1", kind: "offline", username: "Zeleff", mcUuid: "00000000-0000-3000-8000-000000000000", isActive: true },
];

const logs: LogEntry[] = [
  { seq: 1, timestampMs: now - 5000, level: "INFO", target: "tandem_lib", message: "Tandem started (aperçu navigateur)" },
  { seq: 2, timestampMs: now - 4000, level: "DEBUG", target: "tandem_core::db", message: "database ready" },
];

/** Simulates an install then a running game, emitting the real event names. */
async function fakeLaunch(id: string) {
  const total = 331_000_000;
  for (let i = 0; i <= 20; i++) {
    await new Promise((r) => setTimeout(r, 150));
    await emit("install://progress", {
      instanceId: id,
      stage: "downloading",
      doneFiles: i * 75,
      totalFiles: 1500,
      doneBytes: (total * i) / 20,
      totalBytes: total,
    });
  }
  await emit("game://started", id);
  for (const line of ["[Render thread/INFO]: Setting user: Zeleff", "[Render thread/INFO]: Backend library: LWJGL", "[Render thread/INFO]: Sound engine started"]) {
    await emit("game://output", { instanceId: id, stream: "stdout", line });
  }
}

export function installMocks() {
  mockWindows("main");
  mockIPC(
    async (cmd, payload) => {
      const args = (payload ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "app_info":
          return { version: "0.1.0", dataDir: "C:\\Users\\you\\AppData\\Roaming\\Tandem" };
        case "get_logs":
          return logs;
        case "list_instances":
          return instances;
        case "list_accounts":
          return accounts;
        case "running_instances":
          return [];
        case "list_versions":
          return {
            latest: { release: "26.3", snapshot: "26.4-snapshot-3" },
            versions: ["26.4-snapshot-3", "26.3", "26.2", "1.21.4", "1.21.1", "1.20.1", "1.12.2"].map((id) => ({
              id,
              type: id.includes("snapshot") ? "snapshot" : "release",
              url: "",
              releaseTime: "",
              sha1: "",
            })),
          };
        case "list_loader_versions":
          await new Promise((r) => setTimeout(r, 300));
          return fakeLoaderVersions(args.loader as Loader, args.gameVersion as string);
        case "create_instance": {
          const input = args.instance as NewInstance;
          const created: Instance = {
            id: input.name.toLowerCase().replace(/[^a-z0-9]+/g, "-"),
            name: input.name,
            gameVersion: input.gameVersion,
            loader: input.loader ?? "vanilla",
            loaderVersion: input.loaderVersion ?? null,
            javaPath: null,
            memoryMb: null,
            jvmArgs: null,
            icon: null,
            createdAt: new Date().toISOString(),
            lastPlayedAt: null,
            packProjectId: null,
            packVersionId: null,
            packVersion: null,
          };
          instances.unshift(created);
          return created;
        }
        case "search_content": {
          await new Promise((r) => setTimeout(r, 250));
          const q = (args.query as string).toLowerCase();
          const hits = CATALOGUE[args.kind as ProjectType].filter((h) => h.title.toLowerCase().includes(q));
          return { hits, offset: 0, limit: 20, totalHits: hits.length };
        }
        case "list_content":
          return content[args.instanceId as string] ?? [];
        case "install_content":
          await new Promise((r) => setTimeout(r, 900));
          return fakeInstall(args.instanceId as string, args.projectId as string);
        case "check_content_updates":
          await new Promise((r) => setTimeout(r, 600));
          return (content[args.instanceId as string] ?? [])
            .filter((c) => c.versionId === "old")
            .map((c) => ({ projectId: c.projectId, title: c.title, currentVersion: c.versionNumber, newVersion: "1.2.0+1.21.4" }));
        case "update_content": {
          await new Promise((r) => setTimeout(r, 900));
          const ids = args.projectIds as string[] | null;
          const changed = (content[args.instanceId as string] ?? []).filter(
            (c) => c.versionId === "old" && (!ids || ids.includes(c.projectId)),
          );
          for (const c of changed) Object.assign(c, { versionId: "v2", versionNumber: "1.2.0+1.21.4" });
          return changed.map((c) => ({ ...c }));
        }
        case "set_content_enabled": {
          const item = (content[args.instanceId as string] ?? []).find((c) => c.projectId === args.projectId)!;
          item.enabled = args.enabled as boolean;
          return { ...item };
        }
        case "remove_content":
          content[args.instanceId as string] = (content[args.instanceId as string] ?? []).filter((c) => c.projectId !== args.projectId);
          return null;
        case "install_modpack": {
          const pack = CATALOGUE.modpack.find((h) => h.projectId === args.projectId)!;
          const created: Instance = {
            ...instances[0],
            id: pack.title.toLowerCase().replace(/[^a-z0-9]+/g, "-"),
            name: pack.title,
            gameVersion: "26.2",
            loader: "fabric",
            loaderVersion: "0.19.5",
            icon: pack.iconUrl,
            createdAt: new Date().toISOString(),
            lastPlayedAt: null,
            packProjectId: pack.projectId,
            packVersionId: "v1",
            packVersion: "14.1.0",
          };
          instances.unshift(created);
          await emit("instances://changed");
          const total = 120_000_000;
          for (let i = 0; i <= 20; i++) {
            await new Promise((r) => setTimeout(r, 120));
            await emit("install://progress", {
              instanceId: created.id,
              stage: "downloading",
              doneFiles: Math.round(i * 2.55),
              totalFiles: 51,
              doneBytes: (total * i) / 20,
              totalBytes: total,
            });
          }
          await emit("install://finished", created.id);
          return created;
        }
        case "export_modpack":
          return null;
        case "launch_instance":
          void fakeLaunch(args.id as string);
          return null;
        case "stop_instance":
          await emit("game://exited", { instanceId: args.id, code: null, stopped: true, crashReport: null });
          return true;
        case "add_offline_account": {
          const account: Account = { id: crypto.randomUUID(), kind: "offline", username: args.username as string, mcUuid: "", isActive: true };
          accounts = accounts.map((a) => ({ ...a, isActive: false })).concat(account);
          return account;
        }
        case "set_active_account":
          accounts = accounts.map((a) => ({ ...a, isActive: a.id === args.id }));
          return accounts.find((a) => a.isActive);
        case "remove_account":
          accounts = accounts.filter((a) => a.id !== args.id);
          return null;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
