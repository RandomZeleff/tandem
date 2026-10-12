import { createSignal, For } from "solid-js";
import Alert from "../components/Alert";
import { Icon, type IconName } from "../components/pixel";
import Select from "../components/Select";
import Tabs from "../components/Tabs";

/** Development page (`?fondations`): every primitive of the visual system at true size. */
const ICONS: IconName[] = ["home", "grid", "search", "duo", "gear", "play", "plus", "folder", "download", "invite", "heart", "trash"];

export default function Specimen() {
  const [tab, setTab] = createSignal("contenu");
  const [seg, setSeg] = createSignal("mods");
  const [version, setVersion] = createSignal("1.21.1");
  const [row, setRow] = createSignal(1);

  return (
    <div class="h-full overflow-y-auto bg-slate-800">
      <div class="mx-auto flex max-w-[1100px] flex-col gap-10 px-10 py-10">
        <section class="flex flex-col gap-4">
          <h2 class="panel-title">Échelle de texte</h2>
          <div class="panel flex flex-col gap-3 p-6">
            <p class="title-display text-5xl">Reprendre ma partie</p>
            <p class="title-display text-3xl">Instances</p>
            <p class="text-xl font-semibold">Titre de section (20, demi-gras)</p>
            <p class="text-lg font-semibold">Sous-titre (17, demi-gras)</p>
            <p class="text-sm text-chalk-2">
              Texte courant (14). Tandem installe le modpack, copie les mods chez ton ami et ouvre le monde sans serveur.
            </p>
            <p class="text-meta text-muted">Texte secondaire (13) · Fabric 1.20.1 · 678 mods · joué il y a 2 h</p>
            <p class="text-xs text-faint">Légende (12) · Modifié le 12 octobre</p>
          </div>
        </section>

        <section class="flex flex-col gap-4">
          <h2 class="panel-title">Boutons</h2>
          <div class="panel flex flex-col gap-5 p-6">
            <div class="flex flex-wrap items-center gap-3">
              <button class="btn btn-primary px-corners-md title-display h-[54px] px-8 text-[28px] uppercase">
                <Icon name="play" size={18} />
                Jouer
              </button>
              <button class="btn btn-primary">
                <Icon name="play" size={12} />
                Jouer
              </button>
              <button class="btn btn-light">
                <Icon name="invite" size={12} />
                Inviter des amis
              </button>
              <button class="btn">
                <Icon name="folder" size={12} />
                Ouvrir le dossier
              </button>
              <button class="btn btn-danger">Supprimer</button>
              <button class="btn btn-ghost">Annuler</button>
            </div>
            <div class="flex flex-wrap items-center gap-3">
              <button class="btn btn-primary" disabled>
                Jouer
              </button>
              <button class="btn btn-light" disabled>
                Inviter des amis
              </button>
              <button class="btn" disabled>
                Ouvrir le dossier
              </button>
              <button class="btn btn-microsoft">Se connecter avec Microsoft</button>
              <button class="btn h-8 px-3 text-meta">Petit bouton</button>
              <button class="btn btn-ghost size-9 px-0" aria-label="Plus">
                <Icon name="dots" size={12} />
              </button>
            </div>
          </div>
        </section>

        <section class="grid grid-cols-2 gap-4">
          <div class="flex flex-col gap-4">
            <h2 class="panel-title">Champs</h2>
            <div class="panel flex flex-col gap-3 p-6">
              <label class="flex flex-col gap-1.5">
                <span class="text-meta font-medium text-muted">Nom de l'instance</span>
                <input class="field" value="Prominence II" />
              </label>
              <label class="flex flex-col gap-1.5">
                <span class="text-meta font-medium text-muted">Code d'invitation</span>
                <input class="field" placeholder="Colle le code de ton ami" />
              </label>
              <div class="flex flex-col gap-1.5">
                <span class="text-meta font-medium text-muted">Version</span>
                <Select
                  value={version()}
                  onChange={setVersion}
                  label="Version"
                  options={[
                    { value: "1.21.1", label: "1.21.1", hint: "recommandée" },
                    { value: "1.20.1", label: "1.20.1" },
                    { value: "1.19.2", label: "1.19.2" },
                  ]}
                />
              </div>
            </div>
          </div>
          <div class="flex flex-col gap-4">
            <h2 class="panel-title">Onglets, puces, icônes</h2>
            <div class="panel flex flex-col gap-4 p-6">
              <Tabs
                label="Fiche"
                idPrefix="spec"
                value={tab()}
                onChange={setTab}
                class="border-b border-divider"
                items={[
                  { id: "console", label: "Console" },
                  { id: "contenu", label: "Contenu" },
                  { id: "mondes", label: "Mondes" },
                  { id: "captures", label: "Captures" },
                ]}
              />
              <Tabs
                label="Filtre"
                idPrefix="spec-seg"
                variant="segmented"
                value={seg()}
                onChange={setSeg}
                tabClass="h-7"
                items={[
                  { id: "mods", label: "Mods" },
                  { id: "packs", label: "Packs de ressources" },
                  { id: "shaders", label: "Shaders" },
                ]}
              />
              <div class="flex flex-wrap gap-2">
                <span class="chip">Fabric 1.20.1</span>
                <span class="chip">678 mods</span>
                <span class="chip text-xp-text">Bêta</span>
              </div>
              <div class="flex flex-wrap gap-2">
                <For each={ICONS}>
                  {(name) => (
                    <span class="slot size-9 text-chalk-2">
                      <Icon name={name} size={14} />
                    </span>
                  )}
                </For>
              </div>
            </div>
          </div>
        </section>

        <section class="flex flex-col gap-4">
          <h2 class="panel-title">Listes et messages</h2>
          <div class="panel flex flex-col p-2">
            <For each={["Sodium", "Create", "Jade"]}>
              {(name, i) => (
                <button
                  class="row flex h-14 items-center gap-3 px-3 text-left"
                  classList={{ "is-selected": row() === i() }}
                  onClick={() => setRow(i())}
                >
                  <span class="slot size-9">
                    <Icon name="grid" size={14} />
                  </span>
                  <span class="flex min-w-0 flex-1 flex-col">
                    <span class="text-sm font-semibold">{name}</span>
                    <span class="text-meta text-muted">Mis à jour il y a 3 jours</span>
                  </span>
                  <span class="chip">0.6.2</span>
                </button>
              )}
            </For>
          </div>
          <Alert tone="error">Le téléchargement a échoué : vérifie ta connexion puis réessaie.</Alert>
          <Alert tone="warning">Cette instance utilise 6 Go de mémoire, ton PC en a 8.</Alert>
          <Alert tone="success">Ton ami a rejoint la partie.</Alert>
          <div class="flex flex-col gap-2">
            <div class="skeleton px-corners-md h-14" />
            <div class="skeleton px-corners-md h-14" style={{ "animation-delay": "120ms" }} />
          </div>
        </section>
      </div>
    </div>
  );
}
