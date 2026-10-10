import { createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage, type GlossaryTerm } from "../../lib/api";
import Alert from "../Alert";
import { Checkbox, Icon } from "../pixel";

/**
 * Terms the model must translate a given way (names of the pack's world, factions,
 * custom items). Minecraft's own names are already known; these come on top.
 */
export default function GlossaryPanel(props: { instanceId: string; locale: string }) {
  const [terms, { refetch }] = createResource(
    () => ({ id: props.instanceId, locale: props.locale }),
    ({ id, locale }) => api.glossaryTerms(id, locale),
  );
  const [term, setTerm] = createSignal("");
  const [translation, setTranslation] = createSignal("");
  const [shared, setShared] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  async function add(e: Event) {
    e.preventDefault();
    if (!term().trim() || !translation().trim()) return;
    setError(null);
    try {
      await api.setGlossaryTerm(shared() ? null : props.instanceId, props.locale, term(), translation());
      setTerm("");
      setTranslation("");
      await refetch();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function remove(t: GlossaryTerm) {
    try {
      await api.removeGlossaryTerm(t.instanceId || null, props.locale, t.term);
      await refetch();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <section class="panel px-corners-md flex flex-col gap-3 p-4">
      <div class="flex flex-col gap-0.5">
        <h2 class="panel-title">Glossaire</h2>
        <p class="text-xs text-muted">
          Les noms officiels de Minecraft et ceux déjà traduits par les auteurs sont repris tout seuls. Ajoute ici les termes
          du pack à traduire toujours pareil, ou à ne jamais traduire (même mot des deux côtés).
        </p>
      </div>
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>
      <form class="flex flex-wrap items-center gap-2" onSubmit={(e) => void add(e)}>
        <input class="field h-9 min-w-40 flex-1 text-[13px]" placeholder="Terme anglais (ex. Void Essence)" aria-label="Terme anglais" value={term()} onInput={(e) => setTerm(e.currentTarget.value)} />
        <Icon name="arrow" size={10} class="text-faint" />
        <input class="field h-9 min-w-40 flex-1 text-[13px]" placeholder="Traduction (ex. Essence du Néant)" aria-label="Traduction" value={translation()} onInput={(e) => setTranslation(e.currentTarget.value)} />
        <Checkbox class="text-xs text-muted" checked={shared()} label="Toutes les instances" onChange={setShared} />
        <button class="btn px-corners h-9 text-[13px]" type="submit" disabled={!term().trim() || !translation().trim()}>
          <Icon name="plus" size={10} />
          Ajouter
        </button>
      </form>
      <Show when={(terms() ?? []).length > 0}>
        <ul class="flex flex-wrap gap-1.5">
          <For each={terms()}>
            {(t) => (
              <li class="chip h-7 gap-1.5 pr-1 text-[12px]">
                <span class="text-chalk-2">{t.term}</span>
                <Icon name="arrow" size={8} class="text-faint" />
                <span>{t.translation}</span>
                <Show when={!t.instanceId}>
                  <span class="text-[10px] text-muted" title="Pour toutes les instances">partout</span>
                </Show>
                <button class="flex size-5 items-center justify-center text-muted hover:text-redstone-text" aria-label={`Retirer ${t.term}`} onClick={() => void remove(t)}>
                  <Icon name="close" size={8} />
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </section>
  );
}
