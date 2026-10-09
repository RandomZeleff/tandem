import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { GalleryImage } from "../../lib/api";
import { openExternal } from "../../lib/projects";
import { trapFocus } from "../../lib/ui";
import { Icon } from "../pixel";

/** Images of a project page, as a grid with a full-size viewer. */
export default function Gallery(props: { images: GalleryImage[] }) {
  const [viewing, setViewing] = createSignal<number | null>(null);

  return (
    <>
      <Show
        when={props.images.length > 0}
        fallback={<p class="panel px-corners-md py-12 text-center text-chalk-2">Pas d'images pour ce projet.</p>}
      >
        <ul class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-3">
          <For each={props.images}>
            {(image, index) => (
              <li>
                <button
                  class="group flex w-full flex-col text-left focus-visible:brightness-110"
                  aria-label={image.title ?? `Image ${index() + 1}`}
                  onClick={() => setViewing(index())}
                >
                  <span class="block aspect-video w-full overflow-hidden bg-slate-900 shadow-[inset_0_0_0_1px_var(--color-line)]">
                    <img
                      src={image.thumbUrl}
                      alt=""
                      loading="lazy"
                      decoding="async"
                      class="size-full object-cover transition-transform duration-150 group-hover:scale-[1.03]"
                    />
                  </span>
                  <Show when={image.title}>
                    <span class="mt-1.5 truncate text-[13px] text-chalk-2 group-hover:text-chalk">{image.title}</span>
                  </Show>
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>

      <Show when={viewing() !== null && props.images[viewing()!]}>
        {(image) => (
          <Viewer
            image={image()}
            position={`${viewing()! + 1} / ${props.images.length}`}
            hasPrev={viewing()! > 0}
            hasNext={viewing()! < props.images.length - 1}
            onMove={(step) => setViewing(viewing()! + step)}
            onClose={() => setViewing(null)}
          />
        )}
      </Show>
    </>
  );
}

function Viewer(props: {
  image: GalleryImage;
  position: string;
  hasPrev: boolean;
  hasNext: boolean;
  onMove: (step: -1 | 1) => void;
  onClose: () => void;
}) {
  let root: HTMLDivElement | undefined;
  trapFocus(() => root);
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") props.onClose();
      else if (e.key === "ArrowLeft" && props.hasPrev) props.onMove(-1);
      else if (e.key === "ArrowRight" && props.hasNext) props.onMove(1);
    };
    window.addEventListener("keydown", onKey);
    onCleanup(() => window.removeEventListener("keydown", onKey));
  });

  return (
    <div
      ref={root}
      tabIndex={-1}
      role="dialog"
      aria-modal="true"
      aria-label={props.image.title ?? "Image"}
      class="fixed inset-0 z-20 flex flex-col bg-black/90"
      onClick={(e) => e.target === e.currentTarget && props.onClose()}
    >
      <div
        class="relative flex min-h-0 flex-1 items-center justify-center px-16 pt-12 pb-4"
        onClick={(e) => e.target === e.currentTarget && props.onClose()}
      >
        <img src={props.image.url} alt={props.image.title ?? ""} class="max-h-full max-w-full object-contain shadow-2xl" />
        <Show when={props.hasPrev}>
          <button class="btn absolute top-1/2 left-4 size-10 -translate-y-1/2 px-0" aria-label="Image précédente" onClick={() => props.onMove(-1)}>
            <Icon name="arrow" size={14} class="-scale-x-100" />
          </button>
        </Show>
        <Show when={props.hasNext}>
          <button class="btn absolute top-1/2 right-4 size-10 -translate-y-1/2 px-0" aria-label="Image suivante" onClick={() => props.onMove(1)}>
            <Icon name="arrow" size={14} />
          </button>
        </Show>
      </div>
      <div class="flex shrink-0 items-center gap-3 border-t border-line bg-slate-800 px-5 py-3">
        <div class="flex min-w-0 flex-1 flex-col">
          <span class="truncate text-sm">{props.image.title ?? "Sans titre"}</span>
          <span class="line-clamp-2 text-xs text-muted">
            {props.position}
            <Show when={props.image.description}> · {props.image.description}</Show>
          </span>
        </div>
        <button class="btn btn-ghost h-8 px-2.5 text-xs" onClick={() => openExternal(props.image.url)}>
          <Icon name="external" size={10} />
          Ouvrir dans le navigateur
        </button>
        <button class="btn btn-ghost h-8 w-8 px-0" aria-label="Fermer" onClick={props.onClose}>
          <Icon name="close" size={12} />
        </button>
      </div>
    </div>
  );
}
