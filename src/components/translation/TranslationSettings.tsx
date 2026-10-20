import { createEffect, createSignal, For, type JSX, on, Show } from "solid-js";
import { api, errorMessage, type ProviderConfig, type TranslationPreset } from "../../lib/api";
import { openExternal } from "../../lib/projects";
import { toast } from "../../lib/toast";
import { refetchTranslationSettings, translationSettings } from "../../lib/translation";
import Alert from "../Alert";
import { Icon, Toggle } from "../pixel";
import Select from "../Select";
import { Row, Section } from "../SettingsSection";

/** Settings of the AI translation: service, model, API key and language. */
export default function TranslationSettings() {
  const [preset, setPreset] = createSignal("ollama");
  const [baseUrl, setBaseUrl] = createSignal("");
  const [model, setModel] = createSignal("");
  const [concurrency, setConcurrency] = createSignal(1);
  const [key, setKey] = createSignal("");
  const [models, setModels] = createSignal<string[] | null>(null);
  const [testing, setTesting] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [tested, setTested] = createSignal<string | null>(null);

  // Start from what is saved.
  createEffect(
    on(translationSettings, (settings) => {
      if (!settings) return;
      const config = settings.provider.config;
      const first = config ?? { preset: "ollama", baseUrl: settings.presets[0].baseUrl, model: "", concurrency: 1 };
      setPreset(first.preset);
      setBaseUrl(first.baseUrl);
      setModel(first.model);
      setConcurrency(first.concurrency || 1);
    }),
  );

  const presetInfo = () => translationSettings()?.presets.find((p) => p.id === preset());
  const saved = () => translationSettings()?.provider;
  const config = (): ProviderConfig => ({ preset: preset(), baseUrl: baseUrl().trim(), model: model().trim(), concurrency: concurrency() });
  const hasKey = () => saved()?.hasKey && saved()?.config?.preset === preset();
  const dirty = () => {
    const current = saved()?.config;
    return !current || JSON.stringify(current) !== JSON.stringify(config()) || key().trim() !== "";
  };

  function choosePreset(id: string) {
    if (id === preset()) return;
    const info = translationSettings()?.presets.find((p) => p.id === id);
    const current = saved()?.config;
    setPreset(id);
    setModels(null);
    setTested(null);
    setKey("");
    // Back on the saved service: show what is saved rather than a blank form.
    if (current && current.preset === id) {
      setBaseUrl(current.baseUrl);
      setModel(current.model);
      setConcurrency(current.concurrency || 1);
      return;
    }
    if (info && info.baseUrl) setBaseUrl(info.baseUrl);
    if (info) setConcurrency(info.concurrency);
    setModel("");
  }

  async function test() {
    setTesting(true);
    setError(null);
    setTested(null);
    try {
      const list = await api.testTranslationProvider(config(), key().trim() || null);
      setModels(list);
      setTested(`Connecté : ${list.length} modèle${list.length > 1 ? "s" : ""} disponible${list.length > 1 ? "s" : ""}.`);
      if (!model() || !list.includes(model())) setModel(list[0] ?? "");
    } catch (err) {
      setModels(null);
      setError(errorMessage(err));
    } finally {
      setTesting(false);
    }
  }

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await api.setTranslationProvider(config(), key().trim() || null);
      setKey("");
      await refetchTranslationSettings();
      toast("Agent IA enregistré");
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  async function forgetKey() {
    try {
      await api.setTranslationProvider(config(), "");
      await refetchTranslationSettings();
      toast("Clé API supprimée du coffre");
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function savePreferences(change: Partial<{ locale: string; setGameLanguage: boolean }>) {
    const current = translationSettings()?.preferences;
    if (!current) return;
    try {
      await api.setTranslationPreferences({ ...current, ...change });
      await refetchTranslationSettings();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  const presets = (local: boolean) => translationSettings()?.presets.filter((p) => p.local === local) ?? [];

  return (
    <>
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <Show when={translationSettings()}>
        {(settings) => (
          <>
            <section class="panel px-corners-md flex flex-col gap-4 px-5 pt-4 pb-5">
              <div class="flex flex-col gap-0.5">
                <h2 class="panel-title">Agent IA</h2>
                <p class="text-xs text-muted">Traduit les mods depuis l'onglet « Traduction » d'une instance.</p>
              </div>
              <PresetGroup
                title="Sur ton PC"
                hint="Gratuit et privé. Plus lent sans carte graphique."
                presets={presets(true)}
                selected={preset()}
                active={saved()?.config?.preset}
                onSelect={choosePreset}
              />
              <PresetGroup
                title="En ligne"
                hint="Payant à l'usage, avec ta clé. Seuls les textes des mods sont envoyés."
                presets={presets(false)}
                selected={preset()}
                active={saved()?.config?.preset}
                onSelect={choosePreset}
              />
            </section>

            <section class="panel px-corners-md flex flex-col gap-4 px-5 pt-4 pb-5">
              <div class="flex items-center justify-between gap-4">
                <h2 class="panel-title">{presetInfo()?.label ?? "Connexion"}</h2>
                <Show when={presetInfo()?.helpUrl}>
                  {(url) => (
                    <button class="flex items-center gap-1.5 text-xs text-xp-text hover:underline" onClick={() => openExternal(url())}>
                      <Icon name="external" size={10} />
                      {presetInfo()?.local ? `Télécharger ${presetInfo()?.label}` : "Obtenir une clé API"}
                    </button>
                  )}
                </Show>
              </div>
              <Show when={preset() === "ollama"}>
                <p class="text-xs text-chalk-2">
                  Après l'installation, télécharge un modèle, par exemple{" "}
                  <code class="bg-slate-900 px-1 font-mono text-xs">ollama pull qwen2.5:7b</code>.
                </p>
              </Show>
              <Show when={preset() === "custom" || !presetInfo()?.baseUrl}>
                <Field label="Adresse de l'API (compatible OpenAI)" for="tr-url">
                  <input id="tr-url" class="field h-9 w-full font-mono text-meta" placeholder="https://…/v1" value={baseUrl()} onInput={(e) => setBaseUrl(e.currentTarget.value)} />
                </Field>
              </Show>
              <Show when={presetInfo()?.needsKey || preset() === "custom"}>
                <Field label="Clé API" for="tr-key">
                  <div class="flex gap-2">
                    <input
                      id="tr-key"
                      type="password"
                      autocomplete="off"
                      class="field h-9 min-w-0 flex-1 font-mono text-meta"
                      placeholder={hasKey() ? "•••••••• (enregistrée dans le coffre du système)" : "Colle ta clé ici"}
                      value={key()}
                      onInput={(e) => setKey(e.currentTarget.value)}
                    />
                    <Show when={hasKey()}>
                      <button class="btn btn-ghost h-9 px-2.5 text-xs" onClick={() => void forgetKey()}>
                        Oublier
                      </button>
                    </Show>
                  </div>
                </Field>
              </Show>
              <Field label="Modèle" for="tr-model">
                <div class="flex gap-2">
                  <Show
                    when={models() && models()!.length > 0}
                    fallback={
                      <input
                        id="tr-model"
                        class="field h-9 min-w-0 flex-1 font-mono text-meta"
                        placeholder={presetInfo()?.local ? "ex. qwen2.5:7b" : "Teste la connexion pour voir la liste"}
                        value={model()}
                        onInput={(e) => setModel(e.currentTarget.value)}
                      />
                    }
                  >
                    <Select id="tr-model" class="h-9 min-w-0 flex-1 font-mono text-meta" value={model()} options={models()!.map((m) => ({ value: m, label: m }))} onChange={setModel} />
                  </Show>
                  <button class="btn px-corners h-9 shrink-0 text-meta" disabled={testing() || !baseUrl().trim()} onClick={() => void test()}>
                    {testing() ? "Test…" : "Tester"}
                  </button>
                </div>
                <Show when={tested()}>
                  <span class="text-xs text-xp-text">{tested()}</span>
                </Show>
              </Field>
              <Field label="Requêtes en parallèle" for="tr-concurrency">
                <Select
                  id="tr-concurrency"
                  class="h-9 w-40 text-sm"
                  value={String(concurrency())}
                  options={[1, 2, 4, 8].map((n) => ({ value: String(n), label: String(n), hint: n === 1 ? "modèle local" : undefined }))}
                  onChange={(v) => setConcurrency(Number(v))}
                />
              </Field>
              <div class="flex items-center justify-end gap-2">
                <Show when={!dirty() && saved()?.config}>
                  <span class="mr-auto text-xs text-muted">Enregistré.</span>
                </Show>
                <button class="btn btn-primary px-corners h-9 px-5" disabled={saving() || !dirty() || !model().trim() || !baseUrl().trim()} onClick={() => void save()}>
                  {saving() ? "Enregistrement…" : "Enregistrer"}
                </button>
              </div>
            </section>

            <Section title="Langue">
              <Row
                title="Langue de traduction"
                for="tr-locale"
                control={
                  <Select
                    id="tr-locale"
                    class="h-9 w-56 text-sm"
                    value={settings().preferences.locale}
                    options={settings().languages.map((l) => ({ value: l.code, label: l.name }))}
                    onChange={(v) => void savePreferences({ locale: v })}
                  />
                }
              />
              <Row
                title="Mettre le jeu dans cette langue"
                description="Tant que la traduction d'une instance est active."
                control={
                  <Toggle
                    checked={settings().preferences.setGameLanguage}
                    label="Mettre le jeu dans la langue de traduction"
                    onChange={(v) => void savePreferences({ setGameLanguage: v })}
                  />
                }
              />
            </Section>
          </>
        )}
      </Show>
    </>
  );
}

/** Services of one kind (on the PC or online), as cards to pick from. */
function PresetGroup(props: {
  title: string;
  hint: string;
  presets: TranslationPreset[];
  selected: string;
  active: string | undefined;
  onSelect: (id: string) => void;
}) {
  return (
    <div class="flex flex-col gap-2">
      <div class="flex items-baseline gap-2">
        <span class="text-sm font-medium">{props.title}</span>
        <span class="text-xs text-muted">{props.hint}</span>
      </div>
      <div role="radiogroup" aria-label={props.title} class="grid grid-cols-[repeat(auto-fill,minmax(170px,1fr))] gap-2">
        <For each={props.presets}>
          {(p) => {
            const selected = () => props.selected === p.id;
            return (
              <button
                type="button"
                role="radio"
                aria-checked={selected()}
                class="flex min-h-12 items-center justify-between gap-2 rounded-sm bg-slate-750 px-3.5 py-2 text-left text-sm leading-tight transition-colors duration-100"
                classList={{
                  "font-semibold text-chalk shadow-[inset_0_0_0_2px_var(--color-grass-light)]": selected(),
                  "font-medium text-chalk-2 shadow-[inset_0_0_0_1px_var(--color-line)] hover:bg-slate-650 hover:text-chalk": !selected(),
                }}
                onClick={() => props.onSelect(p.id)}
              >
                <span>{p.label}</span>
                <Show when={props.active === p.id}>
                  <span class="flex shrink-0 items-center gap-1 text-xs font-medium text-xp-text">
                    <Icon name="check" size={10} />
                    Actif
                  </span>
                </Show>
              </button>
            );
          }}
        </For>
      </div>
    </div>
  );
}

function Field(props: { label: string; for: string; children: JSX.Element }) {
  return (
    <div class="flex flex-col gap-1.5">
      <label class="text-xs text-muted" for={props.for}>
        {props.label}
      </label>
      {props.children}
    </div>
  );
}
