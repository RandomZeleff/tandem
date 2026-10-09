import { For, Show } from "solid-js";
import { formatBytes } from "../lib/format";
import { stats } from "../lib/games";

/** Memory and CPU of a running game, with the last two minutes as bar sparklines. */
export default function GameStatsPanel(props: { instanceId: string }) {
  const samples = () => stats[props.instanceId] ?? [];
  const last = () => samples()[samples().length - 1];
  const memory = () => samples().map((s) => s.memoryBytes);
  const peakMemory = () => Math.max(...memory());
  const cpu = () => samples().map((s) => s.cpuPercent);
  const averageCpu = () => cpu().reduce((a, b) => a + b, 0) / Math.max(1, cpu().length);

  return (
    <div class="panel px-corners-md grid grid-cols-2 gap-6 p-4">
      <Show
        when={last()}
        fallback={<p class="col-span-2 text-sm text-muted">Mesure de la mémoire et du processeur…</p>}
      >
        {(now) => (
          <>
            <Metric
              label="Mémoire"
              value={formatBytes(now().memoryBytes)}
              detail={`pic ${formatBytes(peakMemory())}`}
              values={memory()}
              max={peakMemory() * 1.15}
            />
            <Metric
              label="Processeur"
              value={`${Math.round(now().cpuPercent)} %`}
              detail={`moyenne ${Math.round(averageCpu())} %`}
              values={cpu()}
              max={100}
            />
          </>
        )}
      </Show>
    </div>
  );
}

const BARS = 60;

function Metric(props: { label: string; value: string; detail: string; values: number[]; max: number }) {
  // Right-aligned: the newest sample is always the last bar.
  const bars = () => {
    const padded = Array<number>(Math.max(0, BARS - props.values.length)).fill(0);
    return [...padded, ...props.values.slice(-BARS)];
  };
  return (
    <div class="flex min-w-0 flex-col gap-2">
      <div class="flex items-baseline justify-between gap-3">
        <span class="panel-title">{props.label}</span>
        <span class="flex items-baseline gap-2">
          <span class="font-mono text-xs text-muted">{props.detail}</span>
          <span class="font-mono text-sm text-xp-text">{props.value}</span>
        </span>
      </div>
      <svg
        viewBox={`0 0 ${BARS * 2} 24`}
        preserveAspectRatio="none"
        shape-rendering="crispEdges"
        class="h-8 w-full bg-bedrock"
        aria-hidden="true"
      >
        <For each={bars()}>
          {(v, i) => {
            const h = () => (props.max > 0 ? Math.max(v > 0 ? 1 : 0, Math.round((v / props.max) * 24)) : 0);
            return <rect x={i() * 2} y={24 - h()} width={1.5} height={h()} fill="var(--color-xp)" />;
          }}
        </For>
      </svg>
    </div>
  );
}
