import { createEffect, createMemo, createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage, type Instance, type Loader } from "../lib/api";
import { loaderLabel } from "../lib/format";
import Dialog from "./Dialog";
import { LoaderIcon } from "./pixel";

interface Props {
  onClose: () => void;
  onCreated: (instance: Instance) => void;
  /** Version to preselect (e.g. a snapshot from the news card). */
  initialVersion?: string;
}

const LOADERS: Loader[] = ["vanilla", "fabric", "quilt"];

export default function NewInstanceDialog(props: Props) {
  const [versions] = createResource(api.listVersions);
  const [showSnapshots, setShowSnapshots] = createSignal(false);
  const [name, setName] = createSignal("");
  const [version, setVersion] = createSignal(props.initialVersion ?? "");
  const [loader, setLoader] = createSignal<Loader>("vanilla");
  const [loaderVersion, setLoaderVersion] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);
  const [saving, setSaving] = createSignal(false);

  const choices = createMemo(() =>
    (versions()?.versions ?? []).filter(
      (v) => v.type === "release" || ((showSnapshots() || v.id === version()) && v.type === "snapshot"),
    ),
  );

  const loaderQuery = () => (loader() !== "vanilla" && version() ? ([loader(), version()] as const) : null);
  const [loaderVersions] = createResource(loaderQuery, ([l, v]) => api.listLoaderVersions(l, v));

  createEffect(() => {
    const list = versions();
    if (!list) return;
    if (!version()) setVersion(list.latest.release);
    if (list.versions.find((v) => v.id === version())?.type === "snapshot") setShowSnapshots(true);
  });

  // Preselect the latest stable loader version whenever the list changes.
  createEffect(() => {
    const list = loaderVersions.latest ?? [];
    setLoaderVersion((list.find((v) => v.stable) ?? list[0])?.version ?? "");
  });

  const loaderReady = () => loader() === "vanilla" || (!loaderVersions.loading && !!loaderVersion());
  const defaultName = () => `${loader() === "vanilla" ? "Minecraft" : loaderLabel(loader())} ${version()}`;

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      props.onCreated(
        await api.createInstance({
          name: name().trim() || defaultName(),
          gameVersion: version(),
          loader: loader(),
          loaderVersion: loader() === "vanilla" ? undefined : loaderVersion(),
        }),
      );
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
            placeholder={version() ? defaultName() : "Ma survie"}
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

        <div class="flex flex-col gap-1.5">
          <span class="text-xs text-muted" id="instance-loader">
            Loader
          </span>
          <div
            role="radiogroup"
            aria-labelledby="instance-loader"
            class="flex w-fit gap-0.5 bg-slate-900 p-[3px] shadow-[inset_0_0_0_1px_var(--color-line)]"
          >
            <For each={LOADERS}>
              {(l) => (
                <button
                  type="button"
                  role="radio"
                  aria-checked={loader() === l}
                  class="flex h-7 items-center gap-1.5 px-3 text-[13px]"
                  classList={{
                    "bg-slate-600 font-medium text-chalk shadow-[inset_0_1px_0_rgb(255_255_255/0.06)]": loader() === l,
                    "text-muted hover:text-chalk": loader() !== l,
                  }}
                  onClick={() => setLoader(l)}
                >
                  <LoaderIcon loader={l} size={12} />
                  {loaderLabel(l)}
                </button>
              )}
            </For>
          </div>
        </div>

        <Show when={loader() !== "vanilla"}>
          <div class="flex flex-col gap-1.5">
            <label class="text-xs text-muted" for="instance-loader-version">
              Version de {loaderLabel(loader())}
            </label>
            <Show
              when={!loaderVersions.error}
              fallback={
                <p class="text-sm text-redstone-text">
                  Impossible de charger les versions de {loaderLabel(loader())} : {errorMessage(loaderVersions.error)}
                </p>
              }
            >
              <Show
                when={loaderVersions.loading || (loaderVersions.latest ?? []).length > 0}
                fallback={
                  <p class="text-sm text-chalk-2">
                    {loaderLabel(loader())} n'est pas disponible pour Minecraft {version()}.
                  </p>
                }
              >
                <select
                  id="instance-loader-version"
                  class="field text-sm"
                  value={loaderVersion()}
                  onChange={(e) => setLoaderVersion(e.currentTarget.value)}
                  disabled={loaderVersions.loading}
                >
                  <Show when={loaderVersions.loading}>
                    <option>Chargement…</option>
                  </Show>
                  <Show when={!loaderVersions.loading}>
                    <For each={loaderVersions.latest}>
                      {(v) => (
                        <option value={v.version}>
                          {v.version}
                          {v.stable ? "" : " (bêta)"}
                        </option>
                      )}
                    </For>
                  </Show>
                </select>
              </Show>
            </Show>
          </div>
        </Show>

        <Show when={error()}>
          <p class="text-sm text-redstone-text">{error()}</p>
        </Show>

        <div class="flex justify-end gap-2 pt-1">
          <button type="button" class="btn btn-ghost" onClick={props.onClose}>
            Annuler
          </button>
          <button
            type="submit"
            class="btn btn-primary px-corners px-5"
            disabled={!version() || !loaderReady() || saving()}
          >
            Créer
          </button>
        </div>
      </form>
    </Dialog>
  );
}
