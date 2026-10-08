import { createSignal, For, onMount, Show } from "solid-js";
import type { ContentKind, InstalledContent } from "../lib/api";
import { installedContent, isBusy, loadContent, removeContent } from "../lib/content";
import { errorMessage } from "../lib/api";
import { navigate } from "../lib/store";
import { Icon } from "./pixel";
import ProjectIcon from "./ProjectIcon";

const SECTIONS: { kind: ContentKind; label: string }[] = [
  { kind: "mod", label: "Mods" },
  { kind: "resourcepack", label: "Packs de textures" },
  { kind: "shader", label: "Shaders" },
];

/** Installed mods, resource packs and shaders of an instance. */
export default function ContentList(props: { instanceId: string; locked: boolean }) {
  const [error, setError] = createSignal<string | null>(null);
  const items = () => installedContent(props.instanceId);
  const browse = () => navigate({ page: "discover", instanceId: props.instanceId });

  onMount(() => {
    loadContent(props.instanceId).catch((err) => setError(errorMessage(err)));
  });

  async function remove(item: InstalledContent) {
    setError(await removeContent(props.instanceId, item.projectId));
  }

  return (
    <div class="flex h-full flex-col gap-4 overflow-y-auto">
      <div class="flex items-center justify-between gap-3">
        <span class="text-[13px] text-muted">
          {items().length === 0 ? "Aucun contenu installé." : `${items().length} élément(s) installé(s)`}
          <Show when={props.locked}> · arrête le jeu pour en retirer</Show>
        </span>
        <button class="btn btn-primary px-corners h-9" onClick={browse}>
          <Icon name="plus" size={12} />
          Ajouter du contenu
        </button>
      </div>

      <Show when={error()}>
        <p class="bg-[#2A1414] px-3 py-2 text-sm text-redstone-text shadow-[inset_0_0_0_1px_#6E2A26]">{error()}</p>
      </Show>

      <Show
        when={items().length > 0}
        fallback={
          <div class="panel px-corners-md flex flex-col items-center gap-3 py-12 text-center">
            <p class="text-chalk-2">Ajoute des mods, des packs de textures ou des shaders depuis Modrinth.</p>
            <button class="btn px-corners" onClick={browse}>
              <Icon name="search" size={12} />
              Parcourir Modrinth
            </button>
          </div>
        }
      >
        <For each={SECTIONS.filter((s) => items().some((i) => i.kind === s.kind))}>
          {(section) => {
            const list = () => items().filter((i) => i.kind === section.kind);
            return (
              <section class="flex flex-col gap-2">
                <h2 class="panel-title">
                  {section.label} <span class="font-mono text-muted">{list().length}</span>
                </h2>
                <ul class="panel px-corners-md flex flex-col divide-y divide-line">
                  <For each={list()}>
                    {(item) => (
                      <li class="flex items-center gap-3 px-3 py-2.5">
                        <ProjectIcon url={item.iconUrl} size={36} />
                        <div class="flex min-w-0 flex-1 flex-col">
                          <span class="flex items-center gap-2">
                            <span class="truncate text-sm font-medium">{item.title}</span>
                            <Show when={item.isDependency}>
                              <span class="chip h-5 px-1.5 text-[11px] text-muted">dépendance</span>
                            </Show>
                          </span>
                          <span class="truncate font-mono text-xs text-muted">{item.versionNumber}</span>
                        </div>
                        <button
                          class="btn btn-ghost h-8 w-8 px-0 hover:text-redstone-text"
                          aria-label={`Retirer ${item.title}`}
                          title="Retirer"
                          disabled={props.locked || isBusy(props.instanceId, item.projectId)}
                          onClick={() => void remove(item)}
                        >
                          <Icon name="trash" size={12} />
                        </button>
                      </li>
                    )}
                  </For>
                </ul>
              </section>
            );
          }}
        </For>
      </Show>
    </div>
  );
}
