import { createEffect, createMemo, createResource, createSignal, For, type JSX, on, onCleanup, Show } from "solid-js";
import { Checkbox, Icon, LoaderIcon } from "../components/pixel";
import Alert from "../components/Alert";
import { ListShelf, PictureShelf } from "../components/DiscoverShelf";
import ModpackInstallDialog from "../components/ModpackInstallDialog";
import ProjectIcon from "../components/ProjectIcon";
import Select from "../components/Select";
import Tabs, { tabPanel } from "../components/Tabs";
import { api, errorMessage, type ProjectType, type SearchHit, type SearchNarrowing, type SearchSort } from "../lib/api";
import { installContent, isBusy, isInstalled, loadContent } from "../lib/content";
import { FILTER_CATEGORIES, SORT_LABELS } from "../lib/discover";
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
  // Empty: the automatic order (relevance with a query, most downloaded otherwise).
  const [sort, setSort] = remembered<SearchSort | "">("sort", "");
  const [categories, setCategories] = remembered<string[]>("categories", []);
  const [packVersion, setPackVersion] = remembered("packVersion", "");

  const modpacks = () => kind() === "modpack";
  // Each kind has its own categories, and opens on its shelves.
  createEffect(
    on(
      kind,
      () => {
        setCategories([]);
        setSort("");
      },
      { defer: true },
    ),
  );
  let top: HTMLDivElement | undefined;
  // "Most downloaded" is the automatic order without a query.
  createEffect(() => {
    if (!debounced().trim() && (sort() === "downloads" || sort() === "relevance")) setSort("");
  });
  const [gameVersions] = createResource(
    () => modpacks() || false,
    () => api.listVersions().then((list) => list.versions.filter((v) => v.type === "release").map((v) => v.id)),
  );
  const narrowing = (): SearchNarrowing => ({
    sort: sort() || undefined,
    categories: categories(),
    gameVersion: modpacks() ? packVersion() || undefined : undefined,
  });
  /** Nothing typed or narrowed: shelves come first. */
  const browsing = () => !debounced().trim() && !sort() && categories().length === 0;
  const sortOptions = () => {
    const searching = !!debounced().trim();
    const order: SearchSort[] = searching ? ["downloads", "follows", "newest", "updated"] : ["follows", "newest", "updated"];
    return [
      { value: "", label: SORT_LABELS[searching ? "relevance" : "downloads"] },
      ...order.map((value) => ({ value, label: SORT_LABELS[value] })),
    ];
  };
  function toggleCategory(category: string) {
    setCategories((list) => (list.includes(category) ? list.filter((c) => c !== category) : [...list, category]));
  }
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
      const results = await api.searchContent(debounced(), kind(), instanceId() || null, append ? hits().length : 0, narrowing());
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
    on([debounced, kind, instanceId, searchable, sort, categories, packVersion], () => {
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

  const shelf = (sort: SearchSort) => ({
    sort,
    title: SORT_LABELS[sort],
    kind: kind(),
    instanceId: modpacks() ? null : instanceId() || null,
    gameVersion: modpacks() ? packVersion() || undefined : undefined,
    onOpen: (hit: SearchHit) => openProject(hit.slug || hit.projectId, modpacks() ? undefined : instanceId() || undefined),
    onSeeAll: () => {
      setSort(sort);
      top?.scrollIntoView({ block: "nearest" });
    },
  });

  return (
    <div class="mx-auto flex max-w-[1400px] flex-col gap-6">
      <h1 class="title-display text-title">Découvrir</h1>

      {/* Kinds are the page's sections; what they apply to sits at the right end. */}
      <div class="flex items-center gap-3 shadow-[inset_0_-1px_0_var(--color-divider)]">
        <Tabs label="Type de contenu" idPrefix="discover-kind" class="mr-auto" items={KINDS} value={kind()} onChange={setKind} />
        <Show when={!modpacks() && instances().length > 0}>
          <Select
            class="w-60 text-sm"
            label="Instance"
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
            class="w-48 text-sm"
            label="Monde"
            value={world()}
            options={(worlds() ?? []).map((w) => ({ value: w.folder, label: w.name, hint: w.version ?? undefined }))}
            onChange={setWorld}
          />
        </Show>
        <Show when={modpacks()}>
          <button
            class="btn btn-ghost h-9 px-3 text-sm"
            title="Un fichier .mrpack : il crée sa propre instance"
            disabled={importing()}
            onClick={() => void importFile()}
          >
            <Icon name="folder" size={12} />
            {importing() ? "Import…" : "Importer un fichier"}
          </button>
        </Show>
      </div>

      <div ref={top} class="grid scroll-mt-6 items-start gap-8" classList={{ "grid-cols-[200px_minmax(0,1fr)]": searchable() }}>
        <Show when={searchable()}>
          <aside aria-label="Filtres" class="sticky top-0 flex flex-col gap-5">
            <div class="flex flex-col gap-2">
              <span class="text-meta font-medium text-muted">Trier par</span>
              <Select class="w-full text-sm" label="Trier par" value={sort()} options={sortOptions()} onChange={(v) => setSort(v as SearchSort | "")} />
            </div>
            <Show when={modpacks()}>
              <div class="flex flex-col gap-2">
                <span class="text-meta font-medium text-muted">Version du jeu</span>
                <Select
                  class="w-full text-sm"
                  label="Version du jeu"
                  value={packVersion()}
                  options={[
                    { value: "", label: "Toutes les versions" },
                    ...(gameVersions() ?? []).map((v) => ({ value: v, label: v })),
                  ]}
                  onChange={setPackVersion}
                />
              </div>
            </Show>
            <div role="group" aria-labelledby="discover-categories" class="flex flex-col">
              <div class="flex h-7 items-center justify-between">
                <span id="discover-categories" class="text-meta font-medium text-muted">
                  Catégories
                </span>
                <Show when={categories().length > 0}>
                  <button class="btn btn-ghost h-7 px-2 text-xs" onClick={() => setCategories([])}>
                    Effacer
                  </button>
                </Show>
              </div>
              <For each={FILTER_CATEGORIES[kind()]}>
                {(category) => (
                  <Checkbox
                    class="row -mx-2 h-8 px-2 text-sm text-chalk-2"
                    checked={categories().includes(category)}
                    label={categoryLabel(category)}
                    onChange={() => toggleCategory(category)}
                  />
                )}
              </For>
            </div>
          </aside>
        </Show>

        <div class="@container flex min-w-0 flex-col gap-8">
          <div class="flex flex-col gap-4">
            <div class="relative">
              <span class="pointer-events-none absolute top-1/2 left-3.5 -translate-y-1/2 text-muted">
                <Icon name="search" size={12} />
              </span>
              <input
                id="discover-search"
                type="search"
                class="field h-10 w-full pl-10 text-sm"
                placeholder={`Rechercher sur Modrinth… (${navigator.userAgent.includes("Mac") ? "⌘K" : "Ctrl+K"})`}
                aria-label="Rechercher"
                value={query()}
                onInput={(e) => setQuery(e.currentTarget.value)}
              />
            </div>

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
          </div>

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

          <Show when={searchable() && browsing()}>
            <PictureShelf {...shelf("newest")} />
            <div class="grid gap-x-6 gap-y-8 @2xl:grid-cols-2">
              <ListShelf {...shelf("updated")} />
              <ListShelf {...shelf("follows")} />
            </div>
          </Show>

          <Show when={searchable()}>
            <section class="flex flex-col gap-3" aria-labelledby="discover-results">
              <div class="flex h-8 items-baseline gap-3">
                <h2 id="discover-results" class="text-lg font-semibold">
                  {browsing() ? SORT_LABELS.downloads : "Résultats"}
                </h2>
                <span class="text-meta text-muted">
                  {loading() && hits().length === 0 ? "Recherche…" : `${formatCount(total())} résultat${total() > 1 ? "s" : ""}`}
                </span>
              </div>
              <Show when={!loading() && hits().length === 0 && !error()}>
                <div class="panel px-corners-md flex flex-col items-center gap-1.5 py-12 text-center">
                  <p class="text-chalk-2">Rien trouvé{debounced() ? ` pour « ${debounced()} »` : ""}.</p>
                  <p class="text-sm text-muted">
                    {categories().length > 0 ? "Essaie avec moins de catégories." : "Essaie un autre mot, ou un autre type de contenu."}
                  </p>
                </div>
              </Show>
              <ul {...tabPanel("discover-kind", kind())} class="panel flex flex-col divide-y divide-line overflow-hidden" classList={{ hidden: hits().length === 0 }}>
                <For each={hits()}>
                  {(hit) => (
                    <li class="flex items-start gap-4 p-3.5 transition-colors duration-100 hover:bg-white/[0.03] has-[button:focus-visible]:bg-white/[0.03]">
                      <button
                        class="group/card flex min-w-0 flex-1 items-start gap-4 text-left"
                        aria-label={`Voir la fiche de ${hit.title}`}
                        onClick={() => openProject(hit.slug || hit.projectId, modpacks() ? undefined : instanceId() || undefined)}
                      >
                        <ProjectIcon url={hit.iconUrl} size={56} />
                        <div class="flex min-w-0 flex-1 flex-col gap-1">
                          <div class="flex items-baseline gap-2">
                            <h3 class="truncate text-base font-semibold group-hover/card:text-xp-text group-focus-visible/card:text-xp-text">
                              {hit.title}
                            </h3>
                            <span class="shrink-0 text-meta text-muted">par {hit.author}</span>
                          </div>
                          <p class="line-clamp-2 text-meta text-chalk-2">{hit.description}</p>
                          <div class="mt-1 flex flex-wrap items-center gap-1.5 text-meta text-muted">
                            <span class="flex items-center gap-1">
                              <Icon name="download" size={12} />
                              {formatCount(hit.downloads)}
                            </span>
                            <For each={hit.displayCategories.slice(0, 3)}>
                              {(category) => (
                                <span class="chip h-5 px-1.5 text-xs">
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
            </section>
          </Show>
        </div>
      </div>
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
