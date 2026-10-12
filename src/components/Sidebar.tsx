import { createEffect, createResource, createSignal, For, type JSX, Show } from "solid-js";
import { api, errorMessage } from "../lib/api";
import { gameState } from "../lib/games";
import { duoActive, markArrivalsSeen, unseenArrivals } from "../lib/duo";
import { toast } from "../lib/toast";
import { openAccounts, signInWithMicrosoft } from "../lib/accounts";
import { AccountAvatar, AccountKindLabel, MicrosoftLogo } from "./accounts/AccountVisuals";
import { onDismiss } from "../lib/ui";
import InstanceSlot from "./InstanceSlot";
import { accounts, activeAccount, instances, navigate, refetchAccounts, route, type Route } from "../lib/store";
import { Icon, type IconName, LoaderTag } from "./pixel";

interface NavItem {
  label: string;
  icon: IconName;
  to: Route;
  /** Shown in the tooltip. */
  shortcut?: string;
}

const MOD = navigator.userAgent.includes("Mac") ? "⌘" : "Ctrl+";

const NAV: NavItem[] = [
  { label: "Accueil", icon: "home", to: { page: "home" } },
  { label: "Instances", icon: "grid", to: { page: "instances" } },
  { label: "Découvrir", icon: "search", to: { page: "discover" }, shortcut: "K" },
  { label: "Jouer ensemble", icon: "duo", to: { page: "multi" } },
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
    <div ref={menu} class="panel px-corners-md absolute top-full right-0 left-0 z-20 mt-1 bg-slate-750 p-1.5 shadow-[0_12px_32px_rgb(0_0_0/0.55)]">
      {props.children}
    </div>
  );
}

function AccountCard() {
  const [open, setOpen] = createSignal(false);
  let anchor: HTMLButtonElement | undefined;
  const others = () => accounts().filter((a) => !a.isActive);

  async function use(id: string) {
    setOpen(false);
    try {
      await api.setActiveAccount(id);
      await refetchAccounts();
    } catch (err) {
      toast(errorMessage(err), { tone: "error" });
    }
  }

  function manage() {
    setOpen(false);
    openAccounts();
  }

  return (
    <div class="relative mx-3 mt-3.5 mb-1.5">
      <Show
        when={activeAccount()}
        fallback={
          <button
            class="px-corners-md flex w-full items-center gap-3 bg-slate-800 p-2 text-left shadow-[inset_0_0_0_1px_rgb(255_255_255/0.05)] transition-colors duration-100 hover:bg-slate-750 focus-visible:bg-slate-750"
            onClick={() => openAccounts()}
          >
            <span class="slot size-10 bg-white!">
              <MicrosoftLogo size={20} />
            </span>
            <span class="flex min-w-0 flex-1 flex-col gap-0.5">
              <span class="text-sm font-semibold">Se connecter</span>
              <span class="text-xs text-muted">Compte Microsoft</span>
            </span>
          </button>
        }
      >
        {(account) => (
          <button
            ref={anchor}
            class="px-corners-md flex w-full items-center gap-3 bg-slate-800 p-2 text-left shadow-[inset_0_0_0_1px_rgb(255_255_255/0.05)] transition-colors duration-100 hover:bg-slate-750 focus-visible:bg-slate-750"
            onClick={() => setOpen((v) => !v)}
            aria-expanded={open()}
            aria-label={`Compte : ${account().username}`}
          >
            <span class="slot size-10">
              <AccountAvatar account={account()} size={32} />
            </span>
            <span class="flex min-w-0 flex-1 flex-col gap-0.5">
              <span class="truncate text-sm font-semibold">{account().username}</span>
              <span class="text-xs text-muted">
                <AccountKindLabel account={account()} />
              </span>
            </span>
            <Icon name="caret" size={12} class={`mr-1 text-faint transition-transform duration-100 ${open() ? "rotate-180" : ""}`} />
          </button>
        )}
      </Show>

      <Show when={open()}>
        <AccountMenu anchor={() => anchor} onClose={() => setOpen(false)}>
          <Show when={others().length > 0}>
            <p class="px-2 pt-1 pb-1.5 text-xs font-semibold text-faint">Changer de compte</p>
            <ul class="space-y-0.5">
              <For each={others()}>
                {(account) => (
                  <li>
                    <button
                      class="row flex w-full items-center gap-2.5 px-2 py-1.5 text-left text-sm"
                      onClick={() => void use(account.id)}
                    >
                      <AccountAvatar account={account} size={22} />
                      <span class="flex min-w-0 flex-1 flex-col">
                        <span class="truncate">{account.username}</span>
                        <span class="text-xs text-muted">
                          <AccountKindLabel account={account} />
                        </span>
                      </span>
                    </button>
                  </li>
                )}
              </For>
            </ul>
            <div class="mx-1 my-1.5 h-px bg-line" />
          </Show>
          <button
            class="row flex w-full items-center gap-2.5 px-2 py-1.5 text-left text-sm"
            onClick={manage}
          >
            <Icon name="gear" size={14} />
            Gérer les comptes
          </button>
          <button
            class="row flex w-full items-center gap-2.5 px-2 py-1.5 text-left text-sm"
            onClick={() => {
              setOpen(false);
              openAccounts();
              void signInWithMicrosoft();
            }}
          >
            <MicrosoftLogo size={14} />
            Ajouter un compte Microsoft
          </button>
        </AccountMenu>
      </Show>
    </div>
  );
}

export default function Sidebar() {
  const [info] = createResource(api.appInfo);
  const recents = () => instances().slice(0, 4);
  // Opening "Jouer ensemble" is what makes a friend's arrival seen.
  createEffect(() => route().page === "multi" && markArrivalsSeen());

  return (
    <nav
      aria-label="Navigation principale"
      class="flex w-60 shrink-0 flex-col bg-slate-900 shadow-[inset_-1px_0_0_var(--color-divider)]"
    >
      <AccountCard />

      <div class="flex flex-col gap-0.5 px-3 py-2">
        <For each={NAV}>
          {(item) => (
            <button
              class="flex h-11 items-center gap-3 rounded-sm px-2.5 text-left transition-colors duration-100"
              classList={{
                "bg-white/[0.08] font-semibold text-chalk": isActive(item),
                "font-medium text-muted hover:bg-white/[0.04] hover:text-chalk focus-visible:bg-white/[0.04] focus-visible:text-chalk":
                  !isActive(item),
              }}
              aria-current={isActive(item) ? "page" : undefined}
              title={item.shortcut ? `${item.label} (${MOD}${item.shortcut})` : undefined}
              onClick={() => navigate(item.to)}
            >
              <Icon name={item.icon} size={24} color={isActive(item) ? "var(--color-grass-light)" : undefined} />
              <span class="flex-1">{item.label}</span>
              <Show when={item.to.page === "multi"}>
                <Show
                  when={unseenArrivals() > 0}
                  fallback={
                    <Show when={duoActive()}>
                      <span class="step-pulse mr-1 size-2 rounded-[1px] bg-xp" title="Partie à plusieurs en cours" />
                    </Show>
                  }
                >
                  <span
                    class="flex h-5 min-w-5 items-center justify-center rounded-sm bg-grass px-1.5 text-xs font-bold text-white shadow-[inset_0_-2px_0_var(--color-grass-edge)]"
                    title={unseenArrivals() > 1 ? `${unseenArrivals()} amis sont arrivés` : "Un ami est arrivé"}
                  >
                    {unseenArrivals()}
                  </span>
                </Show>
              </Show>
            </button>
          )}
        </For>
      </div>

      <div class="flex min-h-0 flex-col gap-0.5 px-3 pt-4">
        <span class="px-2.5 pb-1.5 text-xs font-semibold text-faint">Récentes</span>
        <Show when={recents().length > 0} fallback={<p class="px-2.5 text-meta text-faint">Aucune instance pour l'instant.</p>}>
          <For each={recents()}>
            {(instance) => {
              const state = () => gameState(instance.id);
              const ratio = () => {
                const p = state().progress;
                return p && p.totalBytes > 0 ? p.doneBytes / p.totalBytes : 0;
              };
              return (
                <button
                  class="row flex h-12 items-center gap-2.5 px-2 text-left"
                  classList={{
                    "is-selected": route().page === "instance" && (route() as { id: string }).id === instance.id,
                  }}
                  onClick={() => navigate({ page: "instance", id: instance.id })}
                >
                  <InstanceSlot instance={instance} size={32} />
                  <span class="flex min-w-0 flex-1 flex-col">
                    <span class="truncate text-meta font-semibold">{instance.name}</span>
                    <span class="text-xs text-faint">
                      {instance.gameVersion} · <LoaderTag loader={instance.loader} size={12} />
                    </span>
                  </span>
                  <Show when={state().status === "preparing"}>
                    <span class="font-mono text-xs text-xp-text">{Math.round(ratio() * 100)} %</span>
                  </Show>
                  <Show when={state().status === "running"}>
                    <span class="mr-1 size-2 rounded-[1px] bg-xp shadow-[0_0_0_2px_rgb(127_209_79/0.25)]" title="En cours" />
                  </Show>
                </button>
              );
            }}
          </For>
        </Show>
      </div>

      <div class="flex-1" />

      <div class="p-3 shadow-[inset_0_1px_0_var(--color-divider)]">
        <button
          class="flex h-10 w-full items-center gap-3 rounded-sm px-2.5 text-left transition-colors duration-100"
          classList={{
            "bg-white/[0.08] font-semibold text-chalk": route().page === "settings",
            "font-medium text-muted hover:bg-white/[0.04] hover:text-chalk focus-visible:bg-white/[0.04] focus-visible:text-chalk":
              route().page !== "settings",
          }}
          aria-current={route().page === "settings" ? "page" : undefined}
          title={`Réglages (${MOD},)`}
          onClick={() => navigate({ page: "settings" })}
        >
          <Icon name="gear" size={24} color={route().page === "settings" ? "var(--color-grass-light)" : undefined} />
          <span class="flex-1">Réglages</span>
          <span class="font-mono text-xs font-normal text-faint">v{info()?.version ?? "…"}</span>
        </button>
      </div>
    </nav>
  );
}
