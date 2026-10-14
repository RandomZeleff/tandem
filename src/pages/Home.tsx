import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import Alert from "../components/Alert";
import InstanceCard from "../components/InstanceCard";
import InstanceVisual from "../components/InstanceVisual";
import { Icon, LoaderTag } from "../components/pixel";
import PlayButton from "../components/PlayButton";
import Scene from "../components/Scene";
import { openAccounts } from "../lib/accounts";
import { api, errorMessage, type Instance, type World } from "../lib/api";
import { duo, formatCode, joinFriend } from "../lib/duo";
import { formatRelative } from "../lib/format";
import { gameState } from "../lib/games";
import { activeAccount, instances, navigate, setNewInstanceDialog } from "../lib/store";
import { onDismiss } from "../lib/ui";
import { GAME_MODES, lastWorld, opensWorld, worldPlayedAt } from "../lib/worlds";
import { formatNumber, formatPlayTime, isEmptyJournal, nextMilestone } from "../lib/journal";

/** The world "Continuer" opens, and the other ones (or the title screen) on demand. */
function WorldPicker(props: { worlds: World[]; chosen: World | null; onChoose: (world: World | null) => void }) {
  const [open, setOpen] = createSignal(false);
  let anchor: HTMLButtonElement | undefined;
  let menu: HTMLDivElement | undefined;
  const choose = (world: World | null) => {
    props.onChoose(world);
    setOpen(false);
    anchor?.focus();
  };
  const Menu = () => {
    onDismiss(() => [menu, anchor], () => setOpen(false));
    return (
      <div
        ref={menu}
        class="panel px-corners-md absolute bottom-full left-0 z-20 mb-1.5 flex max-h-[260px] w-[320px] flex-col overflow-y-auto bg-slate-750 p-1.5 shadow-[0_12px_32px_rgb(0_0_0/0.55)]"
      >
        <ul role="listbox" aria-label="Monde" class="flex flex-col gap-0.5">
          <For each={props.worlds}>
            {(world) => (
              <li>
                <button
                  role="option"
                  aria-selected={props.chosen?.folder === world.folder}
                  class="row flex h-12 w-full items-center gap-3 px-2 text-left"
                  onClick={() => choose(world)}
                >
                  <WorldIcon world={world} size={32} />
                  <span class="flex min-w-0 flex-1 flex-col">
                    <span class="truncate text-sm font-semibold">{world.name}</span>
                    <span class="truncate text-xs text-muted">
                      {GAME_MODES[world.gameMode]} · joué {formatRelative(worldPlayedAt(world))}
                    </span>
                  </span>
                  <Show when={props.chosen?.folder === world.folder}>
                    <Icon name="check" size={12} color="var(--color-xp-text)" />
                  </Show>
                </button>
              </li>
            )}
          </For>
        </ul>
        <div class="my-1.5 h-px shrink-0 bg-divider" />
        <button class="row flex h-9 w-full shrink-0 items-center gap-3 px-2 text-left text-sm text-chalk-2" onClick={() => choose(null)}>
          <span class="flex w-8 justify-center">
            <Icon name="home" size={12} />
          </span>
          <span class="flex-1">Écran titre du jeu</span>
          <Show when={!props.chosen}>
            <Icon name="check" size={12} color="var(--color-xp-text)" />
          </Show>
        </button>
      </div>
    );
  };

  return (
    <div class="relative w-fit max-w-full">
      <button
        ref={anchor}
        class="px-corners flex h-11 max-w-full items-center gap-2.5 bg-black/35 py-1 pr-3 pl-1.5 text-left transition-colors duration-100 hover:bg-black/55 focus-visible:bg-black/55"
        aria-expanded={open()}
        aria-label={props.chosen ? `Monde : ${props.chosen.name}. Changer de monde` : "Écran titre du jeu. Choisir un monde"}
        onClick={() => setOpen((v) => !v)}
      >
        <Show
          when={props.chosen}
          fallback={
            <>
              <span class="slot size-8">
                <Icon name="home" size={12} />
              </span>
              <span class="text-sm font-semibold">Écran titre du jeu</span>
            </>
          }
        >
          {(world) => (
            <>
              <WorldIcon world={world()} size={32} />
              <span class="truncate text-sm font-semibold">{world().name}</span>
              <span class="shrink-0 text-meta text-chalk-3">{GAME_MODES[world().gameMode]}</span>
            </>
          )}
        </Show>
        <Icon name="caret" size={12} class={`ml-0.5 shrink-0 text-chalk-3 transition-transform duration-100 ${open() ? "rotate-180" : ""}`} />
      </button>
      <Show when={open()}>
        <Menu />
      </Show>
    </div>
  );
}

function WorldIcon(props: { world: World; size: number }) {
  return (
    <span class="slot shrink-0" style={{ width: `${props.size}px`, height: `${props.size}px` }}>
      <Show when={props.world.icon} fallback={<Icon name="grid" size={12} color="var(--color-slate-500)" />}>
        {(icon) => (
          <img
            src={icon()}
            alt=""
            class="[image-rendering:pixelated]"
            style={{ width: `${props.size - 8}px`, height: `${props.size - 8}px` }}
          />
        )}
      </Show>
    </span>
  );
}

function Hero(props: { instance: Instance }) {
  const state = () => gameState(props.instance.id);
  const exit = () => state().lastExit;
  const [error, setError] = createSignal<string | null>(null);
  const guests = () => duo.host?.guests.length ?? 0;
  // Worlds are read again after each run: the one just played comes first.
  const [worlds] = createResource(
    () => (opensWorld(props.instance.gameVersion) ? { id: props.instance.id, exit: exit() } : false),
    ({ id }) => api.listWorlds(id).catch(() => [] as World[]),
  );
  /** `undefined`: follow the last played world; `null`: the title screen. */
  const [picked, setPicked] = createSignal<{ instance: string; folder: string | null }>();
  const chosen = createMemo<World | null>(() => {
    const list = worlds() ?? [];
    const pick = picked();
    if (pick?.instance === props.instance.id) return list.find((w) => w.folder === pick.folder) ?? null;
    return lastWorld(list) ?? null;
  });
  const [journal] = createResource(
    () => (chosen() ? { id: props.instance.id, world: chosen()!.folder, at: chosen()!.lastPlayed } : false),
    ({ id, world }) => api.worldJournal(id, world).catch(() => null),
  );
  /** A few words of the chosen world's logbook, the reason to go back in. */
  const summary = () => {
    const j = journal();
    if (!j || isEmptyJournal(j)) return [];
    const next = nextMilestone(j);
    return [
      j.day !== null && `Jour ${formatNumber(j.day)}`,
      j.stats && `${formatPlayTime(j.stats.playSeconds)} de jeu`,
      j.quests && `Quêtes ${formatNumber(j.quests.done)} / ${formatNumber(j.quests.total)}`,
      !j.quests && next && `Prochaine étape : ${next}`,
    ].filter((x): x is string => !!x);
  };

  return (
    <section
      aria-label="Reprendre la dernière partie"
      class="px-corners-lg relative flex h-[clamp(340px,48vh,460px)] shrink-0 overflow-hidden bg-[#1E1A33]"
    >
      <InstanceVisual instance={props.instance} fallback="scene" />
      {/* Scrim under the text only: the picture stays bright on the right. */}
      <div class="absolute inset-0 bg-[linear-gradient(0deg,rgb(16_17_19/0.94)_0%,rgb(16_17_19/0.55)_42%,transparent_72%),linear-gradient(90deg,rgb(16_17_19/0.6)_0%,transparent_60%)]" />

      <div class="relative mt-auto flex w-full flex-col gap-5 px-9 pb-8">
        <div class="flex min-w-0 flex-col gap-2">
          <h1 class="title-display truncate text-hero">{props.instance.name}</h1>
          <p class="flex flex-wrap items-center gap-x-2 text-sm text-chalk-2">
            <span>{props.instance.gameVersion}</span>
            <span class="text-faint">·</span>
            <LoaderTag loader={props.instance.loader} />
            <span class="text-faint">·</span>
            <Show
              when={state().status === "running"}
              fallback={
                <span>
                  {props.instance.lastPlayedAt ? `Joué ${formatRelative(props.instance.lastPlayedAt)}` : "Jamais lancée"}
                </span>
              }
            >
              <span class="font-medium text-xp-text">En cours</span>
            </Show>
          </p>
        </div>

        <Show when={error() ?? state().error}>
          <Alert class="max-w-[640px]" onClose={error() ? () => setError(null) : undefined}>
            {error() ?? state().error}
          </Alert>
        </Show>
        <Show when={exit() && !exit()!.stopped && exit()!.code !== 0}>
          <Alert tone="warning" class="max-w-[640px]">
            Le jeu s'est arrêté (code {exit()!.code ?? "?"}). La console de l'instance contient les détails.
          </Alert>
        </Show>

        <Show when={state().status === "idle" && (worlds() ?? []).length > 0}>
          <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
            <WorldPicker
              worlds={worlds()!}
              chosen={chosen()}
              onChoose={(world) => setPicked({ instance: props.instance.id, folder: world?.folder ?? null })}
            />
            <Show when={chosen() && summary().length > 0}>
              <button
                class="flex items-center gap-2 text-meta text-chalk-2 transition-colors duration-100 hover:text-chalk focus-visible:text-chalk"
                title="Ouvrir le carnet de ce monde"
                onClick={() => navigate({ page: "instance", id: props.instance.id, tab: "worlds", world: chosen()!.folder })}
              >
                <Icon name="book" size={12} />
                {summary().join(" · ")}
              </button>
            </Show>
          </div>
        </Show>

        <div class="flex flex-wrap items-center gap-3">
          <PlayButton
            id={props.instance.id}
            size="lg"
            name={props.instance.name}
            world={chosen()?.folder ?? null}
            worldName={chosen()?.name}
          />
          <button class="btn btn-light px-corners-md h-[54px] px-6 text-base" onClick={() => navigate({ page: "multi" })}>
            <Icon name="invite" size={12} />
            <Show when={duo.host} fallback="Inviter des amis">
              {guests() === 0 ? "Partie ouverte" : guests() === 1 ? "1 ami connecté" : `${guests()} amis connectés`}
            </Show>
          </button>
          <button
            class="btn px-corners-md size-[54px] px-0"
            aria-label="Détails de l'instance"
            title="Détails de l'instance"
            onClick={() => navigate({ page: "instance", id: props.instance.id })}
          >
            <Icon name="dots" size={24} />
          </button>
          <button
            class="btn px-corners-md size-[54px] px-0"
            aria-label="Ouvrir le dossier de l'instance"
            title="Ouvrir le dossier"
            onClick={() => api.openInstanceFolder(props.instance.id).catch((err) => setError(errorMessage(err)))}
          >
            <Icon name="folder" size={24} />
          </button>
        </div>
      </div>
    </section>
  );
}

function Welcome() {
  return (
    <section class="px-corners-lg relative flex h-[clamp(340px,48vh,460px)] shrink-0 overflow-hidden bg-[#3E6FB0]">
      <Scene variant="day" />
      <div class="absolute inset-0 bg-[linear-gradient(0deg,rgb(16_17_19/0.85)_0%,rgb(16_17_19/0.4)_45%,transparent_75%)]" />
      <div class="relative mt-auto flex max-w-[600px] flex-col gap-4 px-9 pb-8">
        <h1 class="title-display text-hero">Prêt à miner ?</h1>
        <p class="text-base text-chalk-2">
          Crée une instance, Tandem s'occupe du reste.
        </p>
        <button
          class="btn btn-primary px-corners-md title-display h-[54px] w-fit px-7 text-[28px] uppercase"
          onClick={() => setNewInstanceDialog({})}
        >
          <Icon name="plus" size={12} />
          Créer une instance
        </button>
      </div>
    </section>
  );
}

/** Join a friend straight from the home page, with the code they sent. */
function JoinFriend() {
  const [code, setCode] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const canPlayOnline = () => activeAccount()?.kind === "microsoft";

  async function join() {
    setBusy(true);
    setError(null);
    try {
      await joinFriend(code());
      navigate({ page: "multi" });
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="panel flex flex-col gap-3 p-4" aria-labelledby="home-join">
      <h2 id="home-join" class="panel-title">
        Rejoindre un ami
      </h2>
      <Show
        when={!duo.guest}
        fallback={
          <button class="btn btn-primary px-corners h-10" onClick={() => navigate({ page: "multi" })}>
            Voir la partie de {duo.guest!.hostPlayer}
          </button>
        }
      >
        <Show
          when={canPlayOnline()}
          fallback={
            <div class="flex flex-col gap-2.5">
              <p class="text-meta text-muted">Connecte ton compte Microsoft pour jouer à plusieurs.</p>
              <button class="btn px-corners h-10" onClick={() => openAccounts()}>
                Ajouter mon compte
              </button>
            </div>
          }
        >
          <form
            class="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              void join();
            }}
          >
            <input
              class="field h-10 min-w-0 flex-1 text-center font-mono text-base tracking-[0.2em] uppercase"
              placeholder="XXXX-XXXX"
              aria-label="Code d'invitation"
              autocomplete="off"
              spellcheck={false}
              value={code()}
              onInput={(e) => {
                const formatted = formatCode(e.currentTarget.value);
                e.currentTarget.value = formatted;
                setCode(formatted);
              }}
            />
            <button
              type="submit"
              class="btn btn-primary px-corners h-10 px-4"
              disabled={busy() || code().replace("-", "").length !== 8}
            >
              {busy() ? "Connexion…" : "Rejoindre"}
            </button>
          </form>
          <Show when={error()}>
            <Alert onClose={() => setError(null)}>{error()}</Alert>
          </Show>
        </Show>
      </Show>
    </section>
  );
}

function News() {
  const [versions] = createResource(api.listVersions);
  return (
    <section class="panel flex flex-col gap-3.5 p-4" aria-labelledby="home-news">
      <h2 id="home-news" class="panel-title">
        Nouveautés
      </h2>
      <Show
        when={versions()}
        fallback={<p class="text-meta text-faint">{versions.error ? "Hors ligne pour l'instant." : "Chargement…"}</p>}
      >
        {(list) => (
          <>
            <div class="flex items-center gap-3">
              <span class="slot size-10">
                <Icon name="check" size={12} color="var(--color-grass-light)" />
              </span>
              <div class="flex flex-col">
                <span class="text-sm font-semibold">Minecraft {list().latest.release}</span>
                <span class="text-meta text-muted">Dernière version stable</span>
              </div>
            </div>
            <Show when={list().latest.snapshot !== list().latest.release}>
              <div class="flex items-start gap-3">
                <span class="slot size-10">
                  <Icon name="sparkle" size={12} color="var(--color-amethyst)" />
                </span>
                <div class="flex min-w-0 flex-col gap-2">
                  <div class="flex flex-col">
                    <span class="text-sm font-semibold">Snapshot {list().latest.snapshot}</span>
                    <span class="text-meta text-muted">Version de test</span>
                  </div>
                  <button
                    class="btn px-corners h-8 w-fit px-3 text-meta"
                    onClick={() => setNewInstanceDialog({ version: list().latest.snapshot })}
                  >
                    Créer une instance de test
                  </button>
                </div>
              </div>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}

export default function Home() {
  const featured = () => instances()[0];
  const others = () => instances().slice(1, 4);

  return (
    <div class="mx-auto flex max-w-[1400px] flex-col gap-6">
      <Show when={featured()} fallback={<Welcome />}>
        {(instance) => <Hero instance={instance()} />}
      </Show>

      <div class="flex gap-6 max-[1100px]:flex-col">
        <section aria-labelledby="home-instances" class="flex min-w-0 flex-1 flex-col gap-3">
          <div class="flex h-9 items-center justify-between gap-3">
            <h2 id="home-instances" class="text-xl font-semibold">
              Tes autres instances
            </h2>
            <div class="flex gap-2">
              <Show when={instances().length > 4}>
                <button class="btn btn-ghost h-9 text-sm" onClick={() => navigate({ page: "instances" })}>
                  Tout voir ({instances().length})
                </button>
              </Show>
              <button class="btn px-corners h-9 text-sm" onClick={() => setNewInstanceDialog({})}>
                <Icon name="plus" size={12} />
                Nouvelle instance
              </button>
            </div>
          </div>
          <Show
            when={others().length > 0}
            fallback={
              <p class="text-sm text-muted">
                {featured()
                  ? "Pas d'autre instance pour l'instant."
                  : "Tes instances apparaîtront ici."}
              </p>
            }
          >
            <div class="grid grid-cols-[repeat(auto-fit,minmax(190px,1fr))] gap-4">
              <For each={others()}>{(instance) => <InstanceCard instance={instance} />}</For>
            </div>
          </Show>
        </section>

        <aside class="flex w-[320px] shrink-0 flex-col gap-4 max-[1100px]:w-full">
          <JoinFriend />
          <News />
        </aside>
      </div>
    </div>
  );
}
