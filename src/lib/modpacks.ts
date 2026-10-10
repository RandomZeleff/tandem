import { open, save } from "@tauri-apps/plugin-dialog";
import { createSignal } from "solid-js";
import { api, errorMessage, type Instance } from "./api";
import { navigate, refetchInstances } from "./store";

/** Modpacks being installed, by project id (shared by Discover and project pages). */
const [installing, setInstalling] = createSignal<ReadonlySet<string>>(new Set());

export const isInstallingPack = (projectId: string) => installing().has(projectId);

function mark(projectId: string, on: boolean) {
  setInstalling((set) => {
    const next = new Set(set);
    if (on) next.add(projectId);
    else next.delete(projectId);
    return next;
  });
}

/** Installs a modpack version into a new instance, then opens it. Resolves to an error message, or null. */
export async function installPack(projectId: string, versionId: string): Promise<string | null> {
  mark(projectId, true);
  try {
    const created = await api.installModpack(projectId, versionId);
    await refetchInstances();
    navigate({ page: "instance", id: created.id });
    return null;
  } catch (err) {
    return errorMessage(err);
  } finally {
    mark(projectId, false);
  }
}

const MRPACK_FILTER = { name: "Modpack Modrinth", extensions: ["mrpack"] };
const PACK_FILTER = { name: "Modpack (Modrinth ou CurseForge)", extensions: ["mrpack", "zip"] };

/** Pending request for the player's CurseForge key; resolves to true once one is saved. */
export const [curseforgeKeyPrompt, setCurseforgeKeyPrompt] = createSignal<((saved: boolean) => void) | null>(null);

function askCurseforgeKey(): Promise<boolean> {
  return new Promise((resolve) => setCurseforgeKeyPrompt(() => resolve));
}

/**
 * Lets the player pick a `.mrpack` or a CurseForge `.zip`, installs it and opens the new
 * instance. A CurseForge pack first asks for the player's key if none is saved. Null if cancelled.
 */
export async function importModpackFile(): Promise<Instance | null> {
  const path = await open({ multiple: false, directory: false, filters: [PACK_FILTER] });
  if (typeof path !== "string") return null;
  if ((await api.modpackKind(path)) === "curseforge" && !(await api.curseforgeKeySaved())) {
    if (!(await askCurseforgeKey())) return null;
  }
  const instance = await api.importModpack(path);
  await refetchInstances();
  navigate({ page: "instance", id: instance.id });
  return instance;
}

/** Asks where to save, then exports the instance. Resolves to the saved path, or null if cancelled. */
export async function exportModpackFile(instance: Instance): Promise<string | null> {
  const path = await save({ defaultPath: `${instance.name}.mrpack`, filters: [MRPACK_FILTER] });
  if (!path) return null;
  await api.exportModpack(instance.id, path, instance.packVersion ?? "1.0.0");
  return path;
}
