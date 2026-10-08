import { Match, Switch } from "solid-js";

/**
 * Pixel-art backdrops. Skies are drawn as flat colour bands (no smooth gradients),
 * anchored to the bottom so the horizon stays visible at any size.
 */
export default function Scene(props: { variant: "sunset" | "day" | "night" }) {
  return (
    <svg
      class="absolute inset-0 block h-full w-full"
      viewBox="0 0 200 60"
      preserveAspectRatio="xMidYMax slice"
      shape-rendering="crispEdges"
      aria-hidden="true"
    >
      <Switch>
        <Match when={props.variant === "sunset"}>
          <Sunset />
        </Match>
        <Match when={props.variant === "day"}>
          <Day />
        </Match>
        <Match when={props.variant === "night"}>
          <Night />
        </Match>
      </Switch>
    </svg>
  );
}

function Bands(props: { colors: string[]; heights: number[] }) {
  let y = 0;
  return props.colors.map((color, i) => {
    const rect = <rect y={y} width="200" height={props.heights[i]} fill={color} />;
    y += props.heights[i];
    return rect;
  });
}

function Ground(props: { grass: string; dirt: string; speck: string }) {
  return (
    <>
      <path d="M0 52h24v-2h20v3h26v-2h30v1h22v-3h18v2h28v-1h32V60H0z" fill={props.grass} />
      <path d="M0 54h24v-2h20v3h26v-2h30v1h22v-3h18v2h28v-1h32V60H0z" fill={props.dirt} />
      <path d="M9 57h1v1H9zM33 58h1v1h-1zM61 56h1v1h-1zM88 58h1v1h-1zM117 55h1v1h-1zM146 57h1v1h-1zM183 58h1v1h-1z" fill={props.speck} />
    </>
  );
}

function Tree(props: { x: number; y: number; trunk: string; leaves: string }) {
  const { x, y } = props;
  return (
    <>
      <rect x={x + 4} y={y + 6} width="2" height="7" fill={props.trunk} />
      <path d={`M${x} ${y}h10v6H${x}zM${x + 1} ${y - 1}h8v1h-8zM${x - 1} ${y + 2}h1v3h-1zM${x + 10} ${y + 1}h1v3h-1z`} fill={props.leaves} />
    </>
  );
}

function Sunset() {
  return (
    <>
      <Bands
        colors={["#1E1A33", "#2B2142", "#43284E", "#6A3054", "#9A3F55", "#C95A4F", "#E58550", "#F3AE5E", "#F3AE5E"]}
        heights={[8, 7, 6, 5, 4, 3, 3, 4, 20]}
      />
      <path d="M14 3h1v1h-1zM37 6h1v1h-1zM58 2h1v1h-1zM83 9h1v1h-1zM101 4h1v1h-1zM122 11h1v1h-1zM139 3h1v1h-1zM176 7h1v1h-1zM191 2h1v1h-1zM66 12h1v1h-1z" fill="#E8D9FF" opacity="0.7" />
      <rect x="150" y="23" width="14" height="14" fill="#FFD27A" />
      <rect x="155" y="27" width="5" height="5" fill="#FFE9B3" />
      <rect x="150" y="31" width="14" height="1" fill="#F3AE5E" />
      <rect x="150" y="34" width="14" height="1" fill="#F3AE5E" />
      <path d="M0 40h12v-4h8v-3h10v-5h8v3h10v4h14v-6h10v-4h8v5h12v4h16v-3h14v-6h10v4h12v5h18v-2h10v-2h28V60H0z" fill="#3A2648" />
      <path d="M0 46h18v-3h16v2h20v-4h14v3h22v-5h16v4h18v-2h24v3h20v-4h32V60H0z" fill="#241B33" />
      <Tree x={43} y={28} trunk="#150F1E" leaves="#1A2A1C" />
      <Tree x={169} y={27} trunk="#150F1E" leaves="#1A2A1C" />
      <Ground grass="#3F7A2A" dirt="#2E2219" speck="#3D2D20" />
      <path d="M72 44h1v1h-1zM96 41h1v1h-1zM110 47h1v1h-1zM131 43h1v1h-1z" fill="#FFD27A" opacity="0.85" />
    </>
  );
}

function Day() {
  return (
    <>
      <Bands colors={["#3E6FB0", "#4C7EC0", "#5E8FCD", "#75A3D9", "#93BCE5", "#93BCE5"]} heights={[10, 8, 7, 6, 6, 23]} />
      <path d="M120 6h18v2h6v3h-30V8h6zM30 11h12v2h4v2H24v-2h6zM170 14h14v2h4v2h-22v-2h4z" fill="#F4F7FB" opacity="0.9" />
      <path d="M0 38h14v-3h12v-4h10v3h16v-2h12v4h18v-5h10v3h14v2h20v-4h12v3h16v-3h10v4h24v-2h20V60H0z" fill="#3A6B44" />
      <Tree x={146} y={33} trunk="#5A3E24" leaves="#2F7A2A" />
      <Ground grass="#4E9A33" dirt="#6B4A2C" speck="#57391F" />
    </>
  );
}

function Night() {
  return (
    <>
      <Bands colors={["#0F1124", "#141733", "#1A1D40", "#21254D", "#21254D"]} heights={[18, 8, 7, 7, 20]} />
      <path d="M8 16h1v1H8zM19 20h1v1h-1zM31 15h1v1h-1zM44 22h1v1h-1zM53 17h1v1h-1zM67 14h1v1h-1zM79 19h1v1h-1zM112 16h1v1h-1zM144 23h1v1h-1zM160 15h1v1h-1zM186 21h1v1h-1z" fill="#D9E2FF" opacity="0.75" />
      <rect x="172" y="17" width="7" height="7" fill="#E6E9F5" />
      <path d="M173 18h2v2h-2zM177 21h1v1h-1z" fill="#C3C9DE" />
      <path d="M0 44h18v-3h20v2h24v-4h18v3h22v-4h20v3h18v-2h24v3h36V60H0z" fill="#191B38" />
      <Tree x={20} y={34} trunk="#141020" leaves="#13241A" />
      <Ground grass="#2F5F28" dirt="#2A2018" speck="#3A2B1F" />
    </>
  );
}
