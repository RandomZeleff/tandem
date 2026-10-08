/**
 * Browser-only preview: fakes the Tauri backend so the UI can be designed in a
 * regular browser (`pnpm dev`, then open http://localhost:1420). Never bundled
 * in the desktop app: main.tsx only imports it in dev outside Tauri.
 */
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Account, Instance, LogEntry } from "../lib/api";

const now = Date.now();
const iso = (hoursAgo: number) => new Date(now - hoursAgo * 3_600_000).toISOString();

const instances: Instance[] = [
  ["survie-avec-leo", "Survie avec Léo", "26.3", "fabric", iso(20)],
  ["pack-create", "Pack Create", "1.21.1", "neoforge", iso(70)],
  ["crea-redstone", "Créa redstone", "26.3", "vanilla", iso(100)],
  ["nostalgie-1-12", "Nostalgie 1.12", "1.12.2", "vanilla", iso(500)],
].map(([id, name, gameVersion, loader, lastPlayedAt]) => ({
  id,
  name,
  gameVersion,
  loader,
  loaderVersion: null,
  javaPath: null,
  memoryMb: null,
  jvmArgs: null,
  icon: null,
  createdAt: iso(900),
  lastPlayedAt,
}));

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
