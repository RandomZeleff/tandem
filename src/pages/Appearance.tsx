import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { createMemo, createResource, createSignal, For, type JSX, onCleanup, onMount, Show } from "solid-js";
import Alert from "../components/Alert";
import SkinViewer from "../components/SkinViewer";
import Tabs from "../components/Tabs";
import { Icon } from "../components/pixel";
import { openAccounts, signInWithMicrosoft } from "../lib/accounts";
import {
  ACCOUNT_NEEDED,
  api,
  errorMessage,
  LOGIN_REQUIRED,
  type LibrarySkin,
  type SkinVariant,
  type Wardrobe,
} from "../lib/api";
import { CAPE_FRONT, skinFront } from "../lib/skins";
import { activeAccount, refetchAccounts } from "../lib/store";
import { removeWithUndo, toast } from "../lib/toast";

const VARIANTS: { id: SkinVariant; label: string }[] = [
  { id: "classic", label: "Bras larges" },
  { id: "slim", label: "Bras fins" },
];

function Section(props: { title: string; actions?: JSX.Element; children: JSX.Element }) {
  return (
    <section class="panel px-corners-md flex flex-col gap-4 px-5 pt-4 pb-5">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h2 class="panel-title">{props.title}</h2>
        {props.actions}
      </div>
      {props.children}
    </section>
  );
}

/** The player seen from the front, drawn from the texture. */
function SkinThumb(props: { texture: string; variant: SkinVariant }) {
  const [front] = createResource(() => [props.texture, props.variant] as const, ([t, v]) => skinFront(t, v));
  return (
    <Show when={front()} fallback={<span class="h-[112px] w-14" />}>
      {(src) => <img src={src()} alt="" class="h-[112px] w-14 [image-rendering:pixelated]" draggable={false} />}
    </Show>
  );
}

function SkinCard(props: { skin: LibrarySkin; selected: boolean; worn: boolean; onSelect: () => void }) {
  return (
    <button
      class="px-corners-md relative flex flex-col items-center gap-2 bg-slate-800 px-2 pt-3 pb-2.5 shadow-[inset_0_0_0_1px_var(--color-line)] transition-colors duration-100 hover:bg-slate-750 focus-visible:bg-slate-750"
      classList={{ "shadow-[inset_0_0_0_2px_var(--color-grass-light)]! bg-slate-750": props.selected }}
      aria-pressed={props.selected}
      title={props.skin.name}
      onClick={props.onSelect}
    >
      <SkinThumb texture={props.skin.texture} variant={props.skin.variant} />
      <span class="w-full truncate text-center text-xs">{props.skin.name}</span>
      <Show when={props.worn}>
        <span class="absolute top-1.5 right-1.5 flex size-5 items-center justify-center rounded-sm bg-grass-edge" title="Porté">
          <Icon name="check" size={12} color="white" />
        </span>
      </Show>
    </button>
  );
}

function CapeTile(props: { name: string; texture: string | null; active: boolean; disabled: boolean; onClick: () => void }) {
  const scale = 4;
  return (
    <button
      class="px-corners-md flex w-[84px] flex-col items-center gap-2 bg-slate-800 px-2 pt-3 pb-2 shadow-[inset_0_0_0_1px_var(--color-line)] transition-colors duration-100 hover:bg-slate-750 focus-visible:bg-slate-750 disabled:opacity-50"
      classList={{ "shadow-[inset_0_0_0_2px_var(--color-grass-light)]! bg-slate-750": props.active }}
      aria-pressed={props.active}
      disabled={props.disabled}
      onClick={props.onClick}
    >
      <Show
        when={props.texture}
        fallback={
          <span class="slot" style={{ width: `${CAPE_FRONT.width * scale}px`, height: `${CAPE_FRONT.height * scale}px` }}>
            <Icon name="close" size={12} class="text-faint" />
          </span>
        }
      >
        {(texture) => (
          <span
            class="[image-rendering:pixelated]"
            style={{
              width: `${CAPE_FRONT.width * scale}px`,
              height: `${CAPE_FRONT.height * scale}px`,
              "background-image": `url(${texture()})`,
              "background-size": `${CAPE_FRONT.textureWidth * scale}px ${CAPE_FRONT.textureHeight * scale}px`,
              "background-position": `-${CAPE_FRONT.x * scale}px -${CAPE_FRONT.y * scale}px`,
            }}
          />
        )}
      </Show>
      <span class="w-full truncate text-center text-xs">{props.name}</span>
    </button>
  );
}

export default function Appearance() {
  const microsoft = () => activeAccount()?.kind === "microsoft";
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const [wardrobe, { mutate: setWardrobe }] = createResource(
    () => (microsoft() ? activeAccount()!.id : false),
    async () => {
      try {
        return await api.skinWardrobe();
      } catch (err) {
        fail(err);
        return null;
      }
    },
  );
  const [library, { mutate: setLibrary }] = createResource(api.skinLibrary, { initialValue: [] });
  const [defaults] = createResource(api.skinDefaults, { initialValue: [] });
  const [pack] = createResource(api.skinPack, { initialValue: [] });
  const [defaultVariant, setDefaultVariant] = createSignal<SkinVariant>("classic");
  const shownDefaults = () => defaults().filter((s) => s.variant === defaultVariant());

  /** The skin previewed instead of the one worn, until it is put on or dropped. */
  const [selected, setSelected] = createSignal<LibrarySkin | null>(null);
  const selectedInLibrary = createMemo(() => {
    const s = selected();
    return s ? library().find((l) => l.id === s.id) ?? null : null;
  });
  const worn = (skin: LibrarySkin) => !!wardrobe() && wardrobe()!.libraryId === skin.id;
  const activeCape = () => wardrobe()?.capes.find((c) => c.active) ?? null;

  function fail(err: unknown) {
    const message = errorMessage(err);
    if (message === LOGIN_REQUIRED || message === ACCOUNT_NEEDED) {
      openAccounts(message === LOGIN_REQUIRED ? "expired" : "needed");
      return;
    }
    setError(message);
  }

  async function run(task: () => Promise<Wardrobe>, done?: string) {
    setBusy(true);
    setError(null);
    try {
      setWardrobe(await task());
      void refetchAccounts();
      if (done) toast(done);
      return true;
    } catch (err) {
      fail(err);
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function wear() {
    const skin = selected();
    if (!skin) return;
    if (await run(() => api.skinApply(skin.id), `Tu portes maintenant « ${skin.name} ».`)) setSelected(null);
  }

  function added(skin: LibrarySkin) {
    setLibrary((list) => [skin, ...list.filter((s) => s.id !== skin.id)]);
    setSelected(skin);
  }

  async function importFiles(paths: string[]) {
    for (const path of paths) {
      try {
        added(await api.skinAddFile(path));
      } catch (err) {
        setError(errorMessage(err));
      }
    }
  }

  async function pickFile() {
    const picked = await open({ multiple: true, directory: false, filters: [{ name: "Skin", extensions: ["png"] }] });
    if (picked) await importFiles(Array.isArray(picked) ? picked : [picked]);
  }

  const [player, setPlayer] = createSignal("");
  const [fetchingPlayer, setFetchingPlayer] = createSignal(false);
  async function addPlayer(e: SubmitEvent) {
    e.preventDefault();
    const name = player().trim();
    if (!name) return;
    setFetchingPlayer(true);
    setError(null);
    try {
      added(await api.skinAddPlayer(name));
      setPlayer("");
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setFetchingPlayer(false);
    }
  }

  async function rename(skin: LibrarySkin, name: string) {
    const trimmed = name.trim();
    if (!trimmed || trimmed === skin.name) return;
    setLibrary((list) => list.map((s) => (s.id === skin.id ? { ...s, name: trimmed } : s)));
    setSelected((s) => (s && s.id === skin.id ? { ...s, name: trimmed } : s));
    await api.skinRename(skin.id, trimmed).catch(fail);
  }

  async function setVariant(skin: LibrarySkin, variant: SkinVariant) {
    setLibrary((list) => list.map((s) => (s.id === skin.id ? { ...s, variant } : s)));
    setSelected((s) => (s && s.id === skin.id ? { ...s, variant } : s));
    await api.skinSetVariant(skin.id, variant).catch(fail);
  }

  function remove(skin: LibrarySkin) {
    const before = library();
    setSelected(null);
    removeWithUndo({
      message: `« ${skin.name} » retiré de la bibliothèque.`,
      hide: () => setLibrary((list) => list.filter((s) => s.id !== skin.id)),
      show: () => setLibrary(before),
      commit: () => api.skinRemove(skin.id),
    });
  }

  // PNG files dropped anywhere on the window go to the library.
  const [dragging, setDragging] = createSignal(false);
  onMount(() => {
    const unlisten = getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === "enter" || payload.type === "over") setDragging(true);
        else if (payload.type === "leave") setDragging(false);
        else if (payload.type === "drop") {
          setDragging(false);
          void importFiles(payload.paths.filter((p) => p.toLowerCase().endsWith(".png")));
        }
      })
      .catch(() => () => {});
    onCleanup(() => void unlisten.then((stop) => stop()));
  });

  const preview = () => {
    const s = selected();
    if (s) return { texture: s.texture, variant: s.variant };
    const w = wardrobe();
    return { texture: w?.skin ?? null, variant: w?.variant ?? "classic" };
  };

  return (
    <div class="relative mx-auto flex w-full max-w-[1400px] flex-col gap-5">
      <h1 class="title-display text-title">Apparence</h1>

      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>

      <Show
        when={microsoft()}
        fallback={
          <section class="panel px-corners-md flex flex-col items-start gap-3 p-6">
            <p class="font-semibold">Les skins se changent avec un compte Microsoft.</p>
            <p class="text-sm text-muted">Un compte hors ligne garde le skin par défaut en jeu.</p>
            <button
              class="btn btn-primary px-corners mt-1 px-4"
              onClick={() => {
                openAccounts();
                void signInWithMicrosoft();
              }}
            >
              Se connecter avec Microsoft
            </button>
          </section>
        }
      >
        <div class="grid grid-cols-[300px_minmax(0,1fr)] items-start gap-5 max-[900px]:grid-cols-1">
          <aside class="panel px-corners-md sticky top-0 flex flex-col items-center gap-4 px-5 pt-5 pb-5">
            <div class="relative">
              <SkinViewer
                skin={preview().texture}
                variant={preview().variant}
                cape={activeCape()?.texture ?? null}
                width={260}
                height={340}
              />
              <Show when={wardrobe.loading}>
                <p class="absolute inset-0 flex items-center justify-center text-sm text-muted">Chargement du skin…</p>
              </Show>
            </div>

            <Show
              when={selected()}
              fallback={
                <div class="flex w-full flex-col items-center gap-1 text-center">
                  <p class="font-semibold">{activeAccount()?.username}</p>
                  <p class="text-xs text-muted">Choisis un skin pour l'essayer ici.</p>
                </div>
              }
            >
              {(skin) => (
                <div class="flex w-full flex-col gap-3">
                  <Show
                    when={selectedInLibrary()}
                    fallback={<p class="text-center font-semibold">{skin().name}</p>}
                  >
                    {(saved) => (
                      <>
                        <input
                          class="field w-full text-center font-semibold"
                          aria-label="Nom du skin"
                          maxLength={40}
                          value={saved().name}
                          onChange={(e) => void rename(saved(), e.currentTarget.value)}
                          onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
                        />
                        <Tabs
                          items={VARIANTS}
                          value={saved().variant}
                          onChange={(v) => void setVariant(saved(), v)}
                          label="Modèle du skin"
                          idPrefix="skin-variant"
                          variant="segmented"
                          class="w-full"
                          tabClass="h-8 flex-1 justify-center"
                        />
                      </>
                    )}
                  </Show>
                  <div class="flex gap-2">
                    <button class="btn btn-primary px-corners h-10 flex-1" disabled={busy()} onClick={() => void wear()}>
                      {busy() ? "Envoi…" : "Porter ce skin"}
                    </button>
                    <Show when={selectedInLibrary()}>
                      {(saved) => (
                        <button
                          class="btn px-corners size-10 shrink-0 justify-center p-0"
                          aria-label="Retirer de la bibliothèque"
                          title="Retirer de la bibliothèque"
                          onClick={() => remove(saved())}
                        >
                          <Icon name="trash" size={12} />
                        </button>
                      )}
                    </Show>
                  </div>
                  <button class="btn btn-ghost h-8 text-sm" onClick={() => setSelected(null)}>
                    Revenir au skin porté
                  </button>
                </div>
              )}
            </Show>
          </aside>

          <div class="flex min-w-0 flex-col gap-5">
            <Show when={(wardrobe()?.capes.length ?? 0) > 0}>
              <Section title="Capes">
                <div class="flex flex-wrap gap-2">
                  <CapeTile
                    name="Aucune"
                    texture={null}
                    active={!activeCape()}
                    disabled={busy()}
                    onClick={() => activeCape() && void run(() => api.skinSetCape(null))}
                  />
                  <For each={wardrobe()?.capes}>
                    {(cape) => (
                      <CapeTile
                        name={cape.name}
                        texture={cape.texture}
                        active={cape.active}
                        disabled={busy()}
                        onClick={() => !cape.active && void run(() => api.skinSetCape(cape.id))}
                      />
                    )}
                  </For>
                </div>
              </Section>
            </Show>

            <Section
              title="Mes skins"
              actions={
                <div class="flex flex-wrap items-center gap-2">
                  <form class="flex gap-2" onSubmit={addPlayer}>
                    <input
                      class="field h-9 w-44 text-sm"
                      placeholder="Pseudo d'un joueur"
                      aria-label="Reprendre le skin d'un joueur"
                      maxLength={16}
                      value={player()}
                      onInput={(e) => setPlayer(e.currentTarget.value)}
                    />
                    <button class="btn px-corners h-9 px-3 text-sm" disabled={fetchingPlayer() || !player().trim()}>
                      {fetchingPlayer() ? "Recherche…" : "Ajouter"}
                    </button>
                  </form>
                  <button class="btn px-corners h-9 px-3 text-sm" onClick={() => void pickFile()}>
                    <Icon name="folder" size={12} />
                    Importer un PNG
                  </button>
                </div>
              }
            >
              <Show
                when={library().length > 0}
                fallback={
                  <p class="px-corners-md bg-slate-900 px-4 py-8 text-center text-sm text-muted shadow-[inset_0_0_0_1px_var(--color-line)]">
                    Glisse un skin (PNG) ici, importe-le ou reprends celui d'un joueur.
                  </p>
                }
              >
                <div class="grid grid-cols-[repeat(auto-fill,minmax(104px,1fr))] gap-2">
                  <For each={library()}>
                    {(skin) => (
                      <SkinCard
                        skin={skin}
                        selected={selected()?.id === skin.id}
                        worn={worn(skin)}
                        onSelect={() => setSelected(skin)}
                      />
                    )}
                  </For>
                </div>
              </Show>
            </Section>

            <Show when={pack().length > 0}>
              <Section title="Pack Tandem">
                <div class="grid grid-cols-[repeat(auto-fill,minmax(104px,1fr))] gap-2">
                  <For each={pack()}>
                    {(skin) => (
                      <SkinCard
                        skin={skin}
                        selected={selected()?.id === skin.id}
                        worn={false}
                        onSelect={() => setSelected(skin)}
                      />
                    )}
                  </For>
                </div>
              </Section>
            </Show>

            <Section
              title="Skins par défaut"
              actions={
                <Show when={defaults().length > 0}>
                  <Tabs
                    items={VARIANTS}
                    value={defaultVariant()}
                    onChange={setDefaultVariant}
                    label="Modèle des skins par défaut"
                    idPrefix="default-variant"
                    variant="segmented"
                    tabClass="h-7"
                  />
                </Show>
              }
            >
              <Show
                when={defaults().length > 0}
                fallback={
                  <div class="flex flex-wrap items-center justify-between gap-3">
                    <p class="text-sm text-muted">Ils apparaissent après avoir lancé une version 1.19.3 ou plus récente.</p>
                    <button
                      class="btn px-corners h-9 px-3 text-sm"
                      disabled={busy()}
                      onClick={() => void run(() => api.skinReset(), "Skin par défaut remis.")}
                    >
                      Remettre le skin par défaut
                    </button>
                  </div>
                }
              >
                <div class="grid grid-cols-[repeat(auto-fill,minmax(104px,1fr))] gap-2">
                  <For each={shownDefaults()}>
                    {(skin) => (
                      <SkinCard
                        skin={skin}
                        selected={selected()?.id === skin.id}
                        worn={false}
                        onSelect={() => setSelected(skin)}
                      />
                    )}
                  </For>
                </div>
              </Show>
            </Section>
          </div>
        </div>
      </Show>

      <Show when={dragging() && microsoft()}>
        <div class="pointer-events-none fixed inset-0 z-30 flex items-center justify-center bg-black/50">
          <p class="panel px-corners-lg bg-slate-750 px-6 py-4 font-semibold shadow-[inset_0_0_0_2px_var(--color-grass-light)]">
            Dépose le skin pour l'ajouter à ta bibliothèque
          </p>
        </div>
      </Show>
    </div>
  );
}
