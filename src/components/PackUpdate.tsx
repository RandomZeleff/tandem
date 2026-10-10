import { createEffect, createResource, createSignal, Show } from "solid-js";
import { api, errorMessage, type Instance, type Loader, type PackVersion } from "../lib/api";
import { formatRelative, loaderLabel } from "../lib/format";
import { refetchInstances } from "../lib/store";
import { toast } from "../lib/toast";
import Alert from "./Alert";
import Dialog from "./Dialog";
import { Icon, LoaderIcon } from "./pixel";
import RichText from "./project/RichText";
import Select from "./Select";

const integer = new Intl.NumberFormat("fr-FR");

/** "A new version of the modpack is out", with the update dialog. Shown on modpack instances. */
export default function PackUpdateBanner(props: { instance: Instance; locked: boolean }) {
  const [updates, { refetch }] = createResource(
    () => (props.instance.packProjectId ? `${props.instance.id}:${props.instance.packVersionId}` : false),
    () => api.modpackUpdates(props.instance.id).catch(() => [] as PackVersion[]),
  );
  const [open, setOpen] = createSignal(false);
  const [dismissed, setDismissed] = createSignal(false);
  const latest = () => updates()?.[0];

  return (
    <>
      <Show when={latest() && !dismissed()}>
        <div class="panel px-corners-md flex items-center gap-4 p-3.5 shadow-[inset_0_0_0_1px_var(--color-gold-deep)]">
          <Icon name="sparkle" size={14} color="var(--color-gold)" />
          <div class="flex min-w-0 flex-1 flex-col">
            <span class="text-sm">
              Nouvelle version du modpack : <strong class="font-mono">{latest()!.versionNumber}</strong>
            </span>
            <span class="text-xs text-muted">
              Tu as la {props.instance.packVersion}
              {updates()!.length > 1 ? ` · ${updates()!.length} versions plus récentes` : ""} · publiée{" "}
              {formatRelative(latest()!.datePublished)}
            </span>
          </div>
          <button class="btn btn-ghost h-8 px-2.5 text-xs" onClick={() => setDismissed(true)}>
            Plus tard
          </button>
          <button class="btn btn-gold px-corners h-9" disabled={props.locked} onClick={() => setOpen(true)}>
            <Icon name="download" size={12} />
            Mettre à jour
          </button>
        </div>
      </Show>
      <Show when={open() && updates()}>
        {(list) => (
          <UpdateDialog
            instance={props.instance}
            versions={list()}
            onClose={() => setOpen(false)}
            onDone={() => {
              setOpen(false);
              void refetch();
            }}
          />
        )}
      </Show>
    </>
  );
}

function target(v: PackVersion): string {
  const loader = v.loaders.find((l) => l !== "minecraft");
  return [loader ? loaderLabel(loader) : null, v.gameVersions[0]].filter(Boolean).join(" ");
}

function UpdateDialog(props: { instance: Instance; versions: PackVersion[]; onClose: () => void; onDone: () => void }) {
  const [chosen, setChosen] = createSignal(props.versions.find((v) => v.recommended)?.id ?? props.versions[0].id);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const version = () => props.versions.find((v) => v.id === chosen())!;
  const [changelog] = createResource(chosen, (id) => api.versionChangelog(props.instance.packProjectId!, id).catch(() => ""));
  const newGame = () => version().gameVersions[0];
  const changesGame = () => !!newGame() && newGame() !== props.instance.gameVersion;
  createEffect(() => chosen() && setError(null));

  async function update() {
    setBusy(true);
    setError(null);
    try {
      const report = await api.updateModpack(props.instance.id, chosen());
      await refetchInstances();
      const parts = [`${integer.format(report.added + report.replaced)} fichiers mis à jour`];
      if (report.removed) parts.push(`${integer.format(report.removed)} retirés`);
      if (report.kept.length) parts.push(`${integer.format(report.kept.length)} modifiés par toi gardés`);
      toast(`Modpack mis à jour : ${parts.join(", ")}`, { durationMs: 8000 });
      props.onDone();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog title="Mettre à jour le modpack" onClose={() => !busy() && props.onClose()} width={620}>
      <div class="flex flex-col gap-1.5">
        <label class="text-xs text-muted" for="pack-update-version">
          Version
        </label>
        <Select
          id="pack-update-version"
          value={chosen()}
          disabled={busy()}
          options={props.versions.map((v) => ({
            value: v.id,
            label: v.versionNumber,
            hint: `${target(v)} · ${v.recommended ? "recommandée" : v.versionType === "release" ? "stable" : v.versionType}`,
            icon: <LoaderIcon loader={(v.loaders.find((l) => l !== "minecraft") ?? "vanilla") as Loader} size={12} />,
          }))}
          onChange={setChosen}
        />
      </div>

      <div class="max-h-56 overflow-y-auto bg-slate-800 p-3.5 shadow-[inset_0_0_0_1px_var(--color-line)]">
        <Show when={changelog() !== undefined} fallback={<div class="skeleton h-16" />}>
          <Show when={changelog()} fallback={<p class="text-sm text-muted">Pas de journal des changements pour cette version.</p>}>
            <RichText html={changelog()!} class="text-[13px]" />
          </Show>
        </Show>
      </div>

      <ul class="flex flex-col gap-1.5 text-[13px] text-chalk-2">
        <li class="flex gap-2">
          <Icon name="check" size={11} class="mt-1 shrink-0 text-xp-text" />
          Tes mondes, captures, réglages du jeu et fichiers que tu as modifiés sont gardés ; un mod que tu as désactivé le reste.
        </li>
        <li class="flex gap-2">
          <Icon name="check" size={11} class="mt-1 shrink-0 text-xp-text" />
          Tu pourras revenir à la {props.instance.packVersion} depuis l'onglet Informations.
        </li>
      </ul>
      <Show when={changesGame()}>
        <Alert tone="warning">
          Cette version passe de Minecraft {props.instance.gameVersion} à {newGame()} : tes mondes seront convertis à leur
          prochaine ouverture. Tandem en fait une sauvegarde avant la mise à jour.
        </Alert>
      </Show>
      <Show when={version().versionType !== "release"}>
        <Alert tone="warning">Version encore en test chez ses auteurs : elle peut planter ou abîmer tes mondes.</Alert>
      </Show>
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>
      <div class="flex justify-end gap-2">
        <button class="btn btn-ghost" disabled={busy()} onClick={props.onClose}>
          Annuler
        </button>
        <button class="btn btn-primary px-corners px-5" disabled={busy()} onClick={() => void update()}>
          <Icon name="download" size={12} />
          {busy() ? "Mise à jour…" : `Passer à la ${version().versionNumber}`}
        </button>
      </div>
    </Dialog>
  );
}

/** "Go back to the previous version" after a modpack update. */
export function PackRollback(props: { instance: Instance; locked: boolean; onError: (message: string) => void }) {
  const [previous, { refetch }] = createResource(
    () => `${props.instance.id}:${props.instance.packVersionId}`,
    () => api.modpackRollbackVersion(props.instance.id),
  );
  const [confirm, setConfirm] = createSignal(false);
  const [busy, setBusy] = createSignal(false);

  async function rollback() {
    setConfirm(false);
    setBusy(true);
    try {
      await api.rollbackModpack(props.instance.id);
      await refetchInstances();
      await refetch();
      toast("Version précédente du modpack restaurée");
    } catch (err) {
      props.onError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Show when={previous()}>
      <button class="btn btn-ghost h-7 px-2 text-xs" disabled={props.locked || busy()} onClick={() => setConfirm(true)}>
        {busy() ? "Retour…" : `Revenir à la ${previous()}`}
      </button>
      <Show when={confirm()}>
        <Dialog title={`Revenir à la ${previous()} ?`} onClose={() => setConfirm(false)}>
          <p class="text-chalk-2">
            Les fichiers de la mise à jour sont retirés et ceux d'avant remis en place. Si tu as joué depuis avec une autre
            version de Minecraft, restaure aussi la sauvegarde « avant mise à jour » de tes mondes.
          </p>
          <div class="flex justify-end gap-2">
            <button class="btn btn-ghost" onClick={() => setConfirm(false)}>
              Annuler
            </button>
            <button class="btn btn-primary px-corners" onClick={() => void rollback()}>
              Revenir en arrière
            </button>
          </div>
        </Dialog>
      </Show>
    </Show>
  );
}
