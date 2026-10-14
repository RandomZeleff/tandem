import { createResource, createSignal, For, Show } from "solid-js";
import { api, type Journal } from "../lib/api";
import { formatDistance, formatNumber, formatPlayTime, formatStepDate, isEmptyJournal, MILESTONES } from "../lib/journal";
import LoadingRows from "./LoadingRows";
import { Icon, XpBar } from "./pixel";

const CHAPTERS_SHOWN = 6;

/** A world's logbook: its numbers, the big steps reached, and the quest book. */
export default function WorldJournal(props: { instanceId: string; world: string; version: unknown }) {
  const [journal] = createResource(
    () => ({ id: props.instanceId, world: props.world, version: props.version }),
    ({ id, world }) => api.worldJournal(id, world),
  );

  return (
    <div class="border-t border-line px-3.5 py-4">
      <Show
        when={journal()}
        fallback={
          <Show when={journal.error} fallback={<LoadingRows count={2} height={44} label="Lecture du carnet…" />}>
            <p class="text-sm text-muted">Le carnet de ce monde n'a pas pu être lu.</p>
          </Show>
        }
      >
        {(j) => (
          <Show
            when={!isEmptyJournal(j())}
            fallback={<p class="text-sm text-muted">Rien de noté pour l'instant : le carnet se remplit en jouant.</p>}
          >
            <div class="flex flex-col gap-6">
              <Figures journal={j()} />
              <div class="grid gap-x-8 gap-y-6" classList={{ "lg:grid-cols-2": !!j().quests && j().milestones.length > 0 }}>
                <Show when={j().milestones.length > 0}>
                  <Steps journal={j()} />
                </Show>
                <Show when={j().quests}>{(quests) => <QuestBook quests={quests()} />}</Show>
              </div>
            </div>
          </Show>
        )}
      </Show>
    </div>
  );
}

function Figures(props: { journal: Journal }) {
  const items = () => {
    const s = props.journal.stats;
    return [
      props.journal.day !== null && { label: "Jour", value: formatNumber(props.journal.day) },
      s && { label: "Temps de jeu", value: formatPlayTime(s.playSeconds) },
      s && { label: "Parcourus", value: formatDistance(s.distanceM) },
      s && { label: "Créatures vaincues", value: formatNumber(s.mobKills) },
      s && { label: "Blocs minés", value: formatNumber(s.blocksMined) },
      s && { label: s.deaths === 1 ? "Mort" : "Morts", value: formatNumber(s.deaths) },
    ].filter((x): x is { label: string; value: string } => !!x);
  };
  return (
    <dl class="grid grid-cols-3 gap-x-6 gap-y-4 xl:grid-cols-6">
      <For each={items()}>
        {(item) => (
          <div class="flex flex-col-reverse gap-0.5">
            <dt class="text-xs text-muted">{item.label}</dt>
            <dd class="text-lg font-semibold tabular-nums">{item.value}</dd>
          </div>
        )}
      </For>
    </dl>
  );
}

function Steps(props: { journal: Journal }) {
  const reached = () => props.journal.milestones.filter((m) => m.doneAt !== null).length;
  const next = () => props.journal.milestones.find((m) => m.doneAt === null)?.id;
  return (
    <section class="flex flex-col gap-2" aria-label="Grandes étapes">
      <h3 class="flex items-baseline gap-2 text-sm font-semibold">
        Grandes étapes
        <span class="text-meta font-normal text-muted tabular-nums">
          {reached()} / {props.journal.milestones.length}
        </span>
      </h3>
      <ol class="flex flex-col">
        <For each={props.journal.milestones}>
          {(step) => {
            const done = step.doneAt !== null;
            return (
              <li class="flex h-8 items-center gap-2.5 text-sm">
                <span
                  class="flex size-4 shrink-0 items-center justify-center rounded-[2px]"
                  classList={{
                    "bg-grass shadow-[inset_0_-2px_0_rgb(0_0_0/0.3)]": done,
                    "bg-slate-900 shadow-[inset_0_0_0_1px_var(--color-line-strong)]": !done,
                  }}
                >
                  <Show when={done}>
                    <Icon name="check" size={12} color="#fff" />
                  </Show>
                </span>
                <span class="min-w-0 flex-1 truncate" classList={{ "text-chalk": done, "text-muted": !done && step.id !== next() }}>
                  {MILESTONES[step.id] ?? step.id}
                </span>
                <Show when={done} fallback={<Show when={step.id === next()}><span class="chip h-5 px-1.5 text-xs">Prochaine</span></Show>}>
                  <span class="shrink-0 text-xs text-muted">{formatStepDate(step.doneAt)}</span>
                </Show>
              </li>
            );
          }}
        </For>
      </ol>
    </section>
  );
}

function QuestBook(props: { quests: NonNullable<Journal["quests"]> }) {
  const [all, setAll] = createSignal(false);
  const shown = () => (all() ? props.quests.chapters : props.quests.chapters.slice(0, CHAPTERS_SHOWN));
  const share = () => (props.quests.total ? props.quests.done / props.quests.total : 0);
  return (
    <section class="flex flex-col gap-2" aria-label="Quêtes">
      <h3 class="flex items-baseline gap-2 text-sm font-semibold">
        Quêtes
        <span class="text-meta font-normal text-muted tabular-nums">
          {formatNumber(props.quests.done)} / {formatNumber(props.quests.total)} · {Math.floor(share() * 100)} %
        </span>
      </h3>
      <XpBar value={share()} segments={20} label={`Quêtes terminées : ${props.quests.done} sur ${props.quests.total}`} />
      <ul class="mt-1 flex flex-col">
        <For each={shown()}>
          {(chapter) => (
            <li class="flex h-8 items-center gap-2.5 text-sm">
              <span class="min-w-0 flex-1 truncate" classList={{ "text-muted": chapter.done === 0 }}>
                {chapter.title}
              </span>
              <Show when={chapter.done === chapter.total}>
                <Icon name="check" size={12} color="var(--color-xp-text)" />
              </Show>
              <span class="w-16 shrink-0 text-right text-xs text-muted tabular-nums">
                {chapter.done} / {chapter.total}
              </span>
            </li>
          )}
        </For>
      </ul>
      <Show when={props.quests.chapters.length > CHAPTERS_SHOWN}>
        <button class="btn btn-ghost h-8 w-fit px-2 text-meta" aria-expanded={all()} onClick={() => setAll(!all())}>
          {all() ? "Moins de chapitres" : `Les ${props.quests.chapters.length} chapitres`}
        </button>
      </Show>
    </section>
  );
}
