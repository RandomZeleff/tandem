import { createMemo, createSignal, For, type JSX, Match, onMount, Show, Switch } from "solid-js";
import Alert from "../components/Alert";
import { AccountAvatar } from "../components/accounts/AccountVisuals";
import InstanceSlot from "../components/InstanceSlot";
import PlayButton from "../components/PlayButton";
import Scene from "../components/Scene";
import Select from "../components/Select";
import { Icon, LoaderTag, SkinHead } from "../components/pixel";
import { openAccounts } from "../lib/accounts";
import { errorMessage, type InstanceDiff, type LinkStatus } from "../lib/api";
import {
  duo,
  formatCode,
  joinFriend,
  kickGuest,
  leaveFriend,
  playWithFriend,
  startDuoEvents,
  startHosting,
  stopHosting,
} from "../lib/duo";
import { loaderLabel } from "../lib/format";
import { gameState } from "../lib/games";
import { skinLook } from "../lib/look";
import { activeAccount, instances, remembered } from "../lib/store";
import { toast } from "../lib/toast";

/** Ping and kind of link, as a small chip. */
function LinkChip(props: { link?: LinkStatus | null }) {
  return (
    <Show when={props.link} fallback={<span class="chip h-6 px-2 text-xs text-muted">connexion…</span>}>
      {(link) => (
        <span
          class="chip h-6 gap-1.5 px-2 font-mono text-xs"
          classList={{
            "text-xp-text": link().pingMs < 80,
            "text-gold": link().pingMs >= 80 && link().pingMs < 180,
            "text-redstone-text": link().pingMs >= 180,
          }}
          title={
            link().direct
              ? "Liaison directe entre vos deux PC"
              : "Liaison par un relais : vos box ne se laissent pas joindre directement. Ça marche, avec un peu plus de latence."
          }
        >
          {link().pingMs} ms · {link().direct ? "direct" : "relais"}
        </span>
      )}
    </Show>
  );
}

/** What differs between the two instances, in a few words. */
function DiffNote(props: { diff: InstanceDiff | null | undefined; who: string }) {
  const lines = () => {
    const d = props.diff;
    if (!d) return [];
    const out: string[] = [];
    if (!d.sameGame) out.push("une autre version de Minecraft");
    if (!d.sameLoader) out.push("un autre loader");
    if (d.missing.length > 0) out.push(`${d.missing.length} mod${d.missing.length > 1 ? "s" : ""} en moins`);
    if (d.extra.length > 0) out.push(`${d.extra.length} mod${d.extra.length > 1 ? "s" : ""} en plus`);
    return out;
  };
  return (
    <Show when={lines().length > 0}>
      <p class="text-xs text-gold">
        {props.who} : {lines().join(", ")}. La partie peut refuser la connexion si les mods ne sont pas les mêmes.
      </p>
    </Show>
  );
}

/** A step of the host's checklist. */
function Step(props: { done: boolean; current: boolean; title: string; children?: JSX.Element }) {
  return (
    <li class="flex gap-3 px-4 py-3" classList={{ "opacity-50": !props.done && !props.current }}>
      <span
        class="mt-0.5 flex size-5 shrink-0 items-center justify-center"
        classList={{ "bg-xp-deep": props.done, "bg-slate-600": !props.done }}
      >
        <Show when={props.done} fallback={<Show when={props.current}><span class="step-pulse size-2 bg-gold" /></Show>}>
          <Icon name="check" size={11} color="#fff" />
        </Show>
      </span>
      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <span class="text-sm font-semibold">{props.title}</span>
        {props.children}
      </div>
    </li>
  );
}

function instanceOptions() {
  return instances().map((i) => ({
    value: i.id,
    label: i.name,
    hint: `${i.gameVersion} · ${loaderLabel(i.loader)}`,
  }));
}

/** Both players need a Microsoft account: said before anything else. */
function AccountGate(props: { children: JSX.Element }) {
  return (
    <Show
      when={activeAccount()?.kind === "microsoft"}
      fallback={
        <div class="panel px-corners-md flex items-center gap-4 p-5 shadow-[inset_0_0_0_1px_var(--color-gold-deep)]">
          <Icon name="user" size={22} class="shrink-0 text-gold" />
          <div class="flex flex-1 flex-col gap-1">
            <span class="font-semibold">Un compte Microsoft est nécessaire</span>
            <span class="text-sm text-chalk-2">
              Le monde ouvert vérifie les comptes auprès de Mojang : toi et ton ami devez jouer avec votre compte
              Microsoft, pas un compte hors ligne.
            </span>
          </div>
          <button class="btn btn-gold px-corners shrink-0" onClick={() => openAccounts()}>
            Comptes
          </button>
        </div>
      }
    >
      {props.children}
    </Show>
  );
}

function Start() {
  const [hostInstance, setHostInstance] = remembered<string>("duo-host-instance", instances()[0]?.id ?? "");
  const [guestInstance, setGuestInstance] = remembered<string>("duo-guest-instance", instances()[0]?.id ?? "");
  const [code, setCode] = createSignal("");
  const [busy, setBusy] = createSignal<"host" | "join" | null>(null);
  const [error, setError] = createSignal<string | null>(null);

  async function host() {
    setBusy("host");
    setError(null);
    try {
      await startHosting(hostInstance() || null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  }

  async function join() {
    setBusy("join");
    setError(null);
    try {
      await joinFriend(code(), guestInstance());
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  }

  return (
    <AccountGate>
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>
      <div class="grid grid-cols-2 gap-5 max-[1180px]:grid-cols-1">
        <section class="panel px-corners-md flex flex-col gap-4 p-5">
          <div class="flex flex-col gap-1">
            <h2 class="panel-title text-gold!">Inviter un ami</h2>
            <p class="text-sm text-chalk-2">
              Tu héberges : ton ami rejoint ton monde avec un code. Ton monde reste chez toi.
            </p>
          </div>
          <label class="flex flex-col gap-1.5">
            <span class="text-xs text-muted">Instance où tu joues</span>
            <Select class="h-10 w-full text-sm" value={hostInstance()} options={instanceOptions()} onChange={setHostInstance} />
          </label>
          <button class="btn btn-gold px-corners mt-auto h-11" disabled={busy() !== null || !hostInstance()} onClick={() => void host()}>
            <Icon name="invite" size={14} />
            {busy() === "host" ? "Création de l'invitation…" : "Créer une invitation"}
          </button>
        </section>

        <section class="panel px-corners-md flex flex-col gap-4 p-5">
          <div class="flex flex-col gap-1">
            <h2 class="panel-title text-gold!">Rejoindre un ami</h2>
            <p class="text-sm text-chalk-2">Entre le code que ton ami t'a donné.</p>
          </div>
          <form
            class="flex flex-col gap-4"
            onSubmit={(e) => {
              e.preventDefault();
              void join();
            }}
          >
            <input
              class="field h-12 w-full text-center font-mono text-2xl tracking-[0.25em] uppercase"
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
            <label class="flex flex-col gap-1.5">
              <span class="text-xs text-muted">Avec l'instance</span>
              <Select class="h-10 w-full text-sm" value={guestInstance()} options={instanceOptions()} onChange={setGuestInstance} />
            </label>
            <button
              type="submit"
              class="btn btn-primary px-corners h-11"
              disabled={busy() !== null || code().replace("-", "").length !== 8 || !guestInstance()}
            >
              {busy() === "join" ? "Connexion…" : "Rejoindre"}
            </button>
          </form>
        </section>
      </div>
      <p class="text-xs text-muted">
        Aucun serveur à louer ni à configurer : vos deux PC se parlent directement quand c'est possible, sinon par un
        relais chiffré gratuit. Il faut la même version de Minecraft et les mêmes mods des deux côtés.
      </p>
    </AccountGate>
  );
}

function Hosting() {
  const host = () => duo.host!;
  const instance = createMemo(() => instances().find((i) => i.id === host().instanceId));
  const running = () => (host().instanceId ? gameState(host().instanceId!).status === "running" : false);
  const [copied, setCopied] = createSignal(false);

  async function copy() {
    await navigator.clipboard.writeText(host().code).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-5 max-[1180px]:grid-cols-1">
      <section class="panel px-corners-md flex flex-col items-center gap-4 p-6 text-center">
        <span class="panel-title text-gold!">Ton code d'invitation</span>
        <button
          class="slot px-6 py-4 font-mono text-4xl font-bold tracking-[0.2em] text-chalk select-all hover:text-gold"
          title="Copier le code"
          onClick={() => void copy()}
        >
          {host().code}
        </button>
        <button class="btn px-corners h-9 px-4 text-sm" onClick={() => void copy()}>
          <Icon name={copied() ? "check" : "invite"} size={12} />
          {copied() ? "Code copié" : "Copier le code"}
        </button>
        <p class="max-w-sm text-sm text-chalk-2">
          Donne ce code à ton ami : il l'entre dans Tandem, page « Jouer à deux ». Il reste valable tant que tu
          n'arrêtes pas l'invitation.
        </p>
        <button class="btn btn-ghost mt-auto text-sm" onClick={() => void stopHosting()}>
          Arrêter l'invitation
        </button>
      </section>

      <div class="flex flex-col gap-5">
        <ol class="panel px-corners-md flex flex-col divide-y divide-line">
          <Step done={running() || !!host().world} current={!running() && !host().world} title={`Lance ${instance()?.name ?? "ton instance"}`}>
            <Show when={instance() && !running() && !host().world}>
              <div>
                <PlayButton id={instance()!.id} size="md" name={instance()!.name} />
              </div>
            </Show>
          </Step>
          <Step done={!!host().world} current={running() && !host().world} title="Ouvre ton monde au réseau local">
            <Show
              when={host().world}
              fallback={
                <span class="text-xs text-muted">
                  Dans le jeu, une fois dans ton monde : Échap → <strong>Ouvrir au réseau local</strong> →{" "}
                  <strong>Démarrer le monde en LAN</strong>. Tandem le détecte tout seul.
                </span>
              }
            >
              {(world) => <span class="text-xs text-xp-text">Monde ouvert : {world().motd}</span>}
            </Show>
          </Step>
          <Step done={host().guests.length > 0} current={!!host().world && host().guests.length === 0} title="Ton ami rejoint avec le code">
            <Show when={host().guests.length === 0}>
              <span class="text-xs text-muted">Son jeu se lance directement dans ton monde.</span>
            </Show>
          </Step>
        </ol>

        <Show when={host().guests.length > 0}>
          <section class="flex flex-col gap-2">
            <h3 class="panel-title">Dans ta partie</h3>
            <ul class="panel px-corners-md flex flex-col divide-y divide-line">
              <For each={host().guests}>
                {(guest) => (
                  <li class="flex flex-col gap-1.5 px-4 py-3">
                    <div class="flex items-center gap-3">
                      <span class="slot h-10 w-10">
                        <SkinHead look={skinLook(guest.player)} size={28} />
                      </span>
                      <span class="flex-1 truncate font-semibold">{guest.player}</span>
                      <LinkChip link={guest.link} />
                      <button
                        class="btn btn-ghost h-8 px-2.5 text-xs hover:text-redstone-text"
                        onClick={() => void kickGuest(guest.id).then(() => toast(`${guest.player} a été retiré`, { tone: "info" }))}
                      >
                        Exclure
                      </button>
                    </div>
                    <DiffNote diff={guest.diff} who={`L'instance de ${guest.player}`} />
                  </li>
                )}
              </For>
            </ul>
          </section>
        </Show>
      </div>
    </div>
  );
}

function Joined() {
  const guest = () => duo.guest!;
  const instance = createMemo(() => instances().find((i) => i.id === guest().instanceId));
  const state = () => gameState(guest().instanceId);
  const open = () => !!guest().world;

  return (
    <div class="flex flex-col gap-5">
      <section class="panel px-corners-md flex items-center gap-5 p-5">
        <span class="slot h-16 w-16">
          <SkinHead look={skinLook(guest().hostPlayer)} size={44} />
        </span>
        <div class="flex min-w-0 flex-1 flex-col gap-1">
          <span class="text-xs tracking-wide text-gold uppercase">Partie de</span>
          <span class="truncate text-2xl font-bold">{guest().hostPlayer}</span>
          <span class="text-sm" classList={{ "text-xp-text": open(), "text-muted": !open() }}>
            {open()
              ? `Monde ouvert : ${guest().world!.motd}`
              : `${guest().hostPlayer} n'a pas encore ouvert son monde au réseau local.`}
          </span>
        </div>
        <LinkChip link={guest().link} />
      </section>

      <section class="panel px-corners-md flex flex-col gap-4 p-5">
        <div class="flex items-center gap-4">
          <Show when={instance()}>{(i) => <InstanceSlot instance={i()} size={48} />}</Show>
          <div class="flex min-w-0 flex-1 flex-col">
            <span class="truncate font-semibold">{instance()?.name ?? "Instance supprimée"}</span>
            <Show when={instance()}>
              {(i) => (
                <span class="flex items-center gap-1.5 text-xs text-muted">
                  Minecraft {i().gameVersion} · <LoaderTag loader={i().loader} />
                </span>
              )}
            </Show>
          </div>
          <button
            class="btn btn-primary px-corners h-12 px-6 text-base"
            disabled={!open() || state().status !== "idle" || !instance()}
            onClick={playWithFriend}
            title={open() ? "Lance le jeu directement dans le monde de ton ami" : "Attends que ton ami ouvre son monde"}
          >
            <Icon name="play" size={14} />
            {state().status === "preparing" ? "Préparation…" : state().status === "running" ? "En jeu" : "Jouer"}
          </button>
        </div>
        <DiffNote diff={guest().diff} who={`Par rapport à ${guest().hostPlayer}, ton instance a`} />
        <p class="text-xs text-muted">
          Déjà en jeu ? La partie apparaît aussi dans Multijoueur, sous « Parties en réseau local ».
        </p>
      </section>

      <div>
        <button class="btn btn-ghost text-sm" onClick={() => void leaveFriend()}>
          Quitter la partie
        </button>
      </div>
    </div>
  );
}

/** "Jouer à deux": invite a friend into your world, or join theirs, with a short code. */
export default function Duo() {
  onMount(() => void startDuoEvents());
  return (
    <div class="flex flex-col gap-5">
      <section class="px-corners-lg relative h-[170px] shrink-0 bg-slate-700">
        <Scene variant="night" />
        <div class="absolute inset-0 bg-[linear-gradient(90deg,rgb(10_11_22/0.92)_0%,rgb(10_11_22/0.6)_50%,transparent_80%)]" />
        <div class="relative flex h-full max-w-[620px] flex-col justify-center gap-2 px-8">
          <span class="flex items-center gap-2 font-pixel text-[13px] font-medium tracking-[2px] text-gold">
            <span class="size-1.5 bg-gold" />
            JOUER À DEUX
          </span>
          <h1 class="font-pixel text-[32px] leading-tight font-bold [text-shadow:4px_4px_0_rgb(0_0_0/0.45)]">
            Ton monde, ton pote. Zéro serveur.
          </h1>
          <Show when={activeAccount()}>
            {(account) => (
              <span class="flex items-center gap-2 text-sm text-chalk-3">
                <AccountAvatar account={account()} size={18} /> Tu joues en tant que {account().username}
              </span>
            )}
          </Show>
        </div>
      </section>

      <Switch fallback={<Start />}>
        <Match when={duo.host}>
          <Hosting />
        </Match>
        <Match when={duo.guest}>
          <Joined />
        </Match>
      </Switch>
    </div>
  );
}
