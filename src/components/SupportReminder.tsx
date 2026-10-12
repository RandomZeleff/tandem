import { createResource, createSignal, Show } from "solid-js";
import { api, type Instance } from "../lib/api";
import { Icon } from "./pixel";

/** Hours on an instance before the reminder shows. */
const AFTER_SECONDS = 10 * 3600;
const dismissedKey = (id: string) => `support_reminder_dismissed.${id}`;

/**
 * Once per instance, after many hours of play: its creators accept support. Never
 * shown again once closed; only when at least one creator has a support link.
 */
export default function SupportReminder(props: { instance: Instance; onShow: () => void }) {
  const [hidden, setHidden] = createSignal(false);
  const [creators] = createResource(
    () => props.instance.id,
    async (id) => {
      const [played, dismissed] = await Promise.all([
        api.getSetting<number>(`play_time.${id}`),
        api.getSetting<boolean>(dismissedKey(id)),
      ]);
      if (dismissed || (played ?? 0) < AFTER_SECONDS) return 0;
      const list = await api.instanceCreators(id);
      return list.creators.filter((c) => c.support.length > 0).length;
    },
  );

  function dismiss() {
    setHidden(true);
    void api.setSetting(dismissedKey(props.instance.id), true).catch(() => {});
  }

  return (
    <Show when={!hidden() && (creators.latest ?? 0) > 0}>
      <div class="panel px-corners-md flex items-center gap-4 p-3.5">
        <Icon name="heart" size={14} class="shrink-0 text-redstone-text" />
        <div class="flex min-w-0 flex-1 flex-col">
          <span class="text-sm">Plus de 10 heures sur {props.instance.name} !</span>
          <span class="text-meta text-muted">
            {creators.latest} de ses créateurs acceptent les soutiens, directement chez eux. Si le pack te plaît, c'est le bon endroit
            pour leur dire merci.
          </span>
        </div>
        <button
          class="btn px-corners h-8 px-3 text-xs"
          onClick={() => {
            dismiss();
            props.onShow();
          }}
        >
          Voir les créateurs
        </button>
        <button class="btn btn-ghost h-8 px-2" aria-label="Ne plus afficher" title="Ne plus afficher" onClick={dismiss}>
          <Icon name="close" size={10} />
        </button>
      </div>
    </Show>
  );
}
