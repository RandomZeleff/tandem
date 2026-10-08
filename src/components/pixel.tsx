import { For, type JSX } from "solid-js";
import type { Loader } from "../lib/api";
import { loaderLabel } from "../lib/format";
import type { BlockLook, SkinLook } from "../lib/look";

/** 12×12 pixel icons, drawn as even-odd paths. */
const ICONS = {
  home: "M5 1h2v1H5zM4 2h4v1H4zM3 3h6v1H3zM2 4h8v1H2zM1 5h10v1H1zM2 6h8v2H2zM2 8h3v3H2zM7 8h3v3H7z",
  grid: "M1 1h4v4H1zM7 1h4v4H7zM1 7h4v4H1zM7 7h4v4H7z",
  search:
    "M3 1h4v1H3zM2 2h1v1H2zM7 2h1v1H7zM1 3h1v4H1zM8 3h1v4H8zM2 7h1v1H2zM7 7h1v1H7zM3 8h4v1H3zM8 8h1v1H8zM9 9h1v1H9zM10 10h1v1h-1z",
  duo: "M1 1h4v4H1zM7 1h4v4H7zM0 6h5v5H0zM7 6h5v5H7z",
  gear: "M5 0h2v2H5zM5 10h2v2H5zM0 5h2v2H0zM10 5h2v2h-2zM2 2h2v2H2zM8 2h2v2H8zM2 8h2v2H2zM8 8h2v2H8zM3 3h6v6H3zM5 5h2v2H5z",
  play: "M2 1h2v1h1v1h1v1h1v1h1v2H7v1H6v1H5v1H4v1H2z",
  stop: "M2 2h8v8H2z",
  plus: "M5 1h2v10H5zM1 5h10v2H1z",
  caret: "M2 4h8v1H2zM3 5h6v1H3zM4 6h4v1H4zM5 7h2v1H5z",
  folder: "M0 2h5v1h7v8H0zM1 5v5h10V5z",
  trash: "M4 0h4v1h3v2H1V1h3zM2 4h8v8H2zM4 5v5h1V5zM7 5v5h1V5z",
  check: "M9 2h2v2H9zM8 4h2v1H8zM7 5h2v1H7zM6 6h2v1H6zM5 7h2v1H5zM3 6h2v2H3zM4 8h2v1H4zM1 4h2v2H1z",
  close: "M1 1h2v1h1v1h1v1h2V3h1V2h1V1h2v2h-1v1H9v1H8v2h1v1h1v1h1v2H9v-1H8V9H7V8H5v1H4v1H3v1H1V9h1V8h1V7h1V5H3V4H2V3H1z",
  invite: "M1 1h4v4H1zM0 6h6v5H0zM9 3h1v2h2v1h-2v2H9V6H7V5h2z",
  download: "M5 0h2v6h3v1H9v1H8v1H7v1H5V9H4V8H3V7H2V6h3zM1 11h10v1H1z",
  terminal: "M0 1h12v10H0zM1 2v8h10V2zM2 3h1v1H2zM3 4h1v1H3zM4 5h1v1H4zM3 6h1v1H3zM2 7h1v1H2zM6 7h4v1H6z",
  dots: "M1 5h2v2H1zM5 5h2v2H5zM9 5h2v2H9z",
  user: "M4 1h4v4H4zM2 6h8v5H2z",
  arrow: "M0 5h8V3h1v1h1v1h1v2h-1v1H9v1H8V7H0z",
  sparkle: "M5 0h2v3h3v2H7v3H5V5H2V3h3zM9 8h1v1h1v1h-1v1H9v-1H8V9h1z",
} as const;

export type IconName = keyof typeof ICONS;

export function Icon(props: { name: IconName; size?: number; class?: string; color?: string }) {
  return (
    <svg
      width={props.size ?? 14}
      height={props.size ?? 14}
      viewBox="0 0 12 12"
      shape-rendering="crispEdges"
      fill-rule="evenodd"
      aria-hidden="true"
      class={props.class}
    >
      <path d={ICONS[props.name]} fill={props.color ?? "currentColor"} />
    </svg>
  );
}

interface Sprite {
  rows: string[];
  palette: Record<string, string>;
}

const ANVIL = [
  "............",
  "............",
  "hhhhhhhhhhh.",
  ".aaaaaaaaaaa",
  "...aaaaaaaa.",
  ".....aaaa...",
  ".....aaaa...",
  "....aaaaaa..",
  "...aaaaaaaa.",
  "..aaaaaaaaaa",
  "..dddddddddd",
  "............",
];

/** 12×12 loader marks, drawn in the launcher's pixel style rather than copied from the official logos. */
const LOADER_SPRITES: Record<Loader, Sprite> = {
  vanilla: {
    rows: [
      "gggggggggggg",
      "glgggglggggg",
      "gggggggggggg",
      "ggdggggdggdg",
      "dgddgdgddddd",
      "dddddddddddd",
      "ddsdddddddsd",
      "dddddddddddd",
      "dddddsdddddd",
      "dddddddddddd",
      "dsddddddsddd",
      "dddddddddddd",
    ],
    palette: { g: "#5DBB3F", l: "#86D45E", d: "#8B5A2B", s: "#6B4423" },
  },
  fabric: {
    rows: [
      ".....oo.....",
      "....ohho....",
      "...ohssho...",
      "..ohccccco..",
      ".ohsccccsco.",
      "ohccccccccco",
      "ohsccccccsco",
      ".ohccccccco.",
      "..ocsccsco..",
      "...occcco...",
      "....occo....",
      ".....oo.....",
    ],
    palette: { o: "#38342A", h: "#F2EAD3", c: "#DBD0B4", s: "#8A7F64" },
  },
  quilt: {
    rows: [
      ".mmmm..pppp.",
      "mMMMMm.pPPPp",
      "mMMMMm.pPPPp",
      "mMMMMm.pPPPp",
      ".mmmm..pppp.",
      "............",
      ".bbbb..oooo.",
      "bBBBBb.oOOOo",
      "bBBBBb.oOOOo",
      "bBBBBb.oOOOo",
      ".bbbb..oooo.",
      "............",
    ],
    palette: {
      m: "#9E1F9F",
      M: "#DC29DD",
      p: "#5E16A8",
      P: "#9722FF",
      b: "#1A6FB0",
      B: "#27A2FD",
      o: "#B86A16",
      O: "#FEA034",
    },
  },
  forge: { rows: ANVIL, palette: { h: "#C9CED6", a: "#7A818A", d: "#4A4F58" } },
  neoforge: { rows: ANVIL, palette: { h: "#F6B26B", a: "#D7742F", d: "#8F4718" } },
};

/** Paths per colour, merging horizontal runs of the same pixel. */
function spritePaths(sprite: Sprite): { color: string; d: string }[] {
  const paths = new Map<string, string>();
  sprite.rows.forEach((row, y) => {
    let x = 0;
    while (x < row.length) {
      const key = row[x];
      let end = x + 1;
      while (end < row.length && row[end] === key) end++;
      const color = sprite.palette[key];
      if (color) paths.set(color, `${paths.get(color) ?? ""}M${x} ${y}h${end - x}v1H${x}z`);
      x = end;
    }
  });
  return [...paths].map(([color, d]) => ({ color, d }));
}

const LOADER_PATHS = Object.fromEntries(
  Object.entries(LOADER_SPRITES).map(([loader, sprite]) => [loader, spritePaths(sprite)]),
) as Record<Loader, { color: string; d: string }[]>;

export function LoaderIcon(props: { loader: Loader; size?: number; class?: string }) {
  return (
    <svg
      width={props.size ?? 14}
      height={props.size ?? 14}
      viewBox="0 0 12 12"
      shape-rendering="crispEdges"
      aria-hidden="true"
      class={props.class}
    >
      <For each={LOADER_PATHS[props.loader] ?? LOADER_PATHS.vanilla}>{(p) => <path d={p.d} fill={p.color} />}</For>
    </svg>
  );
}

/** Loader mark followed by its name, for metadata lines. */
export function LoaderTag(props: { loader: Loader; size?: number }) {
  return (
    <span class="inline-flex items-center gap-1.5 align-middle">
      <LoaderIcon loader={props.loader} size={props.size ?? 12} />
      {loaderLabel(props.loader)}
    </span>
  );
}

/** On/off switch in the inventory style: square knob, grass track when on. */
export function Toggle(props: { checked: boolean; label: string; disabled?: boolean; onChange: (value: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={props.checked}
      aria-label={props.label}
      title={props.label}
      disabled={props.disabled}
      class="relative h-[18px] w-[34px] shrink-0 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
      classList={{
        "bg-grass shadow-[inset_0_-2px_0_rgb(0_0_0/0.3)]": props.checked,
        "bg-slate-900 shadow-[inset_0_0_0_1px_var(--color-line-strong)]": !props.checked,
      }}
      onClick={() => props.onChange(!props.checked)}
    >
      <span
        class="absolute top-[3px] size-3 transition-[left] duration-100"
        classList={{
          "left-[19px] bg-chalk shadow-[inset_0_-2px_0_rgb(0_0_0/0.25)]": props.checked,
          "left-[3px] bg-muted": !props.checked,
        }}
      />
    </button>
  );
}

/** Front face of a block, used as an instance icon. */
export function BlockIcon(props: { look: BlockLook; size?: number }) {
  return (
    <svg
      width={props.size ?? 32}
      height={props.size ?? 32}
      viewBox="0 0 8 8"
      shape-rendering="crispEdges"
      aria-hidden="true"
    >
      <rect width="8" height="8" fill={props.look.side} />
      <path d="M0 0h8v2H0zM0 2h1v1H0zM2 2h2v1H2zM5 2h1v1H5zM7 2h1v1H7z" fill={props.look.top} />
      <path d="M1 0h1v1H1zM4 1h1v1H4zM6 0h1v1H6z" fill={props.look.shine} />
      <path d="M2 4h1v1H2zM5 5h1v1H5zM1 6h1v1H1zM6 7h1v1H6zM3 7h1v1H3z" fill={props.look.spot} />
    </svg>
  );
}

/** Block icon framed in an inventory slot. */
export function BlockSlot(props: { look: BlockLook; size: number; style?: JSX.CSSProperties }) {
  return (
    <span class="slot" style={{ width: `${props.size}px`, height: `${props.size}px`, ...props.style }}>
      <BlockIcon look={props.look} size={Math.round(props.size * 0.66)} />
    </span>
  );
}

/** 8×8 skin face. */
export function SkinHead(props: { look: SkinLook; size?: number; class?: string }) {
  return (
    <svg
      width={props.size ?? 32}
      height={props.size ?? 32}
      viewBox="0 0 8 8"
      shape-rendering="crispEdges"
      aria-hidden="true"
      class={props.class}
    >
      <rect width="8" height="8" fill={props.look.skin} />
      <path d="M0 0h8v2H0zM0 2h1v1H0zM7 2h1v1H7z" fill={props.look.hair} />
      <path d="M1 4h1v1H1zM6 4h1v1H6z" fill="#F4F4F4" />
      <path d="M2 4h1v1H2zM5 4h1v1H5z" fill={props.look.eyes} />
      <path d="M2 6h4v1H2z" fill="#6E4A2E" />
    </svg>
  );
}

/** Segmented progress bar styled after the experience bar. */
export function XpBar(props: { value: number; segments?: number; height?: number; label: string }) {
  const count = () => props.segments ?? 16;
  const filled = () => Math.round(Math.min(1, Math.max(0, props.value)) * count());
  return (
    <div
      role="progressbar"
      aria-label={props.label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(props.value * 100)}
      class="grid gap-px bg-bedrock p-[2px] shadow-[inset_0_0_0_1px_#000]"
      style={{
        height: `${props.height ?? 12}px`,
        "grid-template-columns": `repeat(${count()}, minmax(0, 1fr))`,
      }}
    >
      <For each={Array.from({ length: count() }, (_, i) => i < filled())}>
        {(on) => (
          <span
            style={
              on
                ? { background: "#8BE04E", "box-shadow": "inset 0 -3px 0 #4E9A1E, inset 0 1px 0 #C8F59A" }
                : { background: "#1A2416" }
            }
          />
        )}
      </For>
    </div>
  );
}
