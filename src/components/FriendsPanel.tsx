import { createSignal, For, type JSX, Show } from "solid-js";
import { api, errorMessage, type FriendView, type Presence } from "../lib/api";
import { duo } from "../lib/duo";
import { dismissInvite, friends, invites, joinInvite, refreshFriends } from "../lib/friends";
import { formatCode } from "../lib/duo";
import { removeWithUndo, toast } from "../lib/toast";
import { AccountAvatar } from "./accounts/AccountVisuals";
import { Icon, Toggle } from "./pixel";

function presenceText(presence: Presence | null): string {
  if (!presence) return "Hors ligne";
  switch (presence.kind) {
    case "online":
      return "En ligne";
    case "playing":
      return `En jeu · ${presence.instance}`;
    case "hosting":
      return presence.instance ? `Partie ouverte · ${presence.instance}` : "Partie ouverte";
  }
}

function Row(props: { friend: FriendView; status: JSX.Element; children?: JSX.Element; dim?: boolean }) {
  return (
    <li class="row flex items-center gap-3 px-2 py-2" classList={{ "opacity-60": props.dim }}>
      <span class="slot relative size-10">
        <AccountAvatar account={{ username: props.friend.name, avatar: props.friend.avatar }} size={32} />
      </span>
      <div class="flex min-w-0 flex-1 flex-col">
        <span class="truncate text-sm font-semibold">{props.friend.name}</span>
        <span class="truncate text-meta text-muted">{props.status}</span>
      </div>
      <div class="flex shrink-0 items-center gap-1.5">{props.children}</div>
    </li>
  );
}

/** Friends: the player's code, adding one, requests, invites, who plays what. */
export default function FriendsPanel() {
  const [code, setCode] = createSignal("");
  const [adding, setAdding] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [invited, setInvited] = createSignal<ReadonlySet<string>>(new Set());
  const [hiddenIds, setHiddenIds] = createSignal<ReadonlySet<string>>(new Set());

  const list = () => friends().friends.filter((f) => !hiddenIds().has(f.id));
  const incoming = () => list().filter((f) => f.state === "incoming");
  const outgoing = () => list().filter((f) => f.state === "outgoing");
  const accepted = () =>
    list()
      .filter((f) => f.state === "friend")
      .sort((a, b) => Number(!!b.presence) - Number(!!a.presence));

  async function run(task: () => Promise<void>) {
    setError(null);
    try {
      await task();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      void refreshFriends();
    }
  }

  async function add(e: SubmitEvent) {
    e.preventDefault();
    setAdding(true);
    await run(async () => {
      await api.friendsAdd(code());
      setCode("");
      toast("Demande envoyée");
    });
    setAdding(false);
  }

  async function invite(friend: FriendView) {
    await run(async () => {
      await api.friendsInvite(friend.id);
      setInvited((set) => new Set(set).add(friend.id));
      toast(`${friend.name} est invité`);
    });
  }

  function remove(friend: FriendView, message: string) {
    removeWithUndo({
      message,
      hide: () => setHiddenIds((set) => new Set(set).add(friend.id)),
      show: () =>
        setHiddenIds((set) => {
          const next = new Set(set);
          next.delete(friend.id);
          return next;
        }),
      commit: async () => {
        await api.friendsRemove(friend.id);
        await refreshFriends();
      },
    });
  }

  async function copyCode() {
    const own = friends().code;
    if (!own) return;
    await navigator.clipboard.writeText(own).catch(() => {});
    toast("Code d'ami copié");
  }

  return (
    <section class="panel flex flex-col gap-4 p-5" aria-labelledby="friends-title">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h2 id="friends-title" class="text-xl font-semibold">
          Amis
        </h2>
        <Show when={friends().code}>
          {(own) => (
            <button
              class="flex items-center gap-2 rounded-md bg-slate-900 py-1.5 pr-2.5 pl-3 text-meta shadow-[inset_0_0_0_1px_var(--color-line)] hover:bg-slate-800 focus-visible:bg-slate-800"
              title="Copier mon code d'ami"
              onClick={() => void copyCode()}
            >
              <span class="text-muted">Mon code</span>
              <span class="font-mono font-semibold tracking-widest text-chalk">{own()}</span>
              <Icon name="external" size={12} class="text-faint" />
            </button>
          )}
        </Show>
      </div>

      <Show
        when={friends().available}
        fallback={<p class="text-sm text-muted">Connecte ton compte Microsoft pour avoir des amis.</p>}
      >
        <form class="flex gap-2" onSubmit={add}>
          <input
            class="field h-10 min-w-0 flex-1 font-mono tracking-[0.15em] uppercase placeholder:font-sans placeholder:tracking-normal placeholder:normal-case"
            placeholder="Code d'un ami"
            aria-label="Code d'ami"
            autocomplete="off"
            spellcheck={false}
            value={code()}
            onInput={(e) => {
              const formatted = formatCode(e.currentTarget.value);
              e.currentTarget.value = formatted;
              setCode(formatted);
            }}
          />
          <button class="btn btn-primary px-corners h-10 px-4" disabled={adding() || code().replace("-", "").length !== 8}>
            {adding() ? "Recherche…" : "Ajouter"}
          </button>
        </form>

        <Show when={error()}>
          <p class="text-sm text-redstone-text">{error()}</p>
        </Show>

        <Show when={invites().length > 0 || incoming().length > 0}>
          <ul class="flex flex-col gap-1">
            <For each={invites()}>
              {(inv) => {
                const friend = () => friends().friends.find((f) => f.id === inv.id);
                return (
                  <Show when={friend()}>
                    {(f) => (
                      <Row friend={f()} status={`T'invite${inv.instance ? ` sur ${inv.instance}` : ""}`}>
                        <button class="btn btn-primary px-corners h-8 px-3 text-sm" onClick={() => void joinInvite(inv)}>
                          Rejoindre
                        </button>
                        <button
                          class="btn btn-ghost h-8 w-8 justify-center p-0"
                          aria-label="Ignorer l'invitation"
                          title="Ignorer"
                          onClick={() => dismissInvite(inv.id)}
                        >
                          <Icon name="close" size={12} />
                        </button>
                      </Row>
                    )}
                  </Show>
                );
              }}
            </For>
            <For each={incoming()}>
              {(f) => (
                <Row friend={f} status="Veut devenir ton ami">
                  <button
                    class="btn btn-primary px-corners h-8 px-3 text-sm"
                    onClick={() => void run(() => api.friendsAccept(f.id))}
                  >
                    Accepter
                  </button>
                  <button class="btn btn-ghost h-8 px-3 text-sm" onClick={() => remove(f, `Demande de ${f.name} refusée.`)}>
                    Refuser
                  </button>
                </Row>
              )}
            </For>
          </ul>
        </Show>

        <Show
          when={accepted().length > 0 || outgoing().length > 0}
          fallback={
            <p class="text-sm text-muted">
              Échange ton code avec un ami : vous verrez quand l'autre joue et vous inviterez en un clic.
            </p>
          }
        >
          <ul class="flex flex-col gap-1">
            <For each={accepted()}>
              {(f) => {
                const joinable = () => (f.presence?.kind === "hosting" ? f.presence.code : null);
                return (
                  <Row friend={f} status={presenceText(f.presence)} dim={!f.presence}>
                    <Show when={joinable()}>
                      {(c) => (
                        <button
                          class="btn btn-primary px-corners h-8 px-3 text-sm"
                          disabled={!!duo.guest || !!duo.host}
                          onClick={() => void joinInvite({ id: f.id, code: c() })}
                        >
                          Rejoindre
                        </button>
                      )}
                    </Show>
                    <Show when={duo.host && f.presence && !joinable()}>
                      <button
                        class="btn px-corners h-8 px-3 text-sm"
                        disabled={invited().has(f.id)}
                        onClick={() => void invite(f)}
                      >
                        {invited().has(f.id) ? "Invité" : "Inviter"}
                      </button>
                    </Show>
                    <button
                      class="btn btn-ghost h-8 w-8 justify-center p-0 text-faint"
                      aria-label={`Retirer ${f.name} de mes amis`}
                      title="Retirer de mes amis"
                      onClick={() => remove(f, `${f.name} retiré de tes amis.`)}
                    >
                      <Icon name="trash" size={12} />
                    </button>
                  </Row>
                );
              }}
            </For>
            <For each={outgoing()}>
              {(f) => (
                <Row friend={f} status="Demande envoyée" dim>
                  <button class="btn btn-ghost h-8 px-3 text-sm" onClick={() => remove(f, "Demande annulée.")}>
                    Annuler
                  </button>
                </Row>
              )}
            </For>
          </ul>
        </Show>

        <div class="flex flex-wrap gap-x-8 gap-y-3 border-t border-line pt-4">
          <label class="flex items-center gap-3 text-sm">
            <Toggle
              checked={friends().openToFriends}
              label="Mes amis peuvent rejoindre sans invitation"
              onChange={(v) => void run(() => api.friendsSetOpen(v))}
            />
            Mes amis peuvent rejoindre sans invitation
          </label>
          <label class="flex items-center gap-3 text-sm">
            <Toggle
              checked={friends().hidden}
              label="Apparaître hors ligne"
              onChange={(v) => void run(() => api.friendsSetHidden(v))}
            />
            Apparaître hors ligne
          </label>
        </div>
      </Show>
    </section>
  );
}
