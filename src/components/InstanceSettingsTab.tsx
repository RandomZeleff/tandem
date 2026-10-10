import { open } from "@tauri-apps/plugin-dialog";
import { createEffect, createResource, createSignal, For, type JSX, Show } from "solid-js";
import { api, errorMessage, type Instance, type InstanceSettings } from "../lib/api";
import { formatGigabytes, formatRelative } from "../lib/format";
import { BLOCK_COUNT, blockLook } from "../lib/look";
import { openProject } from "../lib/projects";
import { navigate, refetchInstances } from "../lib/store";
import { toast } from "../lib/toast";
import Alert from "./Alert";
import Dialog from "./Dialog";
import InstanceSlot from "./InstanceSlot";
import { PackRollback } from "./PackUpdate";
import { BlockSlot, Icon, LoaderTag } from "./pixel";
import Select from "./Select";
import VersionChangeDialog from "./VersionChangeDialog";

const WINDOW_SIZES: [number, number][] = [
  [854, 480],
  [1280, 720],
  [1600, 900],
  [1920, 1080],
  [2560, 1440],
];

const MEMORY_STEPS_GB = [1, 2, 3, 4, 6, 8, 10, 12, 16, 24, 32];

const longDate = new Intl.DateTimeFormat("fr-FR", { dateStyle: "long" });

/** An instance's own settings: name, icon, Java, JVM arguments, window size, memory. */
export default function InstanceSettingsTab(props: { instance: Instance; locked: boolean }) {
  const [error, setError] = createSignal<string | null>(null);
  const [duplicating, setDuplicating] = createSignal(false);
  const [changingVersion, setChangingVersion] = createSignal(false);

  /** Saves the editable settings with one field changed. */
  async function save(change: Partial<InstanceSettings>, message?: string) {
    const i = props.instance;
    setError(null);
    try {
      await api.updateInstanceSettings(i.id, {
        name: i.name,
        javaPath: i.javaPath,
        jvmArgs: i.jvmArgs,
        windowWidth: i.windowWidth,
        windowHeight: i.windowHeight,
        ...change,
      });
      await refetchInstances();
      if (message) toast(message);
      return true;
    } catch (err) {
      setError(errorMessage(err));
      return false;
    }
  }

  return (
    <div class="flex h-full flex-col gap-4 overflow-y-auto pr-1">
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>
      <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] items-start gap-4 max-[1180px]:grid-cols-1">
        <div class="flex flex-col gap-4">
          <Section title="Instance">
            <NameField instance={props.instance} onSave={(name) => save({ name }, "Instance renommée")} />
            <IconField instance={props.instance} onError={setError} />
            <Row label="Copie" hint="Une nouvelle instance identique : mods, réglages et mondes. Les mods ne prennent pas de place en plus.">
              <button class="btn px-corners h-9 text-[13px]" disabled={props.locked} onClick={() => setDuplicating(true)}>
                <Icon name="plus" size={11} />
                Dupliquer
              </button>
            </Row>
          </Section>
          <Section title="Informations">
            <dl class="grid grid-cols-[auto_1fr] gap-x-8 gap-y-2.5 text-sm">
              <dt class="text-muted">Version</dt>
              <dd class="flex flex-wrap items-center gap-x-2">
                Minecraft {props.instance.gameVersion}
                <button class="btn btn-ghost h-7 px-2 text-xs" disabled={props.locked} onClick={() => setChangingVersion(true)}>
                  Changer…
                </button>
              </dd>
              <dt class="text-muted">Loader</dt>
              <dd>
                <LoaderTag loader={props.instance.loader} />
                <Show when={props.instance.loaderVersion}>
                  <span class="font-mono text-xs text-muted"> {props.instance.loaderVersion}</span>
                </Show>
              </dd>
              <Show when={props.instance.packProjectId}>
                <dt class="text-muted">Modpack</dt>
                <dd class="flex flex-wrap items-center gap-x-2">
                  <button class="text-xp-text hover:underline focus-visible:underline" onClick={() => openProject(props.instance.packProjectId!)}>
                    Voir la fiche
                  </button>
                  <span class="font-mono text-xs text-muted">{props.instance.packVersion}</span>
                  <PackRollback instance={props.instance} locked={props.locked} onError={setError} />
                </dd>
              </Show>
              <dt class="text-muted">Dernière partie</dt>
              <dd>{formatRelative(props.instance.lastPlayedAt)}</dd>
              <dt class="text-muted">Créée le</dt>
              <dd>{longDate.format(new Date(props.instance.createdAt))}</dd>
              <dt class="text-muted">Identifiant</dt>
              <dd class="font-mono text-xs">{props.instance.id}</dd>
            </dl>
          </Section>
        </div>

        <Section title="Lancement">
          <Row label="Mémoire" hint="En automatique, la mémoire suit la machine et le nombre de mods.">
            <MemoryPicker instance={props.instance} onError={setError} />
          </Row>
          <JavaField instance={props.instance} onSave={(javaPath) => save({ javaPath }, "Java enregistré")} onError={setError} />
          <JvmArgsField instance={props.instance} onSave={(jvmArgs) => save({ jvmArgs }, "Arguments enregistrés")} />
          <WindowField
            instance={props.instance}
            onSave={(size) => save({ windowWidth: size?.[0] ?? null, windowHeight: size?.[1] ?? null }, "Taille de fenêtre enregistrée")}
          />
        </Section>
      </div>

      <Show when={changingVersion()}>
        <VersionChangeDialog instance={props.instance} onClose={() => setChangingVersion(false)} />
      </Show>
      <Show when={duplicating()}>
        <DuplicateDialog instance={props.instance} onClose={() => setDuplicating(false)} />
      </Show>
    </div>
  );
}

function Section(props: { title: string; children: JSX.Element }) {
  return (
    <section class="panel px-corners-md flex flex-col gap-1 px-5 pt-4 pb-2">
      <h2 class="panel-title mb-1">{props.title}</h2>
      <div class="flex flex-col divide-y divide-line">{props.children}</div>
    </section>
  );
}

function Row(props: { label: string; hint?: string; for?: string; children: JSX.Element }) {
  return (
    <div class="flex flex-col gap-2 py-3">
      <div class="flex flex-col gap-0.5">
        <label class="text-sm font-medium" for={props.for}>
          {props.label}
        </label>
        <Show when={props.hint}>
          <span class="text-xs text-muted">{props.hint}</span>
        </Show>
      </div>
      {props.children}
    </div>
  );
}

function NameField(props: { instance: Instance; onSave: (name: string) => Promise<boolean> }) {
  const [value, setValue] = createSignal(props.instance.name);
  createEffect(() => setValue(props.instance.name));
  const commit = () => {
    const name = value().trim();
    if (name && name !== props.instance.name) void props.onSave(name);
    else setValue(props.instance.name);
  };
  return (
    <Row label="Nom" for="instance-name">
      <input
        id="instance-name"
        class="field h-9 w-full text-sm"
        maxLength={64}
        value={value()}
        onInput={(e) => setValue(e.currentTarget.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
      />
    </Row>
  );
}

function IconField(props: { instance: Instance; onError: (message: string) => void }) {
  async function apply(image: string | null, block: number | null) {
    try {
      await api.setInstanceIcon(props.instance.id, image, block);
      await refetchInstances();
    } catch (err) {
      props.onError(errorMessage(err));
    }
  }

  async function pickImage() {
    const path = await open({ multiple: false, directory: false, filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg"] }] });
    if (typeof path === "string") await apply(path, props.instance.block);
  }

  return (
    <Row label="Icône" hint="Une image (carrée de préférence) ou un bloc.">
      <div class="flex flex-wrap items-center gap-3">
        <InstanceSlot instance={props.instance} size={56} />
        <div class="flex flex-col gap-2">
          <div class="flex flex-wrap gap-1.5" role="radiogroup" aria-label="Bloc">
            <For each={Array.from({ length: BLOCK_COUNT }, (_, i) => i)}>
              {(i) => {
                const selected = () => !props.instance.icon && blockLook(props.instance.id, props.instance.block) === blockLook("", i);
                return (
                  <button
                    role="radio"
                    aria-checked={selected()}
                    aria-label={`Bloc ${i + 1}`}
                    class="p-0.5 hover:brightness-125 focus-visible:brightness-125"
                    classList={{ "shadow-[0_0_0_2px_var(--color-xp)]": selected() }}
                    onClick={() => void apply(null, i)}
                  >
                    <BlockSlot look={blockLook("", i)} size={28} />
                  </button>
                );
              }}
            </For>
          </div>
          <div class="flex gap-2">
            <button class="btn btn-ghost h-8 px-2.5 text-xs" onClick={() => void pickImage()}>
              <Icon name="folder" size={10} />
              Choisir une image…
            </button>
          </div>
        </div>
      </div>
    </Row>
  );
}

function JavaField(props: { instance: Instance; onSave: (path: string | null) => Promise<boolean>; onError: (message: string) => void }) {
  const [java] = createResource(() => props.instance.id, api.instanceJava);
  const BROWSE = "__browse__";

  const options = () => {
    const info = java();
    const list = [{ value: "", label: "Automatique", hint: info ? `Java ${info.required} fourni par Mojang` : undefined }];
    for (const j of info?.installs ?? []) {
      const tooOld = info && j.major < info.required;
      list.push({
        value: j.path,
        label: `Java ${j.version}${j.managed ? " (Tandem)" : j.vendor ? ` · ${j.vendor}` : ""}`,
        hint: tooOld ? "trop ancien" : j.major > (info?.required ?? 0) + 4 ? "plus récent que prévu" : undefined,
      });
    }
    const current = props.instance.javaPath;
    if (current && !list.some((o) => o.value === current)) list.push({ value: current, label: current, hint: "choisi à la main" });
    list.push({ value: BROWSE, label: "Parcourir…", hint: undefined });
    return list;
  };

  async function choose(value: string) {
    if (value === BROWSE) {
      const path = await open({
        multiple: false,
        directory: false,
        filters: navigator.userAgent.includes("Windows") ? [{ name: "Java", extensions: ["exe"] }] : undefined,
      });
      if (typeof path === "string") await props.onSave(path);
      return;
    }
    await props.onSave(value || null);
  }

  const selected = () => java()?.installs.find((j) => j.path === props.instance.javaPath);
  return (
    <Row label="Java" for="instance-java" hint="Automatique convient presque toujours : Tandem télécharge le Java prévu par Mojang.">
      <Select id="instance-java" class="h-9 w-full text-[13px]" value={props.instance.javaPath ?? ""} options={options()} onChange={(v) => void choose(v)} />
      <Show when={selected() && java() && selected()!.major < java()!.required}>
        <Alert tone="warning">
          Minecraft {props.instance.gameVersion} demande Java {java()!.required} : avec Java {selected()!.major}, le jeu risque de ne pas
          démarrer.
        </Alert>
      </Show>
    </Row>
  );
}

function JvmArgsField(props: { instance: Instance; onSave: (args: string | null) => Promise<boolean> }) {
  const [value, setValue] = createSignal(props.instance.jvmArgs ?? "");
  createEffect(() => setValue(props.instance.jvmArgs ?? ""));
  const commit = () => {
    const args = value().trim();
    if (args !== (props.instance.jvmArgs ?? "")) void props.onSave(args || null);
  };
  return (
    <Row label="Arguments Java" for="instance-jvm" hint="Pour les joueurs avancés. Tandem ajoute déjà les réglages du ramasse-miettes adaptés.">
      <input
        id="instance-jvm"
        class="field h-9 w-full font-mono text-[12.5px]"
        placeholder="ex. -Dfml.readTimeout=180"
        spellcheck={false}
        value={value()}
        onInput={(e) => setValue(e.currentTarget.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
      />
    </Row>
  );
}

function WindowField(props: { instance: Instance; onSave: (size: [number, number] | null) => Promise<boolean> }) {
  const current = () => (props.instance.windowWidth && props.instance.windowHeight ? `${props.instance.windowWidth}x${props.instance.windowHeight}` : "");
  const [custom, setCustom] = createSignal(false);
  const [width, setWidth] = createSignal(String(props.instance.windowWidth ?? 1280));
  const [height, setHeight] = createSignal(String(props.instance.windowHeight ?? 720));
  const preset = () => (custom() ? "custom" : WINDOW_SIZES.some(([w, h]) => `${w}x${h}` === current()) || !current() ? current() : "custom");

  function choose(value: string) {
    if (value === "custom") {
      setCustom(true);
      return;
    }
    setCustom(false);
    if (!value) void props.onSave(null);
    else {
      const [w, h] = value.split("x").map(Number);
      void props.onSave([w, h]);
    }
  }

  return (
    <Row label="Fenêtre du jeu" for="instance-window" hint="Taille de la fenêtre au lancement ; le plein écran se règle dans le jeu (F11).">
      <div class="flex flex-wrap items-center gap-2">
        <Select
          id="instance-window"
          class="h-9 w-48 text-[13px]"
          value={preset()}
          options={[
            { value: "", label: "Par défaut" },
            ...WINDOW_SIZES.map(([w, h]) => ({ value: `${w}x${h}`, label: `${w} × ${h}` })),
            { value: "custom", label: "Personnalisée…" },
          ]}
          onChange={choose}
        />
        <Show when={preset() === "custom"}>
          <input class="field h-9 w-20 text-center font-mono text-[13px]" inputMode="numeric" aria-label="Largeur" value={width()} onInput={(e) => setWidth(e.currentTarget.value)} />
          <span class="text-muted">×</span>
          <input class="field h-9 w-20 text-center font-mono text-[13px]" inputMode="numeric" aria-label="Hauteur" value={height()} onInput={(e) => setHeight(e.currentTarget.value)} />
          <button
            class="btn px-corners h-9 text-[13px]"
            onClick={async () => {
              if (await props.onSave([Number(width()), Number(height())])) setCustom(false);
            }}
          >
            Appliquer
          </button>
        </Show>
      </div>
    </Row>
  );
}

/** "Automatique" follows the machine and the mod count; fixed sizes stay as chosen. */
function MemoryPicker(props: { instance: Instance; onError: (message: string) => void }) {
  const [info] = createResource(() => props.instance.id, api.memoryInfo);
  const steps = () => {
    const total = info()?.totalMb ?? Infinity;
    const fitting = MEMORY_STEPS_GB.map((gb) => gb * 1024).filter((mb) => mb <= total * 0.75);
    const current = props.instance.memoryMb;
    return current && !fitting.includes(current) ? [...fitting, current].sort((a, b) => a - b) : fitting;
  };

  async function choose(value: string) {
    try {
      await api.setInstanceMemory(props.instance.id, value === "auto" ? null : Number(value));
      await refetchInstances();
    } catch (err) {
      props.onError(errorMessage(err));
    }
  }

  return (
    <div class="flex flex-col gap-1.5">
      <Select
        label="Mémoire allouée"
        class="h-9 w-56 text-sm"
        value={String(props.instance.memoryMb ?? "auto")}
        options={[
          { value: "auto", label: "Automatique", hint: info() ? formatGigabytes(info()!.autoMb) : undefined },
          ...steps().map((mb) => ({ value: String(mb), label: formatGigabytes(mb) })),
        ]}
        onChange={(value) => void choose(value)}
      />
      <Show when={info()}>{(i) => <span class="text-xs text-muted">{formatGigabytes(i().totalMb)} sur cette machine.</span>}</Show>
    </div>
  );
}

function DuplicateDialog(props: { instance: Instance; onClose: () => void }) {
  const [name, setName] = createSignal(`${props.instance.name} (copie)`.slice(0, 64));
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  async function duplicate(e: Event) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const created = await api.duplicateInstance(props.instance.id, name().trim());
      await refetchInstances();
      toast(`« ${created.name} » créée`);
      props.onClose();
      navigate({ page: "instance", id: created.id });
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog title="Dupliquer l'instance" onClose={() => !busy() && props.onClose()}>
      <form class="flex flex-col gap-4" onSubmit={(e) => void duplicate(e)}>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs text-muted" for="duplicate-name">
            Nom de la copie
          </label>
          <input id="duplicate-name" autofocus class="field h-10 w-full" maxLength={64} value={name()} onInput={(e) => setName(e.currentTarget.value)} />
        </div>
        <p class="text-xs text-muted">Mondes et captures compris. Les mods, partagés avec l'original, ne prennent pas de place en plus.</p>
        <Show when={error()}>
          <Alert onClose={() => setError(null)}>{error()}</Alert>
        </Show>
        <div class="flex justify-end gap-2">
          <button type="button" class="btn btn-ghost" disabled={busy()} onClick={props.onClose}>
            Annuler
          </button>
          <button type="submit" class="btn btn-primary px-corners px-5" disabled={busy() || !name().trim()}>
            {busy() ? "Copie…" : "Dupliquer"}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
