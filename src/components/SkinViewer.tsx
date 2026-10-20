import { createEffect, onCleanup, onMount } from "solid-js";
import { IdleAnimation, SkinViewer as Viewer } from "skinview3d";
import type { SkinVariant } from "../lib/api";

/** The player in 3D, turned with the mouse. Loaded with the Appearance page only. */
export default function SkinViewer(props: {
  skin: string | null;
  variant: SkinVariant;
  cape: string | null;
  width: number;
  height: number;
}) {
  let canvas: HTMLCanvasElement | undefined;
  let viewer: Viewer | undefined;

  onMount(() => {
    viewer = new Viewer({ canvas, width: props.width, height: props.height, zoom: 0.85, fov: 40 });
    viewer.controls.enableZoom = false;
    viewer.controls.enablePan = false;
    viewer.playerObject.rotation.y = 0.45;
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (!still) viewer.animation = new IdleAnimation();
    onCleanup(() => viewer?.dispose());
  });

  createEffect(() => {
    const skin = props.skin;
    const model = props.variant === "slim" ? "slim" : "default";
    if (!viewer) return;
    if (skin) void viewer.loadSkin(skin, { model });
    else viewer.loadSkin(null);
  });

  createEffect(() => {
    const cape = props.cape;
    if (!viewer) return;
    if (cape) void viewer.loadCape(cape);
    else viewer.loadCape(null);
  });

  return <canvas ref={canvas} class="cursor-grab active:cursor-grabbing" aria-label="Aperçu du skin, à tourner à la souris" />;
}
