import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage, type Loader, type ProjectDetails, type ProjectVersions, type VersionSummary } from "../../lib/api";
import { formatBytes, formatCount, formatRelative, loaderLabel } from "../../lib/format";
import { formatDate, releasesNewestFirst, versionRange, versionTypeLabel } from "../../lib/projects";
import { remembered } from "../../lib/store";
import Alert from "../Alert";
import LoadingRows from "../LoadingRows";
import { Checkbox, Icon, LoaderIcon } from "../pixel";
import Select from "../Select";
import RichText from "./RichText";

const PAGE = 25;

const KNOWN_LOADERS = new Set(["vanilla", "fabric", "quilt", "forge", "neoforge"]);

/** Every version of a project, filterable, with its changelog and an install button. */
export default function VersionsTab(props: {
  details: ProjectDetails;
  versions: ProjectVersions | undefined;
  loading: boolean;
  error: unknown;
  onRetry: () => void;
  /** Label of the compatibility filter, absent when there is nothing to be compatible with. */
  compatibleLabel: string | null;
  installedVersionId: string | null;
  /** Null when the player cannot install from here (no instance, wrong type). */
  onInstall: ((version: VersionSummary) => void) | null;
  busy: boolean;
  instanceId?: string;
}) {
  const [onlyCompatible, setOnlyCompatible] = remembered("versions.compatible", true);
  const [channel, setChannel] = remembered<"all" | "beta" | "release">("versions.channel", "all");
  const [game, setGame] = remembered("versions.game", "");
  const [loader, setLoader] = remembered("versions.loader", "");
  const [shown, setShown] = createSignal(PAGE);
  const [open, setOpen] = createSignal<string | null>(null);

  const loaders = () => props.details.loaders.filter((l) => l !== "minecraft");
  const filtered = createMemo(() =>
    (props.versions?.versions ?? []).filter(
      (v) =>
        (!onlyCompatible() || !props.compatibleLabel || v.compatible) &&
        (channel() === "all" || v.versionType === "release" || (channel() === "beta" && v.versionType === "beta")) &&
        (!game() || v.gameVersions.includes(game())) &&
        (!loader() || v.loaders.includes(loader())),
    ),
  );
  const filtersActive = () => (onlyCompatible() && !!props.compatibleLabel) || channel() !== "all" || !!game() || !!loader();

  return (
    <div class="flex flex-col gap-3">
      <div class="flex flex-wrap items-center gap-2">
        <Show when={props.compatibleLabel}>
          <Checkbox class="mr-2 text-[13px]" checked={onlyCompatible()} label={props.compatibleLabel!} onChange={setOnlyCompatible} />
        </Show>
        <Select
          class="w-44 text-[13px]"
          label="Type de version"
          value={channel()}
          options={[
            { value: "all", label: "Toutes les versions" },
            { value: "beta", label: "Stables et bêtas" },
            { value: "release", label: "Stables seulement" },
          ]}
          onChange={(v) => setChannel(v as "all" | "beta" | "release")}
        />
        <Select
          class="w-52 text-[13px]"
          label="Version du jeu"
          value={game()}
          options={[
            { value: "", label: "Toutes les versions du jeu" },
            ...releasesNewestFirst(props.details.gameVersions).map((v) => ({ value: v, label: `Minecraft ${v}` })),
          ]}
          onChange={setGame}
        />
        <Show when={loaders().length > 1}>
          <Select
            class="w-40 text-[13px]"
            label="Loader"
            value={loader()}
            options={[
              { value: "", label: "Tous les loaders" },
              ...loaders().map((l) => ({
                value: l,
                label: loaderLabel(l),
                icon: KNOWN_LOADERS.has(l) ? <LoaderIcon loader={l as Loader} size={12} /> : undefined,
              })),
            ]}
            onChange={setLoader}
          />
        </Show>
        <span class="ml-auto font-mono text-xs text-muted">
          <Show when={props.versions}>
            {filtered().length} / {props.versions!.versions.length}
          </Show>
        </span>
      </div>

      <Show when={!props.error} fallback={<Retry error={props.error} onRetry={props.onRetry} />}>
        <Show when={props.versions} fallback={<LoadingRows count={5} height={58} label="Chargement des versions…" />}>
          <Show
            when={filtered().length > 0}
            fallback={
              <div class="panel px-corners-md flex flex-col items-center gap-2 py-10 text-center">
                <p class="text-chalk-2">Aucune version ne correspond.</p>
                <Show when={filtersActive()}>
                  <button
                    class="btn btn-ghost h-8 text-xs"
                    onClick={() => {
                      setOnlyCompatible(false);
                      setChannel("all");
                      setGame("");
                      setLoader("");
                    }}
                  >
                    Retirer les filtres
                  </button>
                </Show>
              </div>
            }
          >
            <ul class="panel px-corners-md flex flex-col divide-y divide-line">
              <For each={filtered().slice(0, shown())}>
                {(version) => (
                  <Row
                    projectId={props.details.id}
                    version={version}
                    recommended={version.id === props.versions?.recommended}
                    installed={version.id === props.installedVersionId}
                    open={open() === version.id}
                    onToggle={() => setOpen(open() === version.id ? null : version.id)}
                    onInstall={props.onInstall && version.compatible ? () => props.onInstall!(version) : null}
                    busy={props.busy}
                    instanceId={props.instanceId}
                  />
                )}
              </For>
            </ul>
            <Show when={filtered().length > shown()}>
              <button class="btn px-corners mx-auto" onClick={() => setShown(shown() + PAGE)}>
                Voir plus ({filtered().length - shown()})
              </button>
            </Show>
          </Show>
        </Show>
      </Show>
    </div>
  );
}

const TYPE_CLASSES: Record<string, string> = {
  release: "text-xp-text",
  beta: "text-gold",
  alpha: "text-redstone-text",
};

function Row(props: {
  projectId: string;
  version: VersionSummary;
  recommended: boolean;
  installed: boolean;
  open: boolean;
  onToggle: () => void;
  onInstall: (() => void) | null;
  busy: boolean;
  instanceId?: string;
}) {
  const v = () => props.version;
  const [changelog] = createResource(
    () => props.open && v().hasChangelog && v().id,
    (id) => api.versionChangelog(props.projectId, id),
  );
  const title = () => (v().name && v().name !== v().versionNumber ? v().name : null);

  return (
    <li class="flex flex-col">
      <div class="flex items-center gap-3 px-3.5 py-2.5">
        <button
          class="flex min-w-0 flex-1 items-center gap-3 text-left"
          aria-expanded={props.open}
          onClick={props.onToggle}
          title={v().hasChangelog ? "Voir le journal des changements" : undefined}
        >
          <Icon name="caret" size={10} class={`shrink-0 text-muted transition-transform ${props.open ? "" : "-rotate-90"}`} />
          <div class="flex min-w-0 flex-1 flex-col gap-0.5">
            <span class="flex min-w-0 items-center gap-2">
              <span class="truncate font-mono text-[13px] text-chalk">{v().versionNumber}</span>
              <span class={`shrink-0 text-[11px] font-semibold uppercase tracking-wide ${TYPE_CLASSES[v().versionType] ?? "text-muted"}`}>
                {versionTypeLabel(v().versionType)}
              </span>
              <Show when={props.recommended}>
                <span class="chip h-5 shrink-0 px-1.5 text-[11px] text-xp-text">recommandée</span>
              </Show>
              <Show when={props.installed}>
                <span class="chip h-5 shrink-0 px-1.5 text-[11px] text-xp-text">
                  <Icon name="check" size={9} />
                  installée
                </span>
              </Show>
            </span>
            <span class="flex min-w-0 items-center gap-1.5 text-xs text-muted">
              <Show when={title()}>
                <span class="truncate text-chalk-3">{title()}</span>
                <span>·</span>
              </Show>
              <span class="shrink-0">{versionRange(v().gameVersions) ? `Minecraft ${versionRange(v().gameVersions)}` : ""}</span>
              <For each={v().loaders.filter((l) => KNOWN_LOADERS.has(l))}>
                {(l) => <LoaderIcon loader={l as Loader} size={11} class="shrink-0" />}
              </For>
            </span>
          </div>
          <span class="flex shrink-0 flex-col items-end text-xs text-muted">
            <span title={formatDate(v().datePublished)}>{formatRelative(v().datePublished)}</span>
            <span class="flex items-center gap-1">
              <Icon name="download" size={9} />
              {formatCount(v().downloads)}
            </span>
          </span>
        </button>
        <Show when={props.onInstall} fallback={<span class="w-[92px] shrink-0" />}>
          <button
            class="btn px-corners h-8 w-[92px] shrink-0 px-2.5 text-xs"
            disabled={props.busy || props.installed}
            onClick={() => props.onInstall!()}
          >
            <Icon name="download" size={10} />
            Installer
          </button>
        </Show>
      </div>
      <Show when={props.open}>
        <div class="flex flex-col gap-3 border-t border-line bg-slate-750 px-10 py-4">
          <span class="text-xs text-muted">
            Publiée le {formatDate(v().datePublished)}
            <Show when={v().fileName}>
              {" "}· {v().fileName} · {formatBytes(v().size)}
            </Show>
            <Show when={v().gameVersions.length > 1}> · Minecraft {v().gameVersions.join(", ")}</Show>
          </span>
          <Show when={v().hasChangelog} fallback={<p class="text-sm text-muted">Pas de journal des changements pour cette version.</p>}>
            <Show
              when={!changelog.error}
              fallback={<Alert>Impossible de lire le journal : {errorMessage(changelog.error)}</Alert>}
            >
              <Show when={changelog() !== undefined} fallback={<div class="skeleton h-16" />}>
                <RichText html={changelog()!} instanceId={props.instanceId} class="text-[13px]" />
              </Show>
            </Show>
          </Show>
        </div>
      </Show>
    </li>
  );
}

export function Retry(props: { error: unknown; onRetry: () => void }) {
  return (
    <div class="panel px-corners-md flex flex-col items-center gap-3 py-10 text-center">
      <p class="text-chalk-2">Impossible de joindre Modrinth.</p>
      <p class="max-w-md text-xs text-muted">{errorMessage(props.error)}</p>
      <button class="btn px-corners h-9" onClick={props.onRetry}>
        Réessayer
      </button>
    </div>
  );
}
