import { createEffect, createMemo, createResource, createSignal, For, on, Show } from "solid-js";
import { api, errorMessage, type Instance, type TranslationCounts, type TranslationSource } from "../../lib/api";
import { formatCount } from "../../lib/format";
import { navigate, remembered } from "../../lib/store";
import { toast } from "../../lib/toast";
import {
  dismissReport,
  expectedSpeed,
  formatDuration,
  isLocalProvider,
  providerLabel,
  readyProvider,
  runState,
  SOURCE_KINDS,
  startTranslation,
  translationSettings,
} from "../../lib/translation";
import Alert from "../Alert";
import Dialog from "../Dialog";
import LoadingRows from "../LoadingRows";
import { Checkbox, Icon, type IconName, Toggle, XpBar } from "../pixel";
import Select from "../Select";
import GlossaryPanel from "./GlossaryPanel";
import ReviewDialog from "./ReviewDialog";

const integer = new Intl.NumberFormat("fr-FR");

const KIND_ICONS: Record<TranslationSource["kind"], IconName> = {
  mod: "grid",
  resourcePack: "sparkle",
  kubeJs: "code",
  quests: "book",
  book: "book",
};

/** Translating an instance's mods, packs and quests with the player's language model. */
export default function TranslationTab(props: { instance: Instance; locked: boolean }) {
  const [locale, setLocale] = remembered("translation.locale", "");
  const currentLocale = () => locale() || translationSettings()?.preferences.locale || "fr_fr";
  const [overview, { refetch, mutate }] = createResource(
    () => (translationSettings() ? { id: props.instance.id, locale: currentLocale() } : false),
    ({ id, locale }) => api.translationOverview(id, locale),
  );
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [filter, setFilter] = remembered<"all" | "missing">("translation.filter", "missing");
  const [query, setQuery] = createSignal("");
  const [reviewing, setReviewing] = createSignal<TranslationSource | null>(null);
  const [forgetting, setForgetting] = createSignal<TranslationSource | null>(null);
  const run = () => runState(props.instance.id);
  const provider = () => readyProvider();

  // Fresh numbers once a run ends.
  createEffect(on(() => run().running, (running, was) => was === true && !running && void refetch()));

  const languageName = (code: string) => translationSettings()?.languages.find((l) => l.code === code)?.name ?? code;
  const active = () => overview()?.activeLocale === currentLocale();
  const otherActive = () => {
    const a = overview()?.activeLocale;
    return a && a !== currentLocale() ? a : null;
  };

  const sources = createMemo(() => {
    const q = query().trim().toLowerCase();
    const all = overview()?.sources ?? [];
    // Once everything is translated, "with texts to translate" would show nothing.
    const onlyMissing = filter() === "missing" && all.some((s) => s.counts.missing > 0);
    return all
      .filter((s) => !onlyMissing || s.counts.missing > 0)
      .filter((s) => !q || s.name.toLowerCase().includes(q))
      .sort((a, b) => Number(a.excluded) - Number(b.excluded) || b.counts.missing - a.counts.missing || a.name.localeCompare(b.name));
  });

  const duration = () => {
    const o = overview();
    const config = provider();
    if (!o || !config || o.estimate.outputTokens === 0) return null;
    const speed = expectedSpeed(config);
    return { text: formatDuration(o.estimate.outputTokens / speed.tokensPerSecond), measured: speed.measured };
  };

  async function translate() {
    setError(null);
    const report = await startTranslation(props.instance.id, currentLocale());
    if (report && !report.error && !report.cancelled && report.progress.done > 0) {
      toast(`${integer.format(report.progress.done)} textes traduits`);
    }
  }

  async function setActive(value: boolean) {
    setBusy(true);
    setError(null);
    try {
      await api.setTranslationActive(props.instance.id, currentLocale(), value);
      await refetch();
      toast(value ? "Traduction activée : relance le jeu pour la voir" : "Traduction désactivée, fichiers d'origine restaurés");
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function setExcluded(source: TranslationSource, excluded: boolean) {
    const current = overview();
    if (current) {
      mutate({ ...current, sources: current.sources.map((s) => (s.id === source.id ? { ...s, excluded } : s)) });
    }
    try {
      await api.setTranslationExcluded(props.instance.id, source.id, excluded);
      await refetch();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function forget(source: TranslationSource) {
    setError(null);
    try {
      await api.forgetTranslations(props.instance.id, currentLocale(), source.id);
      await refetch();
      toast(`Traductions de ${source.name} effacées : elles seront refaites au prochain lancement`);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <div class="flex h-full flex-col gap-4 overflow-y-auto pr-1">
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>
      <Show when={run().error}>
        <Alert onClose={() => dismissReport(props.instance.id)}>{run().error}</Alert>
      </Show>

      <section class="panel px-corners-md flex flex-col gap-4 p-5">
        <div class="flex flex-wrap items-center gap-3">
          <h2 class="panel-title mr-auto">Traduction IA</h2>
          <Select
            class="h-9 w-56 text-[13px]"
            label="Langue de la traduction"
            value={currentLocale()}
            disabled={run().running}
            options={(translationSettings()?.languages ?? []).map((l) => ({ value: l.code, label: l.name }))}
            onChange={setLocale}
          />
          <Show
            when={provider()}
            fallback={
              <button class="btn btn-gold px-corners h-9 text-[13px]" onClick={() => navigate({ page: "settings" })}>
                <Icon name="gear" size={11} />
                Choisir un service de traduction
              </button>
            }
          >
            {(config) => (
              <button
                class="btn btn-ghost h-9 max-w-72 px-2.5 text-[13px]"
                title="Changer de service dans les réglages"
                onClick={() => navigate({ page: "settings" })}
              >
                <Icon name="sparkle" size={11} />
                <span class="truncate">{providerLabel(config())}</span>
              </button>
            )}
          </Show>
        </div>

        <Show when={overview()} fallback={<Show when={!overview.error}><LoadingRows count={2} height={44} label="Analyse des textes de l'instance…" /></Show>}>
          {(o) => (
            <>
              <CoverageBar counts={o().totals} />
              <p class="text-[13px] text-chalk-2">
                <strong class="text-chalk">{integer.format(o().totals.total)}</strong> textes ·{" "}
                <span class="text-diamond">{integer.format(o().totals.existing)}</span> déjà traduits par les auteurs ·{" "}
                <span class="text-xp-text">{integer.format(o().totals.translated)}</span> par Tandem ·{" "}
                <span class="text-gold">{integer.format(o().totals.missing)}</span> à traduire
              </p>

              <Show
                when={run().running}
                fallback={
                  <div class="flex flex-wrap items-center gap-3">
                    <button
                      class="btn btn-primary px-corners h-10"
                      disabled={!provider() || o().estimate.texts === 0 || busy()}
                      onClick={() => void translate()}
                    >
                      <Icon name="sparkle" size={12} />
                      {o().estimate.texts === 0 ? "Tout est traduit" : `Traduire ${integer.format(o().estimate.texts)} textes`}
                    </button>
                    <Show when={o().estimate.texts > 0 && provider()}>
                      <span class="text-xs text-muted">
                        ≈ {formatCount(o().estimate.inputTokens + o().estimate.outputTokens)} tokens
                        <Show when={duration()}>
                          {(d) => (
                            <>
                              {" "}· environ {d().text}
                              {isLocalProvider(provider()) ? " sur ton PC" : ""}
                              {d().measured ? "" : " (estimation)"}
                            </>
                          )}
                        </Show>
                        . Les noms d'objets passent en premier ; tu peux arrêter quand tu veux, rien n'est perdu.
                      </span>
                    </Show>
                  </div>
                }
              >
                <RunProgress instanceId={props.instance.id} />
              </Show>

              <Show when={run().report}>
                {(report) => (
                  <Alert tone={report().error ? "warning" : "success"} onClose={() => dismissReport(props.instance.id)}>
                    {report().cancelled ? "Traduction arrêtée. " : ""}
                    {integer.format(report().progress.done)} textes traduits
                    {report().progress.failed > 0 ? `, ${integer.format(report().progress.failed)} rejetés (codes du jeu abîmés ou réponse illisible)` : ""}.
                    {report().error ? ` Arrêt : ${report().error}` : ""}
                  </Alert>
                )}
              </Show>

              <div class="flex flex-wrap items-center justify-between gap-3 border-t border-line pt-4">
                <div class="flex flex-col gap-0.5">
                  <span class="text-sm font-medium">Traduction active dans le jeu</span>
                  <span class="text-xs text-muted">
                    {active()
                      ? `Pack « Tandem » activé${o().gameLanguage === currentLocale() ? ", jeu en " + languageName(currentLocale()) : ""}.`
                      : otherActive()
                        ? `La traduction en ${languageName(otherActive()!)} est active : l'activer ici la remplace.`
                        : o().totals.translated === 0
                          ? "Traduis d'abord des textes."
                          : "Désactivée : le jeu affiche les textes d'origine."}
                    {props.locked ? " Le jeu tourne : relance-le ou appuie sur F3+T pour voir les changements." : ""}
                  </span>
                </div>
                <Toggle
                  checked={active()}
                  label="Traduction active dans le jeu"
                  disabled={busy() || run().running || (!active() && o().totals.translated === 0)}
                  onChange={(v) => void setActive(v)}
                />
              </div>

              <Show when={o().quests === "inPlace"}>
                <p class="text-xs text-muted">
                  Les quêtes de ce pack sont écrites directement dans leurs fichiers : Tandem les traduit sur place et garde
                  les originaux, remis en place si tu désactives la traduction.
                </p>
              </Show>
            </>
          )}
        </Show>
        <Show when={overview.error}>
          <Alert>Analyse impossible : {errorMessage(overview.error)}</Alert>
        </Show>
      </section>

      <Show when={overview()}>
        {(o) => (
          <section class="flex flex-col gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="panel-title mr-auto">
                Sources <span class="font-mono text-muted">{o().sources.length}</span>
              </h2>
              <div class="relative w-56">
                <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted">
                  <Icon name="search" size={11} />
                </span>
                <input
                  type="search"
                  class="field h-8 w-full pl-7 text-[13px]"
                  placeholder="Chercher un mod…"
                  aria-label="Chercher un mod"
                  value={query()}
                  onInput={(e) => setQuery(e.currentTarget.value)}
                />
              </div>
              <Select
                class="h-8 w-44 text-[13px]"
                label="Filtre"
                value={filter()}
                options={[
                  { value: "missing", label: "Avec textes à traduire" },
                  { value: "all", label: "Toutes les sources" },
                ]}
                onChange={(v) => setFilter(v as "all" | "missing")}
              />
            </div>
            <Show
              when={sources().length > 0}
              fallback={<p class="panel px-corners-md py-8 text-center text-sm text-chalk-2">Rien à afficher.</p>}
            >
              <ul class="panel px-corners-md flex flex-col divide-y divide-line">
                <For each={sources().slice(0, 300)}>
                  {(source) => (
                    <SourceRow
                      source={source}
                      disabled={run().running}
                      onExclude={(excluded) => void setExcluded(source, excluded)}
                      onReview={() => setReviewing(source)}
                      onForget={() => setForgetting(source)}
                    />
                  )}
                </For>
              </ul>
              <Show when={sources().length > 300}>
                <p class="text-center text-xs text-muted">{sources().length - 300} autres : affine la recherche.</p>
              </Show>
            </Show>
          </section>
        )}
      </Show>

      <GlossaryPanel instanceId={props.instance.id} locale={currentLocale()} />

      <Show when={forgetting()} keyed>
        {(source) => (
          <Dialog title={`Retraduire ${source.name} ?`} onClose={() => setForgetting(null)}>
            <p class="text-chalk-2">
              Les {integer.format(source.counts.translated)} traductions faites par l'IA pour cette source seront effacées, puis
              refaites au prochain lancement de la traduction. Tes corrections sont gardées.
            </p>
            <div class="flex justify-end gap-2">
              <button class="btn btn-ghost" onClick={() => setForgetting(null)}>
                Annuler
              </button>
              <button
                class="btn btn-danger px-corners"
                onClick={() => {
                  setForgetting(null);
                  void forget(source);
                }}
              >
                Effacer
              </button>
            </div>
          </Dialog>
        )}
      </Show>

      <Show when={reviewing()} keyed>
        {(source) => (
          <ReviewDialog
            instanceId={props.instance.id}
            locale={currentLocale()}
            source={source}
            onClose={() => {
              setReviewing(null);
              void refetch();
            }}
          />
        )}
      </Show>
    </div>
  );
}

function CoverageBar(props: { counts: TranslationCounts }) {
  const part = (n: number) => `${props.counts.total ? (n / props.counts.total) * 100 : 0}%`;
  return (
    <div
      class="flex h-3 w-full overflow-hidden bg-slate-900 shadow-[inset_0_0_0_1px_var(--color-line)]"
      role="img"
      aria-label={`${props.counts.existing} traduits par les auteurs, ${props.counts.translated} par Tandem, ${props.counts.missing} à traduire`}
    >
      <span class="h-full bg-diamond/70" style={{ width: part(props.counts.existing) }} />
      <span class="h-full bg-xp" style={{ width: part(props.counts.translated) }} />
    </div>
  );
}

function RunProgress(props: { instanceId: string }) {
  const run = () => runState(props.instanceId);
  const p = () => run().progress;
  const ratio = () => (p() && p()!.total > 0 ? (p()!.done + p()!.failed) / p()!.total : 0);
  const eta = () => {
    const progress = p();
    if (!progress || progress.done === 0 || progress.elapsedMs < 3000) return null;
    const rate = (progress.done + progress.failed) / (progress.elapsedMs / 1000);
    return formatDuration((progress.total - progress.done - progress.failed) / rate);
  };
  const speed = () => {
    const progress = p();
    return progress && progress.elapsedMs > 0 ? progress.completionTokens / (progress.elapsedMs / 1000) : 0;
  };
  return (
    <div class="flex flex-col gap-2">
      <XpBar value={ratio()} segments={32} height={14} label="Progression de la traduction" />
      <div class="flex flex-wrap items-center gap-3 text-xs text-muted">
        <span class="font-mono text-chalk-2">
          {p() ? `${integer.format(p()!.done)} / ${integer.format(p()!.total)}` : "Préparation…"}
        </span>
        <Show when={p() && p()!.failed > 0}>
          <span class="text-gold">{integer.format(p()!.failed)} rejetés</span>
        </Show>
        <Show when={speed() > 0}>
          <span>{integer.format(Math.round(speed()))} tokens/s</span>
        </Show>
        <Show when={eta()}>
          <span>reste environ {eta()}</span>
        </Show>
        <button class="btn btn-ghost ml-auto h-8 px-2.5 text-xs" onClick={() => void api.cancelTranslation(props.instanceId)}>
          <Icon name="stop" size={10} />
          Arrêter
        </button>
      </div>
    </div>
  );
}

function SourceRow(props: {
  source: TranslationSource;
  disabled: boolean;
  onExclude: (excluded: boolean) => void;
  onReview: () => void;
  onForget: () => void;
}) {
  const c = () => props.source.counts;
  const done = () => (c().total ? (c().existing + c().translated) / c().total : 1);
  return (
    <li class="flex items-center gap-3 px-3 py-2" classList={{ "opacity-55": props.source.excluded }}>
      <Checkbox
        checked={!props.source.excluded}
        label={<span class="sr-only">Traduire {props.source.name}</span>}
        disabled={props.disabled}
        onChange={(v) => props.onExclude(!v)}
      />
      <Icon name={KIND_ICONS[props.source.kind]} size={12} class="shrink-0 text-muted" />
      <button class="flex min-w-0 flex-1 flex-col text-left" onClick={props.onReview} title="Revoir et corriger les traductions">
        <span class="flex min-w-0 items-baseline gap-2">
          <span class="truncate text-sm hover:text-xp-text">{props.source.name}</span>
          <span class="shrink-0 text-[11px] text-faint">{SOURCE_KINDS[props.source.kind]}</span>
        </span>
        <span class="mt-1 h-1 w-full max-w-60 bg-slate-900">
          <span class="block h-full bg-xp" style={{ width: `${Math.round(done() * 100)}%` }} />
        </span>
      </button>
      <span class="w-28 shrink-0 text-right text-xs">
        <Show when={c().missing > 0} fallback={<span class="text-xp-text">traduit</span>}>
          <span class="text-gold">{integer.format(c().missing)}</span> <span class="text-muted">à traduire</span>
        </Show>
      </span>
      <span class="w-16 shrink-0 text-right font-mono text-[11px] text-faint">{integer.format(c().total)}</span>
      <button class="btn btn-ghost h-7 px-2 text-xs" disabled={props.disabled} onClick={props.onReview}>
        Revoir
      </button>
      <button
        class="btn btn-ghost h-7 w-7 px-0"
        title="Effacer les traductions IA de cette source pour les refaire"
        aria-label={`Retraduire ${props.source.name}`}
        disabled={props.disabled || c().translated === 0}
        onClick={props.onForget}
      >
        <Icon name="trash" size={10} />
      </button>
    </li>
  );
}
