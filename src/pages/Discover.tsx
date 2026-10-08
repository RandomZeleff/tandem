import { createEffect, createMemo, createSignal, For, type JSX, on, onCleanup, Show } from "solid-js";
import { Icon, LoaderIcon } from "../components/pixel";
import ProjectIcon from "../components/ProjectIcon";
import { api, errorMessage, type ProjectType, type SearchHit } from "../lib/api";
import { installContent, isBusy, isInstalled, loadContent } from "../lib/content";
import { formatCount, loaderLabel } from "../lib/format";
import { importModpackFile } from "../lib/modpacks";
import { instances, navigate, refetchInstances, setNewInstanceDialog } from "../lib/store";

const KINDS: { id: ProjectType; label: string }[] = [
  { id: "mod", label: "Mods" },
  { id: "resourcepack", label: "Packs de textures" },
  { id: "shader", label: "Shaders" },
  { id: "modpack", label: "Modpacks" },
];

const SEARCH_DELAY_MS = 300;

export default function Discover(props: { instanceId?: string }) {
  const pickDefault = () =>
    props.instanceId ?? (instances().find((i) => i.loader !== "vanilla") ?? instances()[0])?.id ?? "";
  const [instanceId, setInstanceId] = createSignal(pickDefault());
  // Without any instance, only modpacks make sense: they create one.
  const [kind, setKind] = createSignal<ProjectType>(instances().length > 0 ? "mod" : "modpack");
  const [query, setQuery] = createSignal("");
  const [debounced, setDebounced] = createSignal("");
  const [hits, setHits] = createSignal<SearchHit[]>([]);
  const [total, setTotal] = createSignal(0);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [actionError, setActionError] = createSignal<string | null>(null);
  const [packBusy, setPackBusy] = createSignal<Record<string, boolean>>({});
  const [importing, setImporting] = createSignal(false);

  const modpacks = () => kind() === "modpack";
  const instance = createMemo(() => instances().find((i) => i.id === instanceId()));
  const needsInstance = () => !modpacks() && instances().length === 0;
  const needsLoader = () => kind() === "mod" && instance()?.loader === "vanilla";
  const needsIris = () =>
    kind() === "shader" && instance()?.loader !== "vanilla" && !isInstalled(instanceId(), "YL57xq9U");
  const searchable = () => !needsInstance() && !needsLoader();

  // Instances load asynchronously: pick one as soon as the list arrives.
  createEffect(() => {
    if (!instance() && instances().length > 0) setInstanceId(pickDefault());
  });

  createEffect(() => {
    const value = query();
    const timer = setTimeout(() => setDebounced(value), SEARCH_DELAY_MS);
    onCleanup(() => clearTimeout(timer));
  });

  createEffect(() => {
    const id = instanceId();
    if (id) void loadContent(id);
  });

  let generation = 0;
  async function search(append: boolean) {
    const current = ++generation;
    setLoading(true);
    setError(null);
    try {
      const results = await api.searchContent(debounced(), kind(), instanceId() || null, append ? hits().length : 0);
      if (current !== generation) return;
      setHits(append ? [...hits(), ...results.hits] : results.hits);
      setTotal(results.totalHits);
    } catch (err) {
      if (current === generation) setError(errorMessage(err));
    } finally {
      if (current === generation) setLoading(false);
    }
  }

  createEffect(
    on([debounced, kind, instanceId, searchable], () => {
      setActionError(null);
      if (!searchable()) {
        generation++;
        setHits([]);
        setTotal(0);
        return;
      }
      void search(false);
    }),
  );

  async function install(projectId: string) {
    setActionError(await installContent(instanceId(), projectId));
  }

  async function installPack(projectId: string) {
    setPackBusy((b) => ({ ...b, [projectId]: true }));
    setActionError(null);
    try {
      const created = await api.installModpack(projectId);
      await refetchInstances();
      navigate({ page: "instance", id: created.id });
    } catch (err) {
      setActionError(errorMessage(err));
    } finally {
      setPackBusy((b) => ({ ...b, [projectId]: false }));
    }
  }

  async function importFile() {
    setImporting(true);
    setActionError(null);
    try {
      await importModpackFile();
    } catch (err) {
      setActionError(errorMessage(err));
    } finally {
      setImporting(false);
    }
  }

  function action(hit: SearchHit): JSX.Element {
    if (modpacks()) {
      const existing = instances().find((i) => i.packProjectId === hit.projectId);
      return (
        <Show
          when={!existing}
          fallback={
            <button class="btn px-corners h-9 shrink-0" onClick={() => navigate({ page: "instance", id: existing!.id })}>
              <Icon name="check" size={12} color="var(--color-xp-text)" />
              Ouvrir
            </button>
          }
        >
          <button
            class="btn btn-primary px-corners h-9 shrink-0"
            disabled={packBusy()[hit.projectId]}
            onClick={() => void installPack(hit.projectId)}
          >
            <Icon name="download" size={12} />
            {packBusy()[hit.projectId] ? "Installation…" : "Installer"}
          </button>
        </Show>
      );
    }
    return (
      <Show
        when={!isInstalled(instanceId(), hit.projectId)}
        fallback={
          <span class="flex h-9 shrink-0 items-center gap-1.5 px-3 text-sm text-xp-text">
            <Icon name="check" size={12} />
            Installé
          </span>
        }
      >
        <button
          class="btn btn-primary px-corners h-9 shrink-0"
          disabled={isBusy(instanceId(), hit.projectId)}
          onClick={() => void install(hit.projectId)}
        >
          <Icon name="download" size={12} />
          {isBusy(instanceId(), hit.projectId) ? "Installation…" : "Installer"}
        </button>
      </Show>
    );
  }

  return (
    <div class="flex flex-col gap-5">
      <div class="flex flex-col gap-1">
        <h1 class="pixel-shadow font-pixel text-3xl font-bold">Découvrir</h1>
        <span class="text-[13px] text-muted">
          Mods, packs de textures, shaders et modpacks de Modrinth, filtrés sur la version et le loader de ton instance.
        </span>
      </div>

      <div class="flex flex-wrap items-center gap-3">
        <Show when={!modpacks() && instances().length > 0}>
          <label class="sr-only" for="discover-instance">
            Instance
          </label>
          <div class="relative">
            <span class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2">
              <LoaderIcon loader={instance()?.loader ?? "vanilla"} size={14} />
            </span>
            <select
              id="discover-instance"
              class="field w-64 pl-9 text-sm"
              value={instanceId()}
              onChange={(e) => setInstanceId(e.currentTarget.value)}
            >
              <For each={instances()}>
                {(i) => (
                  <option value={i.id}>
                    {i.name} — {i.gameVersion} · {loaderLabel(i.loader)}
                  </option>
                )}
              </For>
            </select>
          </div>
        </Show>

        <div
          role="tablist"
          aria-label="Type de contenu"
          class="flex w-fit gap-0.5 bg-slate-900 p-[3px] shadow-[inset_0_0_0_1px_var(--color-line)]"
        >
          <For each={KINDS}>
            {(k) => (
              <button
                role="tab"
                aria-selected={kind() === k.id}
                class="h-[34px] px-3 text-[13px]"
                classList={{
                  "bg-slate-600 font-medium text-chalk shadow-[inset_0_1px_0_rgb(255_255_255/0.06)]": kind() === k.id,
                  "text-muted hover:text-chalk": kind() !== k.id,
                }}
                onClick={() => setKind(k.id)}
              >
                {k.label}
              </button>
            )}
          </For>
        </div>

        <div class="relative min-w-[220px] flex-1">
          <span class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-muted">
            <Icon name="search" size={13} />
          </span>
          <input
            type="search"
            class="field w-full pl-9 text-sm"
            placeholder="Rechercher sur Modrinth…"
            aria-label="Rechercher"
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)}
          />
        </div>
      </div>

      <Show when={modpacks()}>
        <div class="panel px-corners-md flex items-center justify-between gap-4 p-4">
          <p class="flex flex-wrap items-center gap-1.5 text-sm text-chalk-2">
            Chaque modpack crée sa propre instance. Pour l'instant :
            <span class="inline-flex items-center gap-1">
              <LoaderIcon loader="fabric" size={12} /> Fabric
            </span>
            et
            <span class="inline-flex items-center gap-1">
              <LoaderIcon loader="quilt" size={12} /> Quilt
            </span>
          </p>
          <button class="btn px-corners shrink-0" disabled={importing()} onClick={() => void importFile()}>
            <Icon name="folder" size={12} />
            {importing() ? "Import…" : "Importer un .mrpack"}
          </button>
        </div>
      </Show>

      <Show when={needsInstance()}>
        <div class="panel px-corners-md flex flex-col items-center gap-3 py-14 text-center">
          <p class="text-chalk-2">Crée d'abord une instance pour y installer du contenu, ou pars d'un modpack.</p>
          <div class="flex gap-2">
            <button class="btn btn-primary px-corners" onClick={() => setNewInstanceDialog({})}>
              <Icon name="plus" size={12} />
              Nouvelle instance
            </button>
            <button class="btn px-corners" onClick={() => setKind("modpack")}>
              Voir les modpacks
            </button>
          </div>
        </div>
      </Show>

      <Show when={needsLoader()}>
        <div class="panel px-corners-md flex items-center justify-between gap-4 p-4">
          <p class="text-sm text-chalk-2">
            Les mods ont besoin d'un loader : « {instance()?.name} » est une instance Vanilla.
          </p>
          <button class="btn px-corners shrink-0" onClick={() => setNewInstanceDialog({ version: instance()?.gameVersion })}>
            <LoaderIcon loader="fabric" size={12} />
            Créer une instance Fabric
          </button>
        </div>
      </Show>

      <Show when={needsIris()}>
        <div class="panel px-corners-md flex items-center justify-between gap-4 p-4">
          <p class="text-sm text-chalk-2">Les shaders ont besoin du mod Iris dans l'instance.</p>
          <button class="btn px-corners shrink-0" disabled={isBusy(instanceId(), "iris")} onClick={() => void install("iris")}>
            <Icon name="download" size={12} />
            {isBusy(instanceId(), "iris") ? "Installation…" : "Installer Iris"}
          </button>
        </div>
      </Show>

      <Show when={actionError() ?? error()}>
        <p class="bg-[#2A1414] px-3 py-2 text-sm text-redstone-text shadow-[inset_0_0_0_1px_#6E2A26]">
          {actionError() ?? error()}
        </p>
      </Show>

      <Show when={searchable()}>
        <span class="font-mono text-xs text-muted">
          {loading() && hits().length === 0 ? "Recherche…" : `${formatCount(total())} résultats`}
        </span>
        <ul class="flex flex-col gap-2">
          <For each={hits()}>
            {(hit) => (
              <li class="panel px-corners-md flex items-start gap-4 p-3.5">
                <ProjectIcon url={hit.iconUrl} size={56} />
                <div class="flex min-w-0 flex-1 flex-col gap-1">
                  <div class="flex items-baseline gap-2">
                    <h2 class="truncate text-[15px] font-semibold">{hit.title}</h2>
                    <span class="shrink-0 text-xs text-muted">par {hit.author}</span>
                  </div>
                  <p class="line-clamp-2 text-[13px] text-chalk-2">{hit.description}</p>
                  <div class="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-muted">
                    <span class="flex items-center gap-1">
                      <Icon name="download" size={10} />
                      {formatCount(hit.downloads)}
                    </span>
                    <For each={hit.displayCategories.slice(0, 3)}>
                      {(category) => <span class="chip h-5 px-1.5 text-[11px]">{category}</span>}
                    </For>
                  </div>
                </div>
                {action(hit)}
              </li>
            )}
          </For>
        </ul>
        <Show when={hits().length < total()}>
          <button class="btn px-corners mx-auto" disabled={loading()} onClick={() => void search(true)}>
            {loading() ? "Chargement…" : "Voir plus"}
          </button>
        </Show>
      </Show>
    </div>
  );
}
