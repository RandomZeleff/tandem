import { createSignal, For, type JSX, Match, Show, Switch } from "solid-js";
import {
  accountsWindow,
  cancelSignIn,
  login,
  type LoginStep,
  resetSignIn,
  setAccountsWindow,
  signInWithMicrosoft,
  withLinks,
} from "../../lib/accounts";
import { api, errorMessage, type Account } from "../../lib/api";
import { openExternal } from "../../lib/projects";
import { accounts, activeAccount, refetchAccounts } from "../../lib/store";
import { removeWithUndo, toast } from "../../lib/toast";
import Dialog from "../Dialog";
import { Icon } from "../pixel";
import { AccountAvatar, AccountKindLabel, GrassBlock, MicrosoftButton, MicrosoftLogo, TandemMark } from "./AccountVisuals";

/** Text with its web links clickable (they open in the browser). */
function Linked(props: { text: string }) {
  return (
    <For each={withLinks(props.text)}>
      {(part) =>
        part.url ? (
          <button class="underline hover:text-chalk" onClick={() => openExternal(part.url!)}>
            {part.text}
          </button>
        ) : (
          part.text
        )
      }
    </For>
  );
}

const STEPS: { step: Exclude<LoginStep, "browser">; label: string; icon: () => JSX.Element }[] = [
  { step: "microsoft", label: "Compte Microsoft", icon: () => <MicrosoftLogo size={14} /> },
  { step: "xbox", label: "Xbox Live", icon: () => <Icon name="user" size={14} /> },
  { step: "minecraft", label: "Minecraft: Java Edition", icon: () => <GrassBlock size={14} /> },
  { step: "profile", label: "Profil et skin", icon: () => <Icon name="sparkle" size={14} /> },
];

/** Tandem ⇄ Microsoft, while the player signs in in the browser. */
function BrowserWait(props: { url: string }) {
  const [copied, setCopied] = createSignal(false);
  async function copy() {
    await navigator.clipboard.writeText(props.url).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }
  return (
    <div class="flex flex-col items-center gap-5 py-2 text-center">
      <div class="flex items-center gap-4" aria-hidden="true">
        <span class="slot h-16 w-16">
          <TandemMark size={36} />
        </span>
        <span class="flex gap-1.5">
          <For each={[0, 1, 2]}>{(i) => <span class="link-dot size-2 bg-xp" style={{ "animation-delay": `${i * 200}ms` }} />}</For>
        </span>
        <span class="slot h-16 w-16 bg-white!">
          <MicrosoftLogo size={30} />
        </span>
      </div>
      <div class="flex flex-col gap-1.5">
        <p class="text-lg font-semibold">Continue dans ton navigateur</p>
        <p class="max-w-sm text-sm text-chalk-2">
          Connecte-toi sur la page de Microsoft, puis reviens ici : la suite se fait toute seule. Tandem ne voit jamais ton
          mot de passe.
        </p>
      </div>
      <Show when={props.url}>
        <p class="text-xs text-muted">
          Le navigateur ne s'est pas ouvert ?{" "}
          <button class="underline hover:text-chalk" onClick={() => openExternal(props.url)}>
            Ouvrir la page
          </button>{" "}
          ·{" "}
          <button class="underline hover:text-chalk" onClick={() => void copy()}>
            {copied() ? "Lien copié" : "Copier le lien"}
          </button>
        </p>
      </Show>
    </div>
  );
}

/** The four steps after the browser, ticked as they complete. */
function Steps(props: { current: Exclude<LoginStep, "browser"> }) {
  const index = () => STEPS.findIndex((s) => s.step === props.current);
  return (
    <div class="flex flex-col gap-4 py-2">
      <p class="text-center text-lg font-semibold">Connexion en cours…</p>
      <ol class="panel px-corners-md flex flex-col divide-y divide-line">
        <For each={STEPS}>
          {(s, i) => {
            const state = () => (i() < index() ? "done" : i() === index() ? "current" : "todo");
            return (
              <li class="flex items-center gap-3 px-4 py-2.5" classList={{ "opacity-45": state() === "todo" }}>
                <span class="flex size-5 items-center justify-center">{s.icon()}</span>
                <span class="flex-1 text-sm" classList={{ "font-semibold": state() === "current" }}>
                  {s.label}
                </span>
                <Switch>
                  <Match when={state() === "done"}>
                    <Icon name="check" size={12} color="var(--color-xp)" />
                  </Match>
                  <Match when={state() === "current"}>
                    <span class="step-pulse size-2 bg-xp" aria-label="en cours" />
                  </Match>
                </Switch>
              </li>
            );
          }}
        </For>
      </ol>
    </div>
  );
}

function Welcome(props: { account: Account; onDone: () => void }) {
  return (
    <div class="flex flex-col items-center gap-4 py-2 text-center">
      <span class="slot pop-in h-28 w-28">
        <AccountAvatar account={props.account} size={88} />
      </span>
      <div class="flex flex-col gap-1">
        <p class="pixel-shadow font-pixel text-2xl font-bold">Bienvenue, {props.account.username} !</p>
        <p class="text-sm text-chalk-2">Ton compte Microsoft est connecté et possède Minecraft. Tu peux jouer.</p>
      </div>
      <button class="btn btn-primary px-corners px-6" onClick={props.onDone}>
        C'est parti
      </button>
    </div>
  );
}

function AccountRow(props: { account: Account; onRemove: () => void }) {
  const [busy, setBusy] = createSignal(false);
  async function use() {
    setBusy(true);
    try {
      await api.setActiveAccount(props.account.id);
      await refetchAccounts();
    } catch (err) {
      toast(errorMessage(err), { tone: "error" });
    } finally {
      setBusy(false);
    }
  }
  return (
    <li class="flex items-center gap-3 px-3 py-2.5">
      <span class="slot h-12 w-12">
        <AccountAvatar account={props.account} size={36} />
      </span>
      <div class="flex min-w-0 flex-1 flex-col gap-0.5">
        <span class="flex items-center gap-2">
          <span class="truncate font-semibold">{props.account.username}</span>
          <Show when={props.account.isActive}>
            <span class="chip h-5 shrink-0 px-1.5 text-[11px] text-xp-text">utilisé</span>
          </Show>
        </span>
        <span class="text-xs text-muted">
          <AccountKindLabel account={props.account} />
        </span>
      </div>
      <Show when={!props.account.isActive}>
        <button class="btn px-corners h-8 px-3 text-xs" disabled={busy()} onClick={() => void use()}>
          Utiliser
        </button>
      </Show>
      <button
        class="btn btn-ghost h-8 w-8 px-0 hover:text-redstone-text"
        aria-label={`Retirer ${props.account.username}`}
        title="Retirer ce compte"
        onClick={props.onRemove}
      >
        <Icon name="trash" size={12} />
      </button>
    </li>
  );
}

function OfflineForm() {
  const [name, setName] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);
  const allowed = () => accounts().some((a) => a.kind === "microsoft");
  async function add() {
    setError(null);
    try {
      await api.addOfflineAccount(name().trim());
      setName("");
      await refetchAccounts();
    } catch (err) {
      setError(errorMessage(err));
    }
  }
  return (
    <section class="flex flex-col gap-2">
      <h3 class="panel-title">Jouer hors ligne</h3>
      <p class="text-xs text-muted">
        <Show
          when={allowed()}
          fallback="Disponible une fois un compte Microsoft connecté : Tandem vérifie d'abord que tu possèdes le jeu."
        >
          Un pseudo local pour jouer sans connexion ou en LAN. Les serveurs en ligne ne l'acceptent pas.
        </Show>
      </p>
      <form
        class="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void add();
        }}
      >
        <input
          class="field h-9 min-w-0 flex-1 text-sm"
          placeholder="Pseudo (3 à 16 caractères)"
          aria-label="Pseudo hors ligne"
          maxLength={16}
          disabled={!allowed()}
          value={name()}
          onInput={(e) => setName(e.currentTarget.value)}
        />
        <button type="submit" class="btn px-corners h-9 px-3 text-sm" disabled={!allowed() || name().trim().length < 3}>
          Ajouter
        </button>
      </form>
      <Show when={error()}>
        <p class="text-xs text-redstone-text">{error()}</p>
      </Show>
    </section>
  );
}

/** Accounts window: the list, Microsoft sign-in step by step, offline names. Mounted once in App. */
export default function AccountsDialog() {
  const [removing, setRemoving] = createSignal<ReadonlySet<string>>(new Set());
  const listed = () => accounts().filter((a) => !removing().has(a.id));
  const reason = () => accountsWindow()?.reason;

  function close() {
    if (login().kind === "browser" || login().kind === "working") cancelSignIn();
    else resetSignIn();
    setAccountsWindow(null);
  }

  function remove(account: Account) {
    const mark = (on: boolean) =>
      setRemoving((set) => {
        const next = new Set(set);
        if (on) next.add(account.id);
        else next.delete(account.id);
        return next;
      });
    removeWithUndo({
      message: `${account.username} retiré`,
      hide: () => mark(true),
      show: () => mark(false),
      commit: async () => {
        try {
          await api.removeAccount(account.id);
          await refetchAccounts();
          mark(false);
          return null;
        } catch (err) {
          return errorMessage(err);
        }
      },
    });
  }

  return (
    <Show when={accountsWindow()}>
      <Dialog title={login().kind === "idle" ? "Comptes" : "Connexion Microsoft"} onClose={close} width={520}>
        <Switch>
          <Match when={login().kind === "browser" && login()}>
            {(state) => (
              <>
                <BrowserWait url={(state() as { url: string }).url} />
                <div class="flex justify-center">
                  <button class="btn btn-ghost" onClick={cancelSignIn}>
                    Annuler
                  </button>
                </div>
              </>
            )}
          </Match>
          <Match when={login().kind === "working" && login()}>
            {(state) => <Steps current={(state() as { step: Exclude<LoginStep, "browser"> }).step} />}
          </Match>
          <Match when={login().kind === "done" && login()}>
            {(state) => (
              <Welcome
                account={(state() as { account: Account }).account}
                onDone={() => {
                  resetSignIn();
                  setAccountsWindow(null);
                }}
              />
            )}
          </Match>
          <Match when={login().kind === "error" && login()}>
            {(state) => (
              <div class="flex flex-col gap-4">
                <div class="flex gap-3 bg-danger p-4 text-sm text-redstone-text shadow-[inset_0_0_0_1px_var(--color-danger-line)]">
                  <Icon name="close" size={14} class="mt-0.5 shrink-0" />
                  <p>
                    <Linked text={(state() as { message: string }).message} />
                  </p>
                </div>
                <div class="flex justify-end gap-2">
                  <button class="btn btn-ghost" onClick={resetSignIn}>
                    Retour
                  </button>
                  <MicrosoftButton label="Réessayer" onClick={() => void signInWithMicrosoft()} />
                </div>
              </div>
            )}
          </Match>
          <Match when={login().kind === "idle"}>
            <Show when={reason()}>
              <div class="-mt-1 flex gap-3 bg-warning p-3.5 text-sm text-gold shadow-[inset_0_0_0_1px_var(--color-gold-deep)]">
                <Icon name="user" size={14} class="mt-0.5 shrink-0" />
                <p>
                  {reason() === "expired"
                    ? `La session de ${activeAccount()?.username ?? "ce compte"} a expiré : reconnecte-toi pour jouer.`
                    : "Pour jouer, connecte le compte Microsoft avec lequel tu as acheté Minecraft: Java Edition."}
                </p>
              </div>
            </Show>

            <Show
              when={listed().length > 0}
              fallback={
                <div class="flex flex-col items-center gap-4 py-3 text-center">
                  <div class="flex items-center gap-3" aria-hidden="true">
                    <span class="slot h-14 w-14">
                      <GrassBlock size={30} />
                    </span>
                    <span class="slot h-14 w-14 bg-white!">
                      <MicrosoftLogo size={26} />
                    </span>
                  </div>
                  <div class="flex flex-col gap-1.5">
                    <p class="text-lg font-semibold">Connecte ton compte Minecraft</p>
                    <p class="max-w-sm text-sm text-chalk-2">
                      La connexion se fait sur le site de Microsoft, dans ton navigateur. Tandem garde seulement de quoi te
                      reconnecter, dans le coffre de ton système.
                    </p>
                  </div>
                  <MicrosoftButton class="h-11 px-5" onClick={() => void signInWithMicrosoft()} />
                </div>
              }
            >
              <ul class="panel px-corners-md flex flex-col divide-y divide-line">
                <For each={listed()}>{(account) => <AccountRow account={account} onRemove={() => remove(account)} />}</For>
              </ul>
              <MicrosoftButton
                label={reason() === "expired" ? "Se reconnecter avec Microsoft" : "Ajouter un compte Microsoft"}
                onClick={() => void signInWithMicrosoft()}
              />
            </Show>

            <div class="h-px bg-line" />
            <OfflineForm />
          </Match>
        </Switch>
      </Dialog>
    </Show>
  );
}
