/** Fake AI translation for the browser preview (see mock.ts). */
import { emit } from "@tauri-apps/api/event";
import type {
  GlossaryTerm,
  ProviderConfig,
  TranslationEntry,
  TranslationOverview,
  TranslationPreferences,
  TranslationSettings,
  TranslationSource,
} from "../lib/api";

const PRESETS: TranslationSettings["presets"] = [
  { id: "ollama", label: "Ollama", baseUrl: "http://localhost:11434/v1", local: true, needsKey: false, helpUrl: "https://ollama.com/download", concurrency: 1 },
  { id: "lmstudio", label: "LM Studio", baseUrl: "http://localhost:1234/v1", local: true, needsKey: false, helpUrl: "https://lmstudio.ai", concurrency: 1 },
  { id: "openai", label: "OpenAI", baseUrl: "https://api.openai.com/v1", local: false, needsKey: true, helpUrl: "https://platform.openai.com/api-keys", concurrency: 4 },
  { id: "anthropic", label: "Anthropic (Claude)", baseUrl: "https://api.anthropic.com/v1", local: false, needsKey: true, helpUrl: "https://console.anthropic.com/settings/keys", concurrency: 4 },
  { id: "custom", label: "Autre (compatible OpenAI)", baseUrl: "", local: false, needsKey: false, helpUrl: "", concurrency: 2 },
];

const LANGUAGES = [
  { code: "fr_fr", name: "Français (France)" },
  { code: "es_es", name: "Español (España)" },
  { code: "de_de", name: "Deutsch" },
];

const SOURCES: [string, string, TranslationSource["kind"], number, number][] = [
  ["mod:create.jar", "Create", "mod", 2400, 1800],
  ["mod:sodium.jar", "Sodium", "mod", 210, 210],
  ["mod:chipped.jar", "Chipped", "mod", 7258, 0],
  ["mod:emi.jar", "EMI", "mod", 744, 194],
  ["pack:Custom Tooltips.zip", "Custom Tooltips", "resourcePack", 320, 0],
  ["kubejs", "KubeJS", "kubeJs", 85, 0],
  ["quests", "Quêtes (FTB Quests)", "quests", 3816, 0],
];

export function translationMocks() {
  let provider: ProviderConfig | null = null;
  let hasKey = false;
  let preferences: TranslationPreferences = { locale: "fr_fr", setGameLanguage: true };
  /** Translated count per source, by instance. */
  const translated: Record<string, Record<string, number>> = {};
  const excluded: Record<string, Set<string>> = {};
  const active: Record<string, string | null> = {};
  const corrections = new Map<string, string>();
  let glossary: GlossaryTerm[] = [{ term: "Void Essence", translation: "Essence du Néant", instanceId: "" }];
  let cancelled = false;

  function overview(instanceId: string, locale: string): TranslationOverview {
    const done = (translated[instanceId] ??= {});
    const skip = (excluded[instanceId] ??= new Set());
    const sources: TranslationSource[] = SOURCES.map(([id, name, kind, total, existing]) => {
      const tr = Math.min(done[id] ?? 0, total - existing);
      return { id, name, kind, excluded: skip.has(id), counts: { total, existing, translated: tr, missing: total - existing - tr } };
    });
    const totals = { total: 0, existing: 0, translated: 0, missing: 0 };
    for (const s of sources.filter((s) => !s.excluded)) {
      totals.total += s.counts.total;
      totals.existing += s.counts.existing;
      totals.translated += s.counts.translated;
      totals.missing += s.counts.missing;
    }
    const texts = Math.round(totals.missing * 0.92);
    return {
      locale,
      sources,
      totals,
      estimate: { texts, characters: texts * 34, inputTokens: texts * 12, outputTokens: texts * 13 },
      activeLocale: active[instanceId] ?? null,
      gameLanguage: active[instanceId] ? locale : "en_us",
      quests: "inPlace",
      running: false,
    };
  }

  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

  return async (cmd: string, args: Record<string, unknown>): Promise<unknown> => {
    const instanceId = args.instanceId as string;
    switch (cmd) {
      case "translation_settings":
        return { languages: LANGUAGES, presets: PRESETS, provider: { config: provider, hasKey }, preferences } satisfies TranslationSettings;
      case "set_translation_preferences":
        preferences = args.preferences as TranslationPreferences;
        return null;
      case "set_translation_provider":
        provider = args.config as ProviderConfig;
        if (typeof args.key === "string") hasKey = args.key.length > 0;
        return null;
      case "test_translation_provider": {
        await wait(500);
        const config = args.config as ProviderConfig;
        if (config.preset === "openai" && !args.key && !hasKey) throw "Clé API refusée par le service.";
        return config.preset === "ollama" ? ["llama3.1:8b", "mistral-nemo:12b", "qwen2.5:7b"] : ["model-small", "model-large"];
      }
      case "translation_overview":
        await wait(600);
        return overview(instanceId, args.locale as string);
      case "translate_instance": {
        cancelled = false;
        const o = overview(instanceId, args.locale as string);
        const jobs = o.sources.filter((s) => !s.excluded && s.counts.missing > 0);
        const total = jobs.reduce((n, s) => n + s.counts.missing, 0);
        const started = Date.now();
        let done = 0;
        let tokens = 0;
        const step = Math.max(30, Math.round(total / 40));
        while (done < total && !cancelled) {
          await wait(250);
          done = Math.min(total, done + step);
          tokens = done * 13;
          await emit("translate://progress", {
            instanceId,
            progress: { done, total, failed: Math.floor(done / 400), promptTokens: done * 12, completionTokens: tokens, elapsedMs: Date.now() - started },
          });
        }
        // Spread what was done over the sources, in order.
        let left = done;
        const map = (translated[instanceId] ??= {});
        for (const s of jobs) {
          const add = Math.min(left, s.counts.missing);
          map[s.id] = (map[s.id] ?? 0) + add;
          left -= add;
        }
        active[instanceId] = args.locale as string;
        return {
          progress: { done, total, failed: Math.floor(done / 400), promptTokens: done * 12, completionTokens: tokens, elapsedMs: Date.now() - started },
          cancelled,
          error: null,
        };
      }
      case "cancel_translation":
        cancelled = true;
        return null;
      case "set_translation_active":
        await wait(400);
        active[instanceId] = args.active ? (args.locale as string) : null;
        return null;
      case "set_translation_excluded": {
        const set = (excluded[instanceId] ??= new Set());
        if (args.excluded) set.add(args.sourceId as string);
        else set.delete(args.sourceId as string);
        return null;
      }
      case "forget_translations":
        (translated[instanceId] ??= {})[args.sourceId as string] = 0;
        return null;
      case "translation_entries": {
        await wait(300);
        const q = (args.query as string).toLowerCase();
        const done = translated[instanceId]?.[args.sourceId as string] ?? 0;
        const list: TranslationEntry[] = Array.from({ length: 140 }, (_, i) => {
          const english = i % 3 === 0 ? `Brass Funnel ${i}` : i % 3 === 1 ? `Hold Shift to see more about item ${i}` : `Mechanical Press Mk${i}`;
          const fr = corrections.get(english) ?? (i < done ? `Entonnoir en laiton ${i}` : null);
          return { key: `block.create.thing_${i}`, english, translation: i % 7 === 0 ? null : fr, byAuthors: i % 7 === 0 };
        });
        return list.filter((e) => !q || e.english.toLowerCase().includes(q) || e.key.includes(q));
      }
      case "correct_translation":
        corrections.set(args.english as string, args.translation as string);
        return null;
      case "glossary_terms":
        return glossary.filter((t) => t.instanceId === "" || t.instanceId === instanceId);
      case "set_glossary_term":
        glossary = glossary.filter((t) => !(t.term === args.term && t.instanceId === ((args.instanceId as string) ?? "")));
        glossary.push({ term: args.term as string, translation: args.translation as string, instanceId: (args.instanceId as string) ?? "" });
        return null;
      case "remove_glossary_term":
        glossary = glossary.filter((t) => !(t.term === args.term && t.instanceId === ((args.instanceId as string) ?? "")));
        return null;
      case "translate_description":
        await wait(1200);
        return `<h2>Présentation (traduite)</h2><p>Ce projet <strong>améliore les performances</strong> sans changer l'apparence du jeu.</p>`;
      default:
        return undefined;
    }
  };
}
