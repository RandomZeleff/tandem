import { createResource, createSignal, type JSX, Show } from "solid-js";
import Alert from "../components/Alert";
import LogView from "../components/LogView";
import { Icon, Toggle } from "../components/pixel";
import Select from "../components/Select";
import TranslationSettings from "../components/translation/TranslationSettings";
import { api, errorMessage } from "../lib/api";
import { ON_GAME_START_SETTING, type OnGameStart } from "../lib/games";
import { ACTIVATION_SETTING, OPTIMIZATIONS_SETTING } from "../lib/optimizations";
import { checkForUpdate, setUpdateDialog, update } from "../lib/updates";

/** Same key as the toggle in the Worlds tab. */
const AUTO_BACKUP_SETTING = "auto_backup_worlds";

/** One setting: what it does on the left, its control on the right. */
function Row(props: { title: string; description?: string; control: JSX.Element; for?: string }) {
  return (
    <div class="flex items-center justify-between gap-6 py-3.5">
      <div class="flex min-w-0 flex-col gap-0.5">
        <label class="text-sm font-medium" for={props.for}>
          {props.title}
        </label>
        <Show when={props.description}>
          <p class="text-meta text-muted">{props.description}</p>
        </Show>
      </div>
      <div class="shrink-0">{props.control}</div>
    </div>
  );
}

function Section(props: { title: string; children: JSX.Element; class?: string }) {
  return (
    <section class={`panel px-corners-md flex flex-col px-5 pt-4 pb-1.5 ${props.class ?? ""}`}>
      <h2 class="panel-title">{props.title}</h2>
      <div class="flex flex-col divide-y divide-line">{props.children}</div>
    </section>
  );
}

export default function Settings() {
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
    <div class="mx-auto flex h-full w-full max-w-[1400px] flex-col gap-5">
      <h1 class="title-display text-title">Réglages</h1>

      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-5 max-[1180px]:grid-cols-1">
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
      </div>

      <TranslationSettings />

      <section class="panel px-corners-md flex min-h-[260px] flex-1 flex-col gap-3 p-4">
        <h2 class="panel-title">Journal du launcher</h2>
        <div class="min-h-0 flex-1">
          <LogView />
        </div>
      </section>
    </div>
  );
}
