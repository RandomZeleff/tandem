import { open, save } from "@tauri-apps/plugin-dialog";
import { api, type Instance } from "./api";
import { navigate, refetchInstances } from "./store";

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
