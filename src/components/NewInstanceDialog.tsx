import { createEffect, createMemo, createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage, type Instance } from "../lib/api";

interface Props {
  onClose: () => void;
  onCreated: (instance: Instance) => void;
}

export default function NewInstanceDialog(props: Props) {
  const [versions] = createResource(api.listVersions);
  const [showSnapshots, setShowSnapshots] = createSignal(false);
  const [name, setName] = createSignal("");
  const [version, setVersion] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);
  const [saving, setSaving] = createSignal(false);

  const choices = createMemo(() =>
    (versions()?.versions ?? []).filter(
      (v) => v.type === "release" || (showSnapshots() && v.type === "snapshot"),
    ),
  );

  // Preselect the latest release once the list arrives.
  createEffect(() => {
    const latest = versions()?.latest.release;
    if (latest && !version()) setVersion(latest);
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const finalName = name().trim() || `Minecraft ${version()}`;
      props.onCreated(await api.createInstance(finalName, version()));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div
      class="fixed inset-0 z-30 flex items-center justify-center bg-black/60 p-4"
      onClick={(e) => e.target === e.currentTarget && props.onClose()}
    >
      <form
        onSubmit={submit}
        class="w-full max-w-md space-y-4 rounded-xl border border-neutral-800 bg-neutral-900 p-5 shadow-2xl"
      >
        <h2 class="text-lg font-semibold">Nouvelle instance</h2>

        <div class="space-y-1">
          <label class="text-xs text-neutral-400" for="instance-name">
            Nom
          </label>
          <input
            id="instance-name"
            class="w-full rounded border border-neutral-700 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-emerald-500"
            placeholder={version() ? `Minecraft ${version()}` : "Ma survie"}
            maxLength={64}
            value={name()}
            onInput={(e) => setName(e.currentTarget.value)}
            autofocus
          />
        </div>

        <div class="space-y-1">
          <div class="flex items-center justify-between">
            <label class="text-xs text-neutral-400" for="instance-version">
              Version
            </label>
            <label class="flex items-center gap-1.5 text-xs text-neutral-400">
              <input
                type="checkbox"
                checked={showSnapshots()}
                onChange={(e) => setShowSnapshots(e.currentTarget.checked)}
              />
              Snapshots
            </label>
          </div>
          <Show
            when={!versions.error}
            fallback={
              <p class="text-sm text-red-400">
                Impossible de charger les versions : {errorMessage(versions.error)}
              </p>
            }
          >
            <select
              id="instance-version"
              class="w-full rounded border border-neutral-700 bg-neutral-950 px-3 py-2 text-sm outline-none focus:border-emerald-500"
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
          <p class="text-sm text-red-400">{error()}</p>
        </Show>

        <div class="flex justify-end gap-2 pt-1">
          <button
            type="button"
            class="rounded-md px-4 py-2 text-sm text-neutral-400 hover:bg-neutral-800"
            onClick={props.onClose}
          >
            Annuler
          </button>
          <button
            type="submit"
            class="rounded-md bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-40"
            disabled={!version() || saving()}
          >
            Créer
          </button>
        </div>
      </form>
    </div>
  );
}
