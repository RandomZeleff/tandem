import { Match, onCleanup, onMount, Show, Switch } from "solid-js";
import NewInstanceDialog from "./components/NewInstanceDialog";
import RosettaDialog from "./components/RosettaDialog";
import Sidebar from "./components/Sidebar";
import TitleBar from "./components/TitleBar";
import Toaster from "./components/Toaster";
import { onGamePlayed, rosettaPrompt, setRosettaPrompt, startGameEvents } from "./lib/games";
import { startLogStream } from "./lib/logs";
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
import Soon from "./pages/Soon";

const isMac = navigator.userAgent.includes("Mac");

/** Ctrl/Cmd + key. */
const SHORTCUTS: Record<string, () => void> = {
  k: searchModrinth,
  f: searchModrinth,
  n: () => setNewInstanceDialog({}),
  ",": () => navigate({ page: "settings" }),
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
    onGamePlayed(() => void refetchInstances());

    // Back / Forward: mouse side buttons, Alt+arrows, and Cmd+[ / Cmd+] on macOS.
    const onMouse = (e: MouseEvent) => {
      if (e.button !== 3 && e.button !== 4) return;
      e.preventDefault();
      if (e.type === "mouseup") (e.button === 3 ? goBack : goForward)();
    };
    const onKey = (e: KeyboardEvent) => {
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
          class="dither min-w-0 flex-1 bg-slate-800"
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
              <Show when={detail()} keyed fallback={<p class="p-6 text-faint">Instance introuvable.</p>}>
                {(instance) => <InstanceDetail instance={instance} />}
              </Show>
            </Match>
            <Match when={route().page === "discover"}>
              <Discover instanceId={(route() as { instanceId?: string }).instanceId} />
            </Match>
            <Match when={route().page === "multi"}>
              <Soon feature="multi" />
            </Match>
            <Match when={route().page === "settings"}>
              <Settings />
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

      <Toaster />

      <Show when={rosettaPrompt()} keyed>
        {(id) => <RosettaDialog instanceId={id} onClose={() => setRosettaPrompt(null)} />}
      </Show>
    </div>
  );
}

export default App;
