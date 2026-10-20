import { createSignal } from "solid-js";
import { api } from "./api";

/** Set once the welcome steps were completed or skipped. */
export const ONBOARDING_SETTING = "onboarding_done";

const [firstRun, setFirstRun] = createSignal(false);
export { firstRun };

/**
 * Shows the welcome steps on a fresh install only: someone who already has instances or
 * accounts (an update from an older Tandem) never sees them.
 */
export async function checkFirstRun() {
  if (await api.getSetting<boolean>(ONBOARDING_SETTING)) return;
  const [instances, accounts] = await Promise.all([api.listInstances(), api.listAccounts()]);
  if (instances.length > 0 || accounts.length > 0) {
    await api.setSetting(ONBOARDING_SETTING, true);
    return;
  }
  setFirstRun(true);
}

export async function finishOnboarding() {
  setFirstRun(false);
  await api.setSetting(ONBOARDING_SETTING, true).catch(() => {});
}
