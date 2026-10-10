import { createEffect, createMemo, createResource, createSignal, For, type JSX, on, onCleanup, Show } from "solid-js";
import { Icon, LoaderIcon } from "../components/pixel";
import Alert from "../components/Alert";
import ModpackInstallDialog from "../components/ModpackInstallDialog";
import ProjectIcon from "../components/ProjectIcon";
import Select from "../components/Select";
import Tabs, { tabPanel } from "../components/Tabs";
import { api, errorMessage, type ProjectType, type SearchHit } from "../lib/api";
import { installContent, isBusy, isInstalled, loadContent } from "../lib/content";
import { formatCount, loaderLabel } from "../lib/format";
import { importModpackFile, installPack, isInstallingPack } from "../lib/modpacks";
import { categoryLabel, isLoaderCategory, openProject } from "../lib/projects";
import { instances, navigate, remembered, setNewInstanceDialog } from "../lib/store";
import { toast } from "../lib/toast";

const KINDS: { id: ProjectType; label: string }[] = [
  { id: "mod", label: "Mods" },
  { id: "resourcepack", label: "Packs de textures" },
  { id: "shader", label: "Shaders" },
  { id: "datapack", label: "Datapacks" },
  { id: "modpack", label: "Modpacks" },
];

const SEARCH_DELAY_MS = 300;

export default function Discover(props: { instanceId?: string; query?: string; kind?: ProjectType; world?: string }) {
  const pickDefault = () =>
    props.instanceId ?? (instances().find((i) => i.loader !== "vanilla") ?? instances()[0])?.id ?? "";
  const [instanceId, setInstanceId] = remembered("instance", pickDefault());
  // Without any instance, only modpacks make sense: they create one.
  const [kind, setKind] = remembered<ProjectType>("kind", props.kind ?? (instances().length > 0 ? "mod" : "modpack"));
  const datapacks = () => kind() === "datapack";
  // Datapacks go into a world of the instance.
  const [worlds] = createResource(
    () => (datapacks() && instanceId() ? instanceId() : false),
    (id) => api.listWorlds(id),
  );
  const [world, setWorld] = remembered("world", props.world ?? "");
  createEffect(() => {
    const list = worlds();
    if (list && !list.some((w) => w.folder === world())) setWorld(list[0]?.folder ?? "");
  });
  const [worldPacks, { refetch: refetchWorldPacks }] = createResource(
    () => (datapacks() && instanceId() && world() ? { id: instanceId(), world: world() } : false),
    ({ id, world }) => api.listDatapacks(id, world).catch(() => []),
  );
  const [packBusy, setPackBusy] = createSignal<Record<string, boolean>>({});
  const needsWorld = () => datapacks() && !!instanceId() && worlds() !== undefined && worlds()!.length === 0;
  const [query, setQuery] = remembered("query", props.query ?? "");
  // A remembered query searches right away, without waiting for the debounce.
  const [debounced, setDebounced] = createSignal(query());
  const [hits, setHits] = createSignal<SearchHit[]>([]);
  const [total, setTotal] = createSignal(0);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [actionError, setActionError] = createSignal<string | null>(null);
  const [importing, setImporting] = createSignal(false);

  const modpacks = () => kind() === "modpack";
  const instance = createMemo(() => instances().find((i) => i.id === instanceId()));
  const needsInstance = () => !modpacks() && instances().length === 0;
  const needsLoader = () => kind() === "mod" && instance()?.loader === "vanilla";
  const needsIris = () =>
    kind() === "shader" && instance()?.loader !== "vanilla" && !isInstalled(instanceId(), "YL57xq9U");
  const searchable = () => !needsInstance() && !needsLoader() && !needsWorld();

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

  /** Modpack whose version picker is open. */
  const [picking, setPicking] = createSignal<SearchHit | null>(null);

  async function installModpack(projectId: string, versionId: string) {
    setPicking(null);
    setActionError(null);
    setActionError(await installPack(projectId, versionId));
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

  async function installDatapack(projectId: string) {
    setPackBusy((b) => ({ ...b, [projectId]: true }));
    setActionError(null);
    try {
      const installed = await api.installDatapack(instanceId(), world(), projectId);
      await refetchWorldPacks();
      toast(`${installed.title} ajouté au monde`);
    } catch (err) {
      setActionError(errorMessage(err));
    } finally {
      setPackBusy((b) => ({ ...b, [projectId]: false }));
    }
  }

  function action(hit: SearchHit): JSX.Element {
    if (datapacks()) {
      return (
        <Show
          when={!(worldPacks() ?? []).some((p) => p.projectId === hit.projectId)}
          fallback={
            <span class="flex h-9 shrink-0 items-center gap-1.5 px-3 text-sm text-xp-text">
              <Icon name="check" size={12} />
              Dans le monde
            </span>
          }
        >
          <button
            class="btn btn-primary px-corners h-9 shrink-0"
            disabled={!world() || packBusy()[hit.projectId]}
            onClick={() => void installDatapack(hit.projectId)}
          >
            <Icon name="download" size={12} />
            {packBusy()[hit.projectId] ? "Ajout…" : "Ajouter"}
          </button>
        </Show>
      );
    }
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
            disabled={isInstallingPack(hit.projectId)}
            onClick={() => setPicking(hit)}
          >
            <Icon name="download" size={12} />
            {isInstallingPack(hit.projectId) ? "Installation…" : "Installer"}
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
          Mods, packs de textures, shaders, datapacks et modpacks de Modrinth, filtrés sur la version et le loader de ton instance.
        </span>
      </div>

      <div class="flex flex-wrap items-center gap-3">
        <Show when={!modpacks() && instances().length > 0}>
          <label class="sr-only" for="discover-instance">
            Instance
          </label>
          <Select
            id="discover-instance"
            class="w-64 text-sm"
            value={instanceId()}
            options={instances().map((i) => ({
              value: i.id,
              label: i.name,
              hint: `${i.gameVersion} · ${loaderLabel(i.loader)}`,
              icon: <LoaderIcon loader={i.loader} size={14} />,
            }))}
            onChange={setInstanceId}
          />
        </Show>

        <Show when={datapacks() && (worlds() ?? []).length > 0}>
          <Select
            class="w-52 text-sm"
            label="Monde"
            value={world()}
            options={(worlds() ?? []).map((w) => ({ value: w.folder, label: w.name, hint: w.version ?? undefined }))}
            onChange={setWorld}
          />
        </Show>

        <Tabs
          label="Type de contenu"
          idPrefix="discover-kind"
          variant="segmented"
          tabClass="h-[34px]"
          items={KINDS}
          value={kind()}
          onChange={setKind}
        />

        <div class="relative min-w-[220px] flex-1">
          <span class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-muted">
            <Icon name="search" size={13} />
          </span>
          <input
            id="discover-search"
            type="search"
            class="field w-full pl-9 text-sm"
            placeholder={`Rechercher sur Modrinth… (${navigator.userAgent.includes("Mac") ? "⌘K" : "Ctrl+K"})`}
            aria-label="Rechercher"
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)}
          />
        </div>
      </div>

      <Show when={modpacks()}>
        <div class="panel px-corners-md flex items-center justify-between gap-4 p-4">
          <p class="text-sm text-chalk-2">
            Chaque modpack crée sa propre instance, avec la bonne version du jeu et du loader.
          </p>
          <button class="btn px-corners shrink-0" disabled={importing()} onClick={() => void importFile()}>
            <Icon name="folder" size={12} />
            {importing() ? "Import…" : "Importer un modpack"}
          </button>
        </div>
      </Show>

      <Show when={needsWorld()}>
        <div class="panel px-corners-md flex flex-col items-center gap-2 py-12 text-center">
          <p class="text-chalk-2">Les datapacks s'ajoutent à un monde, et « {instance()?.name} » n'en a pas encore.</p>
          <p class="text-sm text-muted">Lance le jeu et crée un monde : il apparaîtra ici.</p>
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
        {(message) => (
          <Alert
            onClose={() => {
              setActionError(null);
              setError(null);
            }}
          >
            {message()}
          </Alert>
        )}
      </Show>

      <Show when={searchable()}>
        <span class="font-mono text-xs text-muted">
          {loading() && hits().length === 0 ? "Recherche…" : `${formatCount(total())} résultats`}
        </span>
        <Show when={!loading() && hits().length === 0 && !error()}>
          <div class="panel px-corners-md flex flex-col items-center gap-1.5 py-12 text-center">
            <p class="text-chalk-2">Rien trouvé{debounced() ? ` pour « ${debounced()} »` : ""}.</p>
            <p class="text-sm text-muted">Essaie un autre mot, ou un autre type de contenu.</p>
          </div>
        </Show>
        <ul {...tabPanel("discover-kind", kind())} class="flex flex-col gap-2">
          <For each={hits()}>
            {(hit) => (
              <li class="panel px-corners-md flex items-start gap-4 p-3.5">
                <button
                  class="group/card flex min-w-0 flex-1 items-start gap-4 text-left"
                  aria-label={`Voir la fiche de ${hit.title}`}
                  onClick={() => openProject(hit.slug || hit.projectId, modpacks() ? undefined : instanceId() || undefined)}
                >
                  <ProjectIcon url={hit.iconUrl} size={56} />
                  <div class="flex min-w-0 flex-1 flex-col gap-1">
                    <div class="flex items-baseline gap-2">
                      <h2 class="truncate text-[15px] font-semibold group-hover/card:text-xp-text group-focus-visible/card:text-xp-text">
                        {hit.title}
                      </h2>
                      <span class="shrink-0 text-xs text-muted">par {hit.author}</span>
                    </div>
                    <p class="line-clamp-2 text-[13px] text-chalk-2">{hit.description}</p>
                    <div class="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-muted">
                      <span class="flex items-center gap-1">
                        <Icon name="download" size={10} />
                        {formatCount(hit.downloads)}
                      </span>
                      <For each={hit.displayCategories.slice(0, 3)}>
                        {(category) => (
                          <span class="chip h-5 px-1.5 text-[11px]">
                            {isLoaderCategory(category) ? loaderLabel(category) : categoryLabel(category)}
                          </span>
                        )}
                      </For>
                    </div>
                  </div>
                </button>
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
      {/* Keyed: the dialog keeps its pack while it closes. */}
      <Show when={picking()} keyed>
        {(hit) => (
          <ModpackInstallDialog
            projectId={hit.projectId}
            title={hit.title}
            onClose={() => setPicking(null)}
            onInstall={(versionId) => void installModpack(hit.projectId, versionId)}
          />
        )}
      </Show>
    </div>
  );
}
