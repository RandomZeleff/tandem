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
- [x] Gestion Java : téléchargement auto des runtimes Mojang selon la version ; détection des JRE installés et choix par instance (Phase 4.7)
- [x] Construction de la ligne de commande (format 1.13+ et legacy `minecraftArguments`)
- [x] Lancement du process (sans console), capture stdout/stderr, détection crash + rapport
- [x] Comptes offline (UUID identique au serveur vanilla), un seul actif
- [x] UI : liste des instances, création (choix version), bouton Jouer/Arrêter, progression, console du jeu

## Phase 2 — Comptes

> En pause (2026-10-09) : attend la réponse de Mojang à la demande d'App ID (D9). On avance sur les autres phases en attendant ; pas de build distribuable avant (D31).

- [ ] OAuth Microsoft (auth code + loopback ou device code)
- [ ] Chaîne Xbox Live → XSTS → Minecraft Services → profil
- [ ] Refresh token automatique, stockage sécurisé (keyring Windows)
- [ ] Multi-comptes, switch en un clic, avatars (skins)
- [ ] Gestion des erreurs XSTS (compte enfant, pas de Xbox, pas de jeu possédé)

## Phase 3 — Loaders & contenu

- [x] Fabric / Quilt (API meta, simple)
- [x] NeoForge / Forge (exécution des installers / processors) — Forge 1.12.2 → actuel, NeoForge 1.20.2 → actuel
- [x] Client API Modrinth : recherche + installation mods, shaders, resource packs, modpacks, datapacks (par monde)
- [x] Résolution de dépendances + compatibilité version/loader : dépendances requises récursives, choix de la version compatible, détection des incompatibilités
- [x] Import/export `.mrpack` (+ installation de modpacks depuis Découvrir ; Fabric/Quilt uniquement)
- [ ] Import modpacks CurseForge (`manifest.json`, clé API, gestion des mods non distribuables)
- [x] Mises à jour de contenu par instance (+ activer/désactiver un contenu)
- [x] Changement de version d'une instance (avec vérif de compatibilité des mods) — Phase 4.7

## Phase 4 — Performance & UX

- [x] Presets JVM (RAM auto selon machine, GC flags) — D18 ; mémoire réglable par instance dans l’onglet Informations
- [x] Suggestion de mods de perf selon loader (Sodium, Lithium, FerriteCore, ModernFix…) — D19
- [x] Gestion des instances en cours : RAM/CPU (panneau au-dessus de la console, échantillon toutes les 2 s, 2 min d’historique), logs live, kill, plusieurs instances simultanées
- [x] Analyse de crash (lecture du crash report, mod coupable probable) — D21
- [x] Gestion des mondes (liste, sauvegardes, backup auto) — D22
- [x] Captures d'écran par instance — D23 ; onglet « Captures » (grille, visionneuse, suppression)
- [x] Démarrage du launcher < 1 s — D24 ; ≈ 240-300 ms jusqu’à la première image (release, Windows, à chaud), plus de flash blanc
- [x] UI fluide : audit des vues lourdes — D25 ; sortie du jeu par paquets (≤ 50 ms), console 12× moins coûteuse, listes de 250 mods sans virtualisation

## Phase 4.5 — Finitions (avant la v0.1)

Objectif : une version 0.1 propre et solide avant les grosses features (traduction IA, P2P).

- [x] Audit de l'UI écran par écran → [AUDIT-UI.md](AUDIT-UI.md) (13/20, 6 P1, 11 P2, 3 P3)
- [x] Composants de base — D26 : `Select`, `Checkbox`, `Alert`, `Tabs`, `Dialog` avec focus piégé/restauré, fermeture Échap + clic extérieur (`lib/ui.ts`)
- [x] Navigation avec historique — D27 : retour/avant, boutons 4/5 de la souris, Alt+←/→ (⌘[ / ⌘] sur Mac), flèches dans la barre de titre ; onglet, recherche, filtres et défilement retrouvés
- [x] Menus déroulants maison : les 5 `<select>` et la case « Snapshots » natifs remplacés
- [x] Corrections issues de l'audit : P1, P2 (D28) et P3 traités
- [~] Tests sous charge ([fiche](TEST-CHARGE.md), [résultats](TEST-CHARGE-RESULTATS.md)) : côté moteur fait (Prominence II, Create+, Cobblemon NeoForge) ; reste la partie interface à faire par un humain (gros Fabric, NeoForge 1.21.1, Forge 1.20.1 + FTB Quests) : installation, lancement, RAM/CPU du launcher, onglet Contenu, mises à jour, crash, export
- [~] Optimisations issues des tests sous charge : téléchargements ×4-6, arrêt du jeu avec ses processus enfants, console en UTF-8, cache des modpacks (D29) ; dépendances entre mods (D30), choix de la version d'un modpack
- [~] Version 0.1 (build installable Windows + Mac) — prête (workflow `release.yml` : tag `v*` → brouillon de Release, notes dans `docs/releases/v0.1.0.md`), **en attente de la réponse de Mojang** et de la Phase 2 (D31)
- [x] Discussion traduction : s'inspirer des mods existants (AutoTranslation-Next, AutoTranslator, autotranslator-cn) → décision avant la Phase 5 (D33)

## Phase 4.6 — Fiches de contenu

Objectif : savoir ce qu'est un mod, un pack de textures, un shader ou un modpack avant de l'installer. Une fiche complète pour tout projet Modrinth, faite pour durer : on y reviendra pour des détails, pas pour la refaire. Elle servira de premier terrain à la Phase 5 (traduire la description).

Moteur (Rust) :
- [x] Détails d'un projet : description longue, galerie, licence, liens (code source, bugs, wiki, Discord, dons), client/serveur, catégories, loaders, versions du jeu, dates, téléchargements, abonnés
- [x] Auteurs : membres de l'équipe ou de l'organisation (nom, avatar, rôle)
- [x] Versions : liste légère (sans fichiers ni journal) + journal des changements d'une version à la demande
- [x] Mods inclus dans une version de modpack (dépendances « embedded » de Modrinth), résolus en projets (titre, icône, type) par lots
- [x] Dépendances d'un mod (requises, optionnelles, incompatibles) pour la version qui irait dans l'instance choisie
- [x] Description Markdown/HTML → HTML sûr (`pulldown-cmark` + `ammonia`) : ni script ni style, vidéos YouTube en vignette cliquable, liens vers d'autres projets Modrinth reconnus pour s'ouvrir dans Tandem
- [x] Cache mémoire (~10 min) : rouvrir une fiche ou revenir en arrière ne refait pas de requête — D32

Interface :
- [x] Page « fiche » dans l'historique (retour/avance, onglet et défilement retrouvés)
- [x] En-tête : icône, titre, résumé, auteurs, chiffres, catégories, compatibilité, client/serveur, licence, liens externes
- [x] Action selon le contexte : installer dans l'instance choisie (en disant si une version compatible existe), « Installé », « Mettre à jour » ; modpack → choix de la version, ou « Ouvrir » l'instance qui en vient
- [x] Onglet Description (style Deepslate, images chargées au défilement, liens externes ouverts dans le navigateur)
- [x] Onglet Galerie : grille + visionneuse (← → Échap) avec titre et légende
- [x] Onglet Versions : filtres (version du jeu, loader, stable/bêta/alpha), journal dépliable, installer une version précise
- [x] Onglet Mods inclus (modpacks) : liste de la version choisie, recherche, chaque mod ouvre sa fiche
- [x] Onglet Dépendances (mods) : chaque dépendance ouvre sa fiche
- [x] Points d'entrée : Découvrir, onglet Contenu d'une instance, modpack d'origine (onglet Informations), dépendances, liens dans les descriptions
- [x] États : chargement (squelettes), hors ligne / erreur avec « Réessayer », projet introuvable
- [x] Backend simulé pour tester dans le navigateur ; tests Rust (nettoyage du HTML, lecture des réponses)

Hors périmètre : CurseForge (avec son import, Phase 3), traduction de la description (Phase 5), fiches des mods ajoutés à la main hors Modrinth.

## Phase 5 — Traduction IA de modpacks

Décisions : D33.

- [x] Extraction : `assets/*/lang/en_us.json` (+ ancien format `.lang`) des mods activés, des packs de ressources activés et de KubeJS, avec la priorité du jeu (le dernier pack gagne)
- [x] Extraction : FTB Quests (fichiers `lang/` récents, ou texte dans les chapitres traduit sur place avec originaux gardés), Patchouli (livres dans les jars et livres en vrac)
- [x] Textes déjà traduits par les auteurs gardés ; copies de l'anglais dans les autres langues (phrases) retraduites
- [x] Protection des codes de format (`§x`, `&x`, `%s`, `%1$d`, `{0}`, `$(…)`, sauts de ligne) avant envoi au LLM, restauration + validation après
- [x] Traduction par lots avec contexte (nom du mod, clé de traduction comme indice) et glossaire : noms officiels de Minecraft, termes déjà traduits par les auteurs, termes du joueur
- [x] Sortie en resource pack généré (non destructif, activable/désactivable) + `options.txt` (pack sous ceux du joueur, langue du jeu, retour à l'ancienne langue)
- [x] Cache local par texte anglais + langue (SQLite), corrections du joueur jamais écrasées
- [x] Services compatibles OpenAI (Ollama, LM Studio, OpenAI, Anthropic, Mistral, Gemini, OpenRouter, DeepSeek, autre), clé dans le coffre du système
- [x] UI : onglet Traduction (couverture, estimation tokens/temps, progression, arrêt, choix des sources, relecture et correction, glossaire), réglages, traduction des descriptions de fiches
- [x] Mise à jour automatique de la traduction active après un changement de contenu
- [ ] (plus tard) Cache communautaire partagé

## Phase 4.7 — Instances et contenu, au complet

Objectif : tout ce qu'un joueur solo attend d'un launcher avant le jeu à deux.

- [x] Messages d'erreur en français partout (les erreurs Rust sont aujourd'hui en anglais)
- [x] Mise à jour d'un modpack vers une nouvelle version (mods remplacés ; mondes, réglages, captures et fichiers modifiés par le joueur gardés)
- [x] Renommer, changer l'icône, dupliquer une instance
- [x] Réglages avancés par instance : arguments JVM, Java (automatique / installé / chemin), taille de la fenêtre du jeu
- [x] Détection des Java installés
- [x] Changement de version d'une instance (jeu et/ou loader) avec vérification des mods
- [x] Datapacks par monde (Modrinth)
- [x] Détection des incompatibilités entre mods (déclarées dans les jars et sur Modrinth)
- [~] Import CurseForge (`.zip`) avec la clé du joueur ; mods non distribuables retrouvés sur Modrinth par empreinte — moteur fait (`content::curseforge`, testé sans clé), reste : commande Tauri, champ de clé dans Réglages, import unifié .mrpack/.zip, panneau « fichiers à télécharger à la main »
- [ ] Import d'instances d'autres launchers (Modrinth App, Prism / MultiMC, CurseForge, launcher officiel)
- [ ] Passe finale : audit de l'UI, mesures de performance (démarrage, mémoire, gros packs), hors ligne

## Phase 6 — Multijoueur P2P sans serveur

- [ ] Détection d'un monde ouvert en LAN (écoute multicast `224.0.2.60:4445`)
- [ ] Tunnel P2P QUIC via `iroh` (hole punching + relais de secours)
- [ ] Codes d'invitation courts (`CRAFT-7K2P`)
- [ ] Côté invité : faux serveur local `127.0.0.1:<port>` + annonce LAN
- [ ] Vérif que les deux ont la même instance (version/loader/mods) → proposer de synchroniser
- [ ] Indicateurs : ping, type de connexion (direct/relais)
- [ ] (plus tard) Synchro du monde / « host migration »

## Portage macOS

- [x] Compiler et lancer l’app sur Mac (jamais testé : code écrit multi-plateforme, chemins `~/Library/Application Support/Tandem`)
- [x] Apple Silicon : versions sans natives LWJGL `natives-macos-arm64` (≤ 1.18.2) lancées en mode Intel via Rosetta (runtime `mac-os` dans `java/<composant>-x86_64`, règles de libs évaluées en `osx`/`x86_64`) ; 1.19+ restent natives
- [x] JNA < 5.13 plante au démarrage sur macOS récent (assertion `snprintf` dans `dispatch.c` quand oshi charge IOKit) → 1.17 à 1.20.2 reçoivent JNA 5.13.0 sur Mac
- [x] UI : si Rosetta manque (`RosettaMissing`), fenêtre qui propose de l’installer (`softwareupdate --install-rosetta` derrière l’invite administrateur de macOS), puis relance l’instance
- [x] Fenêtre : `titleBarStyle: Overlay` + boutons natifs sur Mac, masquer nos boutons de fenêtre façon Windows
- [x] CI : job macOS (`.dmg` universel x86_64 + arm64, non signé) + lint/tests aussi sur macOS
- [ ] Signature + notarisation Apple (compte Apple Developer requis)
- [ ] Phase 2 : tokens dans le Trousseau macOS (crate `keyring`), en plus du gestionnaire Windows (D9)

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

### 2026-10-09
- Fait : audit de compatibilité macOS (voir section « Portage macOS ») ; tout est poussé sur `origin/master` pour reprendre le dev sur Mac.
- Prochaine étape : sur le Mac, `pnpm install` puis `pnpm tauri dev`, corriger ce qui casse, puis le mode Rosetta pour les vieilles versions sur Apple Silicon.
- Sur le Mac (Apple Silicon) : build et tests OK après un correctif (`safe_relative` laissait passer `C:/…` hors Windows) ; barre de titre native (feux tricolores, logo masqué). Vanilla 1.21.4 atteint le menu en natif arm64 ; 1.16.5 échoue comme prévu (`JavaUnavailable { jre-legacy, mac-os-arm64 }`).
- Mode Rosetta + correctif JNA : vérifié jusqu’au menu (exemple `play`, hors ligne) pour 1.12.2, 1.16.5, 1.17.1, 1.18.2 (Rosetta), 1.19.2, 1.20.1, 1.20.4, 1.21.4 (natif) et Forge 47.4.10 (1.20.1). L’exemple `play` affiche maintenant aussi le stderr du jeu.
- Installation de Rosetta depuis l’UI : parcours vérifié avec le backend simulé (`pnpm dev` dans le navigateur, l’instance 1.12.2 du mock réclame Rosetta) ; le vrai `osascript` n’a pas pu être testé, Rosetta étant déjà installé sur ce Mac.
- CI : lint/tests aussi sur macOS, job « Build macOS bundle » (`.dmg` universel non signé, artefact `tandem-macos`), tout vert.
- Phase 4 : mémoire automatique + flags G1 (D18), sélecteur de mémoire dans l’onglet Informations ; flags vérifiés sur Java 8 (1.16.5, Rosetta), Java 17 (Forge 1.20.1) et Java 21 (1.21.4).
- Suggestion de mods de perf (D19) : vérifiée sur Modrinth pour Fabric 1.21.4, Forge 1.20.1 (Embeddium, pas de Lithium), NeoForge 1.21.1 et Fabric 1.16.5 ; les 7 mods suggérés installés ensemble sur Fabric 1.21.4 atteignent le menu. Panneau testé avec le backend simulé.
- Suivi RAM/CPU : `tandem_core::stats` (sysinfo), event `game://stats` toutes les 2 s, panneau mémoire (pic) + processeur (moyenne) avec barres sur 2 min. Mesuré sur de vrais jeux (exemple `play`) : 1.21.4 natif ≈ 840 Mo / 4 % CPU au menu ; 1.16.5 sous Rosetta ≈ 1,1 Go / 75-85 % CPU pendant le chargement.
- CPU sous Rosetta, creusé : ce n’est pas le menu (≈ 5 % une fois au menu, les FPS y sont plafonnés à 60) mais le chargement, ~4× plus lent car Rosetta doit traduire en continu le code généré par le JIT. « Setting user » → « Sound engine started » : 1.16.5 / 1.17.1 / 1.18.2 sous Rosetta ≈ 20-23 s, contre 3-6 s en natif (1.19.2, 1.20.1, 1.21.4) ; 1.12.2 reste rapide (3 s).
- Piste pour aller plus loin : Mojang fournit Java 17 arm64 (`java-runtime-gamma`) mais ni 8 ni 16 → 1.17 à 1.18.2 pourraient tourner en natif avec Java 17 arm64 et LWJGL 3.3.x arm64 à la place de 3.2.x (approche de Prism) ; ≤ 1.16 demanderait un Java 8 arm64 tiers (Azul Zulu).
- 1.18.x en natif sur Apple Silicon (D20) : vanilla 1.18.2 ≈ 4 s de chargement, Fabric 1.18.2 ≈ 5 s, Forge 40.x (1.18.2) ≈ 7 s, contre ≈ 19 s sous Rosetta. 1.17.1 essayé en natif mais l’icône de fenêtre fait planter LWJGL 3.3 → laissé sous Rosetta.
- Analyse de crash (D21) : `tandem_core::crash`, panneau dans la page de l’instance (cause, exception, suspects, « Désactiver », « Voir le rapport »). Vrai crash testé : Sodium 1.20.1 sur Fabric 1.21.4 → Sodium désigné « incompatible », bon fichier retrouvé. Formats Forge/NeoForge, Mixin et pile d’appels couverts par des tests unitaires ; panneau testé avec le backend simulé.
- Mondes (D22) : onglet « Mondes », sauvegardes zip manuelles et automatiques (après chaque partie, 5 gardées), restauration réversible. Lecture de `level.dat`, sauvegarde, restauration et rotation couvertes par des tests sur des mondes générés ; onglet testé avec le backend simulé. Pas encore vérifié sur un vrai monde créé en jeu.
- Prochaine étape : captures d’écran par instance (dernier point de la Phase 4 hors « démarrage < 1 s »).

### 2026-10-09 (Windows)
- Fait : pull des 14 commits du Mac ; Rust mis à jour 1.93 → 1.99 (`sysinfo` 0.39 demande 1.95) ; fmt, clippy et tests verts sur Windows, l’app tourne.
- Captures d’écran (D23) : `tandem_core::screenshots` (liste, miniatures JPEG 480 px en cache, suppression), protocole `tandem-shot://` pour les images, onglet « Captures » avec grille paresseuse, visionneuse (← → Échap Suppr), « Afficher dans le dossier », rafraîchissement en direct sur « Saved screenshot as ». Tests unitaires sur des images générées ; onglet testé avec le backend simulé.
- Pas encore vérifié : le protocole dans la vraie app (prendre une capture F2 en jeu, l’ouvrir dans l’onglet).
- Démarrage (D24) : temps loggés (`backend ready`, `UI ready`, en ms depuis le lancement). Mesuré en release : ≈ 240-300 ms jusqu’à la première image, dont ≈ 150-170 ms de création fenêtre + WebView2 par Tauri avant `setup`, ≈ 5-8 ms pour notre init (dossiers, SQLite, HTTP), ≈ 100 ms de chargement de l’UI (JS ≈ 140 Ko). La fenêtre restait blanche jusqu’au premier rendu → `backgroundColor` #111316, vérifié en échantillonnant la couleur de la fenêtre toutes les ~30 ms.
- Attention en mesurant : un `tauri dev` ouvert recompile à chaque modif et fausse les temps (jusqu’à 4-5 s).
- Pas mesuré : démarrage à froid (premier lancement après redémarrage de Windows), ni sur le Mac.
- UI fluide (D25) : mesuré dans le navigateur (backend simulé). Console pleine (5000 lignes) : 1,5 ms par ligne avec un événement par ligne → 1000 lignes = 1,5 s de gel, un gros modpack Forge (~20 000 lignes) ≈ 30 s. Le backend envoie maintenant la sortie par paquets (≤ 50 ms ou 500 lignes) : 0,12 ms par ligne, gel max ≈ 33 ms. Logs du launcher regroupés côté UI (≤ 50 ms). Bug corrigé : la console ne défilait plus toute seule une fois pleine (longueur constante). Onglet Contenu à 250 mods (nouveau jeu de données du mock « Pack Create ») : 77 ms au premier affichage, 17 ms pour activer/désactiver un mod.
- Pas encore vérifié dans la vraie app : lancer un modpack Forge et regarder la console défiler (le panneau navigateur était masqué, donc mesures de calcul et de mise en page, pas d’images par seconde).
- Phase 4 terminée. Prochaine étape : Phase 5 (traduction IA de modpacks), en commençant par l’extraction des `lang/en_us.json`.
- Finitions (D26) : anneau de focus jaune retiré à la demande ; le focus clavier reprend l'apparence du survol. Composants `Select` (liste dans un portail, flèches, Début/Fin, recherche par frappe, Échap qui ne ferme que la liste), `Checkbox`, `Alert` (refermable, remplace 6 bandeaux copiés), `Tabs` (un seul arrêt Tab, flèches, `tabpanel` relié), focus piégé dans les dialogues et la visionneuse (pile pour les dialogues imbriqués), menu du compte fermé par Échap/clic extérieur, cartes d'instance ouvrables au clavier, erreurs de « Ouvrir le dossier » affichées, état « rien trouvé » dans Découvrir. Vérifié dans le navigateur (backend simulé) : clavier, Échap, boucle Tab, rendu des menus.
- Historique (D27) : pile en mémoire (50 pages) dans `lib/store.ts`, `remembered()` pour l'état d'une page (onglet d'instance, type/recherche/instance de Découvrir, filtre des instances), défilement sauvé par page, pages d'instances supprimées sautées. Vérifié dans le navigateur (backend simulé, qui gère maintenant la suppression d'instance) : Alt+←, bouton 4 de la souris, boutons de la barre, avance, avance effacée par une nouvelle navigation, défilement restauré, instance supprimée sautée.
- Pas vérifié : les boutons de souris et ⌘[ dans la vraie fenêtre Tauri (WebView2 / WKWebView).
- P2 de l'audit (D28) : notifications (`lib/toast.ts`, `Toaster`), retrait d'un mod ou d'un compte annulable ~6 s, squelettes de chargement dans Contenu / Mondes / Captures, vraie page Réglages (comportement au lancement du jeu, sauvegarde auto des mondes, dossier des données avec bouton « Ouvrir » via la nouvelle commande `open_data_folder`), nombres au format français (`2,6 Mo`), couleurs d'interface en tokens (`--color-danger`, `--color-xp-deep`…), accueil à 2 cartes sous 1280 px. Vérifié dans le navigateur (backend simulé) : retrait → Annuler → retrait confirmé, squelettes, réglage enregistré, accueil à 1100 px sans débordement.
- Pas vérifié : la réduction/réouverture du launcher au lancement du jeu dans la vraie fenêtre Tauri.
- P3 de l'audit : raccourcis Ctrl/⌘+K ou F (chercher sur Modrinth), Ctrl/⌘+N (nouvelle instance), Ctrl/⌘+, (réglages), rappelés dans les infobulles ; `Index` au lieu de `For` pour la barre d'XP et les graphes RAM/CPU (segments réutilisés, vérifié sur un faux lancement). Mouvement réduit : animations des notifications et des squelettes coupées.
- Tests sous charge (côté moteur, par Claude) : Prominence II (678 mods), Create+ (Forge 1.19.2, 257 mods), Cobblemon NeoForge 1.21.1. Menu en 38-80 s selon le pack, ~4 Go au menu. Trouvé et corrigé (D29) : téléchargements bridés par la fenêtre HTTP/2 (6,7 → 26-36 Mo/s), assistants de modpack (Crash Assistant) qui survivaient à l'arrêt du jeu et bloquaient la fin de partie (Job Object Windows / groupe de processus Unix + lecture de sortie bornée à 2 s), sortie de Java en cp1252 (forcée en UTF-8), `.mrpack` de 400+ Mo gardé en cache. Détails : `docs/TEST-CHARGE-RESULTATS.md`.
- Pas vérifié : le groupe de processus sous macOS (la CI compile la variante Unix) ; la partie interface de la fiche de test.
- Dépendances entre mods (D30) : `content::deps` lit ce que chaque jar fournit et exige (Fabric, Quilt, Forge, NeoForge, jars imbriqués), en parallèle et avec un cache par jar (`cache/deps/<instance>.json`, préchauffé à l'ouverture de l'onglet Contenu). Désactiver ou retirer une bibliothèque nécessaire demande confirmation en listant les mods concernés. Panneau « il manque des mods » quand Forge/NeoForge refuse de démarrer, avec « Réactiver et relancer » ou « Chercher sur Modrinth ». Vérifié sur les vrais packs : Create+ sans Architectury → les 5 mods cités par Forge (scan à froid 1 s, depuis le cache 8 ms ; Prominence 2,6 s puis 11 ms) ; l'analyseur retrouve le rapport du vrai log Forge ; parcours UI testés dans le navigateur (backend simulé).
- Choix de la version d'un modpack : « Installer » dans Découvrir ouvre une boîte qui montre la version recommandée (dernière stable, comme avant) avec son loader, sa version du jeu et sa taille, et permet d'en choisir une autre (bêta/alpha avec avertissement). Commande `modpack_versions`, `install_modpack` prend un `versionId` optionnel. Testé dans le navigateur (backend simulé).
- Prochaine étape : partie interface des tests sous charge (Raphaël), puis build installable de la v0.1.
- Publication v0.1 préparée : métadonnées de l'installeur, workflow `release.yml` (tag `v*` → installeurs Windows + `.dmg` → brouillon de Release marqué pré-version, notes `docs/releases/v<version>.md`, vérifie que le tag correspond à la version de l'app), notes de la v0.1.0, README à jour. Aucun tag poussé.
- Décision (D31) : pas de build ni de Release tant que Mojang n'a pas répondu à la demande d'App ID et que la connexion Microsoft (Phase 2) n'est pas faite. Jobs de build de la CI désactivés (lint et tests gardés). Section « Comptes » des notes de la v0.1.0 à écrire à ce moment-là.
- Anciens installeurs produits par la CI supprimés (41 artefacts). Phase 2 mise en pause en attendant Mojang : on continue le développement sur les autres phases.
- Prochaine étape : choisir avec Raphaël entre la Phase 5 (traduction IA : discussion de conception d'abord, pistes réunies) et la Phase 6 (jeu à deux sans serveur, le différenciateur).

### 2026-10-09 (Windows, suite)
- Choix avec Raphaël : peaufiner l'existant avant la traduction, en finissant un chantier complet. Cadrage de la Phase 4.6 (fiches de contenu) puis réalisation.
- Moteur (D32) : `content::project` (détails, auteurs équipe + organisation via l'API v3, versions marquées compatibles avec l'instance ou lançables par Tandem pour un modpack, journaux, dépendances et contenu d'un modpack résolus par lots de 100 en parallèle, cache 10 min dans `AppState`), `content::markdown` (pulldown-cmark + ammonia, vidéos YouTube en vignettes, liens relatifs vers modrinth.com). `install_content` accepte une version précise. Exemple `project <slug> [description.html]`. Mesuré sur Prominence II : page 0,45 s, 102 versions 0,9-1,5 s (réponse de 5 Mo), 563 mods inclus 1,1 s (4,6 s avant la parallélisation), 3 ms depuis le cache.
- Interface : page `project` dans l'historique (chargée à la demande : +36 Ko hors du démarrage), en-tête avec action selon le contexte, onglets Description / Galerie / Versions / Contenu ou Dépendances, colonne Compatibilité / Créateurs / Liens / Détails, catégories traduites (aussi dans Découvrir). Points d'entrée : cartes de Découvrir, titres de l'onglet Contenu, modpack d'origine (Informations), dépendances, liens Modrinth dans les descriptions.
- Vérifié dans le navigateur (backend simulé) : tous les onglets, visionneuse, liens internes, installation, mise à jour, instance Vanilla, aucune version compatible, projet inexistant, retour arrière, largeur 1100 px ; vraie description de Prominence II rendue dans l'aperçu. Vu dans la vraie fenêtre (`tauri dev`) : fiche de Prominence II avec le vrai backend.
- Pas vérifié dans la vraie fenêtre : ouverture des liens externes (plugin opener), installation depuis la fiche, galerie.
- Prochaine étape : discussion de conception de la traduction (Phase 5), avec la description d'une fiche comme premier usage.

### 2026-10-10 (Windows)
- Mandat de Raphaël : tout faire jusqu'au multijoueur exclu, en autonomie, sans publier de build. Feuille de route ajoutée au plan (Phase 4.7).
- Phase 5 terminée (D33) : moteur `translate` (sources, masquage des codes, glossaire vanilla + termes des auteurs + joueur, lots par mod avec clé comme indice, cache SQLite par texte, pack généré + quêtes FTB en place avec originaux, manifeste réversible, langue du jeu restaurée), services compatibles OpenAI, clé dans le coffre (`keyring`), onglet Traduction, réglages, traduction des descriptions. Testé avec Ollama qwen2.5:7b sur RTX 4060 (~45 tokens/s) : 400 textes de Cobblemon traduits et chargés en jeu (pack reconnu compatible).
- Phase 4.7 : erreurs en français ; mise à jour de modpack avec retour arrière (testé sur Fabulously Optimized 6.3.0 → 6.5.0 → retour → 6.5.0, jeu lancé) ; onglet Réglages d'instance (nom, icône, duplication, Java détectés, arguments JVM, fenêtre) ; changement de version d'instance (testé FO 1.21.1 → 1.21.4 : 44 mods mis à jour, 2 désactivés ; le jeu plante ensuite à cause de la config de Controlify, d'où l'option « garder une copie ») ; datapacks par monde ; incompatibilités entre mods (aucune fausse alerte sur 3 gros packs) ; moteur d'import CurseForge.
- Pas vérifié dans la vraie fenêtre Tauri : la plupart des écrans ajoutés (testés dans l'aperçu navigateur avec le backend simulé) ; liens externes (opener).
- Prochaine étape : finir l'import CurseForge (commande, clé dans Réglages, import unifié, panneau téléchargements manuels), puis import d'instances d'autres launchers, puis passe finale (audit UI, perfs, hors ligne). Ne pas commencer le multijoueur.

