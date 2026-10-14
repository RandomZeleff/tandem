import { createResource, createSignal, createUniqueId, For, type JSX, Show } from "solid-js";
import type { ProjectType, SearchHit, SearchSort } from "../lib/api";
import { shelfResults } from "../lib/discover";
import { formatCount, formatRelative } from "../lib/format";
import ProjectIcon from "./ProjectIcon";
import { Icon } from "./pixel";

interface ShelfProps {
  title: string;
  sort: SearchSort;
  kind: ProjectType;
  instanceId: string | null;
  gameVersion?: string;
  onOpen: (hit: SearchHit) => void;
  onSeeAll: () => void;
}

function useShelf(props: ShelfProps, count: number) {
  const [results] = createResource(
    () => ({ kind: props.kind, instanceId: props.instanceId, sort: props.sort, gameVersion: props.gameVersion }),
    ({ kind, instanceId, sort, gameVersion }) => shelfResults(kind, instanceId, { sort, gameVersion }),
  );
  const hits = () => (results.error ? [] : (results()?.hits ?? []).slice(0, count));
  // An empty or failed shelf disappears: the full list below still works.
  const visible = () => results.loading || hits().length > 0;
  return { results, hits, visible };
}

function meta(sort: SearchSort, hit: SearchHit): string {
  if (sort === "newest") return `Sorti ${formatRelative(hit.dateCreated)}`;
  if (sort === "updated") return `Mis à jour ${formatRelative(hit.dateModified)}`;
  return `${formatCount(hit.follows)} abonnés`;
}

function ShelfHeader(props: { id: string; title: string; onSeeAll: () => void; children?: JSX.Element }) {
  return (
    <div class="flex h-8 items-center gap-1">
      <h2 id={props.id} class="mr-auto text-lg font-semibold">
        {props.title}
      </h2>
      <button class="btn btn-ghost h-8 px-2.5 text-sm" onClick={props.onSeeAll}>
        Tout voir
      </button>
      {props.children}
    </div>
  );
}

const CARD_WIDTH = 248;

/** The lead shelf: picture cards (featured image, else the icon on its colour), scrolled by pages. */
export function PictureShelf(props: ShelfProps) {
  const titleId = createUniqueId();
  const { results, hits, visible } = useShelf(props, 12);
  let row: HTMLUListElement | undefined;
  function page(direction: 1 | -1) {
    row?.scrollBy({ left: direction * Math.max(CARD_WIDTH, row.clientWidth - CARD_WIDTH / 2), behavior: "smooth" });
  }

  return (
    <Show when={visible()}>
      <section aria-labelledby={titleId} class="flex flex-col gap-3">
        <ShelfHeader id={titleId} title={props.title} onSeeAll={props.onSeeAll}>
          <button class="btn btn-ghost h-8 w-8 px-0" aria-label="Précédents" onClick={() => page(-1)}>
            <Icon name="arrow" size={12} class="rotate-180" />
          </button>
          <button class="btn btn-ghost h-8 w-8 px-0" aria-label="Suivants" onClick={() => page(1)}>
            <Icon name="arrow" size={12} />
          </button>
        </ShelfHeader>
        <Show
          when={!results.loading}
          fallback={
            <div role="status" aria-label="Chargement…" class="flex gap-3 overflow-hidden">
              <For each={Array.from({ length: 5 })}>
                {(_, i) => <div class="skeleton h-[208px] shrink-0 rounded-md" style={{ width: `${CARD_WIDTH}px`, "animation-delay": `${i() * 120}ms` }} />}
              </For>
            </div>
          }
        >
          <ul ref={row} class="flex snap-x gap-3 overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            <For each={hits()}>
              {(hit) => (
                <li class="shrink-0 snap-start" style={{ width: `${CARD_WIDTH}px` }}>
                  <PictureCard hit={hit} meta={meta(props.sort, hit)} onOpen={() => props.onOpen(hit)} />
                </li>
              )}
            </For>
          </ul>
        </Show>
      </section>
    </Show>
  );
}

function PictureCard(props: { hit: SearchHit; meta: string; onOpen: () => void }) {
  const [failed, setFailed] = createSignal(false);
  const tint = () => `#${(props.hit.color ?? 0x383a3f).toString(16).padStart(6, "0")}`;
  return (
    <button
      class="panel group/card flex w-full flex-col overflow-hidden text-left transition-colors duration-100 hover:bg-slate-650 focus-visible:bg-slate-650"
      aria-label={`Voir la fiche de ${props.hit.title}`}
      onClick={props.onOpen}
    >
      {/* 16:8 like instance cards; without a picture, the icon sits on its own colour. */}
      <div class="relative flex aspect-[16/8] w-full items-center justify-center overflow-hidden" style={{ background: `color-mix(in srgb, ${tint()} 35%, var(--color-slate-900))` }}>
        <Show when={props.hit.featuredGallery && !failed()} fallback={<ProjectIcon url={props.hit.iconUrl} size={64} />}>
          <img src={props.hit.featuredGallery!} alt="" loading="lazy" class="absolute inset-0 size-full object-cover" onError={() => setFailed(true)} />
          <div class="absolute inset-0 bg-[linear-gradient(0deg,rgb(16_17_19/0.55),transparent_45%)]" />
          <ProjectIcon url={props.hit.iconUrl} size={40} style={{ position: "absolute", left: "10px", bottom: "10px" }} />
        </Show>
      </div>
      <div class="flex flex-col gap-1 px-3 pt-2.5 pb-3">
        <span class="truncate text-[15px] font-semibold group-hover/card:text-xp-text group-focus-visible/card:text-xp-text">{props.hit.title}</span>
        <span class="truncate text-meta text-muted">{props.meta}</span>
      </div>
    </button>
  );
}

/** A short ranked list (five rows), for the secondary shelves set side by side. */
export function ListShelf(props: ShelfProps) {
  const titleId = createUniqueId();
  const { results, hits, visible } = useShelf(props, 5);
  return (
    <Show when={visible()}>
      <section aria-labelledby={titleId} class="flex min-w-0 flex-col gap-3">
        <ShelfHeader id={titleId} title={props.title} onSeeAll={props.onSeeAll} />
        <Show
          when={!results.loading}
          fallback={
            <div role="status" aria-label="Chargement…" class="panel flex flex-col gap-1 p-1.5">
              <For each={Array.from({ length: 5 })}>
                {(_, i) => <div class="skeleton h-[52px] rounded-sm" style={{ "animation-delay": `${i() * 120}ms` }} />}
              </For>
            </div>
          }
        >
          <ol class="panel flex flex-col p-1.5">
            <For each={hits()}>
              {(hit, i) => (
                <li>
                  <button
                    class="row group/row flex h-[52px] w-full items-center gap-3 px-2 text-left"
                    aria-label={`Voir la fiche de ${hit.title}`}
                    onClick={() => props.onOpen(hit)}
                  >
                    <span class="w-4 shrink-0 text-right text-meta text-faint tabular-nums">{i() + 1}</span>
                    <ProjectIcon url={hit.iconUrl} size={40} />
                    <span class="flex min-w-0 flex-1 flex-col">
                      <span class="truncate text-sm font-semibold group-hover/row:text-xp-text group-focus-visible/row:text-xp-text">{hit.title}</span>
                      <span class="truncate text-xs text-muted">{meta(props.sort, hit)}</span>
                    </span>
                  </button>
                </li>
              )}
            </For>
          </ol>
        </Show>
      </section>
    </Show>
  );
}
