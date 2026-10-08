# Plan de développement

Légende : `[ ]` à faire · `[~]` en cours · `[x]` fait

---

## Phase 0 — Fondations

- [x] Choisir le nom définitif du projet : **Tandem**
- [x] Valider le framework frontend : SolidJS + Tailwind v4
- [x] Scaffold Tauri 2 (`pnpm create tauri-app`, template solid-ts)
- [x] Workspace Cargo : `crates/tandem-core` (sans Tauri) + `src-tauri` (app)
- [x] Base SQLite (`sqlx`, WAL) + migrations : instances, comptes, réglages
- [x] Dossier de données (`%APPDATA%/Tandem`, surchargeable via `TANDEM_DATA_DIR`) : `store/`, `assets/`, `instances/`, `java/`, `cache/`, `logs/`
- [x] Logging (`tracing`) : fichiers journaliers (7 conservés) + panneau de logs live côté UI
- [x] CI GitHub Actions : fmt, clippy, tests, build Windows (artefacts MSI/NSIS)
- [~] **Demande d'accès API Minecraft** : app Azure créée → formulaire https://aka.ms/mce-reviewappid (voir DECISIONS D7)

## Phase 1 — Lancer du vanilla

- [x] Client piston-meta : `version_manifest_v2.json` → JSON de version (cache hors ligne)
- [x] Téléchargement : client.jar, libraries (règles OS/arch), natives, assets (index + objets, layouts legacy `virtual`/`resources`)
- [x] Moteur de téléchargement : parallèle borné (16), retry avec backoff, reprise `.part`, vérif SHA1, progression → UI
- [x] ~~Store content-addressed + hardlinks~~ → fait en Phase 3 pour le contenu des instances (voir D10)
- [~] Gestion Java : téléchargement auto des runtimes Mojang selon la version ✅ ; détection des JRE déjà installés à faire (override `java_path` par instance déjà en base, sans UI)
- [x] Construction de la ligne de commande (format 1.13+ et legacy `minecraftArguments`)
- [x] Lancement du process (sans console), capture stdout/stderr, détection crash + rapport
- [x] Comptes offline (UUID identique au serveur vanilla), un seul actif
- [x] UI : liste des instances, création (choix version), bouton Jouer/Arrêter, progression, console du jeu

## Phase 2 — Comptes

- [ ] OAuth Microsoft (auth code + loopback ou device code)
- [ ] Chaîne Xbox Live → XSTS → Minecraft Services → profil
- [ ] Refresh token automatique, stockage sécurisé (keyring Windows)
- [ ] Multi-comptes, switch en un clic, avatars (skins)
- [ ] Gestion des erreurs XSTS (compte enfant, pas de Xbox, pas de jeu possédé)

## Phase 3 — Loaders & contenu

- [x] Fabric / Quilt (API meta, simple)
- [x] NeoForge / Forge (exécution des installers / processors) — Forge 1.12.2 → actuel, NeoForge 1.20.2 → actuel
- [~] Client API Modrinth : recherche + installation mods, shaders, resource packs, modpacks ✅ ; datapacks (par monde) à faire
- [~] Résolution de dépendances + compatibilité version/loader : dépendances requises récursives + choix de la version compatible ✅ ; détection des incompatibilités à faire
- [x] Import/export `.mrpack` (+ installation de modpacks depuis Découvrir ; Fabric/Quilt uniquement)
- [ ] Import modpacks CurseForge (`manifest.json`, clé API, gestion des mods non distribuables)
- [x] Mises à jour de contenu par instance (+ activer/désactiver un contenu)
- [ ] Changement de version d'une instance (avec vérif de compatibilité des mods)

## Phase 4 — Performance & UX

- [ ] Presets JVM (RAM auto selon machine, GC flags)
- [ ] Suggestion de mods de perf selon loader (Sodium, Lithium, FerriteCore, ModernFix…)
- [ ] Gestion des instances en cours : RAM/CPU, logs live, kill, plusieurs instances simultanées
- [ ] Analyse de crash (lecture du crash report, mod coupable probable)
- [ ] Gestion des mondes (liste, sauvegardes, backup auto)
- [ ] Captures d'écran par instance
- [ ] Démarrage du launcher < 1 s, UI fluide

## Phase 5 — Traduction IA de modpacks

- [ ] Extraction : `assets/*/lang/en_us.json` (+ ancien format `.lang`)
- [ ] Extraction : FTB Quests (`.snbt`), Patchouli, KubeJS
- [ ] Protection des codes de format (`§x`, `%s`, `%1$d`, `{0}`) avant envoi au LLM, restauration + validation après
- [ ] Traduction par lots avec contexte (nom du mod, glossaire du modpack)
- [ ] Sortie en resource pack généré (non destructif, activable/désactivable)
- [ ] Cache local indexé par hash de mod + langue
- [ ] (plus tard) Cache communautaire partagé

## Phase 6 — Multijoueur P2P sans serveur

- [ ] Détection d'un monde ouvert en LAN (écoute multicast `224.0.2.60:4445`)
- [ ] Tunnel P2P QUIC via `iroh` (hole punching + relais de secours)
- [ ] Codes d'invitation courts (`CRAFT-7K2P`)
- [ ] Côté invité : faux serveur local `127.0.0.1:<port>` + annonce LAN
- [ ] Vérif que les deux ont la même instance (version/loader/mods) → proposer de synchroniser
- [ ] Indicateurs : ping, type de connexion (direct/relais)
- [ ] (plus tard) Synchro du monde / « host migration »

---

## Journal des sessions

### 2026-10-08
- Fait : création du dossier, git init, plan initial, CLAUDE.md, docs d'archi et de décisions.
- Prochaine étape : Phase 0 (nom, valider le frontend, scaffold Tauri).
- Blocages : aucun.

### 2026-10-08 (2)
- Fait : scaffold Tauri 2 + SolidJS + Tailwind v4, workspace Cargo (`tandem-core` + `src-tauri`), commande `core_version` reliée à l'UI. fmt/clippy/tests OK.
- Prochaine étape : finir Phase 0 (SQLite, dossier de données, logging, CI) puis Phase 1.
- En attente : nom du projet ; validation Mojang du Client ID Azure.

### 2026-10-08 (3)
- Fait : nom choisi (Tandem), vérifs de disponibilité, renommage complet, Client ID Azure enregistré.
- Prochaine étape : Phase 0 (SQLite, dossier de données, logging, CI).
- En attente : formulaire Mojang (envoyé par l'utilisateur), renommage du dossier `mc-launcher` → `tandem`.

### 2026-10-08 (4)
- Fait : fin de Phase 0 — `tandem-core` : `paths` (DataDir), `db` (SQLite + migration 0001 + settings JSON), `logging` (fichiers + buffer UI), `error`. App : `AppState`, commandes `app_info`, `get_logs`, `get_setting`, `set_setting`, event `log://entry`. UI : header + panneau de logs filtrable. CI Windows. Démarrage vérifié (`%APPDATA%/Tandem` créé, log « Tandem started »).
- Prochaine étape : Phase 1 — client piston-meta (manifest + JSON de version).
- En attente : réponse Mojang au formulaire App ID.

### 2026-10-08 (5)
- Fait : Phase 1 — modules `meta`, `download`, `java`, `install`, `launch`, `account`, `instance` dans `tandem-core` ; `game.rs` côté app (préparation, streaming de la sortie, arrêt, crash) ; UI instances + comptes offline + console jeu. Exemple CLI `cargo run -p tandem-core --example play -- <version>`.
- Vérifié de bout en bout (exemple CLI) : 1.12.2 (Java 8, natives legacy) et 26.3 (Java 25) téléchargent et atteignent le menu.
- Reste : test de l'UI par l'utilisateur ; détection des JRE installés.
- Prochaine étape : Phase 2 (comptes Microsoft) si Mojang a validé, sinon Phase 3 (Fabric + Modrinth).

### 2026-10-08 (6)
- Fait : direction artistique v2 (canevas de design : écrans + planche « Fondations ») puis intégration dans l'app : thème Tailwind (couleurs Deepslate/Herbe/XP/Or/Améthyste/Redstone), polices embarquées (Pixelify Sans, Geist, Geist Mono), coins en escalier, boutons biseautés, cases d'inventaire, barre d'XP, icônes pixel, scènes en bandes. Fenêtre sans bordure avec barre de titre maison. Pages : Accueil, Instances, détail d'instance (console, infos, suppression), Réglages (journal), Découvrir / Jouer à deux en « bientôt ».
- Aperçu navigateur : `pnpm dev` puis http://localhost:1420 simule le backend (`src/dev/mock.ts`, jamais inclus dans l'app).
- Prochaine étape : reprendre les features (Phase 2 si Mojang a validé, sinon Phase 3).

### 2026-10-08 (7)
- Fait : début de Phase 3 — Fabric et Quilt. `meta::loader` (liste des versions via meta.fabricmc.net / meta.quiltmc.org, tri semver car Quilt renvoie un ordre aléatoire, profil mis en cache dans `versions/<id>/`), fusion du profil dans le JSON vanilla (`VersionJson::apply_loader`), libraries Maven (`url` + sha1/size optionnels), `install::Target`, version du loader figée à la création. UI : choix Vanilla/Fabric/Quilt + version du loader dans « Nouvelle instance », version du loader affichée dans l'onglet Informations.
- Vérifié de bout en bout (exemple CLI `play -- fabric@1.21.4` / `quilt@1.21.4`) : Fabric 0.19.5 et Quilt 0.30.1 atteignent le menu. Dialogue testé dans l'aperçu navigateur.
- Prochaine étape : client API Modrinth (recherche + installation de mods dans une instance), puis store par hash (D4/D10).
- Blocages : Phase 2 toujours en attente de la validation Mojang.

### 2026-10-08 (8)
- Fait : contenu Modrinth. Core : `content::modrinth` (recherche avec facettes version/loader — Quilt accepte aussi les mods Fabric —, projets, versions), `content::install` (version compatible la plus récente, préférence release, dépendances requises récursives, plafond 64), `content::remove`, table `instance_content` (migration 0002), `store` (fichiers par SHA-1 + hardlink, repli copie). Commandes `search_content`, `list_content`, `install_content`, `remove_content`. UI : page Découvrir (instance, Mods / Packs de textures / Shaders, recherche, « Voir plus », aide si instance Vanilla ou si Iris manque pour les shaders), onglet Contenu de l'instance (par type, badge dépendance, retrait). Icônes de loaders en pixel art (Vanilla, Fabric, Quilt, Forge, NeoForge) partout où le loader apparaît. `.gitattributes` : migrations SQL figées en LF (checksums sqlx).
- Vérifié : exemple CLI `mods -- fabric@1.21.4 sodium modmenu iris complementary-reimagined faithful-32x` (Fabric API + Placeholder API tirées comme dépendances, Sodium non dupliqué pour Iris), puis `play -- instance:dev-fabric-1-21-4` : le jeu charge Sodium, Iris, Mod Menu. UI testée dans l'aperçu navigateur (mock).
- Note : l'instance de test « Dev fabric 1.21.4 » existe dans la base réelle (supprimable depuis l'UI).
- Prochaine étape : mises à jour de contenu par instance, import/export `.mrpack` (modpacks), activer/désactiver un mod.
- Blocages : Phase 2 toujours en attente de la validation Mojang.

### 2026-10-08 (9)
- Fait : mises à jour et activation du contenu. Core : `check_updates` (une requête groupée `POST /version_files/update` par type de contenu, releases d’abord puis repli bêta, jamais de retour à une version plus ancienne grâce à `date_published`), `update` (réutilise le plan d’installation : nouvelles dépendances requises installées, ancien fichier remplacé, choix activé/dépendance conservés), `set_enabled` (renommage `x.jar` ↔ `x.jar.disabled`), `install` accepte une version épinglée. Migration 0003 (`enabled`), `installed_at` conservé lors d’une mise à jour. UI : onglet Contenu avec vérification auto à l’ouverture, bandeau « N mises à jour disponibles » + « Tout mettre à jour », mise à jour par ligne (ancienne → nouvelle version), interrupteur pixel par élément, éléments désactivés grisés. Tout est bloqué pendant que le jeu tourne.
- Vérifié : exemple CLI `mods` dans un dossier de données temporaire (Sodium 0.6.9 et Mod Menu 13.0.2 épinglés → 2 mises à jour détectées puis appliquées, anciens fichiers supprimés, Lithium à jour non touché) ; tests unitaires du renommage `.disabled` ; UI testée dans l’aperçu navigateur (mock).
- Prochaine étape : import/export `.mrpack` (modpacks Modrinth).
- Blocages : Phase 2 toujours en attente de la validation Mojang.

### 2026-10-08 (10)
- Fait : modpacks Modrinth. Core `content::mrpack` : lecture de l’index, installation (fichiers via le store, filtrage `env.client`, liste blanche de domaines de la spec, chemins vérifiés contre le « zip slip », `overrides/` puis `client-overrides/`), reconnaissance des fichiers Modrinth par SHA-1 (`POST /version_files` + `GET /projects`) pour que mises à jour / interrupteurs marchent sur un pack, export (contenu Modrinth activé référencé par URL, reste en `overrides/` sauf mondes, logs, caches, fichiers `.disabled`). Migration 0004 : origine du pack sur l’instance (projet, version) + icône. App : `modpack.rs` crée l’instance d’abord (progression visible), la supprime si l’installation échoue ; événements `instances://changed` et `install://finished`. Plugin `dialog` pour choisir / enregistrer un `.mrpack`. UI : onglet Modpacks dans Découvrir (Installer → nouvelle instance, « Ouvrir » si déjà installé), « Importer un .mrpack » (Découvrir + Instances), « Exporter » dans l’onglet Contenu, ligne Modpack dans Informations, icône du pack à la place du bloc.
- Vérifié : exemple CLI `modpack -- fabulously-optimized` (51 fichiers, tous reconnus comme contenu), export puis réimport de l’export, lancement : 148 mods chargés, menu atteint. Tests : chemins dangereux, domaines, overrides, cibles. UI testée dans l’aperçu navigateur (mock).
- Prochaine étape : mises à jour de modpack (nouvelle version du pack), puis NeoForge/Forge (beaucoup de modpacks en dépendent).
- Blocages : Phase 2 toujours en attente de la validation Mojang.

### 2026-10-08 (11)
- Fait : Forge et NeoForge. Core `meta::forge` : listes de versions (NeoForge via l’API Maven avec décodage `21.1.x` → 1.21.1 et `26.3.0.x` → 26.3 ; Forge via `maven-metadata.xml` + `promotions_slim.json` pour la version recommandée), téléchargement de l’installeur (SHA-1 du Maven), lecture de `version.json` / `install_profile.json`, extraction du dossier `maven/` de l’installeur, exécution des processors côté client (variables `{…}` / `[coords]` / fichiers `/data/…`, `Main-Class` lu dans le manifest, vérification des SHA-1 de sortie, marqueur `.processed` pour ne les lancer qu’une fois). Profil fusionné comme Fabric ; `minecraftArguments` des vieux profils (Forge 1.12.2) pris en compte. Étape de progression `processing`. Modpacks Forge/NeoForge autorisés (import, export, recherche). UI : Forge et NeoForge dans « Nouvelle instance », version recommandée présélectionnée.
- Vérifié de bout en bout (exemple CLI `play`) : NeoForge 21.1.256 (1.21.1), Forge 47.4.10 (1.20.1), Forge 36.2.34 (1.16.5, Java 8), Forge 14.23.5.2859 (1.12.2, launchwrapper) atteignent le menu ; relancement sans processors (< 1 s de préparation). Modpack « Cobblemon Official Modpack [NeoForge] » installé depuis Modrinth et lancé jusqu’à l’écran titre.
- Prochaine étape : mises à jour de modpack (nouvelle version du pack), puis Phase 4 (presets JVM, RAM auto).
- Blocages : Phase 2 toujours en attente de la validation Mojang.
