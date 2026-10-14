import { createResource, createUniqueId, For, Show } from "solid-js";
import type { ProjectType, SearchHit, SearchSort } from "../lib/api";
import { shelfResults } from "../lib/discover";
import { formatCount, formatRelative } from "../lib/format";
import ProjectIcon from "./ProjectIcon";
import { Icon } from "./pixel";

const SHOWN = 12;
const CARD_WIDTH = 232;

/** A row of projects in one order (newest, freshly updated…), scrolled by pages. */
export default function DiscoverShelf(props: {
  title: string;
  sort: SearchSort;
  kind: ProjectType;
  instanceId: string | null;
  gameVersion?: string;
  onOpen: (hit: SearchHit) => void;
  onSeeAll: () => void;
}) {
  const titleId = createUniqueId();
  let row: HTMLUListElement | undefined;
  const [results] = createResource(
    () => ({ kind: props.kind, instanceId: props.instanceId, sort: props.sort, gameVersion: props.gameVersion }),
    ({ kind, instanceId, sort, gameVersion }) => shelfResults(kind, instanceId, { sort, gameVersion }),
  );
  const hits = () => (results.error ? [] : (results()?.hits ?? []).slice(0, SHOWN));

  function page(direction: 1 | -1) {
    if (!row) return;
    row.scrollBy({ left: direction * Math.max(CARD_WIDTH, row.clientWidth - CARD_WIDTH / 2), behavior: "smooth" });
  }

  function meta(hit: SearchHit) {
    if (props.sort === "newest") return `Sorti ${formatRelative(hit.dateCreated)}`;
    if (props.sort === "updated") return `Mis à jour ${formatRelative(hit.dateModified)}`;
    return `${formatCount(hit.follows)} abonnés`;
  }

  // An empty or failed shelf disappears: the full list below still works.
  return (
    <Show when={results.loading || hits().length > 0}>
      <section aria-labelledby={titleId} class="flex flex-col gap-2.5">
        <div class="flex items-center gap-2">
          <h2 id={titleId} class="text-lg font-semibold">
            {props.title}
          </h2>
          <span class="flex-1" />
          <button class="btn btn-ghost h-8 px-2.5 text-sm" onClick={props.onSeeAll}>
            Tout voir
          </button>
          <button class="btn btn-ghost h-8 w-8 px-0" aria-label="Précédents" onClick={() => page(-1)}>
            <Icon name="arrow" size={12} class="rotate-180" />
          </button>
          <button class="btn btn-ghost h-8 w-8 px-0" aria-label="Suivants" onClick={() => page(1)}>
            <Icon name="arrow" size={12} />
          </button>
        </div>
        <Show
          when={!results.loading}
          fallback={
            <div role="status" aria-label="Chargement…" class="flex gap-3 overflow-hidden">
              <For each={Array.from({ length: 6 })}>
                {(_, i) => (
                  <div class="skeleton px-corners-md h-[148px] shrink-0" style={{ width: `${CARD_WIDTH}px`, "animation-delay": `${i() * 120}ms` }} />
                )}
              </For>
            </div>
          }
        >
          <ul ref={row} class="flex snap-x gap-3 overflow-x-auto scroll-smooth [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            <For each={hits()}>
              {(hit) => (
                <li class="shrink-0 snap-start" style={{ width: `${CARD_WIDTH}px` }}>
                  <button
                    class="panel group/card flex h-[148px] w-full flex-col gap-2.5 p-3 text-left transition-colors duration-100 hover:bg-slate-650 focus-visible:bg-slate-650"
                    aria-label={`Voir la fiche de ${hit.title}`}
                    onClick={() => props.onOpen(hit)}
                  >
                    <div class="flex min-w-0 items-center gap-3">
                      <ProjectIcon url={hit.iconUrl} size={48} />
                      <div class="flex min-w-0 flex-col">
                        <span class="truncate text-base font-semibold group-hover/card:text-xp-text group-focus-visible/card:text-xp-text">
                          {hit.title}
                        </span>
                        <span class="truncate text-meta text-muted">par {hit.author}</span>
                      </div>
                    </div>
                    <p class="line-clamp-2 text-meta text-chalk-2">{hit.description}</p>
                    <span class="mt-auto flex items-center gap-1.5 text-xs text-muted">
                      <Show when={props.sort === "follows"} fallback={<Icon name="sparkle" size={12} />}>
                        <Icon name="heart" size={12} />
                      </Show>
                      {meta(hit)}
                    </span>
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </section>
    </Show>
  );
}
