# Plan de développement

Légende : `[ ]` à faire · `[~]` en cours · `[x]` fait

---

## Phase 0 — Fondations

- [ ] Choisir le nom définitif du projet
- [x] Valider le framework frontend : SolidJS + Tailwind v4
- [x] Scaffold Tauri 2 (`pnpm create tauri-app`, template solid-ts)
- [x] Workspace Cargo : `crates/launcher-core` (sans Tauri) + `src-tauri` (app)
- [ ] Base SQLite (`sqlx`) + migrations : instances, comptes, réglages
- [ ] Dossier de données (`%APPDATA%/<nom>`) : `store/`, `instances/`, `java/`, `cache/`
- [ ] Logging (`tracing`) + panneau de logs côté UI
- [ ] CI GitHub Actions : fmt, clippy, tests, build Windows
- [~] **Demande d'accès API Minecraft** : app Azure créée → formulaire https://aka.ms/mce-reviewappid (voir DECISIONS D7)

## Phase 1 — Lancer du vanilla

- [ ] Client piston-meta : `version_manifest_v2.json` → JSON de version
- [ ] Téléchargement : client.jar, libraries (règles OS/arch), natives, assets (index + objets)
- [ ] Moteur de téléchargement : parallèle borné, retry, reprise, vérif SHA1, progression → UI
- [ ] Store content-addressed (fichiers rangés par hash) + hardlinks vers les instances
- [ ] Gestion Java : détection des JRE installés + téléchargement auto (runtimes Mojang) selon la version
- [ ] Construction de la ligne de commande (arguments JVM/jeu, classpath, placeholders)
- [ ] Lancement du process, capture stdout/stderr, détection crash
- [ ] Compte offline (dev) pour tester sans auth
- [ ] UI : liste des instances, création (choix version), bouton Jouer, barre de progression

## Phase 2 — Comptes

- [ ] OAuth Microsoft (auth code + loopback ou device code)
- [ ] Chaîne Xbox Live → XSTS → Minecraft Services → profil
- [ ] Refresh token automatique, stockage sécurisé (keyring Windows)
- [ ] Multi-comptes, switch en un clic, avatars (skins)
- [ ] Gestion des erreurs XSTS (compte enfant, pas de Xbox, pas de jeu possédé)

## Phase 3 — Loaders & contenu

- [ ] Fabric / Quilt (API meta, simple)
- [ ] NeoForge / Forge (exécution des installers / processors)
- [ ] Client API Modrinth : recherche mods, modpacks, shaders, resource packs, datapacks
- [ ] Résolution de dépendances + compatibilité version/loader
- [ ] Import/export `.mrpack`
- [ ] Import modpacks CurseForge (`manifest.json`, clé API, gestion des mods non distribuables)
- [ ] Mises à jour de contenu par instance
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
- Fait : scaffold Tauri 2 + SolidJS + Tailwind v4, workspace Cargo (`launcher-core` + `src-tauri`), commande `core_version` reliée à l'UI. fmt/clippy/tests OK.
- Prochaine étape : finir Phase 0 (SQLite, dossier de données, logging, CI) puis Phase 1.
- En attente : nom du projet ; validation Mojang du Client ID Azure.
