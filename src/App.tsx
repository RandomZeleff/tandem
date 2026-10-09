import { Match, onMount, Show, Switch } from "solid-js";
import NewInstanceDialog from "./components/NewInstanceDialog";
import RosettaDialog from "./components/RosettaDialog";
import Sidebar from "./components/Sidebar";
import TitleBar from "./components/TitleBar";
import { onGamePlayed, rosettaPrompt, setRosettaPrompt, startGameEvents } from "./lib/games";
import { startLogStream } from "./lib/logs";
import {
  instances,
  navigate,
  newInstanceDialog,
  refetchInstances,
  route,
  setNewInstanceDialog,
} from "./lib/store";
import Discover from "./pages/Discover";
import Home from "./pages/Home";
import InstanceDetail from "./pages/InstanceDetail";
import Instances from "./pages/Instances";
import Settings from "./pages/Settings";
import Soon from "./pages/Soon";

function App() {
  onMount(() => {
    void startLogStream();
    void startGameEvents();
    onGamePlayed(() => void refetchInstances());
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
          class="dither min-w-0 flex-1 bg-slate-800"
          classList={{
            "overflow-hidden": route().page === "instance",
            "overflow-y-auto p-6": route().page !== "instance",
          }}
        >
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

      <Show when={rosettaPrompt()} keyed>
        {(id) => <RosettaDialog instanceId={id} onClose={() => setRosettaPrompt(null)} />}
      </Show>
    </div>
  );
}

export default App;
