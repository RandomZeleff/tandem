import { createEffect, createSignal, type JSX, on, Show } from "solid-js";
import { api, errorMessage, type ProviderConfig } from "../../lib/api";
import { openExternal } from "../../lib/projects";
import { toast } from "../../lib/toast";
import { refetchTranslationSettings, translationSettings } from "../../lib/translation";
import Alert from "../Alert";
import { Icon, Toggle } from "../pixel";
import Select from "../Select";

/** Settings of the AI translation: language, service, model and API key. */
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
    const info = translationSettings()?.presets.find((p) => p.id === id);
    setPreset(id);
    if (info && info.baseUrl) setBaseUrl(info.baseUrl);
    if (info) setConcurrency(info.concurrency);
    setModel("");
    setModels(null);
    setTested(null);
    setKey("");
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
      toast("Service de traduction enregistré");
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

  return (
    <section class="panel px-corners-md flex flex-col gap-4 px-5 pt-4 pb-5">
      <div class="flex flex-col gap-0.5">
        <h2 class="panel-title">Traduction IA</h2>
        <p class="text-xs text-muted">
          Traduis les mods, quêtes et livres d'une instance depuis son onglet « Traduction ». Tandem n'a pas de clé à lui :
          utilise un modèle sur ton PC (gratuit) ou ton propre compte chez un service.
        </p>
      </div>
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <Show when={translationSettings()}>
        {(settings) => (
          <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-x-8 gap-y-4 max-[1180px]:grid-cols-1">
            <div class="flex flex-col gap-4">
              <Field label="Langue de traduction" for="tr-locale">
                <Select
                  id="tr-locale"
                  class="h-9 w-full text-sm"
                  value={settings().preferences.locale}
                  options={settings().languages.map((l) => ({ value: l.code, label: l.name }))}
                  onChange={(v) => void savePreferences({ locale: v })}
                />
              </Field>
              <div class="flex items-center justify-between gap-4">
                <div class="flex flex-col gap-0.5">
                  <span class="text-sm font-medium">Mettre le jeu dans cette langue</span>
                  <span class="text-xs text-muted">Quand tu actives la traduction d'une instance ; l'ancienne langue revient si tu la désactives.</span>
                </div>
                <Toggle
                  checked={settings().preferences.setGameLanguage}
                  label="Mettre le jeu dans la langue de traduction"
                  onChange={(v) => void savePreferences({ setGameLanguage: v })}
                />
              </div>
              <Show when={presetInfo()}>
                {(info) => (
                  <div class="flex flex-col gap-2 bg-slate-750 p-3.5 text-xs text-chalk-2 shadow-[inset_0_0_0_1px_var(--color-line)]">
                    <Show
                      when={info().local}
                      fallback={
                        <p>
                          Payant à l'usage, sur ton propre compte. Seuls les textes des mods sont envoyés. Rapide : plusieurs
                          requêtes en parallèle.
                        </p>
                      }
                    >
                      <p>
                        Gratuit et privé : tout reste sur ton PC. Plus lent (une carte graphique aide beaucoup) et un peu moins
                        précis qu'un grand modèle en ligne.
                        <Show when={info().id === "ollama"}>
                          {" "}Après l'installation, télécharge un modèle, par exemple{" "}
                          <code class="bg-slate-900 px-1 font-mono text-[11px]">ollama pull qwen2.5:7b</code>.
                        </Show>
                      </p>
                    </Show>
                    <Show when={info().helpUrl}>
                      <button class="flex w-fit items-center gap-1.5 text-xp-text hover:underline" onClick={() => openExternal(info().helpUrl)}>
                        <Icon name="external" size={10} />
                        {info().local ? `Télécharger ${info().label}` : "Obtenir une clé API"}
                      </button>
                    </Show>
                  </div>
                )}
              </Show>
            </div>

            <div class="flex flex-col gap-4">
              <Field label="Service" for="tr-preset">
                <Select
                  id="tr-preset"
                  class="h-9 w-full text-sm"
                  value={preset()}
                  options={settings().presets.map((p) => ({ value: p.id, label: p.label, hint: p.local ? "sur ton PC" : "en ligne" }))}
                  onChange={choosePreset}
                />
              </Field>
              <Show when={preset() === "custom" || !presetInfo()?.baseUrl}>
                <Field label="Adresse de l'API (compatible OpenAI)" for="tr-url">
                  <input id="tr-url" class="field h-9 w-full font-mono text-[13px]" placeholder="https://…/v1" value={baseUrl()} onInput={(e) => setBaseUrl(e.currentTarget.value)} />
                </Field>
              </Show>
              <Show when={presetInfo()?.needsKey || preset() === "custom"}>
                <Field label="Clé API" for="tr-key">
                  <div class="flex gap-2">
                    <input
                      id="tr-key"
                      type="password"
                      autocomplete="off"
                      class="field h-9 min-w-0 flex-1 font-mono text-[13px]"
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
                        class="field h-9 min-w-0 flex-1 font-mono text-[13px]"
                        placeholder={presetInfo()?.local ? "ex. qwen2.5:7b" : "Teste la connexion pour voir la liste"}
                        value={model()}
                        onInput={(e) => setModel(e.currentTarget.value)}
                      />
                    }
                  >
                    <Select id="tr-model" class="h-9 min-w-0 flex-1 font-mono text-[13px]" value={model()} options={models()!.map((m) => ({ value: m, label: m }))} onChange={setModel} />
                  </Show>
                  <button class="btn px-corners h-9 shrink-0 text-[13px]" disabled={testing() || !baseUrl().trim()} onClick={() => void test()}>
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
            </div>
          </div>
        )}
      </Show>
    </section>
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
