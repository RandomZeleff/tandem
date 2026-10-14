import { createSignal, Match, Show, Switch } from "solid-js";
import Alert from "../components/Alert";
import ContentList from "../components/ContentList";
import CreatorsTab from "../components/CreatorsTab";
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
import InstanceVisual from "../components/InstanceVisual";
import ScreenshotsTab from "../components/ScreenshotsTab";
import SupportReminder from "../components/SupportReminder";
import Tabs, { tabPanel } from "../components/Tabs";
import TranslationTab from "../components/translation/TranslationTab";
import { api, errorMessage, type InstallProgress, type Instance, isTransferStage } from "../lib/api";
import { formatBytes } from "../lib/format";
import { gameState } from "../lib/games";
import { navigate, refetchInstances, remembered } from "../lib/store";

type Tab = "console" | "content" | "worlds" | "screenshots" | "translation" | "creators" | "info";

function stageLabel(p: InstallProgress | undefined): string {
  if (!p || p.stage === "metadata") return "Lecture des métadonnées Mojang…";
  if (p.stage === "finalizing") return "Finalisation…";
  if (p.stage === "processing") return "Préparation du loader (premier lancement, ~30 s)…";
  if (p.totalFiles === 0) return "Vérification des fichiers…";
  if (p.stage === "copying")
    return `Copie depuis l'autre launcher : ${p.doneFiles} / ${p.totalFiles} fichiers · ${formatBytes(p.doneBytes)} / ${formatBytes(p.totalBytes)}`;
  return `${p.doneFiles} / ${p.totalFiles} fichiers · ${formatBytes(p.doneBytes)} / ${formatBytes(p.totalBytes)}`;
}

export default function InstanceDetail(props: { instance: Instance; tab?: string; world?: string }) {
  const [tab, setTab] = remembered<Tab>("tab", (props.tab as Tab | undefined) ?? "content");
  const [confirmDelete, setConfirmDelete] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const state = () => gameState(props.instance.id);
  const ratio = () => {
    const p = state().progress;
    return p && isTransferStage(p.stage) && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : 0;
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
    { id: "content", label: "Contenu" },
    { id: "worlds", label: "Mondes" },
    { id: "screenshots", label: "Captures" },
    { id: "translation", label: "Traduction" },
    { id: "creators", label: "Créateurs" },
    { id: "console", label: "Console" },
    { id: "info", label: "Réglages" },
  ];

  return (
    <div class="flex h-full flex-col">
      <section aria-label="En-tête de l'instance" class="relative h-[212px] shrink-0 overflow-hidden bg-[#1E1A33]">
        <InstanceVisual instance={props.instance} fallback="scene" />
        <div class="absolute inset-0 bg-[linear-gradient(0deg,rgb(27_28_31)_0%,rgb(27_28_31/0.75)_35%,transparent_80%)]" />
        <nav aria-label="Fil d'Ariane" class="absolute top-4 left-7 flex items-center gap-1.5 text-meta text-chalk-2 [text-shadow:0_1px_2px_rgb(0_0_0/0.6)]">
          <button class="font-medium hover:text-chalk focus-visible:text-chalk" onClick={() => navigate({ page: "instances" })}>
            Instances
          </button>
          <span class="text-chalk-3/70">/</span>
          <span class="max-w-[40ch] truncate">{props.instance.name}</span>
        </nav>
        <div class="absolute right-7 bottom-5 left-7 flex items-end gap-5">
          <span class="rounded-md shadow-[0_0_0_3px_rgb(27_28_31),0_6px_16px_rgb(0_0_0/0.4)]">
            <InstanceSlot instance={props.instance} size={84} />
          </span>
          <div class="flex min-w-0 flex-1 flex-col gap-2 pb-0.5">
            <h1 class="title-display truncate text-title">{props.instance.name}</h1>
            <span class="text-sm text-chalk-2">
              Minecraft {props.instance.gameVersion} · <LoaderTag loader={props.instance.loader} />
              <Show when={props.instance.packVersion}> · version {props.instance.packVersion}</Show>
            </span>
          </div>
          <div class="flex items-center gap-2.5">
            <button
              class="btn px-corners-md size-[54px] px-0"
              aria-label="Ouvrir le dossier"
              title="Ouvrir le dossier"
              onClick={() => api.openInstanceFolder(props.instance.id).catch((err) => setError(errorMessage(err)))}
            >
              <Icon name="folder" size={24} />
            </button>
            <PlayButton id={props.instance.id} size="lg" name={props.instance.name} />
          </div>
        </div>
      </section>

      <Tabs
        label="Sections"
        idPrefix="instance"
        class="shrink-0 px-6 shadow-[inset_0_-1px_0_var(--color-divider)]"
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
        <Show when={state().status === "idle"}>
          <SupportReminder instance={props.instance} onShow={() => setTab("creators")} />
        </Show>
        <Show when={props.instance.packProjectId && state().status === "idle"}>
          <PackUpdateBanner instance={props.instance} locked={state().status !== "idle"} />
        </Show>
        <Show when={state().status === "preparing"}>
          <div class="panel px-corners-md flex flex-col gap-2.5 p-4">
            <div class="flex items-center justify-between">
              <span class="panel-title">Installation</span>
              <Show when={state().progress && isTransferStage(state().progress!.stage)}>
                <span class="font-mono text-sm text-xp-text">{Math.round(ratio() * 100)} %</span>
              </Show>
            </div>
            {/* A bar only when there is an amount to fill; the other steps say what they do. */}
            <Show when={state().progress && isTransferStage(state().progress!.stage)}>
              <XpBar value={ratio()} segments={32} height={14} label="Installation de l'instance" />
            </Show>
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
              <WorldsTab instanceId={props.instance.id} locked={state().status !== "idle"} openWorld={props.world} />
            </Match>
            <Match when={tab() === "screenshots"}>
              <ScreenshotsTab instanceId={props.instance.id} />
            </Match>
            <Match when={tab() === "translation"}>
              <TranslationTab instance={props.instance} locked={state().status !== "idle"} />
            </Match>
            <Match when={tab() === "creators"}>
              <CreatorsTab instance={props.instance} />
            </Match>
            <Match when={tab() === "info"}>
              <InstanceSettingsTab instance={props.instance} locked={state().status !== "idle"} onDelete={() => setConfirmDelete(true)} />
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
