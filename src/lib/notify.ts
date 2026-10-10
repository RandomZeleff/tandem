import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";

/** A system notification, only while the launcher is not in front (the player is in game). */
export async function notifyInGame(title: string, body: string) {
  if (document.hasFocus()) return;
  try {
    const granted = (await isPermissionGranted()) || (await requestPermission()) === "granted";
    if (granted) sendNotification({ title, body });
  } catch {
    // Outside Tauri (UI preview): the toasts are enough.
  }
}
