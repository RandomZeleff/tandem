import { createSignal, For, Match, Show, Switch } from "solid-js";
import ContentList from "../components/ContentList";
import Dialog from "../components/Dialog";
import GameConsole from "../components/GameConsole";
import InstanceSlot from "../components/InstanceSlot";
import { Icon, LoaderTag, XpBar } from "../components/pixel";
import PlayButton from "../components/PlayButton";
import Scene from "../components/Scene";
import { api, errorMessage, type InstallProgress, type Instance } from "../lib/api";
import { formatBytes, formatRelative } from "../lib/format";
import { gameState } from "../lib/games";
import { navigate, refetchInstances } from "../lib/store";

type Tab = "console" | "content" | "info";

function stageLabel(p: InstallProgress | undefined): string {
  if (!p || p.stage === "metadata") return "Lecture des métadonnées Mojang…";
  if (p.stage === "finalizing") return "Finalisation…";
  if (p.totalFiles === 0) return "Vérification des fichiers…";
  return `${p.doneFiles} / ${p.totalFiles} fichiers · ${formatBytes(p.doneBytes)} / ${formatBytes(p.totalBytes)}`;
}

export default function InstanceDetail(props: { instance: Instance }) {
  const [tab, setTab] = createSignal<Tab>("console");
  const [confirmDelete, setConfirmDelete] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const state = () => gameState(props.instance.id);
  const ratio = () => {
    const p = state().progress;
    return p && p.stage === "downloading" && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : 0;
  };

  async function remove() {
    try {
      await api.deleteInstance(props.instance.id);
      setConfirmDelete(false);
      navigate({ page: "instances" });
      await refetchInstances();
    } catch (err) {
      setConfirmDelete(false);
      setError(errorMessage(err));
    }
  }

  const tabs: { id: Tab; label: string }[] = [
    { id: "console", label: "Console" },
    { id: "content", label: "Contenu" },
    { id: "info", label: "Informations" },
  ];

  return (
    <div class="flex h-full flex-col">
      <section aria-label="En-tête de l'instance" class="relative h-[196px] shrink-0 bg-[#3E6FB0]">
        <Scene variant="day" />
        <div class="absolute inset-0 bg-[linear-gradient(180deg,transparent_30%,rgb(17_19_22/0.9)_100%)]" />
        <div class="absolute right-7 bottom-5 left-7 flex items-end gap-[18px]">
          <InstanceSlot
            instance={props.instance}
            size={84}
            style={{ "box-shadow": "inset 3px 3px 0 #373737, inset -3px -3px 0 #fff, 0 0 0 4px var(--color-slate-800)" }}
          />
          <div class="flex min-w-0 flex-1 flex-col gap-2">
            <nav aria-label="Fil d'Ariane" class="text-xs text-[#C9CCD1]">
              <button class="hover:text-chalk" onClick={() => navigate({ page: "instances" })}>
                Instances
              </button>
              <span class="px-1.5 text-faint">/</span>
              {props.instance.name}
            </nav>
            <h1 class="truncate font-pixel text-[38px] leading-none font-bold [text-shadow:3px_3px_0_rgb(0_0_0/0.5)]">
              {props.instance.name}
            </h1>
            <span class="text-[13px] text-chalk-2">
              Minecraft {props.instance.gameVersion} · <LoaderTag loader={props.instance.loader} size={13} />
            </span>
          </div>
          <div class="flex items-stretch gap-2">
            <button
              class="btn btn-danger px-corners-md h-[50px] w-12 px-0"
              aria-label="Supprimer l'instance"
              disabled={state().status !== "idle"}
              onClick={() => setConfirmDelete(true)}
            >
              <Icon name="trash" size={16} />
            </button>
            <button
              class="btn px-corners-md h-[50px] w-12 bg-slate-700/90 px-0"
              aria-label="Ouvrir le dossier"
              onClick={() => void api.openInstanceFolder(props.instance.id)}
            >
              <Icon name="folder" size={16} />
            </button>
            <PlayButton id={props.instance.id} size="lg" name={props.instance.name} />
          </div>
        </div>
      </section>

      <div role="tablist" aria-label="Sections" class="flex shrink-0 gap-1 px-6 shadow-[inset_0_-1px_0_var(--color-line)]">
        <For each={tabs}>
          {(t) => (
            <button
              role="tab"
              aria-selected={tab() === t.id}
              class="h-11 px-3.5"
              classList={{
                "font-semibold text-chalk shadow-[inset_0_-3px_0_#8BE04E]": tab() === t.id,
                "font-medium text-muted hover:text-chalk": tab() !== t.id,
              }}
              onClick={() => setTab(t.id)}
            >
              {t.label}
            </button>
          )}
        </For>
      </div>

      <div class="flex min-h-0 flex-1 flex-col gap-4 px-6 pt-4 pb-5">
        <Show when={error() ?? state().error}>
          <p class="bg-[#2A1414] px-3 py-2 text-sm text-redstone-text shadow-[inset_0_0_0_1px_#6E2A26]">
            {error() ?? state().error}
          </p>
        </Show>
        <Show when={state().lastExit && !state().lastExit!.stopped && state().lastExit!.code !== 0}>
          <p class="bg-[#2A1F0E] px-3 py-2 text-sm text-gold shadow-[inset_0_0_0_1px_var(--color-gold-deep)]">
            Le jeu s'est arrêté avec le code {state().lastExit!.code ?? "?"}.
            <Show when={state().lastExit!.crashReport}> Un rapport de crash a été écrit dans le dossier crash-reports.</Show>
          </p>
        </Show>
        <Show when={state().status === "preparing"}>
          <div class="panel px-corners-md flex flex-col gap-2.5 p-4">
            <div class="flex items-center justify-between">
              <span class="panel-title">Installation</span>
              <span class="font-mono text-sm text-xp-text">{Math.round(ratio() * 100)} %</span>
            </div>
            <XpBar value={ratio()} segments={32} height={14} label="Installation de l'instance" />
            <span class="font-mono text-xs text-muted">{stageLabel(state().progress)}</span>
          </div>
        </Show>

        <div class="min-h-0 flex-1">
          <Switch>
            <Match when={tab() === "console"}>
              <GameConsole instanceId={props.instance.id} />
            </Match>
            <Match when={tab() === "content"}>
              <ContentList instance={props.instance} locked={state().status !== "idle"} />
            </Match>
            <Match when={tab() === "info"}>
              <dl class="panel px-corners-md grid max-w-xl grid-cols-[auto_1fr] gap-x-8 gap-y-3 p-5 text-sm">
                <dt class="text-muted">Version</dt>
                <dd>Minecraft {props.instance.gameVersion}</dd>
                <dt class="text-muted">Loader</dt>
                <dd>
                  <LoaderTag loader={props.instance.loader} />
                  <Show when={props.instance.loaderVersion}>
                    <span class="font-mono text-xs text-muted"> {props.instance.loaderVersion}</span>
                  </Show>
                </dd>
                <Show when={props.instance.packProjectId}>
                  <dt class="text-muted">Modpack</dt>
                  <dd>
                    Modrinth
                    <span class="font-mono text-xs text-muted"> {props.instance.packVersion}</span>
                  </dd>
                </Show>
                <dt class="text-muted">Java</dt>
                <dd>{props.instance.javaPath ?? "Automatique (fourni par Mojang)"}</dd>
                <dt class="text-muted">Mémoire</dt>
                <dd>{props.instance.memoryMb ? `${props.instance.memoryMb / 1024} Go` : "4 Go (par défaut)"}</dd>
                <dt class="text-muted">Dernière partie</dt>
                <dd>{formatRelative(props.instance.lastPlayedAt)}</dd>
                <dt class="text-muted">Identifiant</dt>
                <dd class="font-mono text-xs">{props.instance.id}</dd>
              </dl>
            </Match>
          </Switch>
        </div>
      </div>

      <Show when={confirmDelete()}>
        <Dialog title="Supprimer l'instance ?" onClose={() => setConfirmDelete(false)}>
          <p class="text-chalk-2">
            « {props.instance.name} » et tout son dossier seront supprimés, <strong>mondes compris</strong>. Cette action est
            définitive.
          </p>
          <div class="flex justify-end gap-2">
            <button class="btn btn-ghost" onClick={() => setConfirmDelete(false)}>
              Annuler
            </button>
            <button class="btn btn-danger px-corners" onClick={remove}>
              <Icon name="trash" size={12} />
              Supprimer
            </button>
          </div>
        </Dialog>
      </Show>
    </div>
  );
}
