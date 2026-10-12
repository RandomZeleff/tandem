import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { api } from "../lib/api";
import { advice, type Level, memoryReading, type Reading, startHealthEvents, worldReading } from "../lib/health";

const LEVEL_COLOR: Record<Level, string> = {
  ok: "var(--color-xp)",
  warn: "var(--color-gold)",
  bad: "var(--color-redstone)",
  unknown: "var(--color-line)",
};

/** Whether the host's PC keeps up with the session: world health and free memory. */
export default function HostHealth(props: { instanceId: string }) {
  const since = Date.now();
  const [now, setNow] = createSignal(Date.now());
  const [free, setFree] = createSignal<number | null>(null);

  async function refresh() {
    setNow(Date.now());
    setFree(await api.systemFreeMemory().catch(() => null));
  }

  onMount(() => {
    void startHealthEvents();
    void refresh();
    const timer = setInterval(() => void refresh(), 5000);
    onCleanup(() => clearInterval(timer));
  });

  const world = () => worldReading(props.instanceId, since, now());
  const memory = () => memoryReading(free());
  const tip = () => advice(world(), memory());

  return (
    <section class="flex flex-col gap-2">
      <h3 class="panel-title">Santé de la partie</h3>
      <div class="panel flex flex-col gap-3 p-4">
        <div class="grid grid-cols-2 gap-4">
          <Gauge title="Monde" reading={world()} />
          <Gauge title="Mémoire du PC" reading={memory()} />
        </div>
        <Show when={tip()}>
          <p class="text-sm text-chalk-2" aria-live="polite">
            {tip()}
          </p>
        </Show>
      </div>
    </section>
  );
}

function Gauge(props: { title: string; reading: Reading }) {
  return (
    <div class="flex items-start gap-2.5">
      <span
        class="mt-1.5 size-2.5 shrink-0 rounded-[2px]"
        style={{ background: LEVEL_COLOR[props.reading.level], "box-shadow": "inset 0 -2px 0 rgb(0 0 0 / 0.25)" }}
        aria-hidden="true"
      />
      <div class="flex min-w-0 flex-col">
        <span class="text-[13px] text-muted">{props.title}</span>
        <span class="text-sm font-medium">{props.reading.label}</span>
        <Show when={props.reading.detail}>
          <span class="truncate text-[13px] text-faint">{props.reading.detail}</span>
        </Show>
      </div>
    </div>
  );
}
