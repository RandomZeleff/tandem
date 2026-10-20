import { lazy, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import AccountsDialog from "./components/accounts/AccountsDialog";
import { forgetInstancePictures } from "./components/InstanceVisual";
import NewInstanceDialog from "./components/NewInstanceDialog";
import RosettaDialog from "./components/RosettaDialog";
import Sidebar from "./components/Sidebar";
import TitleBar from "./components/TitleBar";
import Toaster from "./components/Toaster";
import UpdateDialog from "./components/UpdateDialog";
import Onboarding from "./components/Onboarding";
import { checkFirstRun, firstRun } from "./lib/onboarding";
import { onGamePlayed, rosettaPrompt, setRosettaPrompt, startGameEvents } from "./lib/games";
import type { ProjectType } from "./lib/api";
import { startLogStream } from "./lib/logs";
import { startTranslationEvents } from "./lib/translation";
import { startDuoEvents } from "./lib/duo";
import { toggleSidebar } from "./lib/ui";
import { checkForUpdateLater, setUpdateDialog, updateDialog } from "./lib/updates";
import {
  currentEntry,
  goBack,
  goForward,
  instances,
  navigate,
  newInstanceDialog,
  refetchInstances,
  route,
  setNewInstanceDialog,
  setPageScroller,
} from "./lib/store";
import Discover from "./pages/Discover";
import Home from "./pages/Home";
import InstanceDetail from "./pages/InstanceDetail";
import Instances from "./pages/Instances";
import Settings from "./pages/Settings";
import Duo from "./pages/Duo";

/** Loaded on first visit: keeps it out of the startup bundle. */
const Project = lazy(() => import("./pages/Project"));
/** Carries the 3D viewer (three.js): only loaded when opened. */
const Appearance = lazy(() => import("./pages/Appearance"));

const isMac = navigator.userAgent.includes("Mac");

/** Ctrl/Cmd + key. */
const SHORTCUTS: Record<string, () => void> = {
  k: searchModrinth,
  f: searchModrinth,
  n: () => setNewInstanceDialog({}),
  ",": () => navigate({ page: "settings" }),
  b: toggleSidebar,
};

function searchModrinth() {
  navigate({ page: "discover" });
  const input = document.getElementById("discover-search") as HTMLInputElement | null;
  input?.focus();
  input?.select();
}

function App() {
  onMount(() => {
    void startLogStream();
    void startGameEvents();
    void startTranslationEvents();
    void startDuoEvents();
    checkForUpdateLater();
    void checkFirstRun().catch(() => {});
    onGamePlayed(() => {
      // A new screenshot may now be the instance's picture.
      forgetInstancePictures();
      void refetchInstances();
    });

    // Back / Forward: mouse side buttons, Alt+arrows, and Cmd+[ / Cmd+] on macOS.
    const onMouse = (e: MouseEvent) => {
      if ((e.button !== 3 && e.button !== 4) || firstRun()) return;
      e.preventDefault();
      if (e.type === "mouseup") (e.button === 3 ? goBack : goForward)();
    };
    const onKey = (e: KeyboardEvent) => {
      // The welcome steps cover the app: nothing to navigate to behind them.
      if (firstRun()) return;
      // Ctrl on Windows, Cmd on macOS.
      const mod = isMac ? e.metaKey : e.ctrlKey;
      if (mod && !e.altKey && !e.shiftKey) {
        const shortcut = SHORTCUTS[e.key.toLowerCase()];
        if (shortcut) {
          e.preventDefault();
          shortcut();
          return;
        }
      }
      const back = (e.altKey && e.key === "ArrowLeft") || (e.metaKey && e.key === "[");
      const forward = (e.altKey && e.key === "ArrowRight") || (e.metaKey && e.key === "]");
      if (!back && !forward) return;
      e.preventDefault();
      (back ? goBack : goForward)();
    };
    window.addEventListener("mousedown", onMouse);
    window.addEventListener("mouseup", onMouse);
    window.addEventListener("keydown", onKey);
    onCleanup(() => {
      window.removeEventListener("mousedown", onMouse);
      window.removeEventListener("mouseup", onMouse);
      window.removeEventListener("keydown", onKey);
    });
  });

  const detail = () => {
    const r = route();
    return r.page === "instance" ? instances().find((i) => i.id === r.id) : undefined;
  };

  return (
    <div class="flex h-full flex-col">
      <TitleBar />
      <div class="flex min-h-0 flex-1">
        <Sidebar />
        <main
          ref={setPageScroller}
          class="min-w-0 flex-1 bg-slate-800"
          classList={{
            "overflow-hidden": route().page === "instance",
            "overflow-y-auto p-6": route().page !== "instance",
          }}
        >
          {/* Keyed on the history entry: Back/Forward recreate the page with its remembered state. */}
          <Show when={currentEntry()} keyed>
            {(_entry) => (
          <Switch>
            <Match when={route().page === "home"}>
              <Home />
            </Match>
            <Match when={route().page === "instances"}>
              <Instances />
            </Match>
            <Match when={route().page === "instance"}>
              <Show
                when={detail()}
                keyed
                fallback={
                  <div class="flex h-full flex-col items-center justify-center gap-3 text-center">
                    <p class="text-lg font-semibold">Cette instance n'existe plus</p>
                    <p class="text-sm text-muted">Elle a peut-être été supprimée ou déplacée.</p>
                    <button class="btn px-corners mt-1" onClick={() => navigate({ page: "instances" })}>
                      Voir mes instances
                    </button>
                  </div>
                }
              >
                {(instance) => {
                  const r = route() as { tab?: string; world?: string };
                  return <InstanceDetail instance={instance} tab={r.tab} world={r.world} />;
                }}
              </Show>
            </Match>
            <Match when={route().page === "discover"}>
              <Discover {...(route() as { instanceId?: string; query?: string; kind?: ProjectType; world?: string })} />
            </Match>
            <Match when={route().page === "project"}>
              <Project {...(route() as { id: string; instanceId?: string })} />
            </Match>
            <Match when={route().page === "multi"}>
              <Duo />
            </Match>
            <Match when={route().page === "settings"}>
              <Settings />
            </Match>
            <Match when={route().page === "appearance"}>
              <Appearance />
            </Match>
          </Switch>
            )}
          </Show>
        </main>
      </div>

      <Show when={newInstanceDialog()}>
        {(options) => (
          <NewInstanceDialog
            initialVersion={options().version}
            onClose={() => setNewInstanceDialog(null)}
            onCreated={(instance) => {
              setNewInstanceDialog(null);
              void refetchInstances();
              navigate({ page: "instance", id: instance.id });
            }}
          />
        )}
      </Show>

      <Show when={firstRun()}>
        <Onboarding />
      </Show>

      <Toaster />

      <Show when={updateDialog()}>
        <UpdateDialog onClose={() => setUpdateDialog(false)} />
      </Show>

      <AccountsDialog />

      <Show when={rosettaPrompt()} keyed>
        {(id) => <RosettaDialog instanceId={id} onClose={() => setRosettaPrompt(null)} />}
      </Show>
    </div>
  );
}

export default App;
