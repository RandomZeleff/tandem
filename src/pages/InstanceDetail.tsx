import { createSignal, Match, Show, Switch } from "solid-js";
import Alert from "../components/Alert";
import ContentList from "../components/ContentList";
import CrashPanel from "../components/CrashPanel";
import Dialog from "../components/Dialog";
import GameConsole from "../components/GameConsole";
import GameStatsPanel from "../components/GameStatsPanel";
import InstanceSlot from "../components/InstanceSlot";
import MissingDepsPanel from "../components/MissingDepsPanel";
import { Icon, LoaderTag, XpBar } from "../components/pixel";
import InstanceSettingsTab from "../components/InstanceSettingsTab";
import PackUpdateBanner from "../components/PackUpdate";
import PlayButton from "../components/PlayButton";
import WorldsTab from "../components/WorldsTab";
import Scene from "../components/Scene";
import ScreenshotsTab from "../components/ScreenshotsTab";
import Tabs, { tabPanel } from "../components/Tabs";
import TranslationTab from "../components/translation/TranslationTab";
import { api, errorMessage, type InstallProgress, type Instance } from "../lib/api";
import { formatBytes } from "../lib/format";
import { gameState } from "../lib/games";
import { navigate, refetchInstances, remembered } from "../lib/store";

type Tab = "console" | "content" | "worlds" | "screenshots" | "translation" | "info";

function stageLabel(p: InstallProgress | undefined): string {
  if (!p || p.stage === "metadata") return "Lecture des métadonnées Mojang…";
  if (p.stage === "finalizing") return "Finalisation…";
  if (p.stage === "processing") return "Préparation du loader (premier lancement, ~30 s)…";
  if (p.totalFiles === 0) return "Vérification des fichiers…";
  return `${p.doneFiles} / ${p.totalFiles} fichiers · ${formatBytes(p.doneBytes)} / ${formatBytes(p.totalBytes)}`;
}

export default function InstanceDetail(props: { instance: Instance }) {
  const [tab, setTab] = remembered<Tab>("tab", "console");
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
    { id: "worlds", label: "Mondes" },
    { id: "screenshots", label: "Captures" },
    { id: "translation", label: "Traduction" },
    { id: "info", label: "Réglages" },
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
            <nav aria-label="Fil d'Ariane" class="text-xs text-chalk-3">
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
              onClick={() => api.openInstanceFolder(props.instance.id).catch((err) => setError(errorMessage(err)))}
            >
              <Icon name="folder" size={16} />
            </button>
            <PlayButton id={props.instance.id} size="lg" name={props.instance.name} />
          </div>
        </div>
      </section>

      <Tabs
        label="Sections"
        idPrefix="instance"
        class="shrink-0 px-6 shadow-[inset_0_-1px_0_var(--color-line)]"
        items={tabs}
        value={tab()}
        onChange={setTab}
      />

      <div class="flex min-h-0 flex-1 flex-col gap-4 px-6 pt-4 pb-5">
        <Show when={error() ?? state().error}>
          {(message) => <Alert onClose={() => setError(null)}>{message()}</Alert>}
        </Show>
        <Show when={state().status === "idle" && state().lastExit?.analysis ? state().lastExit : undefined}>
          {(exit) => (
            <CrashPanel
              instanceId={props.instance.id}
              exit={exit()}
              locked={state().status !== "idle"}
              onShowMemory={() => setTab("info")}
              onError={setError}
            />
          )}
        </Show>
        <MissingDepsPanel instanceId={props.instance.id} />
        <Show when={props.instance.packProjectId && state().status === "idle"}>
          <PackUpdateBanner instance={props.instance} locked={state().status !== "idle"} />
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

        <Show when={state().status === "running"}>
          <GameStatsPanel instanceId={props.instance.id} />
        </Show>

        <div {...tabPanel("instance", tab())} class="min-h-0 flex-1">
          <Switch>
            <Match when={tab() === "console"}>
              <GameConsole instanceId={props.instance.id} />
            </Match>
            <Match when={tab() === "content"}>
              <ContentList instance={props.instance} locked={state().status !== "idle"} />
            </Match>
            <Match when={tab() === "worlds"}>
              <WorldsTab instanceId={props.instance.id} locked={state().status !== "idle"} />
            </Match>
            <Match when={tab() === "screenshots"}>
              <ScreenshotsTab instanceId={props.instance.id} />
            </Match>
            <Match when={tab() === "translation"}>
              <TranslationTab instance={props.instance} locked={state().status !== "idle"} />
            </Match>
            <Match when={tab() === "info"}>
              <InstanceSettingsTab instance={props.instance} locked={state().status !== "idle"} />
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
