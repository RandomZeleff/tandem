import { createResource, For, type JSX, Show } from "solid-js";
import { api, type PackOverview, type SearchHit } from "../../lib/api";
import { formatBytes, formatCount, formatGigabytes } from "../../lib/format";
import { categoryLabel, isLoaderCategory, openProject } from "../../lib/projects";
import ProjectIcon from "../ProjectIcon";
import { Icon, type IconName } from "../pixel";

type Fit = "good" | "tight" | "short";

/** Whether this PC gives the game what the pack wants (automatic memory, D18). */
function memoryFit(memory: PackOverview["memory"]): Fit {
  if (memory.givenMb >= memory.wantedMb) return "good";
  return memory.givenMb >= memory.wantedMb * 0.75 ? "tight" : "short";
}

/** Gold only says "caution" (DESIGN.md); a good fit gets the green check. */
const FIT: Record<Fit, { value: string; icon?: IconName; caution?: boolean }> = {
  good: { value: "Ton PC suit", icon: "check" },
  tight: { value: "Un peu juste", caution: true },
  short: { value: "Trop lourd pour ton PC", caution: true },
};

/**
 * "Is this pack for me?" at a glance: will it run here, how big it is, whether it has
 * quests and what it is mostly about, for the version Install would pick.
 */
export default function PackFit(props: { projectId: string; versionId: string | undefined }) {
  const [overview] = createResource(
    () => (props.versionId ? { project: props.projectId, version: props.versionId } : false),
    ({ project, version }) => api.packOverview(project, version),
  );

  return (
    <Show when={!overview.error && props.versionId}>
      <section aria-label="Est-ce pour moi ?" class="panel px-corners-md grid grid-cols-4 divide-x divide-line">
        <Show
          when={overview()}
          fallback={
            <For each={Array.from({ length: 4 })}>
              {(_, i) => (
                <div class="flex flex-col gap-2 p-4">
                  <div class="skeleton h-3 w-16 rounded-sm" style={{ "animation-delay": `${i() * 120}ms` }} />
                  <div class="skeleton h-5 w-28 rounded-sm" style={{ "animation-delay": `${i() * 120}ms` }} />
                </div>
              )}
            </For>
          }
        >
          {(o) => {
            const fit = () => FIT[memoryFit(o().memory)];
            return (
              <>
                <Cell label="Mémoire" title={`Le jeu aura ${formatGigabytes(o().memory.givenMb)} sur les ${formatGigabytes(o().memory.totalMb)} de ce PC`}>
                  <Value icon={fit().icon} tone="var(--color-xp-text)" caution={fit().caution}>
                    {fit().value}
                  </Value>
                  <Sub>
                    {memoryFit(o().memory) === "good"
                      ? `${formatGigabytes(o().memory.givenMb)} pour le jeu`
                      : `Il aime ${formatGigabytes(o().memory.wantedMb)}, ton PC en donne ${formatGigabytes(o().memory.givenMb)}`}
                  </Sub>
                </Cell>
                <Cell label="Contenu" title="Mods publiés sur Modrinth ; le pack peut en inclure d'autres directement">
                  <Value>{o().mods === 1 ? "1 mod" : `${formatCount(o().mods)} mods`}</Value>
                  <Sub>{formatBytes(o().downloadBytes)} à télécharger</Sub>
                </Cell>
                <Cell label="Quêtes">
                  <Value icon={o().quests ? "book" : undefined}>{o().quests ? "Livre de quêtes" : "Pas annoncées"}</Value>
                  <Sub>{o().quests ? "Une progression guidée" : "Exploration libre"}</Sub>
                </Cell>
                <Cell label="Surtout">
                  <Show when={o().themes.length > 0} fallback={<Value>Varié</Value>}>
                    <Value>
                      {o()
                        .themes.slice(0, 2)
                        .map((t) => categoryLabel(t.category))
                        .join(", ")}
                    </Value>
                    <Sub>
                      {o()
                        .themes.slice(0, 2)
                        .map((t) => `${t.mods} mods`)
                        .join(" · ")}
                    </Sub>
                  </Show>
                </Cell>
              </>
            );
          }}
        </Show>
      </section>
    </Show>
  );
}

function Cell(props: { label: string; title?: string; children: JSX.Element }) {
  return (
    <div class="flex min-w-0 flex-col gap-1 p-4" title={props.title}>
      <span class="text-xs text-muted">{props.label}</span>
      {props.children}
    </div>
  );
}

function Value(props: { icon?: IconName; tone?: string; caution?: boolean; children: JSX.Element }) {
  return (
    <span class="flex min-w-0 items-center gap-2 text-[15px] font-semibold" classList={{ "text-gold": props.caution }}>
      <Show when={props.icon}>{(icon) => <Icon name={icon()} size={12} color={props.tone ?? "var(--color-chalk-3)"} />}</Show>
      <span class="truncate">{props.children}</span>
    </span>
  );
}

function Sub(props: { children: JSX.Element }) {
  return <span class="truncate text-xs text-muted">{props.children}</span>;
}

/** Modpacks sharing this one's themes and game version, most followed first. */
export function SimilarPacks(props: { projectId: string; categories: string[]; gameVersion: string | undefined }) {
  const [packs] = createResource(
    () => ({ id: props.projectId, categories: props.categories.filter((c) => !isLoaderCategory(c)), gameVersion: props.gameVersion }),
    async ({ id, categories, gameVersion }): Promise<SearchHit[]> => {
      if (categories.length === 0) return [];
      const find = async (cats: string[]) =>
        (await api.searchContent("", "modpack", null, 0, { sort: "follows", categories: cats, gameVersion })).hits.filter(
          (h) => h.projectId !== id,
        );
      // Both main themes when enough packs share them, else the first one alone.
      const close = categories.length > 1 ? await find(categories.slice(0, 2)) : [];
      return (close.length >= 3 ? close : await find(categories.slice(0, 1))).slice(0, 5);
    },
  );

  return (
    <Show when={(packs() ?? []).length > 0}>
      <section class="panel px-corners-md flex flex-col gap-2 p-4">
        <h2 class="panel-title">Dans le même esprit</h2>
        <ul class="-mx-2 flex flex-col">
          <For each={packs()}>
            {(pack) => (
              <li>
                <button class="row group flex w-full items-center gap-2.5 px-2 py-1.5 text-left" onClick={() => openProject(pack.slug || pack.projectId)}>
                  <ProjectIcon url={pack.iconUrl} size={32} />
                  <span class="flex min-w-0 flex-col">
                    <span class="truncate text-meta text-chalk group-hover:text-xp-text group-focus-visible:text-xp-text">{pack.title}</span>
                    <span class="truncate text-xs text-muted">{formatCount(pack.follows)} abonnés</span>
                  </span>
                </button>
              </li>
            )}
          </For>
        </ul>
      </section>
    </Show>
  );
}
