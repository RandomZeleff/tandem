import { createSignal, For, onMount, Show } from "solid-js";
import { errorMessage, type ContentKind, type InstalledContent, type Instance } from "../lib/api";
import {
  checkUpdates,
  contentUpdates,
  installedContent,
  isBusy,
  isCheckingUpdates,
  isUpdatingAll,
  loadContent,
  removeContent,
  setContentEnabled,
  updateContent,
  updateFor,
} from "../lib/content";
import { exportModpackFile } from "../lib/modpacks";
import { navigate } from "../lib/store";
import PerfSuggestions from "./PerfSuggestions";
import { Icon, Toggle } from "./pixel";
import ProjectIcon from "./ProjectIcon";

const SECTIONS: { kind: ContentKind; label: string }[] = [
  { kind: "mod", label: "Mods" },
  { kind: "resourcepack", label: "Packs de textures" },
  { kind: "shader", label: "Shaders" },
];

/** Installed mods, resource packs and shaders of an instance, with updates and on/off switches. */
export default function ContentList(props: { instance: Instance; locked: boolean }) {
  const [error, setError] = createSignal<string | null>(null);
  const [notice, setNotice] = createSignal<string | null>(null);
  const [exporting, setExporting] = createSignal(false);
  const items = () => installedContent(props.instance.id);
  const updates = () => contentUpdates(props.instance.id);
  const disabledCount = () => items().filter((i) => !i.enabled).length;
  const browse = () => navigate({ page: "discover", instanceId: props.instance.id });

  async function run(action: Promise<string | null>) {
    setError(await action);
  }

  async function exportPack() {
    setExporting(true);
    setError(null);
    setNotice(null);
    try {
      const path = await exportModpackFile(props.instance);
      if (path) setNotice(`Modpack exporté : ${path}`);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setExporting(false);
    }
  }

  onMount(async () => {
    try {
      await loadContent(props.instance.id);
    } catch (err) {
      setError(errorMessage(err));
      return;
    }
    if (items().length > 0) void run(checkUpdates(props.instance.id));
  });

  const summary = () => {
    const parts = [`${items().length} élément${items().length > 1 ? "s" : ""}`];
    if (disabledCount() > 0) parts.push(`${disabledCount()} désactivé${disabledCount() > 1 ? "s" : ""}`);
    return parts.join(" · ");
  };

  return (
    <div class="flex h-full flex-col gap-4 overflow-y-auto">
      <div class="flex items-center justify-between gap-3">
        <span class="text-[13px] text-muted">
          {items().length === 0 ? "Aucun contenu installé." : summary()}
          <Show when={props.locked && items().length > 0}> · arrête le jeu pour modifier le contenu</Show>
        </span>
        <div class="flex gap-2">
          <Show when={items().length > 0}>
            <button
              class="btn px-corners h-9"
              disabled={isCheckingUpdates(props.instance.id) || isUpdatingAll(props.instance.id)}
              onClick={() => void run(checkUpdates(props.instance.id))}
            >
              <Icon name="sparkle" size={12} />
              {isCheckingUpdates(props.instance.id) ? "Vérification…" : "Vérifier les mises à jour"}
            </button>
          </Show>
          <Show when={props.instance.loader === "fabric" || props.instance.loader === "quilt" || props.instance.loader === "vanilla"}>
            <button class="btn px-corners h-9" disabled={exporting()} onClick={() => void exportPack()} title="Exporter en modpack Modrinth (.mrpack)">
              <Icon name="download" size={12} />
              {exporting() ? "Export…" : "Exporter"}
            </button>
          </Show>
          <button class="btn btn-primary px-corners h-9" onClick={browse}>
            <Icon name="plus" size={12} />
            Ajouter du contenu
          </button>
        </div>
      </div>

      <Show when={notice()}>
        <p class="bg-[#16240F] px-3 py-2 text-sm text-xp-text shadow-[inset_0_0_0_1px_#2E5A1A]">{notice()}</p>
      </Show>

      <Show when={error()}>
        <p class="bg-[#2A1414] px-3 py-2 text-sm text-redstone-text shadow-[inset_0_0_0_1px_#6E2A26]">{error()}</p>
      </Show>

      <PerfSuggestions instance={props.instance} locked={props.locked} onError={setError} />

      <Show when={updates().length > 0}>
        <div class="panel px-corners-md flex items-center justify-between gap-4 p-3.5 shadow-[inset_0_0_0_1px_#4E9A1E]">
          <div class="flex items-center gap-2.5">
            <span class="size-2 bg-xp shadow-[0_0_0_2px_rgb(139_224_78/0.25)]" />
            <span class="text-sm">
              {updates().length === 1 ? "1 mise à jour disponible" : `${updates().length} mises à jour disponibles`}
            </span>
          </div>
          <button
            class="btn btn-primary px-corners h-9"
            disabled={props.locked || isCheckingUpdates(props.instance.id) || isUpdatingAll(props.instance.id)}
            onClick={() => void run(updateContent(props.instance.id))}
          >
            <Icon name="download" size={12} />
            {isUpdatingAll(props.instance.id) ? "Mise à jour…" : "Tout mettre à jour"}
          </button>
        </div>
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
                  <For each={list()}>{(item) => <Row instanceId={props.instance.id} item={item} locked={props.locked} run={run} />}</For>
                </ul>
              </section>
            );
          }}
        </For>
      </Show>
    </div>
  );
}

function Row(props: {
  instanceId: string;
  item: InstalledContent;
  locked: boolean;
  run: (action: Promise<string | null>) => Promise<void>;
}) {
  const busy = () => isBusy(props.instanceId, props.item.projectId);
  const update = () => updateFor(props.instanceId, props.item.projectId);
  const frozen = () => props.locked || busy();

  return (
    <li class="flex items-center gap-3 px-3 py-2.5">
      <span class="transition-opacity" classList={{ "opacity-40 grayscale": !props.item.enabled }}>
        <ProjectIcon url={props.item.iconUrl} size={36} />
      </span>
      <div class="flex min-w-0 flex-1 flex-col" classList={{ "opacity-60": !props.item.enabled }}>
        <span class="flex items-center gap-2">
          <span class="truncate text-sm font-medium">{props.item.title}</span>
          <Show when={props.item.isDependency}>
            <span class="chip h-5 px-1.5 text-[11px] text-muted">dépendance</span>
          </Show>
          <Show when={!props.item.enabled}>
            <span class="chip h-5 px-1.5 text-[11px] text-muted">désactivé</span>
          </Show>
        </span>
        <span class="flex min-w-0 items-center gap-1.5 font-mono text-xs text-muted">
          <span class="truncate">{props.item.versionNumber}</span>
          <Show when={update()}>
            {(u) => (
              <>
                <Icon name="arrow" size={9} class="shrink-0 text-xp-text" />
                <span class="truncate text-xp-text">{u().newVersion}</span>
              </>
            )}
          </Show>
        </span>
      </div>
      <Show when={update()}>
        <button
          class="btn px-corners h-8 shrink-0 px-2.5 text-xs"
          disabled={frozen()}
          onClick={() => void props.run(updateContent(props.instanceId, [props.item.projectId]))}
        >
          <Icon name="download" size={10} />
          {busy() ? "Mise à jour…" : "Mettre à jour"}
        </button>
      </Show>
      <Toggle
        checked={props.item.enabled}
        label={props.item.enabled ? `Désactiver ${props.item.title}` : `Activer ${props.item.title}`}
        disabled={frozen()}
        onChange={(enabled) => void props.run(setContentEnabled(props.instanceId, props.item.projectId, enabled))}
      />
      <button
        class="btn btn-ghost h-8 w-8 px-0 hover:text-redstone-text"
        aria-label={`Retirer ${props.item.title}`}
        title="Retirer"
        disabled={frozen()}
        onClick={() => void props.run(removeContent(props.instanceId, props.item.projectId))}
      >
        <Icon name="trash" size={12} />
      </button>
    </li>
  );
}
