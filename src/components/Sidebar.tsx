import { createResource, createSignal, For, type JSX, Show } from "solid-js";
import { api, errorMessage } from "../lib/api";
import { gameState } from "../lib/games";
import { skinLook } from "../lib/look";
import { removeWithUndo } from "../lib/toast";
import { onDismiss } from "../lib/ui";
import InstanceSlot from "./InstanceSlot";
import { accounts, activeAccount, instances, navigate, refetchAccounts, route, type Route } from "../lib/store";
import { Icon, type IconName, LoaderTag, SkinHead } from "./pixel";

interface NavItem {
  label: string;
  icon: IconName;
  to: Route;
  soon?: boolean;
  /** Shown in the tooltip. */
  shortcut?: string;
}

const MOD = navigator.userAgent.includes("Mac") ? "⌘" : "Ctrl+";

const NAV: NavItem[] = [
  { label: "Accueil", icon: "home", to: { page: "home" } },
  { label: "Instances", icon: "grid", to: { page: "instances" } },
  { label: "Découvrir", icon: "search", to: { page: "discover" }, shortcut: "K" },
  { label: "Jouer à deux", icon: "duo", to: { page: "multi" }, soon: true },
];

function isActive(item: NavItem): boolean {
  const current = route().page;
  return (
    current === item.to.page ||
    (item.to.page === "instances" && current === "instance") ||
    (item.to.page === "discover" && current === "project")
  );
}

function AccountMenu(props: { anchor: () => HTMLElement | undefined; onClose: () => void; children: JSX.Element }) {
  let menu: HTMLDivElement | undefined;
  onDismiss(() => [menu, props.anchor()], props.onClose);
  return (
    <div ref={menu} class="panel px-corners-md absolute top-full right-0 left-0 z-20 mt-1 bg-slate-750 p-3 shadow-2xl">
      {props.children}
    </div>
  );
}

function AccountCard() {
  const [open, setOpen] = createSignal(false);
  let anchor: HTMLButtonElement | undefined;
  const [username, setUsername] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);

  /** Accounts removed but still undoable. */
  const [removing, setRemoving] = createSignal<ReadonlySet<string>>(new Set());
  const listed = () => accounts().filter((a) => !removing().has(a.id));

  function remove(id: string, username: string) {
    const toggle = (on: boolean) =>
      setRemoving((set) => {
        const next = new Set(set);
        if (on) next.add(id);
        else next.delete(id);
        return next;
      });
    removeWithUndo({
      message: `Compte ${username} retiré`,
      hide: () => toggle(true),
      show: () => toggle(false),
      commit: async () => {
        try {
          await api.removeAccount(id);
          await refetchAccounts();
          return null;
        } catch (err) {
          return errorMessage(err);
        } finally {
          toggle(false);
        }
      },
    });
  }

  async function run(action: () => Promise<unknown>) {
    setError(null);
    try {
      await action();
      await refetchAccounts();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <div class="relative mx-3 mt-3.5 mb-1.5">
      <button
        ref={anchor}
        class="panel px-corners-md flex w-full items-center gap-3 p-2.5 text-left hover:bg-slate-600 focus-visible:bg-slate-600"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open()}
      >
        <span class="slot h-11 w-11">
          <Show when={activeAccount()} fallback={<Icon name="user" size={22} color="#4A4A4A" />}>
            {(account) => <SkinHead look={skinLook(account().username)} size={32} />}
          </Show>
        </span>
        <span class="flex min-w-0 flex-1 flex-col gap-0.5">
          <span class="truncate text-[15px] font-semibold">
            {activeAccount()?.username ?? "Aucun compte"}
          </span>
          <span class="flex items-center gap-1.5 text-xs text-muted">
            <span class="size-1.5" classList={{ "bg-xp": !!activeAccount(), "bg-faint": !activeAccount() }} />
            {activeAccount()
              ? activeAccount()!.kind === "offline"
                ? "Hors ligne"
                : "Compte Microsoft"
              : "Ajoute un compte"}
          </span>
        </span>
        <Icon name="caret" size={12} color="#7D828A" />
      </button>

      <Show when={open()}>
        <AccountMenu anchor={() => anchor} onClose={() => setOpen(false)}>
          <Show when={listed().length > 0}>
            <ul class="mb-3 space-y-0.5">
              <For each={listed()}>
                {(account) => (
                  <li class="group flex items-center gap-2 px-1.5 py-1 hover:bg-slate-600 focus-within:bg-slate-600">
                    <button
                      class="flex flex-1 items-center gap-2.5 text-left text-sm"
                      onClick={() => run(() => api.setActiveAccount(account.id))}
                    >
                      <SkinHead look={skinLook(account.username)} size={20} />
                      <span class="flex-1 truncate" classList={{ "font-semibold": account.isActive }}>
                        {account.username}
                      </span>
                      <Show when={account.isActive}>
                        <Icon name="check" size={12} color="var(--color-xp)" />
                      </Show>
                    </button>
                    <button
                      class="text-faint opacity-0 group-focus-within:opacity-100 group-hover:opacity-100 hover:text-redstone-text focus-visible:text-redstone-text"
                      aria-label={`Retirer ${account.username}`}
                      onClick={() => remove(account.id, account.username)}
                    >
                      <Icon name="close" size={10} />
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </Show>
          <form
            class="space-y-2"
            onSubmit={(e) => {
              e.preventDefault();
              void run(async () => {
                await api.addOfflineAccount(username());
                setUsername("");
              });
            }}
          >
            <label class="block text-xs text-muted" for="offline-username">
              Ajouter un compte hors ligne
            </label>
            <div class="flex gap-2">
              <input
                id="offline-username"
                class="field h-9 min-w-0 flex-1 text-sm"
                placeholder="Pseudo"
                maxLength={16}
                value={username()}
                onInput={(e) => setUsername(e.currentTarget.value)}
              />
              <button type="submit" class="btn h-9 px-3 text-sm" disabled={username().trim().length < 3}>
                Ajouter
              </button>
            </div>
            <p class="text-xs text-faint">La connexion Microsoft arrive bientôt.</p>
          </form>
          <Show when={error()}>
            <p class="mt-2 text-xs text-redstone-text">{error()}</p>
          </Show>
        </AccountMenu>
      </Show>
    </div>
  );
}

export default function Sidebar() {
  const [info] = createResource(api.appInfo);
  const recents = () => instances().slice(0, 4);

  return (
    <nav
      aria-label="Navigation principale"
      class="flex w-60 shrink-0 flex-col bg-slate-850 shadow-[inset_-1px_0_0_var(--color-divider)]"
    >
      <AccountCard />

      <div class="flex flex-col gap-0.5 px-3 py-2.5">
        <For each={NAV}>
          {(item) => (
            <button
              class="flex h-[42px] items-center gap-3 px-3 text-left font-medium"
              classList={{
                "px-corners bg-slate-600 font-semibold text-chalk shadow-[inset_0_1px_0_rgb(255_255_255/0.05),inset_0_-2px_0_rgb(0_0_0/0.3)]":
                  isActive(item),
                "text-muted hover:bg-slate-750 hover:text-chalk focus-visible:bg-slate-750 focus-visible:text-chalk": !isActive(item),
              }}
              aria-current={isActive(item) ? "page" : undefined}
              title={item.shortcut ? `${item.label} (${MOD}${item.shortcut})` : undefined}
              onClick={() => navigate(item.to)}
            >
              <Icon name={item.icon} size={18} color={isActive(item) ? "var(--color-xp)" : undefined} />
              <span class="flex-1">{item.label}</span>
              <Show when={item.soon}>
                <span class="bg-slate-700 px-1.5 py-px font-pixel text-[11px] tracking-wide text-faint">BIENTÔT</span>
              </Show>
            </button>
          )}
        </For>
      </div>

      <div class="mx-6 my-1 h-px bg-[var(--color-divider)]" />

      <div class="flex min-h-0 flex-col gap-0.5 px-3 pt-3">
        <span class="px-3 pb-2 font-pixel text-xs font-medium tracking-[2px] text-faint">RÉCENTES</span>
        <Show when={recents().length > 0} fallback={<p class="px-3 text-xs text-faint">Aucune instance.</p>}>
          <For each={recents()}>
            {(instance) => {
              const state = () => gameState(instance.id);
              const ratio = () => {
                const p = state().progress;
                return p && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : 0;
              };
              return (
                <button
                  class="flex h-[46px] items-center gap-2.5 px-2.5 text-left hover:bg-slate-750 focus-visible:bg-slate-750"
                  classList={{
                    "bg-slate-750":
                      route().page === "instance" && (route() as { id: string }).id === instance.id,
                  }}
                  onClick={() => navigate({ page: "instance", id: instance.id })}
                >
                  <InstanceSlot instance={instance} size={30} />
                  <span class="flex min-w-0 flex-1 flex-col">
                    <span class="truncate text-[13px] font-medium">{instance.name}</span>
                    <span class="text-[11px] text-faint">
                      {instance.gameVersion} · <LoaderTag loader={instance.loader} size={10} />
                    </span>
                  </span>
                  <Show when={state().status === "preparing"}>
                    <span class="font-mono text-[11px] text-xp-text">{Math.round(ratio() * 100)} %</span>
                  </Show>
                  <Show when={state().status === "running"}>
                    <span class="size-2 bg-xp shadow-[0_0_0_2px_rgb(139_224_78/0.25)]" title="En cours" />
                  </Show>
                </button>
              );
            }}
          </For>
        </Show>
      </div>

      <div class="flex-1" />

      <button
        class="flex h-[46px] items-center gap-3 px-6 text-left shadow-[inset_0_1px_0_var(--color-divider)]"
        classList={{
          "text-chalk": route().page === "settings",
          "text-muted hover:text-chalk focus-visible:text-chalk": route().page !== "settings",
        }}
        title={`Réglages (${MOD},)`}
        onClick={() => navigate({ page: "settings" })}
      >
        <Icon name="gear" size={18} color={route().page === "settings" ? "var(--color-xp)" : undefined} />
        <span class="flex-1 font-medium">Réglages</span>
        <span class="font-mono text-[11px] text-faint">v{info()?.version ?? "…"}</span>
      </button>
    </nav>
  );
}
