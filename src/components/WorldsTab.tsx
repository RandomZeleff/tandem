import { createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage, type BackupKind, type GameMode, type World, type WorldBackup } from "../lib/api";
import { formatBytes, formatRelative } from "../lib/format";
import { gameState } from "../lib/games";
import Dialog from "./Dialog";
import { Icon, Toggle } from "./pixel";

const MODES: Record<GameMode, string> = {
  survival: "Survie",
  creative: "Créatif",
  adventure: "Aventure",
  spectator: "Spectateur",
  hardcore: "Hardcore",
};

const KINDS: Record<BackupKind, string> = {
  manual: "Manuelle",
  auto: "Automatique",
  beforeRestore: "Avant restauration",
};

const AUTO_BACKUP_SETTING = "auto_backup_worlds";

const iso = (ms: number | null) => (ms === null ? null : new Date(ms).toISOString());

/** Singleplayer worlds of an instance and their backups. */
export default function WorldsTab(props: { instanceId: string; locked: boolean }) {
  const [error, setError] = createSignal<string | null>(null);
  // Re-read after every run: the game changes worlds and Tandem backs them up on exit.
  const source = () => ({ id: props.instanceId, exit: gameState(props.instanceId).lastExit });
  const [worlds, { refetch: refetchWorlds }] = createResource(source, ({ id }) => api.listWorlds(id));
  const [backups, { refetch: refetchBackups }] = createResource(source, ({ id }) => api.listWorldBackups(id));
  const [auto, { mutate: setAuto }] = createResource(async () => (await api.getSetting<boolean>(AUTO_BACKUP_SETTING)) ?? true);
  const [busy, setBusy] = createSignal<string | null>(null);
  const [restoring, setRestoring] = createSignal<WorldBackup | null>(null);

  const backupsOf = (world: string) => (backups() ?? []).filter((b) => b.world === world);

  async function run(key: string, action: () => Promise<unknown>) {
    setBusy(key);
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  }

  async function toggleAuto(value: boolean) {
    setAuto(value);
    await api.setSetting(AUTO_BACKUP_SETTING, value);
  }

  async function confirmRestore() {
    const backup = restoring();
    if (!backup) return;
    setRestoring(null);
    await run(backup.fileName, async () => {
      await api.restoreWorldBackup(props.instanceId, backup.world, backup.fileName);
      await Promise.all([refetchWorlds(), refetchBackups()]);
    });
  }

  return (
    <div class="flex h-full flex-col gap-4 overflow-y-auto">
      <div class="flex items-center justify-between gap-3">
        <span class="text-[13px] text-muted">
          {(worlds() ?? []).length === 0 ? "Aucun monde." : `${worlds()!.length} monde${worlds()!.length > 1 ? "s" : ""}`}
          <Show when={props.locked}> · arrête le jeu pour sauvegarder ou restaurer</Show>
        </span>
        <label class="flex items-center gap-2.5 text-[13px] text-chalk-2">
          Sauvegarde automatique après chaque partie
          <Toggle
            checked={auto() ?? true}
            label="Sauvegarde automatique après chaque partie"
            onChange={(v) => void toggleAuto(v)}
          />
        </label>
      </div>

      <Show when={error()}>
        <p class="bg-[#2A1414] px-3 py-2 text-sm text-redstone-text shadow-[inset_0_0_0_1px_#6E2A26]">{error()}</p>
      </Show>

      <Show
        when={(worlds() ?? []).length > 0}
        fallback={
          <div class="panel px-corners-md flex flex-col items-center gap-2 py-12 text-center">
            <p class="text-chalk-2">Aucun monde pour l'instant.</p>
            <p class="text-sm text-muted">Crée un monde en jeu : il apparaîtra ici, avec ses sauvegardes.</p>
          </div>
        }
      >
        <ul class="flex flex-col gap-3">
          <For each={worlds()}>
            {(world) => (
              <WorldCard
                world={world}
                backups={backupsOf(world.folder)}
                locked={props.locked}
                busy={busy()}
                onBackup={() =>
                  run(world.folder, async () => {
                    await api.backupWorld(props.instanceId, world.folder);
                    await refetchBackups();
                  })
                }
                onRestore={setRestoring}
                onOpen={() => run(`open:${world.folder}`, () => api.openWorldBackups(props.instanceId, world.folder))}
              />
            )}
          </For>
        </ul>
      </Show>
      <p class="text-xs text-faint">Les 5 dernières sauvegardes automatiques de chaque monde sont gardées ; les manuelles, toutes.</p>

      <Show when={restoring()}>
        {(backup) => (
          <Dialog title="Restaurer ce monde ?" onClose={() => setRestoring(null)}>
            <p class="text-chalk-2">
              Le monde revient à son état du {new Date(backup().createdAt).toLocaleString("fr-FR")}. Son état actuel est
              d'abord sauvegardé : tu pourras revenir en arrière.
            </p>
            <div class="flex justify-end gap-2">
              <button class="btn btn-ghost" onClick={() => setRestoring(null)}>
                Annuler
              </button>
              <button class="btn btn-primary px-corners" onClick={() => void confirmRestore()}>
                Restaurer
              </button>
            </div>
          </Dialog>
        )}
      </Show>
    </div>
  );
}

function WorldCard(props: {
  world: World;
  backups: WorldBackup[];
  locked: boolean;
  busy: string | null;
  onBackup: () => void;
  onRestore: (backup: WorldBackup) => void;
  onOpen: () => void;
}) {
  const [open, setOpen] = createSignal(false);
  return (
    <li class="panel px-corners-md flex flex-col">
      <div class="flex items-center gap-3.5 p-3.5">
        <Show when={props.world.icon} fallback={<div class="size-12 shrink-0 bg-slate-700 shadow-[inset_0_0_0_1px_var(--color-line)]" />}>
          {(icon) => <img src={icon()} alt="" class="size-12 shrink-0 [image-rendering:pixelated]" />}
        </Show>
        <div class="flex min-w-0 flex-1 flex-col gap-0.5">
          <span class="truncate text-sm font-medium">{props.world.name}</span>
          <span class="truncate text-xs text-muted">
            {MODES[props.world.gameMode]}
            <Show when={props.world.version}> · {props.world.version}</Show> · {formatBytes(props.world.sizeBytes)} · joué{" "}
            {formatRelative(iso(props.world.lastPlayed))}
          </span>
        </div>
        <button
          class="btn btn-ghost h-8 px-2.5 text-xs"
          aria-expanded={open()}
          disabled={props.backups.length === 0}
          onClick={() => setOpen(!open())}
        >
          {props.backups.length === 0 ? "Aucune sauvegarde" : `Sauvegardes (${props.backups.length})`}
        </button>
        <button
          class="btn px-corners h-8 shrink-0 px-2.5 text-xs"
          disabled={props.locked || props.busy !== null}
          onClick={props.onBackup}
        >
          <Icon name="download" size={10} />
          {props.busy === props.world.folder ? "Sauvegarde…" : "Sauvegarder"}
        </button>
      </div>
      <Show when={open() && props.backups.length > 0}>
        <ul class="flex flex-col divide-y divide-line border-t border-line px-3.5">
          <For each={props.backups}>
            {(backup) => (
              <li class="flex items-center gap-3 py-2 text-sm">
                <span class="flex-1">{new Date(backup.createdAt).toLocaleString("fr-FR")}</span>
                <span class="chip h-5 px-1.5 text-[11px] text-muted">{KINDS[backup.kind]}</span>
                <span class="w-16 text-right font-mono text-xs text-muted">{formatBytes(backup.sizeBytes)}</span>
                <button
                  class="btn btn-ghost h-7 px-2 text-xs"
                  disabled={props.locked || props.busy !== null}
                  onClick={() => props.onRestore(backup)}
                >
                  {props.busy === backup.fileName ? "Restauration…" : "Restaurer"}
                </button>
              </li>
            )}
          </For>
          <li class="py-2">
            <button class="btn btn-ghost h-7 px-2 text-xs" onClick={props.onOpen}>
              <Icon name="folder" size={10} />
              Ouvrir le dossier des sauvegardes
            </button>
          </li>
        </ul>
      </Show>
    </li>
  );
}
