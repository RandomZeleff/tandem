import { createRoot, createSignal } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { api, errorMessage, type ContentUpdate, type InstalledContent } from "./api";

/** Installed content and available updates per instance, shared by the Discover page and the instance tab. */
const state = createRoot(() => {
  const [content, setContent] = createStore<Record<string, InstalledContent[]>>({});
  const [updates, setUpdates] = createStore<Record<string, ContentUpdate[]>>({});
  const [busy, setBusy] = createSignal<Record<string, boolean>>({});
  return { content, setContent, updates, setUpdates, busy, setBusy };
});

/** Busy keys for whole-instance operations. */
const CHECK = "*check";
const UPDATE_ALL = "*update";

const key = (instanceId: string, projectId: string) => `${instanceId}:${projectId}`;

function setBusy(instanceId: string, projectIds: string[], value: boolean) {
  state.setBusy((b) => {
    const next = { ...b };
    for (const id of projectIds) next[key(instanceId, id)] = value;
    return next;
  });
}

/** Runs `action` while `projectIds` show as busy. Resolves to an error message, or null. */
async function track(instanceId: string, projectIds: string[], action: () => Promise<unknown>): Promise<string | null> {
  setBusy(instanceId, projectIds, true);
  try {
    await action();
    return null;
  } catch (err) {
    return errorMessage(err);
  } finally {
    setBusy(instanceId, projectIds, false);
  }
}

function replaceItems(instanceId: string, changed: InstalledContent[]) {
  state.setContent(
    produce((all) => {
      const list = all[instanceId] ?? [];
      for (const item of changed) {
        const index = list.findIndex((c) => c.projectId === item.projectId);
        if (index >= 0) list[index] = item;
        else list.push(item);
      }
      all[instanceId] = list;
    }),
  );
}

/** True once the instance's content has been read at least once. */
export function contentLoaded(instanceId: string): boolean {
  return instanceId in state.content;
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

export function isCheckingUpdates(instanceId: string): boolean {
  return isBusy(instanceId, CHECK);
}

export function isUpdatingAll(instanceId: string): boolean {
  return isBusy(instanceId, UPDATE_ALL);
}

export function contentUpdates(instanceId: string): ContentUpdate[] {
  return state.updates[instanceId] ?? [];
}

export function updateFor(instanceId: string, projectId: string): ContentUpdate | undefined {
  return contentUpdates(instanceId).find((u) => u.projectId === projectId);
}

export async function loadContent(instanceId: string) {
  state.setContent(instanceId, await api.listContent(instanceId));
}

/** Installs a project (and its dependencies), in `versionId` or the best compatible version. */
export function installContent(instanceId: string, projectId: string, versionId?: string): Promise<string | null> {
  return track(instanceId, [projectId], async () => {
    await api.installContent(instanceId, projectId, versionId);
    await loadContent(instanceId);
  });
}

export function removeContent(instanceId: string, projectId: string): Promise<string | null> {
  return track(instanceId, [projectId], async () => {
    await api.removeContent(instanceId, projectId);
    state.setContent(
      produce((all) => {
        all[instanceId] = (all[instanceId] ?? []).filter((c) => c.projectId !== projectId);
      }),
    );
    state.setUpdates(instanceId, (list) => (list ?? []).filter((u) => u.projectId !== projectId));
  });
}

export function setContentEnabled(instanceId: string, projectId: string, enabled: boolean): Promise<string | null> {
  return track(instanceId, [projectId], async () => {
    replaceItems(instanceId, [await api.setContentEnabled(instanceId, projectId, enabled)]);
  });
}

export function checkUpdates(instanceId: string): Promise<string | null> {
  return track(instanceId, [CHECK], async () => {
    state.setUpdates(instanceId, await api.checkContentUpdates(instanceId));
  });
}

/** Updates the given projects, or everything outdated when omitted. */
export function updateContent(instanceId: string, projectIds?: string[]): Promise<string | null> {
  const ids = projectIds ?? contentUpdates(instanceId).map((u) => u.projectId);
  return track(instanceId, projectIds ? ids : [...ids, UPDATE_ALL], async () => {
    const changed = await api.updateContent(instanceId, projectIds);
    replaceItems(instanceId, changed);
    const done = new Set(changed.map((c) => c.projectId));
    state.setUpdates(instanceId, (list) => (list ?? []).filter((u) => !done.has(u.projectId)));
  });
}
