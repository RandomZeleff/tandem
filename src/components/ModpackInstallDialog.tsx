import { createEffect, createResource, createSignal, Show } from "solid-js";
import { api, errorMessage, type Loader, type PackVersion } from "../lib/api";
import { formatBytes, loaderLabel } from "../lib/format";
import Alert from "./Alert";
import Dialog from "./Dialog";
import { Icon, LoaderIcon } from "./pixel";
import Select from "./Select";

const TYPES: Record<string, string> = { release: "stable", beta: "bêta", alpha: "alpha" };

/** `Fabric 1.20.1`, from what the version declares. */
function target(v: PackVersion): string {
  const loader = v.loaders.find((l) => l !== "minecraft");
  return [loader ? loaderLabel(loader) : null, v.gameVersions[0]].filter(Boolean).join(" ");
}

/** Shows which version of a modpack will be installed, and lets the player pick another. */
export default function ModpackInstallDialog(props: {
  projectId: string;
  title: string;
  /** Preselected instead of the recommended version. */
  initialVersionId?: string;
  onClose: () => void;
  onInstall: (versionId: string) => void;
}) {
  const [versions] = createResource(() => props.projectId, api.modpackVersions);
  const [chosen, setChosen] = createSignal("");
  createEffect(() => {
    const list = versions();
    if (list && !chosen()) {
      const initial = list.find((v) => v.id === props.initialVersionId) ?? list.find((v) => v.recommended) ?? list[0];
      setChosen(initial?.id ?? "");
    }
  });
  const version = () => versions()?.find((v) => v.id === chosen());

  return (
    <Dialog title={`Installer ${props.title}`} onClose={props.onClose} width={500}>
      <Show when={!versions.error} fallback={<Alert>Impossible de lire les versions : {errorMessage(versions.error)}</Alert>}>
        <p class="text-chalk-2">Une nouvelle instance est créée avec la bonne version du jeu et du loader.</p>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs text-muted" for="pack-version">
            Version
          </label>
          <Select
            id="pack-version"
            value={chosen()}
            placeholder={versions.loading ? "Chargement…" : "Aucune version installable"}
            disabled={versions.loading || (versions() ?? []).length === 0}
            options={(versions() ?? []).map((v) => ({
              value: v.id,
              label: v.versionNumber,
              hint: `${target(v)} · ${v.recommended ? "recommandée" : TYPES[v.versionType] ?? v.versionType}`,
              icon: <LoaderIcon loader={(v.loaders.find((l) => l !== "minecraft") ?? "vanilla") as Loader} size={12} />,
            }))}
            onChange={setChosen}
          />
          <Show when={version()}>
            {(v) => (
              <span class="text-xs text-muted">
                {target(v())} · {formatBytes(v().size)} à télécharger, plus ses mods
              </span>
            )}
          </Show>
        </div>
        <Show when={version() && version()!.versionType !== "release"}>
          <Alert tone="warning">
            Version {TYPES[version()!.versionType] ?? version()!.versionType} : encore en test chez ses auteurs, elle peut
            planter ou abîmer tes mondes.
          </Alert>
        </Show>
      </Show>
      <div class="flex justify-end gap-2">
        <button class="btn btn-ghost" onClick={props.onClose}>
          Annuler
        </button>
        <button class="btn btn-primary px-corners px-5" disabled={!version()} onClick={() => props.onInstall(chosen())}>
          <Icon name="download" size={12} />
          Installer
        </button>
      </div>
    </Dialog>
  );
}
