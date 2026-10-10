import { type JSX, Show } from "solid-js";
import type { Instance } from "../lib/api";
import { blockLook } from "../lib/look";
import { BlockSlot } from "./pixel";
import ProjectIcon from "./ProjectIcon";

/** An instance's icon: its modpack image when it has one, otherwise its block. */
export default function InstanceSlot(props: { instance: Instance; size: number; style?: JSX.CSSProperties }) {
  return (
    <Show
      when={props.instance.icon}
      fallback={<BlockSlot look={blockLook(props.instance.id, props.instance.block)} size={props.size} style={props.style} />}
    >
      {(icon) => <ProjectIcon url={icon()} size={props.size} style={props.style} />}
    </Show>
  );
}
