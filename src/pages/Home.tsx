import { createResource, createSignal, For, Show } from "solid-js";
import Alert from "../components/Alert";
import InstanceCard from "../components/InstanceCard";
import { Icon, LoaderTag } from "../components/pixel";
import PlayButton from "../components/PlayButton";
import Scene from "../components/Scene";
import { openAccounts } from "../lib/accounts";
import { api, errorMessage, type Instance, screenshotUrl } from "../lib/api";
import { duo, formatCode, joinFriend } from "../lib/duo";
import { formatRelative } from "../lib/format";
import { gameState } from "../lib/games";
import { activeAccount, instances, navigate, setNewInstanceDialog } from "../lib/store";

/**
 * The big picture behind the instance: the modpack's featured gallery image, otherwise the
 * player's latest screenshot in it; null leaves the drawn sky.
 */
async function heroImage(instance: Instance): Promise<string | null> {
  if (instance.packProjectId) {
    const project = await api.projectDetails(instance.packProjectId).catch(() => null);
    // The backend sorts the gallery with the featured image first.
    if (project?.gallery[0]) return project.gallery[0].url;
  }
  const shots = await api.listScreenshots(instance.id).catch(() => []);
  const latest = shots.reduce<(typeof shots)[number] | null>((a, b) => (!a || b.takenAt > a.takenAt ? b : a), null);
  return latest ? screenshotUrl(instance.id, latest) : null;
}

function Hero(props: { instance: Instance }) {
  const state = () => gameState(props.instance.id);
  const exit = () => state().lastExit;
  const [error, setError] = createSignal<string | null>(null);
  const [image] = createResource(() => props.instance, heroImage);
  const [shown, setShown] = createSignal(false);
  const guests = () => duo.host?.guests.length ?? 0;

  return (
    <section
      aria-label="Reprendre la dernière partie"
      class="px-corners-lg relative flex h-[clamp(340px,48vh,460px)] shrink-0 overflow-hidden bg-[#1E1A33]"
    >
      <Scene variant="sunset" />
      <Show when={image()} keyed>
        {(url) => (
          <img
            src={url}
            alt=""
            class="absolute inset-0 h-full w-full object-cover transition-opacity duration-500 ease-out motion-reduce:transition-none"
            classList={{ "opacity-0": !shown() }}
            onLoad={() => setShown(true)}
            onError={(e) => e.currentTarget.remove()}
          />
        )}
      </Show>
      {/* Scrim under the text only: the picture stays bright on the right. */}
      <div class="absolute inset-0 bg-[linear-gradient(0deg,rgb(16_17_19/0.94)_0%,rgb(16_17_19/0.55)_42%,transparent_72%),linear-gradient(90deg,rgb(16_17_19/0.6)_0%,transparent_60%)]" />

      <div class="relative mt-auto flex w-full flex-col gap-5 px-9 pb-8">
        <div class="flex min-w-0 flex-col gap-2">
          <h1 class="title-display truncate text-[64px] leading-[0.9]">{props.instance.name}</h1>
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

        <div class="flex flex-wrap items-center gap-3">
          <PlayButton id={props.instance.id} size="lg" name={props.instance.name} />
          <button class="btn btn-light px-corners-md h-[54px] px-6 text-[15px]" onClick={() => navigate({ page: "multi" })}>
            <Icon name="invite" size={12} />
            <Show when={duo.host} fallback="Inviter des amis">
              {guests() === 0 ? "Partie ouverte" : guests() === 1 ? "1 ami connecté" : `${guests()} amis connectés`}
            </Show>
          </button>
          <button
            class="btn px-corners-md size-[54px] px-0"
            aria-label="Détails et console"
            title="Détails et console"
            onClick={() => navigate({ page: "instance", id: props.instance.id })}
          >
            <Icon name="terminal" size={24} />
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
        <h1 class="title-display text-[64px] leading-[0.9]">Prêt à miner ?</h1>
        <p class="text-base text-chalk-2">
          Crée une instance : Tandem télécharge le jeu et la bonne version de Java pour toi.
        </p>
        <button
          class="btn btn-primary px-corners-md title-display h-[54px] w-fit px-7 text-[26px] uppercase"
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
      <div class="flex flex-col gap-1">
        <h2 id="home-join" class="panel-title">
          Rejoindre un ami
        </h2>
        <p class="text-[13px] text-muted">Entre le code qu'il t'a envoyé : Tandem prépare tout et lance le jeu.</p>
      </div>
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
              <p class="text-[13px] text-chalk-2">Il faut un compte Microsoft pour jouer avec des amis.</p>
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
        fallback={<p class="text-[13px] text-faint">{versions.error ? "Hors ligne pour l'instant." : "Chargement…"}</p>}
      >
        {(list) => (
          <>
            <div class="flex items-center gap-3">
              <span class="slot size-10">
                <Icon name="check" size={12} color="var(--color-grass-light)" />
              </span>
              <div class="flex flex-col">
                <span class="text-sm font-semibold">Minecraft {list().latest.release}</span>
                <span class="text-[13px] text-muted">Dernière version stable</span>
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
                    <span class="text-[13px] text-muted">Teste-la dans une instance à part, sans toucher à tes mondes.</span>
                  </div>
                  <button
                    class="btn px-corners h-8 w-fit px-3 text-[13px]"
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
                  ? "Une seule instance pour l'instant. Installe un modpack depuis Découvrir ou crée-en une autre."
                  : "Tes instances apparaîtront ici."}
              </p>
            }
          >
            <div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3.5">
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
