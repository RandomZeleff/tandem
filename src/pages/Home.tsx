import { createResource, createSignal, For, Show } from "solid-js";
import InstanceCard from "../components/InstanceCard";
import { Icon, LoaderTag } from "../components/pixel";
import PlayButton from "../components/PlayButton";
import Scene from "../components/Scene";
import { api, errorMessage, type Instance } from "../lib/api";
import { formatRelative } from "../lib/format";
import { gameState } from "../lib/games";
import { instances, navigate, setNewInstanceDialog } from "../lib/store";

function Hero(props: { instance: Instance }) {
  const state = () => gameState(props.instance.id);
  const exit = () => state().lastExit;
  const [error, setError] = createSignal<string | null>(null);

  return (
    <section
      aria-label="Reprendre la dernière partie"
      class="px-corners-lg relative h-[316px] shrink-0 bg-[#1E1A33]"
    >
      <Scene variant="sunset" />
      <div class="absolute inset-0 bg-[linear-gradient(90deg,rgb(10_9_14/0.9)_0%,rgb(10_9_14/0.62)_40%,transparent_72%)]" />
      <div class="relative flex h-full flex-col justify-between px-[34px] py-[30px]">
        <div class="flex max-w-[560px] flex-col gap-3">
          <span class="flex items-center gap-2 font-pixel text-[13px] font-medium tracking-[2px] text-gold">
            <span class="size-1.5 bg-gold" />
            {props.instance.lastPlayedAt
              ? `REPRENDRE · ${formatRelative(props.instance.lastPlayedAt).toUpperCase()}`
              : "PRÊTE À JOUER"}
          </span>
          <h1 class="font-pixel text-[52px] leading-[0.95] font-bold tracking-[0.5px] [text-shadow:4px_4px_0_rgb(0_0_0/0.45)]">
            {props.instance.name}
          </h1>
          <div class="flex flex-wrap items-center gap-1.5 text-[13px] text-chalk-2">
            <span class="chip h-[26px]">{props.instance.gameVersion}</span>
            <span class="chip h-[26px]">
              <LoaderTag loader={props.instance.loader} />
            </span>
          </div>
        </div>

        <div class="flex flex-col gap-3">
          <Show when={error() ?? state().error}>
            <p class="w-fit bg-danger/90 px-3 py-1.5 text-sm text-redstone-text">{error() ?? state().error}</p>
          </Show>
          <Show when={exit() && !exit()!.stopped && exit()!.code !== 0}>
            <p class="w-fit bg-warning/90 px-3 py-1.5 text-sm text-gold">
              Le jeu s'est arrêté (code {exit()!.code ?? "?"}). La console de l'instance contient les détails.
            </p>
          </Show>
          <div class="flex items-stretch gap-2.5">
            <PlayButton id={props.instance.id} size="lg" name={props.instance.name} />
            <button
              class="btn px-corners-md h-[54px] bg-slate-700/90 px-5 text-[15px]"
              onClick={() => navigate({ page: "instance", id: props.instance.id })}
            >
              <Icon name="terminal" size={16} />
              Détails et console
            </button>
            <button
              class="btn px-corners-md h-[54px] w-[54px] bg-slate-700/90 px-0"
              aria-label="Ouvrir le dossier de l'instance"
              onClick={() => api.openInstanceFolder(props.instance.id).catch((err) => setError(errorMessage(err)))}
            >
              <Icon name="folder" size={18} />
            </button>
          </div>
        </div>
      </div>
    </section>
  );
}

function Welcome() {
  return (
    <section class="px-corners-lg relative h-[316px] shrink-0 bg-[#3E6FB0]">
      <Scene variant="day" />
      <div class="absolute inset-0 bg-[linear-gradient(90deg,rgb(10_12_20/0.85)_0%,rgb(10_12_20/0.5)_45%,transparent_75%)]" />
      <div class="relative flex h-full max-w-[560px] flex-col justify-center gap-4 px-[34px]">
        <span class="font-pixel text-[13px] font-medium tracking-[2px] text-gold">BIENVENUE</span>
        <h1 class="font-pixel text-5xl leading-none font-bold [text-shadow:4px_4px_0_rgb(0_0_0/0.45)]">
          Prêt à miner ?
        </h1>
        <p class="text-chalk-2">Crée une instance : Tandem télécharge le jeu et la bonne version de Java pour toi.</p>
        <button class="btn btn-primary px-corners-md h-[50px] w-fit px-6 font-pixel text-lg" onClick={() => setNewInstanceDialog({})}>
          <Icon name="plus" size={14} />
          CRÉER MA PREMIÈRE INSTANCE
        </button>
      </div>
    </section>
  );
}

function News() {
  const [versions] = createResource(api.listVersions);
  return (
    <section class="panel px-corners-md flex flex-col gap-3 p-3.5">
      <h2 class="panel-title">Nouveautés</h2>
      <Show when={versions()} fallback={<p class="text-xs text-faint">{versions.error ? "Hors ligne." : "Chargement…"}</p>}>
        {(list) => (
          <>
            <div class="flex items-start gap-3">
              <span class="flex size-9 shrink-0 items-center justify-center bg-[#16261A] shadow-[inset_0_0_0_1px_#2E5A2A]">
                <Icon name="check" size={16} color="var(--color-xp)" />
              </span>
              <div class="flex flex-col gap-0.5">
                <span class="text-sm font-semibold">Version {list().latest.release}</span>
                <span class="text-xs text-muted">Dernière version stable</span>
              </div>
            </div>
            <Show when={list().latest.snapshot !== list().latest.release}>
              <div class="flex items-start gap-3">
                <span class="flex size-9 shrink-0 items-center justify-center bg-[#2A2240] shadow-[inset_0_0_0_1px_#4A3A6E]">
                  <Icon name="sparkle" size={16} color="#B99CF2" />
                </span>
                <div class="flex flex-col gap-1">
                  <span class="text-sm font-semibold">Snapshot {list().latest.snapshot}</span>
                  <span class="text-xs leading-relaxed text-muted">Teste-la dans une instance à part, sans toucher à tes mondes.</span>
                  <button
                    class="btn mt-1 h-[30px] w-fit bg-transparent px-2.5 text-xs text-amethyst shadow-[inset_0_0_0_1px_#4A3A6E]"
                    onClick={() => setNewInstanceDialog({ version: list().latest.snapshot })}
                  >
                    Créer une instance de test
                  </button>
                </div>
              </div>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}

export default function Home() {
  const featured = () => instances()[0];

  return (
    <div class="flex flex-col gap-7">
      <Show when={featured()} fallback={<Welcome />}>
        {(instance) => <Hero instance={instance()} />}
      </Show>

      <div class="flex gap-6">
        <section aria-labelledby="home-instances" class="flex min-w-0 flex-1 flex-col gap-3.5">
          <div class="flex items-center justify-between">
            <h2 id="home-instances" class="font-pixel text-xl font-semibold [text-shadow:2px_2px_0_rgb(0_0_0/0.4)]">
              Instances
            </h2>
            <div class="flex gap-2">
              <Show when={instances().length > 2}>
                <button class="btn btn-ghost h-9 text-sm" onClick={() => navigate({ page: "instances" })}>
                  Tout voir
                </button>
              </Show>
              <button class="btn px-corners h-9 text-sm" onClick={() => setNewInstanceDialog({})}>
                <Icon name="plus" size={12} />
                Nouvelle instance
              </button>
            </div>
          </div>
          <Show
            when={instances().length > 0}
            fallback={<p class="text-sm text-faint">Tes instances apparaîtront ici.</p>}
          >
            {/* Two cards in a narrow window, three from 1280 px. */}
            <div class="grid grid-cols-2 gap-3.5 xl:grid-cols-3 max-xl:[&>*:nth-child(3)]:hidden">
              <For each={instances().slice(0, 3)}>{(instance) => <InstanceCard instance={instance} />}</For>
            </div>
          </Show>
        </section>

        <aside class="flex w-[292px] shrink-0 flex-col gap-3.5">
          <News />
        </aside>
      </div>
    </div>
  );
}
