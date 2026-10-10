import { createResource, createSignal, Show } from "solid-js";
import { api, errorMessage } from "../../lib/api";
import { curseforgeKeyPrompt, setCurseforgeKeyPrompt } from "../../lib/modpacks";
import { openExternal } from "../../lib/projects";
import Dialog from "../Dialog";

const CONSOLE_URL = "https://console.curseforge.com/";

/** Where to get a key, with a link that opens in the browser. */
function KeyHelp() {
  return (
    <p class="text-xs text-muted">
      CurseForge ne laisse télécharger ses fichiers qu'avec une clé d'API. Crée la tienne, gratuite, sur{" "}
      <button class="text-chalk-2 underline hover:text-xp-text" onClick={() => openExternal(CONSOLE_URL)}>
        console.curseforge.com
      </button>{" "}
      (rubrique « API Keys »). Elle reste dans le coffre de ton système.
    </p>
  );
}

/** Key input: checked with CurseForge before being saved. `onSaved` runs after a successful save. */
function KeyInput(props: { saved: boolean; onSaved: () => void; onForget?: () => void; autofocus?: boolean }) {
  const [key, setKey] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  async function save() {
    setBusy(true);
    setError(null);
    try {
      await api.setCurseforgeKey(key().trim());
      setKey("");
      props.onSaved();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="flex flex-col gap-1.5">
      <form
        class="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (key().trim()) void save();
        }}
      >
        <input
          type="password"
          autocomplete="off"
          aria-label="Clé d'API CurseForge"
          autofocus={props.autofocus}
          class="field h-9 min-w-0 flex-1 font-mono text-[13px]"
          placeholder={props.saved ? "•••••••• (enregistrée dans le coffre du système)" : "Colle ta clé ici"}
          value={key()}
          onInput={(e) => setKey(e.currentTarget.value)}
        />
        <button type="submit" class="btn px-corners h-9 px-3 text-sm" disabled={busy() || !key().trim()}>
          {busy() ? "Vérification…" : "Enregistrer"}
        </button>
        <Show when={props.saved && props.onForget}>
          <button type="button" class="btn btn-ghost h-9 px-2.5 text-xs" disabled={busy()} onClick={() => props.onForget?.()}>
            Oublier
          </button>
        </Show>
      </form>
      <Show when={error()}>
        <p class="text-xs text-redstone-text">{error()}</p>
      </Show>
    </div>
  );
}

/** Section of the Settings page. */
export function CurseForgeSettings() {
  const [saved, { refetch }] = createResource(() => api.curseforgeKeySaved().catch(() => false));
  const [error, setError] = createSignal<string | null>(null);

  async function forget() {
    try {
      await api.setCurseforgeKey("");
      void refetch();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <section class="panel px-corners-md flex flex-col gap-3 px-5 py-4">
      <div class="flex items-center justify-between gap-3">
        <h2 class="panel-title">CurseForge</h2>
        <span class="text-xs" classList={{ "text-xp-text": saved(), "text-muted": !saved() }}>
          {saved() ? "Clé enregistrée" : "Aucune clé"}
        </span>
      </div>
      <p class="text-sm text-chalk-2">Pour importer les modpacks CurseForge (fichier .zip).</p>
      <KeyInput saved={saved() ?? false} onSaved={() => void refetch()} onForget={() => void forget()} />
      <Show when={error()}>
        <p class="text-xs text-redstone-text">{error()}</p>
      </Show>
      <KeyHelp />
    </section>
  );
}

/** Asked when a CurseForge pack is imported and no key is saved yet. Mounted once in App. */
export function CurseForgeKeyDialog() {
  const close = (saved: boolean) => {
    curseforgeKeyPrompt()?.(saved);
    setCurseforgeKeyPrompt(null);
  };
  return (
    <Show when={curseforgeKeyPrompt()}>
      <Dialog title="Clé CurseForge" onClose={() => close(false)} width={500}>
        <p class="text-chalk-2">Ce modpack vient de CurseForge. Pour récupérer ses mods, Tandem a besoin de ta clé d'API.</p>
        <KeyInput saved={false} autofocus onSaved={() => close(true)} />
        <KeyHelp />
        <div class="flex justify-end">
          <button class="btn btn-ghost" onClick={() => close(false)}>
            Annuler
          </button>
        </div>
      </Dialog>
    </Show>
  );
}
