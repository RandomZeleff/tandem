import { createSignal, For, onMount, Show } from "solid-js";
import Alert from "./Alert";
import { api, errorMessage, type ContentKind, type Dependent, type InstalledContent, type Instance } from "../lib/api";
import {
  checkUpdates,
  contentLoaded,
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
import { removeWithUndo, toast } from "../lib/toast";
import Dialog from "./Dialog";
import LoadingRows from "./LoadingRows";
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
  const [exporting, setExporting] = createSignal(false);
  /** Removed but still undoable: hidden until the removal is committed. */
  const [removing, setRemoving] = createSignal<ReadonlySet<string>>(new Set());
  const items = () => installedContent(props.instance.id).filter((i) => !removing().has(i.projectId));

  /** A disable or removal waiting for confirmation because other mods need the item. */
  const [needed, setNeeded] = createSignal<{
    item: InstalledContent;
    verb: "disable" | "remove";
    dependents: Dependent[];
    proceed: () => void;
  } | null>(null);

  /** Runs `proceed` at once, or after confirmation when enabled mods depend on `item`. */
  async function checkDependents(item: InstalledContent, verb: "disable" | "remove", proceed: () => void) {
    const dependents =
      item.kind === "mod" && item.enabled
        ? await api.contentDependents(props.instance.id, item.projectId).catch(() => [])
        : [];
    if (dependents.length === 0) proceed();
    else setNeeded({ item, verb, dependents, proceed });
  }

  function toggle(item: InstalledContent, enabled: boolean) {
    const apply = () => void run(setContentEnabled(props.instance.id, item.projectId, enabled));
    if (enabled) apply();
    else void checkDependents(item, "disable", apply);
  }

  function remove(item: InstalledContent) {
    void checkDependents(item, "remove", () => removeNow(item));
  }

  function removeNow(item: InstalledContent) {
    const toggle = (on: boolean) =>
      setRemoving((set) => {
        const next = new Set(set);
        if (on) next.add(item.projectId);
        else next.delete(item.projectId);
        return next;
      });
    removeWithUndo({
      message: `${item.title} retiré`,
      hide: () => toggle(true),
      show: () => toggle(false),
      commit: async () => {
        const error = await removeContent(props.instance.id, item.projectId);
        toggle(false);
        return error;
      },
    });
  }

  async function updateAll() {
    const count = updates().length;
    const failure = await updateContent(props.instance.id);
    setError(failure);
    if (!failure) toast(count === 1 ? "1 élément mis à jour" : `${count} éléments mis à jour`);
  }
  const updates = () => contentUpdates(props.instance.id);
  const disabledCount = () => items().filter((i) => !i.enabled).length;
  const browse = () => navigate({ page: "discover", instanceId: props.instance.id });

  async function run(action: Promise<string | null>) {
    setError(await action);
  }

  async function exportPack() {
    setExporting(true);
    setError(null);
    try {
      const path = await exportModpackFile(props.instance);
      if (path) toast(`Modpack exporté : ${path}`);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setExporting(false);
    }
  }

  onMount(async () => {
    try {
      await loadContent(props.instance.id);
      // Mod metadata read in the background, so dependency checks are instant later.
      void api.warmModDependencies(props.instance.id).catch(() => {});
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

      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <PerfSuggestions instance={props.instance} locked={props.locked} onError={setError} />

      <Show when={updates().length > 0}>
        <div class="panel px-corners-md flex items-center justify-between gap-4 p-3.5 shadow-[inset_0_0_0_1px_var(--color-xp-deep)]">
          <div class="flex items-center gap-2.5">
            <span class="size-2 bg-xp shadow-[0_0_0_2px_rgb(139_224_78/0.25)]" />
            <span class="text-sm">
              {updates().length === 1 ? "1 mise à jour disponible" : `${updates().length} mises à jour disponibles`}
            </span>
          </div>
          <button
            class="btn btn-primary px-corners h-9"
            disabled={props.locked || isCheckingUpdates(props.instance.id) || isUpdatingAll(props.instance.id)}
            onClick={() => void updateAll()}
          >
            <Icon name="download" size={12} />
            {isUpdatingAll(props.instance.id) ? "Mise à jour…" : "Tout mettre à jour"}
          </button>
        </div>
      </Show>

      <Show when={contentLoaded(props.instance.id)} fallback={<LoadingRows count={4} height={52} label="Chargement du contenu…" />}>
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
                  <For each={list()}>{(item) => <Row instanceId={props.instance.id} item={item} locked={props.locked} run={run} onToggle={(on) => toggle(item, on)} onRemove={() => remove(item)} />}</For>
                </ul>
              </section>
            );
          }}
        </For>
      </Show>
      </Show>

      <Show when={needed()}>
        {(n) => {
          const shown = () => n().dependents.slice(0, 5);
          const more = () => n().dependents.length - shown().length;
          const close = () => setNeeded(null);
          return (
            <Dialog title={n().verb === "disable" ? `Désactiver ${n().item.title} ?` : `Retirer ${n().item.title} ?`} onClose={close}>
              <p class="text-chalk-2">
                {n().dependents.length === 1 ? "Un mod en a besoin" : `${n().dependents.length} mods en ont besoin`} pour
                démarrer. Sans lui, le jeu s'arrêtera sur une erreur au lancement.
              </p>
              <ul class="flex flex-col gap-1 text-sm">
                <For each={shown()}>{(d) => <li class="truncate text-chalk-2">· {d.name}</li>}</For>
                <Show when={more() > 0}>
                  <li class="text-muted">et {more()} autre{more() > 1 ? "s" : ""}</li>
                </Show>
              </ul>
              <div class="flex justify-end gap-2">
                <button class="btn btn-ghost" onClick={close}>
                  Annuler
                </button>
                <button
                  class="btn btn-danger px-corners"
                  onClick={() => {
                    const proceed = n().proceed;
                    close();
                    proceed();
                  }}
                >
                  {n().verb === "disable" ? "Désactiver quand même" : "Retirer quand même"}
                </button>
              </div>
            </Dialog>
          );
        }}
      </Show>
    </div>
  );
}

function Row(props: {
  instanceId: string;
  item: InstalledContent;
  locked: boolean;
  run: (action: Promise<string | null>) => Promise<void>;
  onToggle: (enabled: boolean) => void;
  onRemove: () => void;
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
        onChange={props.onToggle}
      />
      <button
        class="btn btn-ghost h-8 w-8 px-0 hover:text-redstone-text"
        aria-label={`Retirer ${props.item.title}`}
        title="Retirer"
        disabled={frozen()}
        onClick={props.onRemove}
      >
        <Icon name="trash" size={12} />
      </button>
    </li>
  );
}
