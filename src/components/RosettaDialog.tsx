import { createSignal, Show } from "solid-js";
import { api, errorMessage } from "../lib/api";
import { launch } from "../lib/games";
import Dialog from "./Dialog";

/** Offered when an old version needs Rosetta 2 and the Mac does not have it yet. */
export default function RosettaDialog(props: { instanceId: string; onClose: () => void }) {
  const [installing, setInstalling] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const install = async () => {
    setInstalling(true);
    setError(null);
    try {
      if (await api.installRosetta()) {
        props.onClose();
        void launch(props.instanceId);
      }
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setInstalling(false);
    }
  };

  return (
    <Dialog title="Rosetta requis" onClose={() => !installing() && props.onClose()}>
      <div class="flex flex-col gap-3 text-chalk-2">
        <p>
          Cette version de Minecraft n'existe pas pour les puces Apple. Tandem la lance en mode Intel grâce à Rosetta 2,
          le traducteur d'Apple, qui n'est pas encore installé sur ce Mac.
        </p>
        <p class="text-sm text-muted">
          L'installation prend une minute et macOS demandera ton mot de passe. En continuant, tu acceptes la licence de
          Rosetta d'Apple.
        </p>
      </div>

      <Show when={error()}>
        <p class="text-sm text-redstone-text">{error()}</p>
      </Show>

      <div class="flex justify-end gap-2">
        <button class="btn btn-ghost" disabled={installing()} onClick={props.onClose}>
          Annuler
        </button>
        <button class="btn btn-primary px-corners px-5" disabled={installing()} onClick={install}>
          {installing() ? "Installation…" : "Installer Rosetta"}
        </button>
      </div>
    </Dialog>
  );
}
