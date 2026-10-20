import { createSignal, Match, Show, Switch } from "solid-js";
import { api, errorMessage, type Instance } from "../lib/api";
import { loaderLabel } from "../lib/format";
import { gameState, launch, stop } from "../lib/games";
import { exportModpackFile } from "../lib/modpacks";
import { navigate, refetchInstances, route } from "../lib/store";
import { toast } from "../lib/toast";
import Alert from "./Alert";
import type { MenuItem } from "./ContextMenu";
import Dialog from "./Dialog";
import { Icon } from "./pixel";

/** The instance window open from anywhere (menu, settings tab): delete or duplicate. */
const [instanceDialog, setInstanceDialog] = createSignal<{ kind: "delete" | "duplicate"; instance: Instance } | null>(null);
export const askDeleteInstance = (instance: Instance) => setInstanceDialog({ kind: "delete", instance });
export const askDuplicateInstance = (instance: Instance) => setInstanceDialog({ kind: "duplicate", instance });

/** Quick actions on an instance, for its right-click menu. */
export function instanceMenuItems(instance: Instance): MenuItem[] {
  const status = gameState(instance.id).status;
  const busy = status !== "idle";
  return [
    { kind: "header", label: `Minecraft ${instance.gameVersion} · ${loaderLabel(instance.loader)}` },
    status === "running"
      ? { label: "Arrêter le jeu", icon: "stop", run: () => void stop(instance.id) }
      : { label: "Jouer", icon: "play", disabled: busy, run: () => void launch(instance.id) },
    { label: "Jouer avec des amis", icon: "invite", run: () => navigate({ page: "multi", instanceId: instance.id }) },
    { kind: "separator" },
    { label: "Ouvrir", icon: "arrow", run: () => navigate({ page: "instance", id: instance.id }) },
    { label: "Mondes", icon: "grid", run: () => navigate({ page: "instance", id: instance.id, tab: "worlds" }) },
    {
      label: "Ajouter du contenu",
      icon: "search",
      run: () => navigate({ page: "discover", instanceId: instance.id }),
    },
    {
      label: "Ouvrir le dossier",
      icon: "folder",
      run: () => void api.openInstanceFolder(instance.id).catch((err) => toast(errorMessage(err), { tone: "error" })),
    },
    { kind: "separator" },
    { label: "Dupliquer…", icon: "plus", disabled: busy, run: () => askDuplicateInstance(instance) },
    {
      label: "Exporter en .mrpack…",
      icon: "download",
      disabled: busy,
      run: () =>
        void exportModpackFile(instance)
          .then((path) => path && toast("Modpack exporté"))
          .catch((err) => toast(errorMessage(err), { tone: "error" })),
    },
    { kind: "separator" },
    {
      label: "Supprimer…",
      icon: "trash",
      danger: true,
      disabled: busy,
      hint: busy ? "Ferme le jeu d'abord" : undefined,
      run: () => askDeleteInstance(instance),
    },
  ];
}

function DeleteDialog(props: { instance: Instance; onClose: () => void }) {
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  async function remove() {
    setBusy(true);
    try {
      await api.deleteInstance(props.instance.id);
      const r = route();
      if (r.page === "instance" && r.id === props.instance.id) navigate({ page: "instances" });
      await refetchInstances();
      toast(`« ${props.instance.name} » supprimée`);
      props.onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog title="Supprimer l'instance ?" onClose={() => !busy() && props.onClose()}>
      <p class="text-chalk-2">
        « {props.instance.name} » et tout son dossier seront supprimés, <strong>mondes compris</strong>. Cette action est
        définitive.
      </p>
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>
      <div class="flex justify-end gap-2">
        <button class="btn btn-ghost" disabled={busy()} onClick={props.onClose}>
          Annuler
        </button>
        <button class="btn btn-danger px-corners" disabled={busy()} onClick={() => void remove()}>
          <Icon name="trash" size={12} />
          {busy() ? "Suppression…" : "Supprimer"}
        </button>
      </div>
    </Dialog>
  );
}

function DuplicateDialog(props: { instance: Instance; onClose: () => void }) {
  const [name, setName] = createSignal(`${props.instance.name} (copie)`.slice(0, 64));
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  async function duplicate(e: Event) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const created = await api.duplicateInstance(props.instance.id, name().trim());
      await refetchInstances();
      toast(`« ${created.name} » créée`);
      props.onClose();
      navigate({ page: "instance", id: created.id });
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog title="Dupliquer l'instance" onClose={() => !busy() && props.onClose()}>
      <form class="flex flex-col gap-4" onSubmit={(e) => void duplicate(e)}>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs text-muted" for="duplicate-name">
            Nom de la copie
          </label>
          <input id="duplicate-name" autofocus class="field h-10 w-full" maxLength={64} value={name()} onInput={(e) => setName(e.currentTarget.value)} />
        </div>
        <p class="text-meta text-muted">Mondes et captures compris.</p>
        <Show when={error()}>
          <Alert onClose={() => setError(null)}>{error()}</Alert>
        </Show>
        <div class="flex justify-end gap-2">
          <button type="button" class="btn btn-ghost" disabled={busy()} onClick={props.onClose}>
            Annuler
          </button>
          <button type="submit" class="btn btn-primary px-corners px-5" disabled={busy() || !name().trim()}>
            {busy() ? "Copie…" : "Dupliquer"}
          </button>
        </div>
      </form>
    </Dialog>
  );
}

/** Mounted once in App. */
export function InstanceDialogs() {
  const close = () => setInstanceDialog(null);
  return (
    <Show when={instanceDialog()} keyed>
      {(open) => (
        <Switch>
          <Match when={open.kind === "delete"}>
            <DeleteDialog instance={open.instance} onClose={close} />
          </Match>
          <Match when={open.kind === "duplicate"}>
            <DuplicateDialog instance={open.instance} onClose={close} />
          </Match>
        </Switch>
      )}
    </Show>
  );
}
