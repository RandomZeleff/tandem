import { createResource, createSignal, Show } from "solid-js";
import { api, type Instance, screenshotUrl } from "../lib/api";
import { blockLook } from "../lib/look";
import Scene from "./Scene";

interface Picture {
  url: string;
  thumb: string;
}

/** One lookup per instance and session; cleared when a game ends (new screenshots). */
const pictures = new Map<string, Promise<Picture | null>>();

export function forgetInstancePictures() {
  pictures.clear();
}

/**
 * An instance's picture: the modpack's featured gallery image, otherwise the player's
 * latest screenshot in it; null when it has neither.
 */
function instancePicture(instance: Instance): Promise<Picture | null> {
  const key = `${instance.id}|${instance.packProjectId ?? ""}`;
  let found = pictures.get(key);
  if (!found) {
    found = (async () => {
      if (instance.packProjectId) {
        const project = await api.projectDetails(instance.packProjectId).catch(() => null);
        // The backend sorts the gallery with the featured image first.
        const image = project?.gallery[0];
        if (image) return { url: image.url, thumb: image.thumbUrl };
      }
      const shots = await api.listScreenshots(instance.id).catch(() => []);
      const latest = shots.reduce<(typeof shots)[number] | null>((a, b) => (!a || b.takenAt > a.takenAt ? b : a), null);
      return latest ? { url: screenshotUrl(instance.id, latest), thumb: screenshotUrl(instance.id, latest, true) } : null;
    })();
    pictures.set(key, found);
  }
  return found;
}

/** Landscape drawn in the instance's block colours, for cards without a picture. */
function Banner(props: { instance: Instance }) {
  const look = () => blockLook(props.instance.id, props.instance.block);
  return (
    <svg class="absolute inset-0 block h-full w-full" viewBox="0 0 64 36" preserveAspectRatio="xMidYMax slice" shape-rendering="crispEdges" aria-hidden="true">
      <rect width="64" height="36" fill={look().sky[0]} />
      <rect y="12" width="64" height="10" fill={look().sky[1]} />
      <path d="M0 26h9v-3h11v2h10v-4h12v3h9v-2h13V36H0z" fill={look().hill} />
      <path d="M0 30h9v-2h11v1h10v-2h12v2h9v-1h13V36H0z" fill="rgb(0 0 0 / 0.18)" />
    </svg>
  );
}

/**
 * The instance's picture filling its box (always `object-cover`, so every visual has the
 * box's format), fading in over a drawn fallback.
 */
export default function InstanceVisual(props: { instance: Instance; small?: boolean; fallback?: "scene" | "banner" }) {
  const [picture] = createResource(() => props.instance, instancePicture);
  const [loaded, setLoaded] = createSignal(false);
  return (
    <>
      <Show when={props.fallback === "scene"} fallback={<Banner instance={props.instance} />}>
        <Scene variant="sunset" />
      </Show>
      <Show when={picture()} keyed>
        {(p) => (
          <img
            src={props.small ? p.thumb : p.url}
            alt=""
            loading="lazy"
            draggable={false}
            class="absolute inset-0 h-full w-full object-cover transition-opacity duration-500 ease-out motion-reduce:transition-none"
            classList={{ "opacity-0": !loaded() }}
            onLoad={() => setLoaded(true)}
            onError={(e) => e.currentTarget.remove()}
          />
        )}
      </Show>
    </>
  );
}
