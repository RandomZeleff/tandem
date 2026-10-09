import { createEffect, createMemo, createResource, createSignal, For, type JSX, Match, Show, Switch } from "solid-js";
import Alert from "../components/Alert";
import ModpackInstallDialog from "../components/ModpackInstallDialog";
import { Icon, type IconName, LoaderIcon, LoaderTag } from "../components/pixel";
import DependenciesTab from "../components/project/DependenciesTab";
import Gallery from "../components/project/Gallery";
import RichText from "../components/project/RichText";
import VersionsTab, { Retry } from "../components/project/VersionsTab";
import ProjectIcon from "../components/ProjectIcon";
import Select from "../components/Select";
import Tabs, { tabPanel } from "../components/Tabs";
import {
  api,
  errorMessage,
  type Loader,
  PROJECT_NOT_FOUND,
  type ProjectDetails,
  type ProjectLink,
  type ProjectVersions,
  type VersionSummary,
} from "../lib/api";
import { installContent, installedContent, isBusy, isInstalled, loadContent, updateContent, updateFor } from "../lib/content";
import { formatCount, formatRelative, loaderLabel } from "../lib/format";
import { installPack, isInstallingPack } from "../lib/modpacks";
import {
  categoryLabel,
  formatDate,
  openExternal,
  openProject,
  projectTypeLabel,
  sideLabel,
  versionRange,
} from "../lib/projects";
import { goBack, instances, navigate, remembered, setNewInstanceDialog } from "../lib/store";
import { toast } from "../lib/toast";

type Tab = "description" | "gallery" | "versions" | "content" | "dependencies";

const CONTENT_TYPES = new Set(["mod", "resourcepack", "shader"]);
const KNOWN_LOADERS = new Set(["vanilla", "fabric", "quilt", "forge", "neoforge"]);
const IRIS = "YL57xq9U";

/** Full page of a Modrinth project: what it is, who makes it, its versions, and installing it. */
export default function Project(props: { id: string; instanceId?: string }) {
  const [details, { refetch }] = createResource(() => props.id, api.projectDetails);
  // Resources wrap the rejected string in an Error.
  const notFound = () => !!details.error && errorMessage(details.error) === PROJECT_NOT_FOUND;

  return (
    <div class="flex flex-col gap-5">
      <Switch>
        <Match when={notFound()}>
          <div class="panel px-corners-md flex flex-col items-center gap-3 py-16 text-center">
            <p class="text-chalk-2">Ce projet n'existe pas, ou plus, sur Modrinth.</p>
            <button class="btn px-corners h-9" onClick={goBack}>
              <Icon name="arrow" size={12} class="-scale-x-100" />
              Retour
            </button>
          </div>
        </Match>
        <Match when={details.error}>
          <Retry error={details.error} onRetry={() => void refetch()} />
        </Match>
        <Match when={details()}>{(d) => <Page details={d()} instanceId={props.instanceId} />}</Match>
        <Match when={true}>
          <div role="status" aria-label="Chargement du projet…" class="flex flex-col gap-5">
            <div class="skeleton px-corners-lg h-[148px]" />
            <div class="grid grid-cols-[minmax(0,1fr)_260px] gap-5">
              <div class="skeleton px-corners-md h-96" />
              <div class="skeleton px-corners-md h-72" />
            </div>
          </div>
        </Match>
      </Switch>
    </div>
  );
}

function Page(props: { details: ProjectDetails; instanceId?: string }) {
  const d = () => props.details;
  const isContent = () => CONTENT_TYPES.has(d().projectType);
  const isModpack = () => d().projectType === "modpack";

  // Where content would be installed: the instance the player came from, else a sensible one.
  const pickDefault = () =>
    props.instanceId ??
    (d().projectType === "mod" ? instances().find((i) => i.loader !== "vanilla") : undefined)?.id ??
    instances()[0]?.id ??
    "";
  const [instanceId, setInstanceId] = remembered("instance", pickDefault());
  const instance = createMemo(() => instances().find((i) => i.id === instanceId()));
  createEffect(() => {
    if (!instance() && instances().length > 0) setInstanceId(pickDefault());
  });
  createEffect(() => {
    const id = instanceId();
    if (id && isContent()) void loadContent(id);
  });

  const [versions, { refetch: refetchVersions }] = createResource(
    () => ({ id: d().id, instanceId: isContent() ? instanceId() || null : null }),
    ({ id, instanceId }) => api.projectVersions(id, instanceId),
  );
  // Keep the previous list on screen while another instance's compatibility loads.
  const [shownVersions, setShownVersions] = createSignal<ProjectVersions>();
  createEffect(() => {
    const list = versions();
    if (list) setShownVersions(list);
  });
  const recommended = () => {
    const v = versions();
    return v?.versions.find((x) => x.id === v.recommended);
  };

  const installed = () => (instanceId() ? installedContent(instanceId()).find((c) => c.projectId === d().id) : undefined);

  const tabs = () => {
    const list: { id: Tab; label: JSX.Element }[] = [{ id: "description", label: "Description" }];
    if (d().gallery.length > 0) list.push({ id: "gallery", label: <Counted label="Galerie" count={d().gallery.length} /> });
    list.push({ id: "versions", label: <Counted label="Versions" count={shownVersions()?.versions.length} /> });
    if (isModpack()) list.push({ id: "content", label: "Contenu" });
    else if (d().projectType === "mod") list.push({ id: "dependencies", label: "Dépendances" });
    return list;
  };
  const [tab, setTab] = remembered<Tab>("tab", "description");
  const currentTab = () => (tabs().some((t) => t.id === tab()) ? tab() : "description");

  const [error, setError] = createSignal<string | null>(null);
  /** Modpack version picker: the version to preselect, or "" for the recommended one. */
  const [picking, setPicking] = createSignal<string | null>(null);

  async function install(version?: VersionSummary) {
    const target = instance();
    if (!target) return;
    setError(null);
    const failure = await installContent(target.id, d().id, version?.id);
    if (failure) setError(failure);
    else toast(`${d().title} installé dans « ${target.name} »`);
  }

  async function update() {
    setError(null);
    const failure = await updateContent(instanceId(), [d().id]);
    if (failure) setError(failure);
    else toast(`${d().title} mis à jour`);
  }

  async function installModpack(versionId: string) {
    setPicking(null);
    setError(null);
    setError(await installPack(d().id, versionId));
  }

  const busy = () => (isModpack() ? isInstallingPack(d().id) : isBusy(instanceId(), d().id));

  return (
    <>
      <header class="panel px-corners-lg relative flex gap-5 overflow-hidden p-5">
        <Show when={d().color !== null}>
          <span
            aria-hidden="true"
            class="pointer-events-none absolute inset-0 opacity-[0.07]"
            style={{ background: `linear-gradient(110deg, #${d().color!.toString(16).padStart(6, "0")}, transparent 55%)` }}
          />
        </Show>
        <ProjectIcon url={d().iconUrl} size={96} />
        <div class="relative flex min-w-0 flex-1 flex-col gap-2">
          <div class="flex items-center gap-2 text-xs text-muted">
            <span class="chip h-5 px-1.5 text-[11px] text-chalk-3">{projectTypeLabel(d().projectType)}</span>
            <Show when={d().organization ?? d().authors[0]}>
              <span>
                par{" "}
                <button
                  class="text-chalk-3 hover:text-chalk hover:underline focus-visible:underline"
                  onClick={() => openExternal(d().organization?.url ?? d().authors[0].url)}
                >
                  {d().organization?.name ?? d().authors[0].name}
                </button>
                <Show when={!d().organization && d().authors.length > 1}>
                  {" "}et {d().authors.length - 1} autre{d().authors.length > 2 ? "s" : ""}
                </Show>
              </span>
            </Show>
          </div>
          <h1 class="pixel-shadow font-pixel text-3xl leading-tight font-bold">{d().title}</h1>
          <p class="text-[14px] text-chalk-2 select-text">{d().summary}</p>
          <div class="mt-1 flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs text-muted">
            <span class="flex items-center gap-1.5" title={`${d().downloads.toLocaleString("fr-FR")} téléchargements`}>
              <Icon name="download" size={10} />
              {formatCount(d().downloads)}
            </span>
            <span class="flex items-center gap-1.5" title={`${d().followers.toLocaleString("fr-FR")} abonnés`}>
              <Icon name="heart" size={10} />
              {formatCount(d().followers)}
            </span>
            <span title={formatDate(d().updated)}>Mis à jour {formatRelative(d().updated)}</span>
            <div class="flex flex-wrap gap-1.5">
              <For each={d().categories}>{(c) => <span class="chip h-5 px-1.5 text-[11px]">{categoryLabel(c)}</span>}</For>
            </div>
          </div>
        </div>
        <div class="relative flex w-64 shrink-0 flex-col gap-2">
          <Switch>
            <Match when={isModpack()}>
              <ModpackAction
                details={d()}
                recommended={recommended()}
                loading={versions.loading}
                busy={busy()}
                onInstall={() => setPicking("")}
              />
            </Match>
            <Match when={isContent()}>
              <ContentAction
                details={d()}
                instanceId={instanceId()}
                onInstance={setInstanceId}
                recommended={recommended()}
                loading={versions.loading}
                versionsError={!!versions.error}
                installedVersion={installed()?.versionNumber ?? null}
                update={updateFor(instanceId(), d().id)?.newVersion ?? null}
                busy={busy()}
                onInstall={() => void install()}
                onUpdate={() => void update()}
              />
            </Match>
            <Match when={true}>
              <p class="text-[13px] text-chalk-2">
                Tandem n'installe pas encore les {projectTypeLabel(d().projectType).toLowerCase()}s.
              </p>
            </Match>
          </Switch>
          <button class="btn btn-ghost h-8 justify-start px-2 text-xs" onClick={() => openExternal(d().url)}>
            <Icon name="external" size={10} />
            Voir sur Modrinth
          </button>
        </div>
      </header>

      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <div class="grid grid-cols-[minmax(0,1fr)_260px] items-start gap-5">
        <div class="flex min-w-0 flex-col gap-4">
          <Tabs label="Sections du projet" idPrefix="project" items={tabs()} value={currentTab()} onChange={setTab} class="border-b border-line" />
          <div {...tabPanel("project", currentTab())}>
            <Switch>
              <Match when={currentTab() === "description"}>
                <Show
                  when={d().bodyHtml.trim()}
                  fallback={<p class="panel px-corners-md py-12 text-center text-chalk-2">L'auteur n'a pas écrit de description.</p>}
                >
                  <article class="panel px-corners-md p-6">
                    <RichText html={d().bodyHtml} instanceId={isContent() ? instanceId() : undefined} />
                  </article>
                </Show>
              </Match>
              <Match when={currentTab() === "gallery"}>
                <Gallery images={d().gallery} />
              </Match>
              <Match when={currentTab() === "versions"}>
                <VersionsTab
                  details={d()}
                  versions={shownVersions()}
                  loading={versions.loading}
                  error={versions.error}
                  onRetry={() => void refetchVersions()}
                  compatibleLabel={
                    isModpack() ? "Installables par Tandem" : isContent() && instance() ? `Compatibles avec « ${instance()!.name} »` : null
                  }
                  installedVersionId={installed()?.versionId ?? null}
                  onInstall={
                    isModpack()
                      ? (v) => setPicking(v.id)
                      : isContent() && instance() && !installed()
                        ? (v) => void install(v)
                        : null
                  }
                  busy={busy()}
                  instanceId={isContent() ? instanceId() : undefined}
                />
              </Match>
              <Match when={currentTab() === "content" || currentTab() === "dependencies"}>
                <DependenciesTab
                  projectId={d().id}
                  mode={currentTab() === "content" ? "content" : "dependencies"}
                  versions={shownVersions()}
                  versionsError={versions.error}
                  onRetry={() => void refetchVersions()}
                  instanceId={isContent() ? instanceId() || undefined : undefined}
                />
              </Match>
            </Switch>
          </div>
        </div>
        <Aside details={d()} />
      </div>

      {/* Keyed: the dialog keeps its version while it closes. */}
      <Show when={picking() !== null ? { initial: picking()! } : null} keyed>
        {(pick) => (
          <ModpackInstallDialog
            projectId={d().id}
            title={d().title}
            initialVersionId={pick.initial || undefined}
            onClose={() => setPicking(null)}
            onInstall={(versionId) => void installModpack(versionId)}
          />
        )}
      </Show>
    </>
  );
}

function Counted(props: { label: string; count: number | undefined }) {
  return (
    <span class="flex items-center gap-1.5">
      {props.label}
      <Show when={props.count !== undefined}>
        <span class="font-mono text-xs text-muted">{props.count}</span>
      </Show>
    </span>
  );
}

function versionTarget(v: VersionSummary): string {
  const loader = v.loaders.find((l) => KNOWN_LOADERS.has(l));
  return [loader ? loaderLabel(loader) : null, v.gameVersions.length ? versionRange(v.gameVersions) : null].filter(Boolean).join(" ");
}

function ModpackAction(props: {
  details: ProjectDetails;
  recommended: VersionSummary | undefined;
  loading: boolean;
  busy: boolean;
  onInstall: () => void;
}) {
  const existing = () => instances().find((i) => i.packProjectId === props.details.id);
  return (
    <>
      <Show
        when={existing()}
        fallback={
          <button class="btn btn-primary px-corners h-10" disabled={props.busy || (!props.loading && !props.recommended)} onClick={props.onInstall}>
            <Icon name="download" size={12} />
            {props.busy ? "Installation…" : "Installer"}
          </button>
        }
      >
        {(instance) => (
          <>
            <button class="btn btn-primary px-corners h-10" onClick={() => navigate({ page: "instance", id: instance().id })}>
              <Icon name="play" size={12} />
              Ouvrir l'instance
            </button>
            <button class="btn px-corners h-8 text-xs" disabled={props.busy} onClick={props.onInstall}>
              {props.busy ? "Installation…" : "Installer une autre copie"}
            </button>
          </>
        )}
      </Show>
      <span class="text-xs text-muted">
        <Switch>
          <Match when={props.loading}>Recherche de la version à installer…</Match>
          <Match when={props.recommended}>
            {(v) => (
              <span class="flex items-center gap-1.5">
                <Show when={v().loaders.find((l) => KNOWN_LOADERS.has(l))}>
                  {(l) => <LoaderIcon loader={l() as Loader} size={11} />}
                </Show>
                Version {v().versionNumber} · {versionTarget(v())}
              </span>
            )}
          </Match>
          <Match when={true}>Aucune version que Tandem sait lancer.</Match>
        </Switch>
      </span>
    </>
  );
}

function ContentAction(props: {
  details: ProjectDetails;
  instanceId: string;
  onInstance: (id: string) => void;
  recommended: VersionSummary | undefined;
  loading: boolean;
  versionsError: boolean;
  installedVersion: string | null;
  update: string | null;
  busy: boolean;
  onInstall: () => void;
  onUpdate: () => void;
}) {
  const instance = () => instances().find((i) => i.id === props.instanceId);
  const needsLoader = () => props.details.projectType === "mod" && instance()?.loader === "vanilla";
  const needsIris = () =>
    props.details.projectType === "shader" && !!instance() && instance()!.loader !== "vanilla" && !isInstalled(props.instanceId, IRIS);

  return (
    <Show
      when={instances().length > 0}
      fallback={
        <>
          <p class="text-[13px] text-chalk-2">Crée d'abord une instance pour l'y installer.</p>
          <button class="btn btn-primary px-corners h-10" onClick={() => setNewInstanceDialog({})}>
            <Icon name="plus" size={12} />
            Nouvelle instance
          </button>
        </>
      }
    >
      <Select
        class="h-9 w-full text-[13px]"
        label="Instance"
        value={props.instanceId}
        options={instances().map((i) => ({
          value: i.id,
          label: i.name,
          hint: `${i.gameVersion} · ${loaderLabel(i.loader)}`,
          icon: <LoaderIcon loader={i.loader} size={12} />,
        }))}
        onChange={props.onInstance}
      />
      <Switch>
        <Match when={props.installedVersion !== null}>
          <Show
            when={props.update}
            fallback={
              <span class="flex h-10 items-center justify-center gap-1.5 bg-success text-sm text-xp-text shadow-[inset_0_0_0_1px_var(--color-success-line)]">
                <Icon name="check" size={12} />
                Installé
              </span>
            }
          >
            <button class="btn btn-primary px-corners h-10" disabled={props.busy} onClick={props.onUpdate}>
              <Icon name="download" size={12} />
              {props.busy ? "Mise à jour…" : "Mettre à jour"}
            </button>
          </Show>
          <span class="truncate text-xs text-muted">
            Version {props.installedVersion}
            <Show when={props.update}> → {props.update}</Show>
          </span>
        </Match>
        <Match when={needsLoader()}>
          <p class="text-xs text-muted">Les mods ont besoin d'un loader : cette instance est Vanilla.</p>
          <button class="btn px-corners h-9 text-xs" onClick={() => setNewInstanceDialog({ version: instance()?.gameVersion })}>
            <LoaderIcon loader="fabric" size={12} />
            Créer une instance Fabric
          </button>
        </Match>
        <Match when={true}>
          <button
            class="btn btn-primary px-corners h-10"
            disabled={props.busy || props.loading || !props.recommended}
            onClick={props.onInstall}
          >
            <Icon name="download" size={12} />
            {props.busy ? "Installation…" : "Installer"}
          </button>
          <span class="text-xs text-muted">
            <Switch>
              <Match when={props.loading}>Recherche d'une version compatible…</Match>
              <Match when={props.versionsError}>Versions indisponibles pour l'instant.</Match>
              <Match when={props.recommended}>{(v) => <>Version {v().versionNumber}, compatible</>}</Match>
              <Match when={true}>
                <span class="text-gold">
                  Aucune version pour Minecraft {instance()?.gameVersion}
                  {props.details.projectType === "mod" ? ` avec ${loaderLabel(instance()?.loader ?? "")}` : ""}.
                </span>
              </Match>
            </Switch>
          </span>
          <Show when={needsIris()}>
            <span class="text-xs text-muted">
              Les shaders ont besoin d'
              <button class="text-xp-text hover:underline" onClick={() => openProject(IRIS, props.instanceId)}>
                Iris
              </button>
              .
            </span>
          </Show>
        </Match>
      </Switch>
    </Show>
  );
}

const LINKS: Record<ProjectLink["kind"], { icon: IconName; label: string }> = {
  source: { icon: "code", label: "Code source" },
  issues: { icon: "bug", label: "Signaler un problème" },
  wiki: { icon: "book", label: "Wiki" },
  discord: { icon: "chat", label: "Discord" },
  donation: { icon: "coin", label: "Faire un don" },
};

const MAX_AUTHORS = 8;

function Aside(props: { details: ProjectDetails }) {
  const d = () => props.details;
  const loaders = () => d().loaders.filter((l) => l !== "minecraft");
  const side = () => sideLabel(d());
  const [allAuthors, setAllAuthors] = createSignal(false);
  const authors = () => (allAuthors() ? d().authors : d().authors.slice(0, MAX_AUTHORS));

  return (
    <aside class="flex flex-col gap-4">
      <Section title="Compatibilité">
        <dl class="flex flex-col gap-2.5 text-[13px]">
          <Show when={d().gameVersions.length > 0}>
            <div>
              <dt class="text-xs text-muted">Minecraft</dt>
              <dd title={d().gameVersions.join(", ")}>{versionRange(d().gameVersions)}</dd>
            </div>
          </Show>
          <Show when={loaders().length > 0}>
            <div>
              <dt class="text-xs text-muted">Loaders</dt>
              <dd class="flex flex-wrap gap-x-3 gap-y-1">
                <For each={loaders()}>
                  {(l) => (KNOWN_LOADERS.has(l) ? <LoaderTag loader={l as Loader} /> : <span>{loaderLabel(l)}</span>)}
                </For>
              </dd>
            </div>
          </Show>
          <Show when={side()}>
            <div>
              <dt class="text-xs text-muted">Où l'installer</dt>
              <dd>{side()}</dd>
            </div>
          </Show>
        </dl>
      </Section>

      <Show when={d().organization || d().authors.length > 0}>
        <Section title="Créateurs">
          <ul class="flex flex-col gap-1">
            <Show when={d().organization}>
              {(org) => (
                <Person name={org().name} avatar={org().iconUrl} role="Organisation" onClick={() => openExternal(org().url)} />
              )}
            </Show>
            <For each={authors()}>
              {(a) => <Person name={a.name} avatar={a.avatarUrl} role={a.role} onClick={() => openExternal(a.url)} />}
            </For>
          </ul>
          <Show when={!allAuthors() && d().authors.length > MAX_AUTHORS}>
            <button class="btn btn-ghost h-7 px-2 text-xs" onClick={() => setAllAuthors(true)}>
              Voir les {d().authors.length - MAX_AUTHORS} autres
            </button>
          </Show>
        </Section>
      </Show>

      <Show when={d().links.length > 0}>
        <Section title="Liens">
          <ul class="flex flex-col">
            <For each={d().links}>
              {(link) => (
                <li>
                  <button
                    class="flex w-full items-center gap-2.5 px-1 py-1.5 text-left text-[13px] text-chalk-2 hover:text-xp-text focus-visible:text-xp-text"
                    title={link.url}
                    onClick={() => openExternal(link.url)}
                  >
                    <Icon name={LINKS[link.kind].icon} size={12} class="shrink-0 text-muted" />
                    <span class="flex-1 truncate">{link.kind === "donation" && link.label ? `Don (${link.label})` : LINKS[link.kind].label}</span>
                    <Icon name="external" size={9} class="shrink-0 text-faint" />
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Section>
      </Show>

      <Section title="Détails">
        <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-[13px]">
          <Show when={d().license}>
            {(license) => (
              <>
                <dt class="text-muted">Licence</dt>
                <dd class="min-w-0 truncate">
                  <Show when={license().url} fallback={<span title={license().id}>{license().name}</span>}>
                    <button class="truncate text-left hover:text-xp-text hover:underline" title={license().id} onClick={() => openExternal(license().url!)}>
                      {license().name}
                    </button>
                  </Show>
                </dd>
              </>
            )}
          </Show>
          <dt class="text-muted">Publié</dt>
          <dd>{formatDate(d().published)}</dd>
          <dt class="text-muted">Mis à jour</dt>
          <dd>{formatDate(d().updated)}</dd>
          <dt class="text-muted">Téléchargé</dt>
          <dd>{d().downloads.toLocaleString("fr-FR")} fois</dd>
        </dl>
      </Section>
    </aside>
  );
}

function Section(props: { title: string; children: JSX.Element }) {
  return (
    <section class="panel px-corners-md flex flex-col gap-3 p-4">
      <h2 class="panel-title">{props.title}</h2>
      {props.children}
    </section>
  );
}

function Person(props: { name: string; avatar: string | null; role: string; onClick: () => void }) {
  const [failed, setFailed] = createSignal(false);
  return (
    <li>
      <button class="group flex w-full items-center gap-2.5 py-1 text-left" onClick={props.onClick} title={`Profil de ${props.name} sur Modrinth`}>
        <span class="flex size-7 shrink-0 items-center justify-center overflow-hidden bg-slate-900 shadow-[inset_0_0_0_1px_var(--color-line)]">
          <Show when={props.avatar && !failed()} fallback={<Icon name="user" size={12} color="var(--color-faint)" />}>
            <img src={props.avatar!} alt="" loading="lazy" class="size-full object-cover" onError={() => setFailed(true)} />
          </Show>
        </span>
        <span class="flex min-w-0 flex-col">
          <span class="truncate text-[13px] text-chalk group-hover:text-xp-text">{props.name}</span>
          <Show when={props.role}>
            <span class="truncate text-[11px] text-muted">{props.role}</span>
          </Show>
        </span>
      </button>
    </li>
  );
}
