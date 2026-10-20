import { createResource, createSignal, For, type JSX, Match, Show, Switch } from "solid-js";
import { openAccounts, signInWithMicrosoft } from "../lib/accounts";
import { api, errorMessage, type FoundInstance } from "../lib/api";
import { formatRelative, loaderLabel } from "../lib/format";
import { finishOnboarding } from "../lib/onboarding";
import { activeAccount, navigate, refetchInstances } from "../lib/store";
import { toast } from "../lib/toast";
import { trapFocus } from "../lib/ui";
import { AccountAvatar, GrassBlock, MicrosoftLogo } from "./accounts/AccountVisuals";
import { Checkbox, Icon } from "./pixel";
import Scene from "./Scene";

/** Fabulously Optimized on Modrinth: vanilla look, much smoother. */
const OPTIMIZED_PACK = "1KVo5zza";

type Step = "account" | "import" | "first";

const LAUNCHERS: Record<FoundInstance["source"], string> = {
  modrinthApp: "Modrinth App",
  prism: "Prism Launcher",
  curseForge: "CurseForge",
  official: "Launcher officiel",
  folder: "Dossier",
};

function foundDetails(f: FoundInstance): string {
  const version = f.gameVersion.startsWith("latest") ? "Dernière version" : f.gameVersion;
  const parts = [LAUNCHERS[f.source], version, loaderLabel(f.loader)];
  if (f.mods > 0) parts.push(`${f.mods} mods`);
  if (f.lastPlayed) parts.push(`joué ${formatRelative(new Date(f.lastPlayed).toISOString())}`);
  return parts.join(" · ");
}

function Choice(props: { title: string; detail: string; icon: JSX.Element; busy?: boolean; disabled?: boolean; onClick: () => void }) {
  return (
    <button
      class="px-corners-md flex items-center gap-4 bg-slate-800 p-4 text-left shadow-[inset_0_0_0_1px_var(--color-line)] transition-colors duration-100 hover:bg-slate-750 focus-visible:bg-slate-750 disabled:opacity-50"
      disabled={props.disabled}
      onClick={props.onClick}
    >
      <span class="slot size-12">{props.icon}</span>
      <span class="flex min-w-0 flex-1 flex-col gap-0.5">
        <span class="font-semibold">{props.title}</span>
        <span class="text-meta text-muted">{props.busy ? "Préparation…" : props.detail}</span>
      </span>
      <Icon name="arrow" size={12} class="text-faint" />
    </button>
  );
}

/** Welcome steps on a fresh install: account, games from other launchers, a first game. */
export default function Onboarding() {
  let panel: HTMLDivElement | undefined;
  trapFocus(() => panel);

  const [step, setStep] = createSignal<Step>("account");
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal<string | null>(null);

  // Looked for while the player signs in.
  const [found] = createResource(() => api.scanOtherLaunchers().catch(() => [] as FoundInstance[]));
  const [chosen, setChosen] = createSignal<ReadonlySet<string>>(new Set());
  const keyOf = (f: FoundInstance) => `${f.gameDir}|${f.name}`;
  const [versions] = createResource(() => api.listVersions().catch(() => null));
  const latest = () => versions()?.latest.release;

  const steps = (): Step[] => (found()?.length ? ["account", "import", "first"] : ["account", "first"]);
  const microsoft = () => activeAccount()?.kind === "microsoft";

  function next() {
    setError(null);
    const list = steps();
    const following = list[list.indexOf(step()) + 1];
    if (following) setStep(following);
    else void finishOnboarding();
  }

  function toggle(f: FoundInstance, on: boolean) {
    setChosen((set) => {
      const copy = new Set(set);
      if (on) copy.add(keyOf(f));
      else copy.delete(keyOf(f));
      return copy;
    });
  }

  async function importChosen() {
    const picked = (found() ?? []).filter((f) => chosen().has(keyOf(f)));
    let done = 0;
    for (const f of picked) {
      setBusy(`Import de ${f.name}… (${done + 1}/${picked.length})`);
      try {
        await api.importFromLauncher(f);
        done++;
      } catch (err) {
        setError(`${f.name} : ${errorMessage(err)}`);
      }
    }
    setBusy(null);
    await refetchInstances();
    if (done === 0) return;
    // Something to play already: no need for a first game.
    toast(done === 1 ? "1 partie importée." : `${done} parties importées.`);
    void finishOnboarding();
    navigate({ page: "home" });
  }

  async function createVanilla() {
    const version = latest();
    if (!version) return;
    setBusy("vanilla");
    setError(null);
    try {
      await api.createInstance({ name: `Minecraft ${version}`, gameVersion: version });
      await refetchInstances();
      void finishOnboarding();
      navigate({ page: "home" });
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  }

  /** The download runs behind: the title bar shows it, the instance opens when ready. */
  function installOptimized() {
    void finishOnboarding();
    navigate({ page: "home" });
    void api
      .installModpack(OPTIMIZED_PACK)
      .then(async (created) => {
        await refetchInstances();
        navigate({ page: "instance", id: created.id });
      })
      .catch((err) => toast(errorMessage(err), { tone: "error" }));
  }

  function browseModpacks() {
    void finishOnboarding();
    navigate({ page: "discover", kind: "modpack" });
  }

  const index = () => steps().indexOf(step());

  // Below the title bar: the window can still be moved and closed.
  return (
    <div class="fixed inset-x-0 top-10 bottom-0 z-20 flex bg-slate-900" role="dialog" aria-modal="true" aria-label="Bienvenue dans Tandem">
      <div class="relative hidden w-[42%] max-w-[560px] overflow-hidden bg-[#3E6FB0] min-[960px]:block">
        <Scene variant="day" />
        <div class="absolute inset-0 bg-[linear-gradient(0deg,rgb(16_17_19/0.8)_0%,transparent_60%)]" />
        <p class="title-display absolute bottom-10 left-10 text-hero">Tandem</p>
      </div>

      <div ref={panel} tabIndex={-1} class="flex min-w-0 flex-1 flex-col overflow-y-auto px-10 py-10 outline-none">
        <div class="flex items-center justify-between gap-4">
          <ol class="flex gap-1.5" aria-label={`Étape ${index() + 1} sur ${steps().length}`}>
            <For each={steps()}>
              {(_, i) => (
                <li
                  class="h-1.5 w-8 rounded-[1px]"
                  classList={{ "bg-grass-light": i() <= index(), "bg-slate-700": i() > index() }}
                />
              )}
            </For>
          </ol>
          <button class="btn btn-ghost h-8 px-3 text-sm" onClick={() => void finishOnboarding()}>
            Passer
          </button>
        </div>

        <div class="mx-auto my-auto flex w-full max-w-[520px] flex-col gap-6 py-8">
          <Switch>
            <Match when={step() === "account"}>
              <h1 class="title-display text-title">Bienvenue dans Tandem</h1>
              <p class="text-chalk-2">Connecte ton compte Microsoft pour jouer, jouer avec tes amis et changer de skin.</p>
              <Show
                when={microsoft() && activeAccount()}
                fallback={
                  <button
                    class="btn btn-primary px-corners-md h-12 w-fit px-5"
                    autofocus
                    onClick={() => {
                      openAccounts();
                      void signInWithMicrosoft();
                    }}
                  >
                    <MicrosoftLogo size={16} />
                    Se connecter avec Microsoft
                  </button>
                }
              >
                {(account) => (
                  <div class="px-corners-md flex items-center gap-3 bg-slate-800 p-3 shadow-[inset_0_0_0_1px_var(--color-success-line)]">
                    <AccountAvatar account={account()} size={32} />
                    <span class="flex-1 font-semibold">{account().username}</span>
                    <Icon name="check" size={12} color="var(--color-xp)" />
                  </div>
                )}
              </Show>
              <div class="flex gap-2">
                <button class="btn px-corners h-10 px-5" classList={{ "btn-primary": microsoft() }} onClick={next}>
                  {microsoft() ? "Continuer" : "Plus tard"}
                </button>
              </div>
            </Match>

            <Match when={step() === "import"}>
              <h1 class="title-display text-title">Tes parties sont déjà là</h1>
              <p class="text-chalk-2">Tandem les copie, l'original ne bouge pas.</p>
              <ul class="flex max-h-[320px] flex-col gap-1 overflow-y-auto">
                <For each={found()}>
                  {(f) => (
                    <li class="row px-2 py-2">
                      <Checkbox
                        checked={chosen().has(keyOf(f))}
                        disabled={!!busy()}
                        onChange={(on) => toggle(f, on)}
                        class="w-full text-left"
                        label={
                          <span class="flex min-w-0 flex-col">
                            <span class="truncate text-sm font-medium text-chalk">{f.name}</span>
                            <span class="truncate text-meta text-muted">{foundDetails(f)}</span>
                          </span>
                        }
                      />
                    </li>
                  )}
                </For>
              </ul>
              <Show when={busy()}>{(message) => <p class="text-sm text-muted">{message()}</p>}</Show>
              <div class="flex gap-2">
                <button
                  class="btn btn-primary px-corners h-10 px-5"
                  disabled={chosen().size === 0 || !!busy()}
                  onClick={() => void importChosen()}
                >
                  {chosen().size > 1 ? `Importer ${chosen().size} parties` : "Importer"}
                </button>
                <button class="btn btn-ghost h-10 px-4" disabled={!!busy()} onClick={next}>
                  Pas maintenant
                </button>
              </div>
            </Match>

            <Match when={step() === "first"}>
              <h1 class="title-display text-title">Ta première partie</h1>
              <div class="flex flex-col gap-2">
                <Choice
                  title={latest() ? `Minecraft ${latest()}` : "Minecraft"}
                  detail="Le jeu tel quel."
                  icon={<GrassBlock size={24} />}
                  busy={busy() === "vanilla"}
                  disabled={!latest() || !!busy()}
                  onClick={() => void createVanilla()}
                />
                <Choice
                  title="Minecraft optimisé"
                  detail="Le même jeu, bien plus fluide (Fabulously Optimized)."
                  icon={<Icon name="sparkle" size={24} color="var(--color-xp)" />}
                  disabled={!!busy()}
                  onClick={installOptimized}
                />
                <Choice
                  title="Un modpack"
                  detail="Aventure, technique, magie… à choisir parmi des milliers."
                  icon={<Icon name="search" size={24} />}
                  disabled={!!busy()}
                  onClick={browseModpacks}
                />
              </div>
            </Match>
          </Switch>

          <Show when={error()}>
            <p class="text-sm text-redstone-text">{error()}</p>
          </Show>
        </div>
      </div>
    </div>
  );
}
