import { createMemo, createResource, For, Show } from "solid-js";
import { api, type DependencyItem, type ProjectVersions } from "../../lib/api";
import { isInstalled } from "../../lib/content";
import { formatCount } from "../../lib/format";
import { openProject, projectTypeLabel, versionTypeLabel } from "../../lib/projects";
import { remembered } from "../../lib/store";
import LoadingRows from "../LoadingRows";
import { Icon } from "../pixel";
import ProjectIcon from "../ProjectIcon";
import Select from "../Select";
import { Retry } from "./VersionsTab";

const KINDS: { kind: DependencyItem["kind"]; label: string; hint: string }[] = [
  { kind: "required", label: "Requises", hint: "Installées automatiquement avec lui." },
  { kind: "optional", label: "Optionnelles", hint: "Ajoutent des fonctions si elles sont présentes." },
  { kind: "incompatible", label: "Incompatibles", hint: "Ne pas installer dans la même instance." },
  { kind: "embedded", label: "Incluses", hint: "Déjà livrées à l'intérieur." },
];

const TYPE_ORDER = ["mod", "resourcepack", "shader", "datapack"];
const TYPE_TITLES: Record<string, string> = {
  mod: "Mods",
  resourcepack: "Packs de textures",
  shader: "Shaders",
  datapack: "Datapacks",
};

/**
 * What a version needs (`dependencies`) or, for a modpack, what it ships (`content`),
 * for a version the player picks. Each project opens its own page.
 */
export default function DependenciesTab(props: {
  projectId: string;
  mode: "dependencies" | "content";
  versions: ProjectVersions | undefined;
  versionsError: unknown;
  onRetry: () => void;
  instanceId?: string;
}) {
  const [picked, setPicked] = remembered(`${props.mode}.version`, "");
  const [query, setQuery] = remembered(`${props.mode}.query`, "");
  const versionId = () => {
    const list = props.versions?.versions ?? [];
    if (picked() && list.some((v) => v.id === picked())) return picked();
    return props.versions?.recommended ?? list[0]?.id ?? "";
  };
  const [items, { refetch }] = createResource(
    () => versionId() || false,
    (id) => api.versionDependencies(props.projectId, id),
  );

  const visible = createMemo(() => {
    const all = items() ?? [];
    const q = query().trim().toLowerCase();
    const scoped = props.mode === "content" ? all.filter((i) => i.kind === "embedded") : all;
    return q ? scoped.filter((i) => i.project.title.toLowerCase().includes(q) || i.project.summary.toLowerCase().includes(q)) : scoped;
  });

  const groups = () => {
    if (props.mode === "dependencies") {
      return KINDS.map((k) => ({ title: k.label, hint: k.hint, items: visible().filter((i) => i.kind === k.kind) })).filter((g) => g.items.length > 0);
    }
    const types = [...new Set(visible().map((i) => i.project.projectType))].sort(
      (a, b) => (TYPE_ORDER.indexOf(a) + 1 || 99) - (TYPE_ORDER.indexOf(b) + 1 || 99),
    );
    return types.map((t) => ({ title: TYPE_TITLES[t] ?? projectTypeLabel(t), hint: "", items: visible().filter((i) => i.project.projectType === t) }));
  };

  const total = () => (props.mode === "content" ? (items() ?? []).filter((i) => i.kind === "embedded").length : (items() ?? []).length);

  return (
    <div class="flex flex-col gap-3">
      <div class="flex flex-wrap items-center gap-2">
        <Select
          class="w-64 text-[13px]"
          label="Version"
          value={versionId()}
          placeholder={props.versions ? "Aucune version" : "Chargement…"}
          disabled={!props.versions || props.versions.versions.length === 0}
          options={(props.versions?.versions ?? []).map((v) => ({
            value: v.id,
            label: v.versionNumber,
            hint: v.id === props.versions?.recommended ? "recommandée" : versionTypeLabel(v.versionType).toLowerCase(),
          }))}
          onChange={setPicked}
        />
        <Show when={props.mode === "content"}>
          <div class="relative min-w-[200px] flex-1">
            <span class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-muted">
              <Icon name="search" size={12} />
            </span>
            <input
              type="search"
              class="field h-[34px] w-full pl-8 text-[13px]"
              placeholder="Chercher dans le modpack…"
              aria-label="Chercher dans le modpack"
              value={query()}
              onInput={(e) => setQuery(e.currentTarget.value)}
            />
          </div>
        </Show>
        <span class="ml-auto font-mono text-xs text-muted">
          <Show when={items()}>
            {visible().length === total() ? total() : `${visible().length} / ${total()}`}
          </Show>
        </span>
      </div>

      <Show when={props.mode === "content"}>
        <p class="text-xs text-muted">
          Liste publiée sur Modrinth pour cette version. Le modpack peut contenir en plus des fichiers venus d'ailleurs.
        </p>
      </Show>

      <Show when={!props.versionsError && !items.error} fallback={<Retry error={props.versionsError ?? items.error} onRetry={() => (props.versionsError ? props.onRetry() : void refetch())} />}>
        <Show when={items() !== undefined && !items.loading} fallback={<LoadingRows count={6} height={52} label="Chargement…" />}>
          <Show
            when={visible().length > 0}
            fallback={
              <p class="panel px-corners-md py-10 text-center text-chalk-2">
                {query().trim()
                  ? `Rien pour « ${query().trim()} ».`
                  : props.mode === "content"
                    ? "Modrinth ne liste pas le contenu de cette version."
                    : "Aucune dépendance : il fonctionne seul."}
              </p>
            }
          >
            <For each={groups()}>
              {(group) => (
                <section class="flex flex-col gap-2">
                  <h3 class="flex items-baseline gap-2">
                    <span class="panel-title">
                      {group.title} <span class="font-mono text-muted">{group.items.length}</span>
                    </span>
                    <Show when={group.hint}>
                      <span class="text-xs text-muted">{group.hint}</span>
                    </Show>
                  </h3>
                  <ul class="panel px-corners-md flex flex-col divide-y divide-line">
                    <For each={group.items}>{(item) => <Row item={item} instanceId={props.instanceId} />}</For>
                  </ul>
                </section>
              )}
            </For>
          </Show>
        </Show>
      </Show>
    </div>
  );
}

function Row(props: { item: DependencyItem; instanceId?: string }) {
  const p = () => props.item.project;
  const installed = () => !!props.instanceId && isInstalled(props.instanceId, p().id);
  return (
    <li>
      <button
        class="group flex w-full items-center gap-3 px-3 py-2.5 text-left hover:bg-slate-750 focus-visible:bg-slate-750"
        onClick={() => openProject(p().slug || p().id, props.instanceId)}
      >
        <ProjectIcon url={p().iconUrl} size={36} />
        <div class="flex min-w-0 flex-1 flex-col">
          <span class="flex items-center gap-2">
            <span class="truncate text-sm font-medium group-hover:text-xp-text">{p().title}</span>
            <Show when={installed()}>
              <span class="chip h-5 shrink-0 px-1.5 text-[11px] text-xp-text">
                <Icon name="check" size={9} />
                installé
              </span>
            </Show>
          </span>
          <span class="truncate text-xs text-muted">{p().summary}</span>
        </div>
        <span class="flex shrink-0 items-center gap-1 text-xs text-muted">
          <Icon name="download" size={9} />
          {formatCount(p().downloads)}
        </span>
        <Icon name="arrow" size={10} class="shrink-0 text-faint group-hover:text-chalk" />
      </button>
    </li>
  );
}
