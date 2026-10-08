import { createRoot, createSignal } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { api, errorMessage, type InstalledContent } from "./api";

/** Installed content per instance, shared by the Discover page and the instance tab. */
const state = createRoot(() => {
  const [content, setContent] = createStore<Record<string, InstalledContent[]>>({});
  const [busy, setBusy] = createSignal<Record<string, boolean>>({});
  return { content, setContent, busy, setBusy };
});

const key = (instanceId: string, projectId: string) => `${instanceId}:${projectId}`;

function setBusy(instanceId: string, projectId: string, value: boolean) {
  state.setBusy((b) => ({ ...b, [key(instanceId, projectId)]: value }));
}

export function installedContent(instanceId: string): InstalledContent[] {
  return state.content[instanceId] ?? [];
}

export function isInstalled(instanceId: string, projectId: string): boolean {
  return installedContent(instanceId).some((c) => c.projectId === projectId);
}

export function isBusy(instanceId: string, projectId: string): boolean {
  return !!state.busy()[key(instanceId, projectId)];
}

export async function loadContent(instanceId: string) {
  state.setContent(instanceId, await api.listContent(instanceId));
}

/** Installs a project (and its dependencies). Resolves to an error message, or null. */
export async function installContent(instanceId: string, projectId: string): Promise<string | null> {
  setBusy(instanceId, projectId, true);
  try {
    await api.installContent(instanceId, projectId);
    await loadContent(instanceId);
    return null;
  } catch (err) {
    return errorMessage(err);
  } finally {
    setBusy(instanceId, projectId, false);
  }
}

export async function removeContent(instanceId: string, projectId: string): Promise<string | null> {
  setBusy(instanceId, projectId, true);
  try {
    await api.removeContent(instanceId, projectId);
    state.setContent(
      produce((all) => {
        all[instanceId] = (all[instanceId] ?? []).filter((c) => c.projectId !== projectId);
      }),
    );
    return null;
  } catch (err) {
    return errorMessage(err);
  } finally {
    setBusy(instanceId, projectId, false);
  }
}
