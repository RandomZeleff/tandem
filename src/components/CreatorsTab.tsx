import { createResource, createSignal, For, type Resource, Show } from "solid-js";
import { api, type Creator, type Creators, errorMessage, type Instance } from "../lib/api";
import { installedContent } from "../lib/content";
import { openExternal } from "../lib/projects";
import Alert from "./Alert";
import LoadingRows from "./LoadingRows";
import { Icon } from "./pixel";

/** Shown at first; the rest behind a button. */
const FIRST_OTHERS = 12;

/**
 * Who made what is in this instance, grouped by author, with their own support links.
 * Tandem shows no ads (they pay authors on Modrinth): this is how players can help.
 */
export default function CreatorsTab(props: { instance: Instance }) {
  // Refetched when installed content changes.
  const [data] = createResource(
    () => ({ id: props.instance.id, count: installedContent(props.instance.id).length }),
    ({ id }) => api.instanceCreators(id),
  );
  return <CreatorsView data={data} empty="Aucun contenu de Modrinth dans cette instance pour l'instant." />;
}

/** The creators of the mods a modpack version contains. */
export function PackCreators(props: { projectId: string; versionId: string | undefined }) {
  const [data] = createResource(
    () => (props.versionId ? { project: props.projectId, version: props.versionId } : false),
    ({ project, version }) => api.packCreators(project, version),
  );
  return <CreatorsView data={data} empty="Ce modpack ne liste pas son contenu sur Modrinth." />;
}

function CreatorsView(props: { data: Resource<Creators>; empty: string }) {
  const [allOthers, setAllOthers] = createSignal(false);
  const data = () => props.data;
  const supported = () => data().latest?.creators.filter((c) => c.support.length > 0) ?? [];
  const others = () => data().latest?.creators.filter((c) => c.support.length === 0) ?? [];

  return (
    <div class="flex h-full flex-col gap-4 overflow-y-auto pr-1">
      <Show when={data().error}>
        <Alert>{`Créateurs indisponibles : ${errorMessage(data().error)}`}</Alert>
      </Show>
      <Show when={data().latest} fallback={<Show when={!data().error}><LoadingRows count={4} /></Show>}>
        {(d) => (
          <Show when={d().creators.length > 0} fallback={<p class="text-sm text-muted">{props.empty}</p>}>
            <section class="panel px-corners-md flex flex-col gap-1.5 p-4">
              <p class="text-[15px]">
                {d().projects} contenu{d().projects > 1 ? "s" : ""}, fait{d().projects > 1 ? "s" : ""} par{" "}
                <strong>
                  {d().creators.length} créateur{d().creators.length > 1 ? "s" : ""}
                </strong>
                <Show when={supported().length > 0}>
                  {" "}
                  : {supported().length} accepte{supported().length > 1 ? "nt" : ""} les soutiens.
                </Show>
              </p>
              <p class="text-xs text-muted">
                Tandem n'affiche pas de publicité, celle qui rémunère les auteurs sur Modrinth. Les liens mènent directement chez eux :
                Tandem ne prend rien.
              </p>
            </section>

            <Show when={supported().length > 0}>
              <ul class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-2">
                <For each={supported()}>{(c) => <CreatorCard creator={c} />}</For>
              </ul>
            </Show>

            <Show when={others().length > 0}>
              <h2 class="panel-title mt-2">Les autres créateurs</h2>
              <ul class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-2">
                <For each={allOthers() ? others() : others().slice(0, FIRST_OTHERS)}>{(c) => <CreatorCard creator={c} />}</For>
              </ul>
              <Show when={!allOthers() && others().length > FIRST_OTHERS}>
                <button class="btn btn-ghost h-8 self-start px-2 text-xs" onClick={() => setAllOthers(true)}>
                  Voir les {others().length - FIRST_OTHERS} autres
                </button>
              </Show>
            </Show>
          </Show>
        )}
      </Show>
    </div>
  );
}

function CreatorCard(props: { creator: Creator }) {
  const c = () => props.creator;
  const [failed, setFailed] = createSignal(false);
  const platform = (url: string, name: string) => name || new URL(url).hostname.replace(/^www\./, "");

  return (
    <li class="panel px-corners flex flex-col gap-2 p-3">
      <button class="group flex items-center gap-2.5 text-left" title={`${c().name} sur Modrinth`} onClick={() => openExternal(c().url)}>
        <span class="flex size-8 shrink-0 items-center justify-center overflow-hidden bg-slate-900 shadow-[inset_0_0_0_1px_var(--color-line)]">
          <Show when={c().avatarUrl && !failed()} fallback={<Icon name="user" size={12} color="var(--color-faint)" />}>
            <img src={c().avatarUrl!} alt="" loading="lazy" class="size-full object-cover" onError={() => setFailed(true)} />
          </Show>
        </span>
        <span class="flex min-w-0 flex-col">
          <span class="truncate text-sm font-medium group-hover:text-xp-text">{c().name}</span>
          <span class="truncate text-xs text-muted" title={c().projects.join(", ")}>
            {c().projects.length > 2
              ? `${c().projects.slice(0, 2).join(", ")} et ${c().projects.length - 2} autre${c().projects.length > 3 ? "s" : ""}`
              : c().projects.join(", ")}
          </span>
        </span>
      </button>
      <Show
        when={c().support.length > 0}
        fallback={
          <button class="btn btn-ghost h-7 self-start px-2 text-xs" onClick={() => openExternal(c().url)}>
            <Icon name="external" size={9} />
            Voir sur Modrinth
          </button>
        }
      >
        <div class="flex flex-wrap gap-1.5">
          <For each={c().support}>
            {(s) => (
              <button class="btn px-corners h-7 gap-1.5 px-2.5 text-xs" title={s.url} onClick={() => openExternal(s.url)}>
                <Icon name="heart" size={10} class="text-redstone-text" />
                {platform(s.url, s.platform)}
              </button>
            )}
          </For>
        </div>
      </Show>
    </li>
  );
}
