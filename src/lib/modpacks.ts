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

/** Lets the player pick a `.mrpack`, installs it and opens the new instance. Null if cancelled. */
export async function importModpackFile(): Promise<Instance | null> {
  const path = await open({ multiple: false, directory: false, filters: [MRPACK_FILTER] });
  if (typeof path !== "string") return null;
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
