import { createResource, createSignal, Match, Show, Switch } from "solid-js";
import Alert from "../components/Alert";
import LogView from "../components/LogView";
import { Icon, type IconName, Toggle } from "../components/pixel";
import Select from "../components/Select";
import { Row, Section } from "../components/SettingsSection";
import Tabs, { tabPanel } from "../components/Tabs";
import TranslationSettings from "../components/translation/TranslationSettings";
import { api, errorMessage } from "../lib/api";
import { ON_GAME_START_SETTING, type OnGameStart } from "../lib/games";
import { ACTIVATION_SETTING, OPTIMIZATIONS_SETTING } from "../lib/optimizations";
import { remembered } from "../lib/store";
import { checkForUpdate, setUpdateDialog, update } from "../lib/updates";

/** Same key as the toggle in the Worlds tab. */
const AUTO_BACKUP_SETTING = "auto_backup_worlds";

type SettingsTab = "game" | "performance" | "ai" | "launcher" | "log";

const TABS: { id: SettingsTab; label: string; icon: IconName }[] = [
  { id: "game", label: "Jeu", icon: "play" },
  { id: "performance", label: "Performances", icon: "bolt" },
  { id: "ai", label: "Agent IA", icon: "sparkle" },
  { id: "launcher", label: "Launcher", icon: "gear" },
  { id: "log", label: "Journal", icon: "terminal" },
];

export default function Settings(props: { section?: string }) {
  const [tab, setTab] = remembered<SettingsTab>("tab", (props.section as SettingsTab | undefined) ?? "game");
  const [info] = createResource(api.appInfo);
  const [error, setError] = createSignal<string | null>(null);
  const [onStart, { mutate: setOnStart }] = createResource(
    async () => (await api.getSetting<OnGameStart>(ON_GAME_START_SETTING)) ?? "keep",
  );
  const [autoBackup, { mutate: setAutoBackup }] = createResource(
    async () => (await api.getSetting<boolean>(AUTO_BACKUP_SETTING)) ?? true,
  );

  const [optimizations, { mutate: setOptimizations }] = createResource(
    async () => (await api.getSetting<boolean>(OPTIMIZATIONS_SETTING)) ?? true,
  );
  const [activation, { mutate: setActivation }] = createResource(
    async () => (await api.getSetting<boolean>(ACTIVATION_SETTING)) ?? true,
  );

  const [checking, setChecking] = createSignal(false);
  const [updateStatus, setUpdateStatus] = createSignal<string | null>(null);
  async function check() {
    setChecking(true);
    setUpdateStatus(null);
    try {
      if (!(await checkForUpdate())) setUpdateStatus("à jour");
    } catch (err) {
      setUpdateStatus(errorMessage(err));
    } finally {
      setChecking(false);
    }
  }

  async function save<T>(key: string, value: T, apply: (v: T) => void) {
    apply(value);
    try {
      await api.setSetting(key, value);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <div class="mx-auto flex h-full w-full max-w-[1100px] flex-col gap-5">
      <h1 class="title-display text-title">Réglages</h1>

      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <div class="flex min-h-0 flex-1 gap-6">
        <Tabs
          label="Réglages"
          idPrefix="settings"
          variant="vertical"
          class="w-44 shrink-0 self-start"
          items={TABS.map((t) => ({
            id: t.id,
            label: (
              <>
                <Icon name={t.icon} size={12} color={tab() === t.id ? "var(--color-grass-light)" : undefined} />
                {t.label}
              </>
            ),
          }))}
          value={tab()}
          onChange={setTab}
        />

        <div {...tabPanel("settings", tab())} class="flex min-w-0 flex-1 flex-col gap-5 pb-5">
          <Switch>
            <Match when={tab() === "game"}>
              <Section title="Jeu">
                <Row
                  title="Quand le jeu démarre"
                  for="setting-on-start"
                  description="Réduit, il revient quand tu quittes le jeu."
                  control={
                    <Select
                      id="setting-on-start"
                      class="h-9 w-48 text-sm"
                      value={onStart() ?? "keep"}
                      options={[
                        { value: "keep", label: "Rester ouvert" },
                        { value: "minimize", label: "Réduire le launcher" },
                      ]}
                      onChange={(v) => void save(ON_GAME_START_SETTING, v as OnGameStart, setOnStart)}
                    />
                  }
                />
                <Row
                  title="Sauvegarder les mondes après chaque partie"
                  description="Les 5 dernières sont gardées."
                  control={
                    <Toggle
                      checked={autoBackup() ?? true}
                      label="Sauvegarder les mondes après chaque partie"
                      onChange={(v) => void save(AUTO_BACKUP_SETTING, v, setAutoBackup)}
                    />
                  }
                />
              </Section>
            </Match>
            <Match when={tab() === "performance"}>
              <Section title="Performances">
                <Row
                  title="Optimisations Tandem"
                  description="Fabric et Quilt, 1.20 à 1.21 : le monde saute le travail inutile. Réglable par instance."
                  control={
                    <Toggle
                      checked={optimizations() ?? true}
                      label="Optimisations Tandem"
                      onChange={(v) => void save(OPTIMIZATIONS_SETTING, v, setOptimizations)}
                    />
                  }
                />
                <Row
                  title="Ralentir les créatures loin des joueurs"
                  description="Au-delà de 32 blocs. Fermes, villageois et combats ne changent pas."
                  control={
                    <Toggle
                      checked={activation() ?? true}
                      disabled={optimizations() === false}
                      label="Ralentir les créatures loin des joueurs"
                      onChange={(v) => void save(ACTIVATION_SETTING, v, setActivation)}
                    />
                  }
                />
              </Section>
            </Match>
            <Match when={tab() === "ai"}>
              <TranslationSettings />
            </Match>
            <Match when={tab() === "launcher"}>
              <Section title="Launcher">
                <div class="flex items-center justify-between gap-6 py-3.5">
                  <div class="flex min-w-0 flex-col gap-0.5">
                    <span class="text-sm font-medium">Version</span>
                    <p class="text-meta text-muted">
                      <span class="font-mono">{info()?.version ?? "…"}</span>
                      {updateStatus() ? ` · ${updateStatus()}` : ""}
                    </p>
                  </div>
                  <Show
                    when={update()}
                    fallback={
                      <button class="btn px-corners h-9 shrink-0 px-3 text-sm" disabled={checking()} onClick={check}>
                        {checking() ? "Vérification…" : "Vérifier les mises à jour"}
                      </button>
                    }
                  >
                    {(found) => (
                      <button class="btn btn-primary px-corners h-9 shrink-0 px-3 text-sm" onClick={() => setUpdateDialog(true)}>
                        <Icon name="download" size={12} />
                        Installer la {found().version}
                      </button>
                    )}
                  </Show>
                </div>
                <div class="flex items-center justify-between gap-6 py-3.5">
                  <div class="flex min-w-0 flex-col gap-0.5">
                    <span class="text-sm font-medium">Dossier des données</span>
                    <p class="truncate font-mono text-xs text-muted select-text" title={info()?.dataDir}>
                      {info()?.dataDir ?? "…"}
                    </p>
                  </div>
                  <button
                    class="btn px-corners h-9 shrink-0 px-3 text-sm"
                    onClick={() => api.openDataFolder().catch((err) => setError(errorMessage(err)))}
                  >
                    <Icon name="folder" size={12} />
                    Ouvrir
                  </button>
                </div>
              </Section>
            </Match>
            <Match when={tab() === "log"}>
              <section class="panel px-corners-md flex min-h-[260px] flex-1 flex-col gap-3 p-4">
                <h2 class="panel-title">Journal du launcher</h2>
                <div class="min-h-0 flex-1">
                  <LogView />
                </div>
              </section>
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  );
}
