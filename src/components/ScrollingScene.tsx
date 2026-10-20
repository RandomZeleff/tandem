import { For, type JSX } from "solid-js";

/** Width of one tile of every layer: each layer is drawn twice and slides by this much. */
const TILE = 160;
const BOTTOM = 160;
/** Only a thin strip of dirt shows under the grass. */
const VISIBLE_HEIGHT = 144;

/** Stepped silhouette across one tile: `heights[i]` is the top of column `i`. */
function ridge(heights: number[]): string {
  const step = TILE / heights.length;
  let d = `M0 ${BOTTOM}`;
  heights.forEach((h, i) => (d += `V${h}H${(i + 1) * step}`));
  return `${d}V${BOTTOM}z`;
}

function Tree(props: { x: number; ground: number; leaves: string; trunk: string; tall?: boolean }) {
  const top = () => props.ground - (props.tall ? 17 : 13);
  return (
    <>
      <rect x={props.x + 4} y={top() + 6} width="2" height={props.ground - top() - 6} fill={props.trunk} />
      <path
        d={`M${props.x} ${top()}h10v6H${props.x}zM${props.x + 1} ${top() - 1}h8v1h-8zM${props.x - 1} ${top() + 2}h1v3h-1zM${props.x + 10} ${top() + 1}h1v3h-1z`}
        fill={props.leaves}
      />
    </>
  );
}

function Cloud(props: { x: number; y: number; w: number }) {
  return (
    <path
      d={`M${props.x + 4} ${props.y}h${props.w - 8}v2h4v3H${props.x}v-3h4z`}
      fill="#F4F7FB"
      opacity="0.92"
    />
  );
}

/** A layer drawn twice side by side, sliding left one tile per `seconds`, forever. */
function Layer(props: { seconds: number; children: () => JSX.Element }) {
  return (
    <g class="scene-scroll" style={{ "--scene-tile": `-${TILE}px`, "animation-duration": `${props.seconds}s` }}>
      <For each={[0, TILE]}>{(x) => <g transform={`translate(${x} 0)`}>{props.children()}</g>}</For>
    </g>
  );
}

const SKY = ["#3E6FB0", "#4677B8", "#4C7EC0", "#5587C6", "#5E8FCD", "#6A99D3", "#75A3D9", "#84AFDF", "#93BCE5"];

/**
 * A day landscape sliding by slowly, nearer layers faster (parallax). Made for tall
 * panels; still when the system asks for less motion.
 */
export default function ScrollingScene() {
  return (
    <svg
      class="absolute inset-0 block h-full w-full"
      viewBox={`0 0 120 ${VISIBLE_HEIGHT}`}
      preserveAspectRatio="xMidYMax slice"
      shape-rendering="crispEdges"
      aria-hidden="true"
    >
      <For each={SKY}>{(color, i) => <rect y={i() * 12} width="120" height={i() === SKY.length - 1 ? 160 : 12} fill={color} />}</For>
      <rect x="78" y="30" width="14" height="14" fill="#FFE38A" />
      <rect x="82" y="34" width="6" height="6" fill="#FFF3C4" />

      <Layer seconds={240}>
        {() => (
          <>
            <Cloud x={6} y={22} w={26} />
            <Cloud x={58} y={44} w={18} />
            <Cloud x={104} y={14} w={34} />
            <Cloud x={132} y={58} w={16} />
          </>
        )}
      </Layer>

      <Layer seconds={180}>
        {() => (
          <>
            <path d={ridge([92, 86, 80, 74, 70, 76, 84, 90, 94, 88, 82, 78, 84, 92, 96, 92])} fill="#7FA0C8" />
            <path d="M40 70h10v2h-2v2h-6v-2h-2zM110 78h10v2h-2v2h-6v-2h-2z" fill="#E7EEF7" />
          </>
        )}
      </Layer>

      <Layer seconds={110}>
        {() => (
          <>
            <path d={ridge([112, 110, 106, 104, 106, 110, 112, 108, 104, 102, 104, 108, 112, 114, 112, 112])} fill="#3A6B44" />
            <Tree x={22} ground={106} leaves="#2D6036" trunk="#4A3420" />
            <Tree x={92} ground={102} leaves="#2D6036" trunk="#4A3420" tall />
            <Tree x={128} ground={112} leaves="#2D6036" trunk="#4A3420" />
          </>
        )}
      </Layer>

      <Layer seconds={55}>
        {() => (
          <>
            <path d={ridge([128, 128, 127, 127, 128, 129, 129, 128, 127, 127, 128, 128, 129, 129, 128, 128, 127, 128, 128, 128])} fill="#4E9A33" />
            <path d={ridge([131, 131, 130, 130, 131, 132, 132, 131, 130, 130, 131, 131, 132, 132, 131, 131, 130, 131, 131, 131])} fill="#6B4A2C" />
            <path d="M12 140h1v1h-1zM37 146h1v1h-1zM61 137h1v1h-1zM88 151h1v1h-1zM117 142h1v1h-1zM146 148h1v1h-1zM24 155h1v1h-1zM73 144h1v1h-1zM132 157h1v1h-1z" fill="#57391F" />
            <path d="M8 127h1v1H8zM30 126h1v1h-1zM69 126h1v1h-1zM104 126h1v1h-1zM141 127h1v1h-1z" fill="#E8463C" />
            <path d="M18 127h1v1h-1zM51 126h1v1h-1zM84 126h1v1h-1zM122 127h1v1h-1zM153 127h1v1h-1z" fill="#F6D04D" />
            <path d="M3 127h1v1H3zM44 126h1v1h-1zM95 126h1v1h-1zM113 126h1v1h-1z" fill="#3E8A28" />
            <Tree x={40} ground={128} leaves="#2F7A2A" trunk="#5A3E24" tall />
            <Tree x={120} ground={127} leaves="#2F7A2A" trunk="#5A3E24" />
          </>
        )}
      </Layer>
    </svg>
  );
}
