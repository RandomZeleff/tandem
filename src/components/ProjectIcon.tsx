import { createSignal, type JSX, Show } from "solid-js";
import { Icon } from "./pixel";

/** Modrinth project icon framed in an inventory slot, with a pixel fallback. */
export default function ProjectIcon(props: { url: string | null; size: number; style?: JSX.CSSProperties }) {
  const [failed, setFailed] = createSignal(false);
  const inner = () => props.size - 8;
  return (
    <span class="slot" style={{ width: `${props.size}px`, height: `${props.size}px`, ...props.style }}>
      <Show when={props.url && !failed()} fallback={<Icon name="grid" size={Math.round(props.size * 0.45)} color="#5A5A5A" />}>
        <img
          src={props.url!}
          alt=""
          loading="lazy"
          width={inner()}
          height={inner()}
          class="object-cover [image-rendering:pixelated]"
          style={{ width: `${inner()}px`, height: `${inner()}px` }}
          onError={() => setFailed(true)}
        />
      </Show>
    </span>
  );
}
