import { createEffect, createMemo, createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage, type Instance } from "../lib/api";
import Dialog from "./Dialog";

interface Props {
  onClose: () => void;
  onCreated: (instance: Instance) => void;
  /** Version to preselect (e.g. a snapshot from the news card). */
  initialVersion?: string;
}

export default function NewInstanceDialog(props: Props) {
  const [versions] = createResource(api.listVersions);
  const [showSnapshots, setShowSnapshots] = createSignal(false);
  const [name, setName] = createSignal("");
  const [version, setVersion] = createSignal(props.initialVersion ?? "");
  const [error, setError] = createSignal<string | null>(null);
  const [saving, setSaving] = createSignal(false);

  const choices = createMemo(() =>
    (versions()?.versions ?? []).filter(
      (v) => v.type === "release" || ((showSnapshots() || v.id === version()) && v.type === "snapshot"),
    ),
  );

  createEffect(() => {
    const list = versions();
    if (!list) return;
    if (!version()) setVersion(list.latest.release);
    if (list.versions.find((v) => v.id === version())?.type === "snapshot") setShowSnapshots(true);
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      props.onCreated(await api.createInstance(name().trim() || `Minecraft ${version()}`, version()));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog title="Nouvelle instance" onClose={props.onClose}>
      <form onSubmit={submit} class="flex flex-col gap-4">
        <div class="flex flex-col gap-1.5">
          <label class="text-xs text-muted" for="instance-name">
            Nom
          </label>
          <input
            id="instance-name"
            class="field text-sm"
            placeholder={version() ? `Minecraft ${version()}` : "Ma survie"}
            maxLength={64}
            value={name()}
            onInput={(e) => setName(e.currentTarget.value)}
            autofocus
          />
        </div>

        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between">
            <label class="text-xs text-muted" for="instance-version">
              Version
            </label>
            <label class="flex cursor-pointer items-center gap-1.5 text-xs text-muted">
              <input
                type="checkbox"
                class="accent-grass"
                checked={showSnapshots()}
                onChange={(e) => setShowSnapshots(e.currentTarget.checked)}
              />
              Snapshots
            </label>
          </div>
          <Show
            when={!versions.error}
            fallback={<p class="text-sm text-redstone-text">Impossible de charger les versions : {errorMessage(versions.error)}</p>}
          >
            <select
              id="instance-version"
              class="field text-sm"
              value={version()}
              onChange={(e) => setVersion(e.currentTarget.value)}
              disabled={versions.loading}
            >
              <Show when={versions.loading}>
                <option>Chargement…</option>
              </Show>
              <For each={choices()}>
                {(v) => (
                  <option value={v.id}>
                    {v.id}
                    {v.type === "snapshot" ? " (snapshot)" : ""}
                  </option>
                )}
              </For>
            </select>
          </Show>
        </div>

        <Show when={error()}>
          <p class="text-sm text-redstone-text">{error()}</p>
        </Show>

        <div class="flex justify-end gap-2 pt-1">
          <button type="button" class="btn btn-ghost" onClick={props.onClose}>
            Annuler
          </button>
          <button type="submit" class="btn btn-primary px-corners px-5" disabled={!version() || saving()}>
            Créer
          </button>
        </div>
      </form>
    </Dialog>
  );
}
