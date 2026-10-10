import { createEffect, createMemo, createResource, createSignal, on, onCleanup, Show } from "solid-js";
import { api, errorMessage, type Instance, type Loader, type RetargetPlan } from "../lib/api";
import { loaderLabel } from "../lib/format";
import { refetchInstances } from "../lib/store";
import { toast } from "../lib/toast";
import Alert from "./Alert";
import Dialog from "./Dialog";
import { Checkbox, Icon, LoaderIcon } from "./pixel";
import Select from "./Select";

const LOADERS: Loader[] = ["vanilla", "fabric", "quilt", "forge", "neoforge"];
const integer = new Intl.NumberFormat("fr-FR");

/** Moves an instance to another game version and/or loader, after showing what happens to its content. */
export default function VersionChangeDialog(props: { instance: Instance; onClose: () => void }) {
  const [versions] = createResource(api.listVersions);
  const [snapshots, setSnapshots] = createSignal(false);
  const [version, setVersion] = createSignal(props.instance.gameVersion);
  const [loader, setLoader] = createSignal<Loader>(props.instance.loader);
  const [loaderVersion, setLoaderVersion] = createSignal("");
  const [keepCopy, setKeepCopy] = createSignal(true);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const choices = createMemo(() =>
    (versions()?.versions ?? []).filter((v) => v.type === "release" || ((snapshots() || v.id === version()) && v.type === "snapshot")),
  );
  const [loaderVersions] = createResource(
    () => (loader() !== "vanilla" && version() ? ([loader(), version()] as const) : null),
    ([l, v]) => api.listLoaderVersions(l, v),
  );
  createEffect(() => {
    const list = loaderVersions.latest ?? [];
    const keep = loader() === props.instance.loader && version() === props.instance.gameVersion ? props.instance.loaderVersion : null;
    setLoaderVersion(
      (list.find((v) => v.version === keep) ?? list.find((v) => v.recommended) ?? list.find((v) => v.stable) ?? list[0])?.version ?? "",
    );
  });

  /** The manifest lists versions newest first. */
  const downgrade = () => {
    const list = versions()?.versions ?? [];
    const from = list.findIndex((v) => v.id === props.instance.gameVersion);
    const to = list.findIndex((v) => v.id === version());
    return from >= 0 && to > from;
  };
  const unchanged = () =>
    version() === props.instance.gameVersion &&
    loader() === props.instance.loader &&
    (loader() === "vanilla" || loaderVersion() === props.instance.loaderVersion);
  const target = () => ({ gameVersion: version(), loader: loader(), loaderVersion: loader() === "vanilla" ? null : loaderVersion() || null });

  // The analysis follows the chosen target, a moment after it settles.
  const [plan, setPlan] = createSignal<RetargetPlan | null>(null);
  const [analysing, setAnalysing] = createSignal(false);
  createEffect(
    on([version, loader, loaderVersion], () => {
      setPlan(null);
      if (unchanged() || (loader() !== "vanilla" && !loaderVersion())) return;
      const wanted = JSON.stringify(target());
      const timer = setTimeout(async () => {
        setAnalysing(true);
        setError(null);
        try {
          const result = await api.planVersionChange(props.instance.id, target());
          if (JSON.stringify(target()) === wanted) setPlan(result);
        } catch (err) {
          setError(errorMessage(err));
        } finally {
          setAnalysing(false);
        }
      }, 300);
      onCleanup(() => clearTimeout(timer));
    }),
  );

  async function apply() {
    setBusy(true);
    setError(null);
    try {
      if (keepCopy()) {
        await api.duplicateInstance(props.instance.id, `${props.instance.name} (${props.instance.gameVersion})`.slice(0, 64));
      }
      const done = await api.changeInstanceVersion(props.instance.id, target());
      await refetchInstances();
      const parts = [`${integer.format(done.updated.length)} mis à jour`];
      if (done.unavailable.length) parts.push(`${integer.format(done.unavailable.length)} désactivés`);
      toast(`Instance passée en ${version()} : ${parts.join(", ")}`, { durationMs: 8000 });
      props.onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog title="Changer de version" onClose={() => !busy() && props.onClose()} width={620}>
      <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-3">
        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between">
            <label class="text-xs text-muted" for="retarget-version">
              Minecraft
            </label>
            <Checkbox checked={snapshots()} label="Snapshots" onChange={setSnapshots} />
          </div>
          <Select
            id="retarget-version"
            value={version()}
            disabled={busy() || versions.loading}
            placeholder={versions.loading ? "Chargement…" : "Choisir"}
            options={choices().map((v) => ({ value: v.id, label: v.id, hint: v.id === props.instance.gameVersion ? "actuelle" : v.type === "snapshot" ? "snapshot" : undefined }))}
            onChange={setVersion}
          />
        </div>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs text-muted" for="retarget-loader">
            Loader
          </label>
          <Select
            id="retarget-loader"
            value={loader()}
            disabled={busy()}
            options={LOADERS.map((l) => ({ value: l, label: loaderLabel(l), icon: <LoaderIcon loader={l} size={12} /> }))}
            onChange={(v) => setLoader(v as Loader)}
          />
        </div>
      </div>
      <Show when={loader() !== "vanilla"}>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs text-muted" for="retarget-loader-version">
            Version de {loaderLabel(loader())}
          </label>
          <Select
            id="retarget-loader-version"
            value={loaderVersion()}
            disabled={busy() || loaderVersions.loading || (loaderVersions() ?? []).length === 0}
            placeholder={loaderVersions.loading ? "Chargement…" : `${loaderLabel(loader())} n'existe pas pour ${version()}`}
            options={(loaderVersions() ?? []).map((v) => ({ value: v.version, label: v.version, hint: v.recommended ? "recommandée" : v.stable ? undefined : "bêta" }))}
            onChange={setLoaderVersion}
          />
        </div>
      </Show>

      <div class="min-h-24 bg-slate-800 p-3.5 text-[13px] shadow-[inset_0_0_0_1px_var(--color-line)]">
        <Show
          when={!unchanged()}
          fallback={<p class="text-muted">Choisis une autre version de Minecraft ou un autre loader.</p>}
        >
          <Show when={plan()} fallback={<p class="text-muted">{analysing() ? "Vérification des mods sur Modrinth…" : " "}</p>}>
            {(p) => (
              <div class="flex flex-col gap-2">
                <p class="text-chalk-2">
                  <Show when={p().kept + p().updated.length > 0}>
                    <span class="text-xp-text">{integer.format(p().kept + p().updated.length)} compatibles</span>
                    {p().updated.length ? ` (dont ${integer.format(p().updated.length)} à mettre à jour)` : ""}
                  </Show>
                  <Show when={p().kept + p().updated.length > 0 && p().unavailable.length > 0}> · </Show>
                  <Show when={p().unavailable.length > 0}>
                    <span class="text-gold">{integer.format(p().unavailable.length)} sans version compatible</span>, désactivés (pas supprimés)
                  </Show>
                  <Show when={p().kept + p().updated.length + p().unavailable.length === 0}>Aucun contenu Modrinth à adapter.</Show>
                </p>
                <Show when={p().unavailable.length > 0}>
                  <p class="text-xs text-muted">
                    {p().unavailable.slice(0, 8).join(", ")}
                    {p().unavailable.length > 8 ? ` et ${p().unavailable.length - 8} autres` : ""}
                  </p>
                </Show>
                <Show when={p().unknown.length > 0}>
                  <p class="text-xs text-muted">
                    {integer.format(p().unknown.length)} fichier{p().unknown.length > 1 ? "s" : ""} ajouté{p().unknown.length > 1 ? "s" : ""} à la main, laissé
                    {p().unknown.length > 1 ? "s" : ""} tel{p().unknown.length > 1 ? "s" : ""} quel{p().unknown.length > 1 ? "s" : ""} : {p().unknown.slice(0, 4).join(", ")}
                    {p().unknown.length > 4 ? "…" : ""}
                  </p>
                </Show>
              </div>
            )}
          </Show>
        </Show>
      </div>

      <Show when={plan()?.gameVersionChanges}>
        <Show
          when={downgrade()}
          fallback={
            <Alert tone="warning">
              Tes mondes seront convertis en {version()} à leur prochaine ouverture ; Tandem les sauvegarde d'abord.
            </Alert>
          }
        >
          <Alert>
            {version()} est plus ancienne que {props.instance.gameVersion} : ouvrir tes mondes actuels avec risque de les
            abîmer (blocs ou objets perdus). Tandem les sauvegarde d'abord ; mieux vaut créer un nouveau monde.
          </Alert>
        </Show>
      </Show>
      <Show when={plan()?.leavesModpack}>
        <Alert tone="warning">Cette instance vient d'un modpack : elle ne suivra plus ses mises à jour.</Alert>
      </Show>
      <Checkbox
        class="text-[13px] text-chalk-2"
        checked={keepCopy()}
        disabled={busy()}
        label={`Garder une copie de l'instance en ${props.instance.gameVersion} (les mods ne prennent pas de place en plus)`}
        onChange={setKeepCopy}
      />
      <Show when={error()}>
        <Alert onClose={() => setError(null)}>{error()}</Alert>
      </Show>
      <div class="flex justify-end gap-2">
        <button class="btn btn-ghost" disabled={busy()} onClick={props.onClose}>
          Annuler
        </button>
        <button class="btn btn-primary px-corners px-5" disabled={busy() || !plan()} onClick={() => void apply()}>
          <Icon name="arrow" size={11} />
          {busy() ? "Changement…" : `Passer en ${version()}`}
        </button>
      </div>
    </Dialog>
  );
}
