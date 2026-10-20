import { listen } from "@tauri-apps/api/event";
import { createSignal } from "solid-js";
import { api, errorMessage, type UpdateInfo } from "./api";

/** Waits a little after startup: the first screen and the instances come first. */
const STARTUP_CHECK_DELAY_MS = 5000;

const [update, setUpdate] = createSignal<UpdateInfo | null>(null);
const [updateDialog, setUpdateDialog] = createSignal(false);
/** Bytes downloaded so far, while installing. */
const [updateProgress, setUpdateProgress] = createSignal<{ downloaded: number; total: number | null } | null>(null);
export { update, updateDialog, setUpdateDialog, updateProgress };

/** Checks quietly once at startup: no network, no message. */
export function checkForUpdateLater() {
  setTimeout(() => void checkForUpdate().catch(() => {}), STARTUP_CHECK_DELAY_MS);
}

/** Asks for a newer Tandem. Throws the message to show when the check fails. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  const found = await api.checkUpdate();
  setUpdate(found);
  return found;
}

/** Downloads and installs the update; Tandem restarts by itself when it succeeds. */
export async function installUpdate(): Promise<string | null> {
  setUpdateProgress({ downloaded: 0, total: null });
  const stop = await listen<{ downloaded: number; total: number | null }>("update://progress", ({ payload }) =>
    setUpdateProgress(payload),
  );
  try {
    await api.installUpdate();
    return null;
  } catch (err) {
    setUpdateProgress(null);
    return errorMessage(err);
  } finally {
    stop();
  }
}
