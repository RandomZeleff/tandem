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
  Datapack,
  InstalledContent,
  Instance,
  InstanceSettings,
  Loader,
  LogEntry,
  NewInstance,
  ProjectType,
  Screenshot,
  SearchHit,
  World,
  WorldBackup,
} from "../lib/api";
import { projectMocks } from "./mockProjects";
import { translationMocks } from "./mockTranslations";

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
  windowWidth: null as number | null,
  windowHeight: null as number | null,
  block: null as number | null,
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
      ["gvQqBUqZ", "Lithium", "jellysquid3", "No-compromises game logic optimization mod.", 64_000_000, ["optimization"]],
      ["uXXizFIs", "FerriteCore", "malte0811", "Memory usage optimizations.", 60_000_000, ["optimization"]],
      ["LQ3K71Q1", "Dynamic FPS", "juliand665", "Dynamically adjust FPS so Minecraft doesn't hog your computer.", 30_000_000, ["optimization"]],
    ] as Row[]
  ).map(hit),
  resourcepack: ([["Bq0hLR4Y", "Faithful 32x", "Faithful", "Faithful to the original textures, at twice the resolution.", 4_200_000, ["32x", "vanilla-like"]]] as Row[]).map(hit),
  datapack: (
    [
      ["8oi3bsk5", "Terralith", "Stardust Labs", "Explore almost 100 new biomes consisting of both realism and light fantasy.", 9_000_000, ["datapack", "worldgen"]],
      ["tpehi7ww", "Dungeons and Taverns", "nova_wostra", "Adds dungeons, taverns and other structures to your world.", 5_000_000, ["datapack", "adventure"]],
    ] as Row[]
  ).map(hit),
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
  // A big modpack, to keep long lists honest.
  "pack-create": Array.from({ length: 250 }, (_, i) => {
    const h = CATALOGUE.mod[i % CATALOGUE.mod.length];
    return { ...make({ ...h, projectId: `${h.projectId}-${i}`, title: `${h.title} ${i + 1}` }, "mod", i % 3 === 0), enabled: i % 17 !== 0 };
  }),
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

const settings: Record<string, unknown> = {};

const ICON = "data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 4" shape-rendering="crispEdges"><rect width="4" height="4" fill="#5DBB3F"/><rect y="2" width="4" height="2" fill="#7A5420"/></svg>');
const worlds: Record<string, World[]> = {
  "survie-avec-leo": [
    { folder: "Survie", name: "Survie avec Léo", lastPlayed: now - 20 * 3_600_000, gameMode: "survival", version: "26.3", sizeBytes: 182_000_000, icon: ICON },
    { folder: "Test redstone", name: "Test redstone", lastPlayed: now - 9 * 86_400_000, gameMode: "creative", version: "26.3", sizeBytes: 12_400_000, icon: null },
  ],
};
const worldBackups: Record<string, WorldBackup[]> = {
  "survie-avec-leo": [
    { world: "Survie", fileName: "1-auto.zip", createdAt: now - 20 * 3_600_000, kind: "auto", sizeBytes: 96_000_000 },
    { world: "Survie", fileName: "2-manual.zip", createdAt: now - 4 * 86_400_000, kind: "manual", sizeBytes: 91_000_000 },
  ],
};

const SKIES = ["#3E6FB0", "#E58B4B", "#1B2440", "#6FA8DC", "#8E5BB5"];
const screenshots: Record<string, Screenshot[]> = {
  "survie-avec-leo": Array.from({ length: 9 }, (_, i) => {
    const fileName = `2026-10-0${9 - i}_18.2${i}.14.png`;
    return {
      fileName,
      path: `C:\\Users\\you\\AppData\\Roaming\\Tandem\\instances\\survie-avec-leo\\screenshots\\${fileName}`,
      takenAt: now - i * 31 * 3_600_000,
      sizeBytes: 2_400_000 + i * 180_000,
    };
  }),
};

/** Fake landscape per screenshot; `#` keeps the URL's `?v=…&thumb` out of the SVG. */
function fakeScreenshot(path: string): string {
  const seed = [...path].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
  const hills = Array.from({ length: 8 }, (_, i) => `<rect x="${i * 20}" y="${50 + ((seed >> i) % 14)}" width="20" height="40" fill="#5DBB3F"/>`).join("");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90" shape-rendering="crispEdges"><rect width="160" height="90" fill="${SKIES[seed % SKIES.length]}"/><rect x="${20 + (seed % 100)}" y="12" width="12" height="12" fill="#F2C744"/>${hills}<rect y="72" width="160" height="18" fill="#7A5420"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}#`;
}

/** Datapacks by `instance/world`. */
const datapacks: Record<string, Datapack[]> = {
  "survie-avec-leo/Survie": [
    { fileName: "custom-recipes", title: "custom-recipes", description: "Recettes maison", projectId: null, versionNumber: null, iconUrl: null, sizeBytes: 0 },
  ],
};

/** Pack version each updated instance can go back to. */
const rollbackFrom: Record<string, string> = {};

/** A face drawn like a skin's, for Microsoft accounts of the mock. */
const MOCK_FACE = `data:image/svg+xml,${encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8" shape-rendering="crispEdges"><rect width="8" height="8" fill="#c58c5f"/><path d="M0 0h8v2H0zM0 2h1v2H0zM7 2h1v2H7z" fill="#3b2213"/><path d="M1 4h2v1H1zM5 4h2v1H5z" fill="#fff"/><path d="M2 4h1v1H2zM5 4h1v1H5z" fill="#2b5fb5"/><path d="M3 6h2v1H3z" fill="#7a4630"/></svg>',
)}`;

let accounts: Account[] = [
  { id: "a1", kind: "offline", username: "Zeleff", mcUuid: "00000000-0000-3000-8000-000000000000", isActive: true, avatar: null },
];

const logs: LogEntry[] = [
  { seq: 1, timestampMs: now - 5000, level: "INFO", target: "tandem_lib", message: "Tandem started (aperçu navigateur)" },
  { seq: 2, timestampMs: now - 4000, level: "DEBUG", target: "tandem_core::db", message: "database ready" },
];

const statsTimers: Record<string, ReturnType<typeof setInterval>> = {};

/** The 1.12.2 instance asks for Rosetta until it is "installed" once. */
let rosettaInstalled = false;

/** CurseForge import: saved key, and files waiting to be downloaded by hand per instance. */
let curseforgeKey = false;
const manualFiles: Record<string, { name: string; fileName: string; sha1: string | null; size: number; url: string; folder: string }[]> = {};
/** Pages opened from the manual downloads panel: as many files "appear" in Downloads. */
let downloadedByHand = 0;
window.addEventListener("mock:manual-download", () => downloadedByHand++);

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
  let memory = 900e6;
  statsTimers[id] = setInterval(() => {
    memory = Math.min(3.2e9, memory + Math.random() * 120e6);
    void emit("game://stats", { instanceId: id, memoryBytes: memory, cpuPercent: 15 + Math.random() * 30 });
  }, 2000);
  // "Survie avec Léo" crashes shortly after starting, blaming Sodium.
  if (id === "survie-avec-leo") {
    setTimeout(() => {
      clearInterval(statsTimers[id]);
      void emit("game://exited", {
        instanceId: id,
        code: 1,
        stopped: false,
        crashReport: "/mock/crash-reports/crash-client.txt",
        analysis: {
          description: "Rendering overlay",
          exception: 'java.lang.NullPointerException: Cannot invoke "Object.hashCode()" because "key" is null',
          suspects: [{ name: "Sodium", fileName: "AANobbMI.jar", reason: "stackTrace" }],
          hint: null,
          source: "/mock/crash-reports/crash-client.txt",
        },
      });
    }, 5000);
  }
  await emit("game://output", {
    instanceId: id,
    stream: "stdout",
    lines: ["[Render thread/INFO]: Setting user: Zeleff", "[Render thread/INFO]: Backend library: LWJGL", "[Render thread/INFO]: Sound engine started"],
  });
}

export function installMocks() {
  mockWindows("main");
  const w = window as unknown as { __TAURI_INTERNALS__?: Record<string, unknown> };
  (w.__TAURI_INTERNALS__ ??= {}).convertFileSrc = fakeScreenshot;
  const projects = projectMocks(CATALOGUE, instances);
  const translations = translationMocks();
  mockIPC(
    async (cmd, payload) => {
      const args = (payload ?? {}) as Record<string, unknown>;
      const project = (await projects(cmd, args)) ?? (await translations(cmd, args));
      if (project !== undefined) return project;
      switch (cmd) {
        case "app_info":
          return { version: "0.1.0", dataDir: "C:\\Users\\you\\AppData\\Roaming\\Tandem" };
        case "get_logs":
          return logs;
        case "list_instances":
          // Fresh objects, like the real backend: the UI sees changes.
          return instances.map((i) => ({ ...i }));
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
        case "delete_instance": {
          const index = instances.findIndex((i) => i.id === args.id);
          if (index >= 0) instances.splice(index, 1);
          return null;
        }
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
            windowWidth: null,
            windowHeight: null,
            block: null,
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
          return [...(content[args.instanceId as string] ?? [])];
        case "install_content":
          await new Promise((r) => setTimeout(r, 900));
          return fakeInstall(args.instanceId as string, args.projectId as string);
        case "perf_suggestions": {
          await new Promise((r) => setTimeout(r, 400));
          const instance = instances.find((i) => i.id === args.instanceId);
          if (!instance || instance.loader === "vanilla") return [];
          const installed = new Set((content[instance.id] ?? []).map((c) => c.projectId));
          return CATALOGUE.mod
            .filter((h) => ["AANobbMI", "gvQqBUqZ", "uXXizFIs", "LQ3K71Q1"].includes(h.projectId) && !installed.has(h.projectId))
            .map((h) => ({ projectId: h.projectId, title: h.title, iconUrl: h.iconUrl }));
        }
        case "list_worlds":
          return [...(worlds[args.instanceId as string] ?? [])];
        case "list_world_backups":
          return [...(worldBackups[args.instanceId as string] ?? [])].sort((a, b) => b.createdAt - a.createdAt);
        case "backup_world": {
          await new Promise((r) => setTimeout(r, 1200));
          const backup: WorldBackup = { world: args.world as string, fileName: `${Date.now()}-manual.zip`, createdAt: Date.now(), kind: "manual", sizeBytes: 97_000_000 };
          (worldBackups[args.instanceId as string] ??= []).push(backup);
          return backup;
        }
        case "restore_world_backup": {
          await new Promise((r) => setTimeout(r, 1200));
          const before: WorldBackup = { world: args.world as string, fileName: `${Date.now()}-before-restore.zip`, createdAt: Date.now(), kind: "beforeRestore", sizeBytes: 97_000_000 };
          (worldBackups[args.instanceId as string] ??= []).push(before);
          return null;
        }
        case "list_screenshots":
          return [...(screenshots[args.instanceId as string] ?? [])];
        case "delete_screenshot": {
          const list = screenshots[args.instanceId as string] ?? [];
          screenshots[args.instanceId as string] = list.filter((s) => s.fileName !== args.fileName);
          return null;
        }
        case "get_setting":
          return settings[args.key as string] ?? null;
        case "set_setting":
          settings[args.key as string] = args.value;
          return null;
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
        case "warm_mod_dependencies":
          return null;
        // The first mod of an instance is a library the others need.
        case "content_dependents": {
          const list = content[args.instanceId as string] ?? [];
          if (list[0]?.projectId !== args.projectId) return [];
          return list.slice(1).filter((c) => c.kind === "mod" && c.enabled).map((c) => ({ name: c.title, fileName: c.fileName }));
        }
        case "content_conflicts": {
          const list = (content[args.instanceId as string] ?? []).filter((c) => c.kind === "mod" && c.enabled);
          const a = list.find((c) => c.projectId === "AANobbMI");
          const b = list.find((c) => c.projectId === "mOgUt4GM");
          return a && b ? [{ name: a.title, fileName: a.fileName, otherName: b.title, otherFileName: b.fileName }] : [];
        }
        case "mod_providers":
          return (args.modIds as string[]).flatMap((id) => {
            const item = (content[args.instanceId as string] ?? []).find((c) => c.title.toLowerCase().startsWith(id));
            return item ? [{ modId: id, name: item.title, fileName: item.fileName, enabled: item.enabled }] : [];
          });
        case "set_content_enabled": {
          const item = (content[args.instanceId as string] ?? []).find((c) => c.projectId === args.projectId)!;
          item.enabled = args.enabled as boolean;
          return { ...item };
        }
        case "remove_content":
          content[args.instanceId as string] = (content[args.instanceId as string] ?? []).filter((c) => c.projectId !== args.projectId);
          return null;
        case "update_instance_settings": {
          const instance = instances.find((i) => i.id === args.id)!;
          const settings = args.settings as InstanceSettings;
          if (settings.jvmArgs?.includes("-Xmx")) throw "La mémoire se règle avec le curseur, pas dans les arguments (-Xmx, -Xms)";
          Object.assign(instance, settings);
          return { ...instance };
        }
        case "set_instance_icon": {
          const instance = instances.find((i) => i.id === args.id)!;
          Object.assign(instance, { icon: args.image ? ICON : null, block: args.block ?? null });
          return { ...instance };
        }
        case "duplicate_instance": {
          await new Promise((r) => setTimeout(r, 900));
          const source = instances.find((i) => i.id === args.id)!;
          const copy = { ...source, id: (args.name as string).toLowerCase().replace(/[^a-z0-9]+/g, "-"), name: args.name as string, lastPlayedAt: null };
          instances.unshift(copy);
          content[copy.id] = (content[source.id] ?? []).map((c) => ({ ...c }));
          return copy;
        }
        case "list_datapacks":
          await new Promise((r) => setTimeout(r, 200));
          return [...(datapacks[`${args.instanceId}/${args.world}`] ?? [])];
        case "install_datapack": {
          await new Promise((r) => setTimeout(r, 700));
          const h = CATALOGUE.datapack.find((d) => d.projectId === args.projectId)!;
          const pack = { fileName: `${h.slug}.zip`, title: h.title, description: h.description, projectId: h.projectId, versionNumber: "2.5.8", iconUrl: h.iconUrl, sizeBytes: 1_400_000 };
          (datapacks[`${args.instanceId}/${args.world}`] ??= []).push(pack);
          return pack;
        }
        case "remove_datapack": {
          const key = `${args.instanceId}/${args.world}`;
          datapacks[key] = (datapacks[key] ?? []).filter((p) => p.fileName !== args.fileName);
          return null;
        }
        case "plan_version_change": {
          await new Promise((r) => setTimeout(r, 500));
          const target = args.target as { gameVersion: string; loader: Loader };
          const instance = instances.find((i) => i.id === args.id)!;
          const mods = (content[instance.id] ?? []).filter((c) => c.kind === "mod");
          const sameFamily = target.loader === instance.loader || (target.loader === "quilt" && instance.loader === "fabric");
          return {
            loaderVersion: target.loader === "vanilla" ? null : "0.19.5",
            kept: sameFamily ? 1 : 0,
            updated: sameFamily ? mods.slice(1).map((m) => ({ title: m.title, from: m.versionNumber, to: `2.0.0+${target.gameVersion}` })) : [],
            unavailable: sameFamily ? [] : mods.map((m) => m.title),
            unknown: ["handmade-mod-1.0.jar"],
            gameVersionChanges: target.gameVersion !== instance.gameVersion,
            leavesModpack: !!instance.packProjectId,
            worldsBackedUp: 0,
          };
        }
        case "change_instance_version": {
          await new Promise((r) => setTimeout(r, 900));
          const target = args.target as { gameVersion: string; loader: Loader; loaderVersion: string | null };
          const instance = instances.find((i) => i.id === args.id)!;
          Object.assign(instance, { gameVersion: target.gameVersion, loader: target.loader, loaderVersion: target.loaderVersion, packProjectId: null, packVersionId: null, packVersion: null });
          return { loaderVersion: target.loaderVersion, kept: 1, updated: [{ title: "Sodium", from: "1.0", to: "2.0" }], unavailable: [], unknown: [], gameVersionChanges: true, leavesModpack: false, worldsBackedUp: 2 };
        }
        case "instance_java":
          await new Promise((r) => setTimeout(r, 200));
          return {
            required: 21,
            installs: [
              { path: "C:\\Program Files\\Java\\jdk-21.0.10\\bin\\java.exe", version: "21.0.10", major: 21, vendor: "Oracle Corporation", managed: false },
              { path: "C:\\Users\\you\\AppData\\Roaming\\Tandem\\java\\java-runtime-delta\\bin\\java.exe", version: "21.0.7", major: 21, vendor: "Microsoft", managed: true },
              { path: "C:\\Program Files\\Java\\jre1.8.0_503\\bin\\java.exe", version: "1.8.0_503", major: 8, vendor: null, managed: false },
            ],
          };
        case "modpack_updates": {
          const instance = instances.find((i) => i.id === args.instanceId);
          if (!instance?.packProjectId || instance.packVersion !== "14.1.0") return [];
          return [
            { id: "fo15", versionNumber: "15.0.0", versionType: "release", gameVersions: ["26.3"], loaders: ["fabric"], datePublished: iso(30), size: 30_000_000, recommended: true },
            { id: "fo142", versionNumber: "14.2.0", versionType: "release", gameVersions: ["26.2"], loaders: ["fabric"], datePublished: iso(300), size: 29_000_000, recommended: false },
          ];
        }
        case "update_modpack": {
          const instance = instances.find((i) => i.id === args.instanceId)!;
          for (let i = 0; i <= 10; i++) {
            await new Promise((r) => setTimeout(r, 120));
            await emit("install://progress", { instanceId: instance.id, stage: "downloading", doneFiles: i * 3, totalFiles: 30, doneBytes: i * 3_000_000, totalBytes: 30_000_000 });
          }
          await emit("install://finished", instance.id);
          const to = args.versionId === "fo15" ? "15.0.0" : "14.2.0";
          rollbackFrom[instance.id] = instance.packVersion!;
          Object.assign(instance, { packVersion: to, packVersionId: args.versionId, gameVersion: to === "15.0.0" ? "26.3" : "26.2" });
          return { added: 12, replaced: 31, removed: 4, kept: ["config/sodium-options.json"], gameVersion: null, worldsBackedUp: 0 };
        }
        case "modpack_rollback_version":
          return rollbackFrom[args.instanceId as string] ?? null;
        case "rollback_modpack": {
          const instance = instances.find((i) => i.id === args.instanceId)!;
          Object.assign(instance, { packVersion: rollbackFrom[instance.id], packVersionId: "v1", gameVersion: "26.2" });
          delete rollbackFrom[instance.id];
          return null;
        }
        case "modpack_versions":
          await new Promise((r) => setTimeout(r, 300));
          return [
            { id: "v6a", versionNumber: "6.0.0-alpha", versionType: "alpha", gameVersions: ["1.21.1"], loaders: ["neoforge"], datePublished: "2026-06-15T00:00:00Z", size: 31_000_000, recommended: false },
            { id: "v52", versionNumber: "5.2.1b", versionType: "release", gameVersions: ["1.19.2"], loaders: ["forge"], datePublished: "2025-08-27T00:00:00Z", size: 27_209_112, recommended: true },
            { id: "v51", versionNumber: "5.1.0c", versionType: "beta", gameVersions: ["1.19.2"], loaders: ["forge"], datePublished: "2024-05-11T00:00:00Z", size: 26_000_000, recommended: false },
          ];
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
        case "plugin:dialog|open": {
          if ((args.options as { directory?: boolean })?.directory) return "C:/Users/mock/PrismLauncher/instances/Better MC";
          const filters = ((args.options as { filters?: { extensions: string[] }[] })?.filters ?? []).flatMap((f) => f.extensions);
          return filters.includes("zip") ? "C:/Users/mock/Downloads/All the Mods 9-0.2.60.zip" : null;
        }
        case "modpack_kind":
          return String(args.path).endsWith(".zip") ? "curseforge" : "modrinth";
        case "curseforge_key_saved":
          return curseforgeKey;
        case "set_curseforge_key": {
          await new Promise((r) => setTimeout(r, 500));
          const key = String(args.key);
          if (key && !key.startsWith("$2a$")) throw "Clé CurseForge refusée : vérifie qu'elle est complète";
          curseforgeKey = key !== "";
          return null;
        }
        case "import_modpack": {
          if (String(args.path).endsWith(".zip") && !curseforgeKey) throw "curseforge-key-missing";
          const created: Instance = {
            ...instances[0],
            id: "all-the-mods-9",
            name: "All the Mods 9",
            gameVersion: "1.20.1",
            loader: "forge",
            loaderVersion: "47.2.0",
            icon: null,
            createdAt: new Date().toISOString(),
            lastPlayedAt: null,
            packProjectId: null,
            packVersionId: null,
            packVersion: null,
          };
          instances.unshift(created);
          await emit("instances://changed");
          for (let i = 0; i <= 10; i++) {
            await new Promise((r) => setTimeout(r, 120));
            await emit("install://progress", { instanceId: created.id, stage: "downloading", doneFiles: i * 40, totalFiles: 400, doneBytes: i * 80e6, totalBytes: 800e6 });
          }
          manualFiles[created.id] = [
            { name: "Sophisticated Backpacks", fileName: "sophisticatedbackpacks-1.20.1-3.20.2.1035.jar", sha1: "a", size: 1_402_311, url: "https://www.curseforge.com/minecraft/mc-mods/sophisticated-backpacks/files/5328571", folder: "mods" },
            { name: "Mekanism", fileName: "Mekanism-1.20.1-10.4.5.19.jar", sha1: "b", size: 10_812_004, url: "https://www.curseforge.com/minecraft/mc-mods/mekanism/files/5134493", folder: "mods" },
            { name: "Stoneholm", fileName: "Stoneholm-1.20.1-forge-1.4.10.jar", sha1: "c", size: 98_210, url: "https://www.curseforge.com/minecraft/mc-mods/stoneholm/files/4993040", folder: "mods" },
          ];
          await emit("install://finished", created.id);
          return created;
        }
        case "scan_other_launchers":
          await new Promise((r) => setTimeout(r, 400));
          return [
            { source: "modrinthApp", name: "Prominence™ II: Hasturian Era", gameDir: "C:/Users/mock/AppData/Roaming/ModrinthApp/profiles/Prominence II", gameVersion: "1.20.1", loader: "fabric", loaderVersion: "0.19.3", packProjectId: "EGs3lC8D", packVersionId: "3uZ1MN34", icon: null, mods: 444, worlds: 1, lastPlayed: Date.now() - 3 * 86_400_000 },
            { source: "curseForge", name: "DawnCraft - Echoes of Legends", gameDir: "C:/Users/mock/curseforge/minecraft/Instances/DawnCraft", gameVersion: "1.18.2", loader: "forge", loaderVersion: "40.2.17", packProjectId: null, packVersionId: null, icon: null, mods: 298, worlds: 1, lastPlayed: Date.now() - 40 * 86_400_000 },
            { source: "official", name: "Minecraft (dernière version)", gameDir: "C:/Users/mock/AppData/Roaming/.minecraft", gameVersion: "latest-release", loader: "vanilla", loaderVersion: null, packProjectId: null, packVersionId: null, icon: null, mods: 0, worlds: 3, lastPlayed: null },
          ];
        case "inspect_launcher_folder":
          await new Promise((r) => setTimeout(r, 300));
          return [
            { source: "prism", name: "Better MC", gameDir: `${args.path}/.minecraft`, gameVersion: "1.20.1", loader: "forge", loaderVersion: "47.3.0", packProjectId: null, packVersionId: null, icon: null, mods: 212, worlds: 2, lastPlayed: Date.now() - 86_400_000 },
          ];
        case "import_from_launcher": {
          const found = args.found as { name: string; gameVersion: string; loader: Instance["loader"]; loaderVersion: string | null };
          const created: Instance = {
            ...instances[0],
            id: found.name.toLowerCase().replace(/[^a-z0-9]+/g, "-"),
            name: found.name,
            gameVersion: found.gameVersion.startsWith("latest") ? "26.3" : found.gameVersion,
            loader: found.loader,
            loaderVersion: found.loaderVersion,
            icon: null,
            createdAt: new Date().toISOString(),
            lastPlayedAt: null,
            packProjectId: null,
            packVersionId: null,
            packVersion: null,
          };
          instances.unshift(created);
          await emit("instances://changed");
          for (let i = 0; i <= 20; i++) {
            await new Promise((r) => setTimeout(r, 150));
            await emit("install://progress", { instanceId: created.id, stage: "copying", doneFiles: i * 265, totalFiles: 5300, doneBytes: i * 56e6, totalBytes: 1120e6 });
          }
          await emit("install://finished", created.id);
          return created;
        }
        case "list_local_content":
          return args.instanceId === "pack-create"
            ? [
                { kind: "mod", fileName: "Mekanism-1.21.1-10.7.7.jar", name: "Mekanism", enabled: true, size: 10_812_004 },
                { kind: "mod", fileName: "my-tweaks.jar", name: "My Tweaks", enabled: false, size: 21_430 },
                { kind: "resourcepack", fileName: "Create Dark Mode", name: "Create Dark Mode", enabled: true, size: 0 },
              ]
            : [];
        case "set_local_content_enabled":
        case "remove_local_content":
          return null;
        case "begin_microsoft_login":
          await new Promise((r) => setTimeout(r, 300));
          return "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize?client_id=mock";
        case "finish_microsoft_login": {
          await new Promise((r) => setTimeout(r, 2500));
          for (const step of ["microsoft", "xbox", "minecraft", "profile"]) {
            await emit("auth://step", step);
            await new Promise((r) => setTimeout(r, 700));
          }
          if ((window as { __mockLoginFails?: boolean }).__mockLoginFails)
            throw "Ce compte Microsoft n'a pas encore de profil Xbox. Crée-le gratuitement sur https://www.xbox.com/live puis reconnecte-toi.";
          const account: Account = { id: "ms-1", kind: "microsoft", username: "Zeleff", mcUuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5", isActive: true, avatar: MOCK_FACE };
          accounts = accounts.filter((a) => a.id !== "ms-1").map((a) => ({ ...a, isActive: false })).concat(account);
          return account;
        }
        case "cancel_microsoft_login":
          return null;
        case "duo_host": {
          await new Promise((r) => setTimeout(r, 900));
          const instanceId = args.instanceId as string | null;
          const link = (pingMs: number, direct: boolean) => ({ pingMs, direct });
          setTimeout(() => void emit("duo://host", { type: "world", world: { motd: "Zeleff - Survie", port: 51234 } }), 4000);
          setTimeout(
            () =>
              void emit("duo://host", {
                type: "guestJoined",
                guest: {
                  id: "57460a3016",
                  player: "Léo",
                  instance: null,
                  diff: { sameGame: true, sameLoader: true, missing: [{ fileName: "sodium.jar", sha1: "a" }, { fileName: "iris.jar", sha1: "b" }], extra: [] },
                },
              }),
            7000,
          );
          let pings = 0;
          const timer = setInterval(() => {
            if (pings++ > 60) clearInterval(timer);
            void emit("duo://host", { type: "link", id: "57460a3016", link: pings < 3 ? link(171, false) : link(23 + (pings % 5), true) });
          }, 2000);
          return { code: "7K2P-QX9M", instanceId, world: null };
        }
        case "duo_stop_host":
        case "duo_kick":
        case "duo_leave":
          return null;
        case "duo_join": {
          await new Promise((r) => setTimeout(r, 1500));
          if (String(args.code).startsWith("0000")) throw "Jeu à deux : aucune partie avec ce code. Vérifie-le, ou demande à l'hôte s'il a bien lancé l'invitation.";
          setTimeout(() => void emit("duo://guest", { type: "world", world: { motd: "Léo - Monde créatif" } }), 3000);
          const timer = setInterval(() => void emit("duo://guest", { type: "link", link: { pingMs: 31, direct: true } }), 2000);
          setTimeout(() => clearInterval(timer), 120_000);
          return {
            hostPlayer: "Léo",
            hostInstance: null,
            world: null,
            port: 51999,
            instanceId: args.instanceId,
            diff: { sameGame: false, sameLoader: true, missing: [], extra: [{ fileName: "x.jar", sha1: "x" }] },
            link: null,
          };
        }
        case "duo_play":
          void fakeLaunch(instances[0].id);
          return null;
        case "duo_state":
          return { host: null, guest: null };
        case "manual_downloads":
          return manualFiles[args.instanceId as string] ?? [];
        case "collect_manual_downloads": {
          const list = manualFiles[args.instanceId as string] ?? [];
          list.splice(0, Math.min(downloadedByHand, list.length));
          downloadedByHand = 0;
          return [...list];
        }
        case "export_modpack":
          return null;
        case "memory_info": {
          const mods = content[args.id as string]?.filter((c) => c.kind === "mod").length ?? 0;
          return { autoMb: mods === 0 ? 2048 : mods <= 50 ? 4096 : 6144, totalMb: 16384 };
        }
        case "set_instance_memory": {
          const instance = instances.find((i) => i.id === args.id)!;
          instance.memoryMb = args.memoryMb as number | null;
          return null;
        }
        case "launch_instance":
          // Offline names only work once a Microsoft account is there (D36).
          if (!accounts.some((a) => a.kind === "microsoft")) throw "account-needed";
          if (!rosettaInstalled && instances.find((i) => i.id === args.id)?.gameVersion === "1.12.2") {
            throw "rosetta-missing";
          }
          void fakeLaunch(args.id as string);
          return null;
        case "install_rosetta":
          await new Promise((r) => setTimeout(r, 1500));
          rosettaInstalled = true;
          return true;
        case "stop_instance":
          clearInterval(statsTimers[args.id as string]);
          await emit("game://exited", { instanceId: args.id, code: null, stopped: true, crashReport: null, analysis: null });
          return true;
        case "add_offline_account": {
          const account: Account = { id: crypto.randomUUID(), kind: "offline", username: args.username as string, mcUuid: "", isActive: true, avatar: null };
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
