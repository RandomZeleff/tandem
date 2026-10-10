import { createMemo, createResource, createSignal, For, type JSX, Match, onMount, Show, Switch } from "solid-js";
import Alert from "../components/Alert";
import { AccountAvatar } from "../components/accounts/AccountVisuals";
import InstanceSlot from "../components/InstanceSlot";
import PlayButton from "../components/PlayButton";
import Scene from "../components/Scene";
import Select from "../components/Select";
import { Checkbox, Icon, LoaderTag, SkinHead } from "../components/pixel";
import { openAccounts } from "../lib/accounts";
import { api, errorMessage, type InstanceDiff, type LinkStatus } from "../lib/api";
import {
  autoLaunch,
  changeInstance,
  diffMatches,
  duo,
  formatCode,
  joinFriend,
  kickGuest,
  leaveFriend,
  playWithFriend,
  setAutoLaunch,
  startDuoEvents,
  startHosting,
  stopHosting,
} from "../lib/duo";
import { loaderLabel } from "../lib/format";
import { gameState, output } from "../lib/games";
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

/** 1.20+ (and the year-numbered versions after 1.21) open a world straight from the launch. */
function opensWorldAtLaunch(gameVersion: string): boolean {
  const [major, minor] = gameVersion.split(".").map(Number);
  return major > 1 || (major === 1 && minor >= 20);
}

/** The host is in a world of their game, judging by its log (any version, any loader). */
function inWorld(instanceId: string): boolean {
  const lines = output[instanceId] ?? [];
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].line;
    if (line.includes("Stopping server")) return false;
    if (line.includes("Starting integrated minecraft server")) return true;
  }
  return false;
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
  const [world, setWorld] = remembered<string>("duo-host-world", "");
  const [code, setCode] = createSignal("");
  const [busy, setBusy] = createSignal<"host" | "join" | null>(null);
  const [error, setError] = createSignal<string | null>(null);

  const chosen = createMemo(() => instances().find((i) => i.id === hostInstance()));
  const quickPlay = () => !!chosen() && opensWorldAtLaunch(chosen()!.gameVersion);
  const [worlds] = createResource(
    () => (quickPlay() ? hostInstance() : null),
    async (id) => {
      const list = await api.listWorlds(id).catch(() => []);
      return [...list].sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0));
    },
  );
  /** The world picked, or the last one played when none (or one since deleted) is. */
  const pickedWorld = () => {
    if (world() === "menu") return null;
    const list = worlds() ?? [];
    return list.find((w) => w.folder === world()) ?? list[0] ?? null;
  };
  const worldOptions = () => [
    ...(worlds() ?? []).map((w) => ({ value: w.folder, label: w.name })),
    { value: "menu", label: "Je choisis dans le jeu" },
  ];
  const running = () => !!hostInstance() && gameState(hostInstance()).status !== "idle";

  async function host() {
    setBusy("host");
    setError(null);
    try {
      await startHosting(hostInstance() || null, quickPlay() ? pickedWorld()?.folder : undefined);
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
      await joinFriend(code());
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
          <Show when={quickPlay() && !running() && (worlds()?.length ?? 0) > 0}>
            <label class="flex flex-col gap-1.5">
              <span class="text-xs text-muted">Monde</span>
              <Select class="h-10 w-full text-sm" value={pickedWorld()?.folder ?? "menu"} options={worldOptions()} onChange={setWorld} />
            </label>
          </Show>
          <button class="btn btn-gold px-corners mt-auto h-11" disabled={busy() !== null || !hostInstance()} onClick={() => void host()}>
            <Icon name="invite" size={14} />
            {busy() === "host" ? "Création de l'invitation…" : running() ? "Créer une invitation" : "Inviter et lancer le jeu"}
          </button>
        </section>

        <section class="panel px-corners-md flex flex-col gap-4 p-5">
          <div class="flex flex-col gap-1">
            <h2 class="panel-title text-gold!">Rejoindre un ami</h2>
            <p class="text-sm text-chalk-2">
              Entre le code que ton ami t'a donné. Tandem prend ton instance qui correspond à la sienne et lance le jeu
              dès que son monde est ouvert.
            </p>
          </div>
          <form
            class="mt-auto flex flex-col gap-4"
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
            <button
              type="submit"
              class="btn btn-primary px-corners h-11"
              disabled={busy() !== null || code().replace("-", "").length !== 8}
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
  const playing = () => running() && inWorld(host().instanceId!);
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
                <div class="flex flex-col gap-1.5 text-xs">
                  <Show when={playing()}>
                    <span class="font-semibold text-gold">Tu es dans ton monde : il ne reste que cette étape.</span>
                  </Show>
                  <ol class="flex flex-col gap-0.5 text-chalk-2">
                    <li>
                      1. Appuie sur <kbd class="chip px-1.5 font-mono text-[11px]">Échap</kbd>
                    </li>
                    <li>
                      2. Clique sur <strong>Ouvrir au réseau local</strong>
                    </li>
                    <li>
                      3. Clique sur <strong>Démarrer le monde en LAN</strong>
                    </li>
                  </ol>
                  <span class="text-muted">Tandem le détecte tout seul.</span>
                </div>
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
  const state = () => (guest().instanceId ? gameState(guest().instanceId!) : null);
  const busy = () => state()?.status === "preparing" || state()?.status === "running";
  const open = () => !!guest().world;
  const matches = () => !guest().diff || diffMatches(guest().diff!);
  const [changing, setChanging] = createSignal(false);

  async function change(id: string) {
    setChanging(true);
    try {
      await changeInstance(id);
    } catch (err) {
      toast(errorMessage(err), { tone: "error" });
    } finally {
      setChanging(false);
    }
  }

  const playLabel = () => {
    const status = state()?.status;
    if (status === "preparing") return "Préparation…";
    if (status === "running") return "En jeu";
    return matches() ? "Jouer" : "Jouer quand même";
  };

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
        <Show
          when={instance()}
          fallback={
            <div class="flex flex-col gap-1">
              <span class="font-semibold">Aucune de tes instances ne correspond</span>
              <Show
                when={guest().hostInstance}
                fallback={<span class="text-sm text-chalk-2">Choisis l'instance avec laquelle jouer.</span>}
              >
                {(h) => (
                  <span class="text-sm text-chalk-2">
                    {guest().hostPlayer} joue à « {h().name} » : Minecraft {h().gameVersion} avec {loaderLabel(h().loader)}.
                    Crée la même instance dans Instances, ou choisis-en une ci-dessous.
                  </span>
                )}
              </Show>
            </div>
          }
        >
          {(i) => (
            <div class="flex items-center gap-4">
              <InstanceSlot instance={i()} size={48} />
              <div class="flex min-w-0 flex-1 flex-col">
                <span class="truncate font-semibold">{i().name}</span>
                <span class="flex items-center gap-1.5 text-xs text-muted">
                  Minecraft {i().gameVersion} · <LoaderTag loader={i().loader} />
                </span>
              </div>
              <button
                class="btn btn-primary px-corners h-12 px-6 text-base"
                disabled={!open() || state()?.status !== "idle"}
                onClick={playWithFriend}
                title={open() ? "Lance le jeu directement dans le monde de ton ami" : "Attends que ton ami ouvre son monde"}
              >
                <Icon name="play" size={14} />
                {playLabel()}
              </button>
            </div>
          )}
        </Show>
        <DiffNote diff={guest().diff} who={`Par rapport à ${guest().hostPlayer}, ton instance a`} />
        <div class="flex flex-wrap items-center gap-x-6 gap-y-3">
          <label class="flex min-w-[280px] flex-1 items-center gap-3">
            <span class="shrink-0 text-xs text-muted">{instance() ? "Autre instance" : "Instance"}</span>
            <Select
              class="h-9 w-full text-sm"
              value={guest().instanceId ?? ""}
              placeholder="Choisir…"
              label="Instance avec laquelle jouer"
              options={instanceOptions()}
              disabled={changing() || busy()}
              onChange={(id) => void change(id)}
            />
          </label>
          <Checkbox
            class="text-xs text-chalk-2"
            checked={autoLaunch()}
            label="Lancer le jeu dès que le monde est ouvert"
            onChange={setAutoLaunch}
          />
        </div>
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
