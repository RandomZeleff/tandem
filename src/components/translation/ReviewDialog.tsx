import { createResource, createSignal, For, Show } from "solid-js";
import { api, errorMessage, type TranslationEntry, type TranslationSource } from "../../lib/api";
import Alert from "../Alert";
import Dialog from "../Dialog";
import LoadingRows from "../LoadingRows";
import { Icon } from "../pixel";
import Select from "../Select";

const PAGE = 100;

/** Every text of one source with its translation; the player corrects any of them. */
export default function ReviewDialog(props: { instanceId: string; locale: string; source: TranslationSource; onClose: () => void }) {
  const [query, setQuery] = createSignal("");
  const [debounced, setDebounced] = createSignal("");
  const [show, setShow] = createSignal<"all" | "missing" | "translated">("all");
  const [limit, setLimit] = createSignal(PAGE);
  const [error, setError] = createSignal<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const [entries, { mutate }] = createResource(
    () => ({ q: debounced() }),
    ({ q }) => api.translationEntries(props.instanceId, props.locale, props.source.id, q),
  );

  const visible = () =>
    (entries() ?? []).filter((e) =>
      show() === "missing" ? !e.translation && !e.byAuthors : show() === "translated" ? !!e.translation : true,
    );

  async function save(entry: TranslationEntry, value: string) {
    const translation = value.trim();
    if (translation === (entry.translation ?? "")) return;
    setError(null);
    try {
      await api.correctTranslation(props.instanceId, props.locale, entry.english, translation);
      mutate((list) => list?.map((e) => (e.english === entry.english ? { ...e, translation: translation || null } : e)));
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <Dialog title={props.source.name} onClose={props.onClose} width={860}>
      <div class="flex flex-wrap items-center gap-2">
        <div class="relative min-w-[240px] flex-1">
          <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted">
            <Icon name="search" size={11} />
          </span>
          <input
            type="search"
            autofocus
            class="field h-9 w-full pl-7 text-[13px]"
            placeholder="Chercher un texte, une clé, une traduction…"
            aria-label="Chercher"
            value={query()}
            onInput={(e) => {
              setQuery(e.currentTarget.value);
              clearTimeout(timer);
              timer = setTimeout(() => {
                setDebounced(query());
                setLimit(PAGE);
              }, 250);
            }}
          />
        </div>
        <Select
          class="h-9 w-48 text-[13px]"
          label="Afficher"
          value={show()}
          options={[
            { value: "all", label: "Tous les textes" },
            { value: "missing", label: "Pas encore traduits" },
            { value: "translated", label: "Traduits par Tandem" },
          ]}
          onChange={(v) => setShow(v as "all" | "missing" | "translated")}
        />
      </div>
      <p class="-mt-2 text-xs text-muted">
        Corrige une traduction puis quitte le champ : elle est enregistrée et ne sera plus jamais remplacée par l'IA. Vide un
        champ pour effacer la traduction.
      </p>
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>
      <div class="max-h-[58vh] min-h-[200px] overflow-y-auto">
        <Show when={entries()} fallback={<LoadingRows count={6} height={48} label="Chargement des textes…" />}>
          <Show when={visible().length > 0} fallback={<p class="py-10 text-center text-sm text-chalk-2">Aucun texte.</p>}>
            <ul class="flex flex-col divide-y divide-line">
              <For each={visible().slice(0, limit())}>
                {(entry) => (
                  <li class="grid grid-cols-2 gap-3 py-2.5">
                    <div class="flex min-w-0 flex-col gap-0.5">
                      <span class="truncate font-mono text-[10.5px] text-faint" title={entry.key}>
                        {entry.key}
                      </span>
                      <span class="text-[13px] break-words whitespace-pre-wrap text-chalk-2 select-text">{entry.english}</span>
                    </div>
                    <Show
                      when={!entry.byAuthors}
                      fallback={<span class="self-center text-xs text-diamond">Traduit par les auteurs du mod</span>}
                    >
                      <textarea
                        class="field min-h-[38px] w-full resize-y py-2 text-[13px] leading-snug"
                        rows={Math.min(5, Math.max(1, Math.ceil(entry.english.length / 60)))}
                        placeholder="Pas encore traduit"
                        aria-label={`Traduction de « ${entry.english.slice(0, 40)} »`}
                        value={entry.translation ?? ""}
                        onBlur={(e) => void save(entry, e.currentTarget.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" && !e.shiftKey) {
                            e.preventDefault();
                            e.currentTarget.blur();
                          }
                        }}
                      />
                    </Show>
                  </li>
                )}
              </For>
            </ul>
            <Show when={visible().length > limit()}>
              <button class="btn px-corners mx-auto my-3 flex" onClick={() => setLimit(limit() + PAGE)}>
                Voir plus ({visible().length - limit()})
              </button>
            </Show>
          </Show>
        </Show>
      </div>
      <div class="flex justify-end">
        <button class="btn btn-primary px-corners px-5" onClick={props.onClose}>
          Terminé
        </button>
      </div>
    </Dialog>
  );
}
