import { createResource, createSignal, type JSX, Show } from "solid-js";
import Alert from "../components/Alert";
import LogView from "../components/LogView";
import { Icon, Toggle } from "../components/pixel";
import Select from "../components/Select";
import { api, errorMessage } from "../lib/api";
import { ON_GAME_START_SETTING, type OnGameStart } from "../lib/games";

/** Same key as the toggle in the Worlds tab. */
const AUTO_BACKUP_SETTING = "auto_backup_worlds";

/** One setting: what it does on the left, its control on the right. */
function Row(props: { title: string; description: string; control: JSX.Element; for?: string }) {
  return (
    <div class="flex items-center justify-between gap-6 py-3.5">
      <div class="flex min-w-0 flex-col gap-0.5">
        <label class="text-sm font-medium" for={props.for}>
          {props.title}
        </label>
        <p class="text-xs text-muted">{props.description}</p>
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

  async function save<T>(key: string, value: T, apply: (v: T) => void) {
    apply(value);
    try {
      await api.setSetting(key, value);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <div class="flex h-full flex-col gap-5">
      <h1 class="pixel-shadow font-pixel text-3xl font-bold">Réglages</h1>

      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-5 max-[1180px]:grid-cols-1">
        <Section title="Jeu">
          <Row
            title="Quand le jeu démarre"
            for="setting-on-start"
            description="Réduit, le launcher revient tout seul quand tu quittes le jeu."
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
            description="Les 5 dernières sauvegardes automatiques de chaque monde sont gardées."
            control={
              <Toggle
                checked={autoBackup() ?? true}
                label="Sauvegarder les mondes après chaque partie"
                onChange={(v) => void save(AUTO_BACKUP_SETTING, v, setAutoBackup)}
              />
            }
          />
        </Section>

        <Section title="Launcher">
          <Row title="Version" description="Tandem, launcher Minecraft." control={<span class="font-mono text-sm">{info()?.version ?? "…"}</span>} />
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

      <section class="panel px-corners-md flex min-h-[260px] flex-1 flex-col gap-3 p-4">
        <h2 class="panel-title">Journal du launcher</h2>
        <div class="min-h-0 flex-1">
          <LogView />
        </div>
      </section>
    </div>
  );
}
