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

> App ID approuvé par Mojang le 2026-10-10 ; connexion vérifiée par Zeleff avec son compte (jeu lancé). Décision D36.

- [x] OAuth Microsoft (auth code + PKCE, retour sur `http://localhost:<port>`)
- [x] Chaîne Xbox Live → XSTS → Minecraft Services → profil (+ vérification de possession)
- [x] Refresh token automatique, stockage sécurisé (coffre du système, jetons découpés)
- [x] Multi-comptes, switch en un clic, avatars (tête du vrai skin)
- [x] Gestion des erreurs XSTS (compte enfant, pas de Xbox, pas de jeu possédé)
- [x] Fenêtre « Comptes » (connexion pas à pas, hors ligne après possession)

## Phase 3 — Loaders & contenu

- [x] Fabric / Quilt (API meta, simple)
- [x] NeoForge / Forge (exécution des installers / processors) — Forge 1.12.2 → actuel, NeoForge 1.20.2 → actuel
- [x] Client API Modrinth : recherche + installation mods, shaders, resource packs, modpacks, datapacks (par monde)
- [x] Résolution de dépendances + compatibilité version/loader : dépendances requises récursives, choix de la version compatible, détection des incompatibilités
- [x] Import/export `.mrpack` (+ installation de modpacks depuis Découvrir ; Fabric/Quilt uniquement)
- [x] ~~Import modpacks CurseForge (`manifest.json`, clé API, gestion des mods non distribuables) — Phase 4.7, D34~~ retiré le 2026-10-11 (D41 : pas de CurseForge)
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
- [x] ~~Import CurseForge (`.zip`) avec la clé du joueur ; mods non distribuables retrouvés sur Modrinth par empreinte, sinon téléchargés à la main et repris dans Téléchargements — D34~~ retiré le 2026-10-11 (D41)
- [x] Import d'instances d'autres launchers (Modrinth App, Prism / MultiMC, CurseForge, launcher officiel, ou dossier choisi) — D35
- [x] Passe finale : audit de l'UI, mesures de performance (démarrage, mémoire, gros packs), hors ligne — [AUDIT-UI.md](AUDIT-UI.md#passe-finale--2026-10-10-fin-de-la-phase-47)

## Phase 4.8 — Soutenir les créateurs (idée de Zeleff, faite le 2026-10-11)
Tandem n'affiche pas de publicité (celle qui paie les auteurs sur Modrinth) : le mettre en valeur auprès des créateurs. Modrinth donne les liens de soutien de chaque projet (`donation_urls` : Ko-fi, Patreon, PayPal, GitHub Sponsors, Buy Me a Coffee, autre) ; aujourd'hui ils sont discrets, une ligne « Don » dans la colonne Liens de la fiche.
- [x] Bouton « Soutenir l'auteur » bien visible dans la fiche, à côté d'« Installer » ; seulement si l'auteur a mis un lien (plusieurs liens : menu)
- [x] Onglet « Créateurs » dans chaque instance : auteurs regroupés par organisation ou propriétaire d'équipe, avec leurs liens (trois requêtes groupées vers Modrinth)
- [x] Fiche d'un modpack : l'auteur du pack (bouton « Soutenir »), onglet « Créateurs » pour les mods qu'il contient
- [x] Petit rappel unique après 10 h de jeu sur une instance (temps de jeu maintenant compté), fermable pour toujours
- Principes : jamais insistant ni bloquant ; toujours le lien direct de l'auteur, Tandem ne prend rien ; ton chaleureux, pas culpabilisant ; repli « Voir sur Modrinth » quand l'auteur n'a pas de lien. Les liens sont par projet, pas par auteur : regrouper par équipe ou organisation Modrinth.

## Phase 6 — Multijoueur P2P sans serveur

Cadrage (2026-10-10, avec Zeleff, D37) : on livre d'abord **le tunnel seul** (6a), puis la synchronisation des mods (6b). L'hôte ouvre son monde au réseau local dans le jeu, comme d'habitude ; Tandem le transporte chez l'invité, dont le jeu le voit comme une partie du réseau local. Aucun mod, toutes versions et loaders.

Choix tranchés :
- Le code suffit pour entrer ; l'hôte est prévenu de chaque arrivée et peut exclure un invité.
- Mise en relation et relais de secours : serveurs publics et gratuits de n0 (auteurs d'iroh). Pas de serveur Tandem. Un relais à nous reste possible plus tard.
- Les deux joueurs ont besoin d'un compte Microsoft (le monde LAN vérifie les comptes auprès de Mojang) : expliqué dans l'interface, aucun contournement.
- Quand l'hôte quitte, la partie s'arrête pour tous. Pas de transfert du monde en 6a.
- Tests réels : deux PC sur deux réseaux, deux comptes (Zeleff et un ami). Claude teste seul en simulation et sur un vrai serveur Minecraft local.

### 6a — Le tunnel
- [x] Détection d'un monde ouvert en LAN (écoute multicast `224.0.2.60:4445`)
- [x] Codes d'invitation courts (`7K2P-QX9M`), publiés sur le relais pkarr public d'iroh, sans serveur à nous
- [x] Session hôte : publie le code, accepte les invités, relaie chaque connexion du jeu vers le monde LAN
- [x] Session invité : résout le code, se connecte, faux serveur local + annonce LAN
- [x] Rejoindre en un clic : l'invité choisit une instance, Tandem lance le jeu directement sur la partie
- [x] Indicateurs : ping, liaison directe ou par relais ; arrivées et départs ; exclure un invité
- [x] Avertissement si les instances diffèrent (version, loader, mods)
- [x] Page « Jouer à deux » (inviter / rejoindre), messages clairs (compte Microsoft requis, monde pas encore ouvert, code inconnu)

- [x] **Test réel à deux** (deux PC, deux comptes Microsoft) : Zeleff + un ami, invité en 4G (≈ 30 ms) et en Wi-Fi (≈ 3 ms), exclusion et inversion des rôles OK — [TEST-DUO.md](TEST-DUO.md#résultats-du-premier-test-réel-10-octobre-2026)

### 6b — Ensuite
- [x] Synchroniser l'invité sur l'hôte (D39) : « Préparer l'instance » crée une copie (modpack Modrinth de l'hôte s'il en a un), puis mêmes mods (Modrinth par SHA-1, sinon envoyés par l'hôte) et mêmes fichiers de pack (configs, scripts, datapacks globaux)
- [x] Moins de clics (D38) : l'invité rejoint sans choisir d'instance (Tandem prend celle qui correspond) et le jeu se lance dès que le monde est ouvert ; l'hôte invite et arrive directement dans son monde en un clic (1.20+)
- [x] Étape LAN mieux guidée : Tandem voit que l'hôte est dans son monde, notification de rappel par-dessus le jeu ; arrivées et départs notifiés
- [ ] Test réel des nouveautés à deux PC (notifications Windows, choix automatique de l'instance, lancement automatique, préparation de la copie)
- [ ] (plus tard) Synchro du monde / « host migration »
- [ ] (plus tard) Relais Tandem en option

### 6c — Optimiser « Jouer ensemble » : feuille de route (recadrée le 2026-10-11, D44)
Après une exploration volontairement poussée (6c, 6d, 6e et 6f ouverts en parallèle le même jour), on arrête de chercher de nouveaux angles et de repousser le nombre de joueurs. On termine ce qui est commencé, **un chantier à la fois, dans l'ordre**, chacun testé en vrai et documenté avant de passer au suivant.

**Ce qu'on sait** (détails et chiffres : [TEST-DUO.md](TEST-DUO.md#optimisation--où-part-le-temps-du-serveur-11-octobre-2026)) :
- Le temps du serveur part surtout dans la génération du terrain neuf, puis l'IA des créatures. Une fois le gaspillage retiré, le reste est du vrai travail.
- Gagné : apparitions inutiles sautées (Cobbleverse 47 → 28 ms à 18 joueurs), distance d'activation (Prominence jusqu'à −30 %, avant sa restriction aux créatures vanilla).
- Le monde à part coûte +1,5 Go (mesuré sur Cobbleverse) et ne rend pas le tick plus rapide : son intérêt est le confort (plus d'étape LAN, partie qui survit au plantage du jeu de l'hôte) et la limite de 8 joueurs levée.
- Plafond réaliste sur un PC : 20 à 30 joueurs sur un pack moyen, 12 à 15 sur un pack lourd. Au-delà, il faudrait répartir le monde entre plusieurs PC, ce que les mods ne supportent pas.

**Déjà fait** (code poussé, vérifié en local) :
- Tandem Optimizations v1 (D42) : mod ajouté au lancement (Fabric, Quilt, 1.20 à 1.21.11), réglages globaux et par instance, vérification de construction contre 20 versions, démarré sur 4 vrais serveurs.
- Monde à part, base (D43) : installation du serveur, mods client écartés, nouvel essai sans les mods qui plantent, inventaire de l'hôte dans les deux sens, branchement dans « Jouer ensemble » (bêta, désactivé par défaut), testé avec le vrai jeu sur Cobbleverse et Prominence II.
- Outils de mesure et de test (notes locales).

**Chantiers, dans l'ordre :**

1. **Terminer Tandem Optimizations v1** ✅ (12 octobre 2026)
   - [x] Tests en vrai, en solo et dans « Jouer ensemble », sur Cobbleverse, Prominence II et Fabulously Optimized : aucun plantage, rien de changé en jeu (apparitions, Pokémon, villageois, fermes simples) — [TEST-DUO.md](TEST-DUO.md#validation-en-vrai-12-octobre-2026)
   - [x] Une seule mesure de Prominence avec la version « créatures vanilla seulement », pour documenter le gain réel (≈ −25 % de temps de tick)
   - [x] Une instance Quilt réelle lancée avec le mod (a révélé un bug d'installation Quilt, corrigé)
   - Terminé quand : les trois packs tournent sans souci avec le mod actif par défaut, et le gain réel est écrit dans TEST-DUO.

2. **Terminer le monde à part (6e)** ✅ (12 octobre 2026)
   - [x] Test du scénario : l'ami rejoint, l'hôte ferme son jeu sans couper l'ami, l'hôte revient. Simulé sur un PC avec le vrai tunnel (relais publics, liaison directe), accord de Zeleff ; un test à deux PC reste bienvenu quand un ami est disponible ([TEST-DUO](TEST-DUO.md#monde-à-part--scénario-complet-12-octobre-2026))
   - [x] Moins de mémoire pour le jeu de l'hôte quand le monde est à part, mesuré une fois (1 Go de moins, D45)
   - [x] Une instance Quilt réelle hébergée à part
   - [x] Décider avec Zeleff : reste en bêta facultative, ou proposé par défaut au-delà de quelques amis → **reste en bêta** (12 octobre 2026)
   - [x] (trouvé en route) Le serveur et le jeu de l'hôte écrivaient les mêmes fichiers de config : dossiers de l'instance recopiés au lieu d'être liés (D45)
   - Terminé quand : le test à deux PC est concluant (décision prise : bêta).

3. **Pré-générer le monde en un clic** (D40, priorité 3) ✅ (12 octobre 2026, D46)
   - [x] « Préparer le monde pour jouer à plusieurs » : génération autour du point d'apparition avant de jouer (le plus gros coût quand tout le monde explore), progression et annulation
   - Terminé quand : le coût du terrain neuf disparaît sur la zone préparée, mesuré une fois. Mesure existante faite avec le même outil (Chunky) : à 6 joueurs, tick 37 → 18 ms et CPU 82 → 32 % ([TEST-DUO](TEST-DUO.md#comparaison--monde-déjà-généré-11-octobre-2026)).

4. **Indicateur de charge pour l'hôte** (D40, priorité 4) ✅ (12 octobre 2026, D47)
   - [x] Mémoire libre et santé du monde visibles pendant une session, avec un conseil simple quand ça sature
   - Terminé quand : l'hôte voit d'un coup d'œil si son PC suit.

5. ~~**Le monde appartient au groupe (6f)**~~ : abandonné (D48). Un groupe laisse naturellement héberger celui qui a le meilleur PC ; copier le monde chez les amis coûterait beaucoup (risque de monde abîmé, connexion, disque) pour un besoin rare.

**Arrêté ou mis de côté** (D44) :
- Repousser le nombre de joueurs au-delà d'un PC : monde réparti entre plusieurs PC, un PC par dimension, génération du terrain par les amis, simulation partagée.
- Nouvelles recherches d'optimisation du tick (cache de la recherche d'apparition : écarté, gain de 1 à 2 %), cache de chunks chez les amis, mesure du débit par joueur.
- Version NeoForge et 26.x du mod et du monde à part : après cette feuille de route, si le besoin existe.
- Distance de simulation adaptée, réglages propres à certains mods.

## Phase 7 — Refonte de l'interface (cadrée le 2026-10-12)

Cadrage complet : [REFONTE.md](REFONTE.md). Direction « le Minecraft moderne » (menus récents de Mojang), sombre, animations sobres. Une étape finie avant la suivante.

- [x] 1. Fondations (couleurs, texte, espacements, boutons, champs, panneaux, onglets, listes, icônes, états) et choix de la police des titres (D49)
- [x] 2. Cadre de l'app (barre de titre, navigation, compte)
- [x] 3. Accueil (reprendre sa partie en un clic, inviter, rejoindre, grand visuel)
- [x] 4. Jouer ensemble
- [x] 5. Instances et fiche d'instance (tous les onglets)
- [x] 6. Découvrir et fiches de projet
- [x] 7. Réglages, Comptes, fenêtres, notifications, états vides et d'erreur
- [x] 8. Finition : revue complète, `DESIGN.md`, décision qui remplace D11

Ensuite : retour aux petits ajouts.

## Phase 8 — Un launcher pour le solo (cadrée le 2026-10-14)

Objectif : qu'un joueur aguerri ait envie d'ouvrir Tandem souvent, pour jouer seul, sans se prendre la tête. Le jeu à deux et la traduction restent des bonus, pas la vitrine. Un chantier fini avant le suivant.

- [x] 1. Découvrir : tri (téléchargements, abonnés, nouveautés, mis à jour), filtres par catégorie, version du jeu pour les modpacks, rayons « Nouveautés », « Mis à jour récemment », « Les plus suivis » quand on ne cherche rien (D51). Pas de « tendances » (Modrinth n'en donne pas).
- [x] 2. « Continuer » en un clic, directement dans le dernier monde (1.20+), avec choix d'un autre monde ou de l'écran titre ; « Jouer » sur chaque monde de l'onglet Mondes
- [x] 3. Carnet de partie par monde : jour, temps de jeu, distance, créatures, blocs, morts ; grandes étapes vanilla datées ; quêtes FTB par chapitre (D52)
- [x] 4. Fiches de modpack « est-ce pour moi ? » : mémoire face au PC, nombre de mods et taille, quêtes, thèmes dominants ; packs « dans le même esprit » (D53)

## Phase 9 — Prêt à distribuer (cadrée le 2026-10-20)

- [x] Mises à jour automatiques de Tandem (D55) : vérification au démarrage, pastille, installation en un clic, `latest.json` signé publié par `release.yml`
- [x] Une seule fenêtre Tandem à la fois (D55)
- [x] Ajouter le secret `TAURI_SIGNING_PRIVATE_KEY` sur GitHub et sauvegarder la clé privée (Zeleff, 2026-10-20)
- [ ] Tester une vraie mise à jour de bout en bout (0.1.0 installée → release suivante publiée)
- [ ] Signature de l'installeur Windows (SmartScreen) : SignPath Foundation, à demander juste après la première release publique (signature faite dans `release.yml`)
- [x] Premier lancement guidé (D57) : compte, import des autres launchers, première partie
- [x] Skins et capes : page Apparence (D56)
- [x] ~~Packs de skins d'événement de Mojang~~ : interdits par la licence ; remplacés par le Pack Tandem, six skins originaux (D58)

## Phase 10 — Amis (cadrée le 2026-10-20, D59)

- [x] Moteur : clé durable, code d'ami publié sur le relais, liste en base, connexions directes entre amis, statut, demandes, invitations (exemple `friends selftest` sur les vrais relais)
- [x] Service démarré avec l'app, statut calculé tout seul, réglages « Apparaître hors ligne » et « Mes amis peuvent rejoindre sans invitation »
- [x] Panneau « Amis » dans Jouer ensemble, invitations reçues (notification par-dessus le jeu, badge, Rejoindre en un clic)
- [ ] Test réel à deux PC (Zeleff + un ami, nouvel installeur des deux côtés)
- [ ] (idée) Amis en ligne dans la barre latérale ; ajouter un ami après une partie jouée ensemble

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
- Prochaine étape : partie interface des tests sous charge (Zeleff), puis build installable de la v0.1.
- Publication v0.1 préparée : métadonnées de l'installeur, workflow `release.yml` (tag `v*` → installeurs Windows + `.dmg` → brouillon de Release marqué pré-version, notes `docs/releases/v<version>.md`, vérifie que le tag correspond à la version de l'app), notes de la v0.1.0, README à jour. Aucun tag poussé.
- Décision (D31) : pas de build ni de Release tant que Mojang n'a pas répondu à la demande d'App ID et que la connexion Microsoft (Phase 2) n'est pas faite. Jobs de build de la CI désactivés (lint et tests gardés). Section « Comptes » des notes de la v0.1.0 à écrire à ce moment-là.
- Anciens installeurs produits par la CI supprimés (41 artefacts). Phase 2 mise en pause en attendant Mojang : on continue le développement sur les autres phases.
- Prochaine étape : choisir avec Zeleff entre la Phase 5 (traduction IA : discussion de conception d'abord, pistes réunies) et la Phase 6 (jeu à deux sans serveur, le différenciateur).

### 2026-10-09 (Windows, suite)
- Choix avec Zeleff : peaufiner l'existant avant la traduction, en finissant un chantier complet. Cadrage de la Phase 4.6 (fiches de contenu) puis réalisation.
- Moteur (D32) : `content::project` (détails, auteurs équipe + organisation via l'API v3, versions marquées compatibles avec l'instance ou lançables par Tandem pour un modpack, journaux, dépendances et contenu d'un modpack résolus par lots de 100 en parallèle, cache 10 min dans `AppState`), `content::markdown` (pulldown-cmark + ammonia, vidéos YouTube en vignettes, liens relatifs vers modrinth.com). `install_content` accepte une version précise. Exemple `project <slug> [description.html]`. Mesuré sur Prominence II : page 0,45 s, 102 versions 0,9-1,5 s (réponse de 5 Mo), 563 mods inclus 1,1 s (4,6 s avant la parallélisation), 3 ms depuis le cache.
- Interface : page `project` dans l'historique (chargée à la demande : +36 Ko hors du démarrage), en-tête avec action selon le contexte, onglets Description / Galerie / Versions / Contenu ou Dépendances, colonne Compatibilité / Créateurs / Liens / Détails, catégories traduites (aussi dans Découvrir). Points d'entrée : cartes de Découvrir, titres de l'onglet Contenu, modpack d'origine (Informations), dépendances, liens Modrinth dans les descriptions.
- Vérifié dans le navigateur (backend simulé) : tous les onglets, visionneuse, liens internes, installation, mise à jour, instance Vanilla, aucune version compatible, projet inexistant, retour arrière, largeur 1100 px ; vraie description de Prominence II rendue dans l'aperçu. Vu dans la vraie fenêtre (`tauri dev`) : fiche de Prominence II avec le vrai backend.
- Pas vérifié dans la vraie fenêtre : ouverture des liens externes (plugin opener), installation depuis la fiche, galerie.
- Prochaine étape : discussion de conception de la traduction (Phase 5), avec la description d'une fiche comme premier usage.

### 2026-10-10 (Windows)
- Mandat de Zeleff : tout faire jusqu'au multijoueur exclu, en autonomie, sans publier de build. Feuille de route ajoutée au plan (Phase 4.7).
- Phase 5 terminée (D33) : moteur `translate` (sources, masquage des codes, glossaire vanilla + termes des auteurs + joueur, lots par mod avec clé comme indice, cache SQLite par texte, pack généré + quêtes FTB en place avec originaux, manifeste réversible, langue du jeu restaurée), services compatibles OpenAI, clé dans le coffre (`keyring`), onglet Traduction, réglages, traduction des descriptions. Testé avec Ollama qwen2.5:7b sur RTX 4060 (~45 tokens/s) : 400 textes de Cobblemon traduits et chargés en jeu (pack reconnu compatible).
- Phase 4.7 : erreurs en français ; mise à jour de modpack avec retour arrière (testé sur Fabulously Optimized 6.3.0 → 6.5.0 → retour → 6.5.0, jeu lancé) ; onglet Réglages d'instance (nom, icône, duplication, Java détectés, arguments JVM, fenêtre) ; changement de version d'instance (testé FO 1.21.1 → 1.21.4 : 44 mods mis à jour, 2 désactivés ; le jeu plante ensuite à cause de la config de Controlify, d'où l'option « garder une copie ») ; datapacks par monde ; incompatibilités entre mods (aucune fausse alerte sur 3 gros packs) ; moteur d'import CurseForge.
- Pas vérifié dans la vraie fenêtre Tauri : la plupart des écrans ajoutés (testés dans l'aperçu navigateur avec le backend simulé) ; liens externes (opener).
- Prochaine étape : finir l'import CurseForge (commande, clé dans Réglages, import unifié, panneau téléchargements manuels), puis import d'instances d'autres launchers, puis passe finale (audit UI, perfs, hors ligne). Ne pas commencer le multijoueur.

### 2026-10-10 (Windows, suite)
- Import CurseForge fini (D34) : un seul bouton « Importer un modpack » (.mrpack ou .zip reconnu à son contenu), clé du joueur vérifiée puis rangée dans le coffre (dialogue au premier import, ou Réglages), panneau « fichiers à télécharger à la main » qui reprend tout seul les fichiers du dossier Téléchargements. Pas de vraie clé CurseForge pour tester un import réel : parcours testé avec le backend simulé, refus d'une clé invalide vérifié sur l'API.
- Import depuis d'autres launchers (D35) : Modrinth App, Prism / PolyMC, app CurseForge, launcher officiel, ou dossier choisi. Testé sur les vraies instances du PC : DawnCraft (CurseForge, Forge 1.18.2, 1,1 Go en 16 s, jeu jusqu'au menu) et Prominence II (Modrinth App, 2,5 Go en 16 s, modpack d'origine 4.1.2 repris).
- Mojang a approuvé l'App ID : la Phase 2 est débloquée.
- Passe finale de la Phase 4.7 : voir [AUDIT-UI.md](AUDIT-UI.md#passe-finale--2026-10-10-fin-de-la-phase-47). Corrigé : fichiers non suivis invisibles dans Contenu (154 mods cachés sur DawnCraft), filtre du contenu, attente de 30 s au lancement sur un réseau sans Internet (→ 5,7 s), messages d'erreur réseau, mise en page à 1100 px.
- Phase 4.7 terminée.
- Prochaine étape : Phase 2, connexion Microsoft (OAuth, Xbox Live → XSTS → Minecraft, jetons dans le coffre, plusieurs comptes).
- Phase 2 (connexion Microsoft, D36) : navigateur + PKCE avec retour sur `localhost`, Xbox Live → XSTS → Minecraft, vérification de possession, jetons dans le coffre (découpés en morceaux, Windows limite un secret à 2,5 Ko), renouvellement avant le lancement, tête du vrai skin, fenêtre « Comptes » et carte de la barre latérale refaites, comptes hors ligne seulement après un compte Microsoft. **Testé par Zeleff avec son vrai compte : connexion OK, jeu lancé.**
- Notes de la v0.1 complétées (section Comptes, traduction, imports). Rien de publié (D31 : la Phase 2 était la condition, la publication attend le feu vert de Zeleff).
- Prochaine étape : au choix de Zeleff, publier la v0.1 (tag `v0.1.0` → brouillon de Release) ou commencer la Phase 6 (jeu à deux sans serveur).
- Phase 6 commencée puis **mise en pause à la demande de Zeleff pour un cadrage dédié** (ampleur de la feature). Fait : preuve de concept iroh 1.3 (code court publié sur le relais pkarr public d'iroh, résolu en 0,2 s, connexion en 0,7 s, chemin direct choisi), briques du moteur `duo` compilées et testées mais pas encore branchées : codes d'invitation (`XXXX-XXXX`, 40 bits, Crockford), détection et annonce des mondes LAN (224.0.2.60:4445), protocole hôte/invité, comparaison des instances. Pas encore écrit : sessions hôte et invité, interface, rejoindre automatiquement au lancement.
- Prochaine étape : discussion de cadrage de la Phase 6 avec Zeleff avant de continuer.
- Phase 6a (tunnel) faite : sessions hôte et invité (`duo::host`, `duo::guest`), page « Jouer à deux » (inviter / rejoindre, code `XXXX-XXXX`, étapes de l'hôte cochées en direct, invités avec ping et liaison directe ou relais, exclure, écart d'instances signalé, compte Microsoft exigé), point doré dans la barre latérale pendant une session, et lancement qui rejoint directement la partie (`--quickPlayMultiplayer` en 1.20+, `--server`/`--port` avant).
- Vérifié sur ce PC : exemple `duo selftest` (vrai multicast LAN, code résolu sur le relais pkarr, ping de la liste des serveurs à travers le tunnel en 0,15 ms, refus d'un code inconnu, fin de session signalée) ; vrais jeux lancés sur un faux monde : Fabulously Optimized 1.21.4 (quickPlay) et Create Forge 1.19.2 (`--server`) se connectent tout seuls. Page testée dans l'aperçu (backend simulé).
- Pas vérifié : une vraie partie entre deux PC sur deux réseaux (hole punching réel, relais, connexion authentifiée au monde LAN).
- Prochaine étape : test réel par Zeleff et un ami, puis 6b (synchroniser les mods).


### 2026-10-10 (reprise Codex : livraison pour test duo)
- À la demande explicite de Zeleff : compilation locale des installateurs Windows x64 EXE et MSI 0.1.0 pour partage privé avec un ami.
- `docs/TEST-DUO.md` complété : prérequis, grille des scénarios, diagnostic et fiche de retour.
- Vérifié : TypeScript, fmt, Clippy, 134 tests Rust, `duo selftest` via le réseau et démarrage de la release sur des données isolées (UI 490 ms).
- Emballage final réussi via `pnpm tauri bundle --bundles nsis,msi`, après un verrou temporaire de l'exécutable lors du premier essai. Avertissement Tauri sur le marqueur de type de bundle déjà patché ; aucun plugin updater dans ce projet.
- Livrables copiés dans le dossier outputs du chat Codex du 10 octobre 2026 (`he`) avec archive ZIP et SHA-256. Aucun tag ni publication GitHub.
- Prochaine étape : vraie partie entre les deux PC ; la case du test réel de la Phase 6a reste ouverte.

### 2026-10-10 (Windows, suite : test réel du jeu à deux)
- Premier vrai test par Zeleff et un ami : inviter, rejoindre et jouer marchent, l'invité en 4G (≈ 30 ms, autre réseau) comme en Wi-Fi (≈ 3 ms) ; exclusion et inversion des rôles OK ; longue partie fluide. Phase 6a validée.
- Second essai : liaison affichée « direct » ; avertissement des mods manquants vérifié (jaune avec une instance différente, rien avec la bonne).
- Prochaine étape : 6b (synchroniser les mods) et peaufinage de l'expérience (accessibilité, moins de frictions), au choix de Zeleff.

### 2026-10-10 (Windows, suite : moins de frictions pour jouer à deux)
- Retours de Zeleff sur le test : gênes = avoir le même modpack, trop de clics, ouvrir le monde en LAN. Ordre choisi : d'abord les améliorations rapides, puis la synchro des mods (6b).
- Fait (D38) : l'invité entre seulement le code (instance choisie par Tandem : même modpack, puis mêmes mods ; modifiable), jeu lancé tout seul quand le monde s'ouvre et que tout correspond ; l'hôte a un bouton « Inviter et lancer le jeu » avec choix du monde, ouvert directement (Quick Play) ; étape LAN détaillée et « tu es dans ton monde » repéré dans le journal ; notifications système (rappel LAN, arrivées, départs, fin de partie) quand le launcher est en arrière-plan. Protocole compatible avec les installeurs déjà donnés.
- Vérifié : vrai jeu NeoForge 1.21.1 (Cobblemon) ouvert directement dans « Tutorial World v4 » (en jeu en 61 s, ligne de journal repérée) ; parcours hôte et invité dans l'aperçu (backend simulé) : lancement automatique, aucune instance correspondante, changement d'instance (pas de lancement automatique si les mods diffèrent) ; app Tauri démarrée avec le plugin de notification ; fmt, Clippy, 135 tests.
- Pas vérifié : les notifications dans la vraie fenêtre et par-dessus le jeu, le choix automatique sur deux vrais PC.
- Prochaine étape : 6b, synchroniser les mods de l'invité (le champ `pack` permet déjà d'installer le même modpack Modrinth chez l'invité).

### 2026-10-10 (Windows, suite : 6b, synchroniser l'invité)
- Choix de Zeleff : la 6b avant l'agent Java d'ouverture LAN automatique (gardé pour une session dédiée, voir les notes).
- Fait (D39) : bouton « Préparer l'instance » chez l'invité quand rien ne correspond. Copie = modpack Modrinth de l'hôte (même version) s'il en a un, sinon instance vide de même version et loader ; puis mods identiques (Modrinth par SHA-1, sinon envoyés par l'hôte, 4 à la fois), mods en trop désactivés, et fichiers de pack (`config`, `defaultconfigs`, `kubejs`, `scripts`, `global_packs`, `datapacks`) recopiés quand ils diffèrent. Copie réutilisée et mise à jour pour le même hôte. Progression et fichiers non récupérés affichés.
- Vérifié à travers une vraie session iroh (exemple `duo copytest`) : Fabulously Optimized 49 mods en 4,6 s ; DawnCraft 299 mods (143 envoyés par l'hôte) + 3 809 fichiers de pack, identiques, en 37 s. Corrigé en route : fichiers identiques reçus en parallèle sous le même nom temporaire. Vraie copie de DawnCraft (`duo copyinto`) lancée jusqu'au menu en 42 s, puis supprimée. Parcours dans l'aperçu (backend simulé). fmt, Clippy, 136 tests, `duo selftest`.
- Pas vérifié : la commande dans la vraie fenêtre entre deux PC (création depuis un modpack Modrinth, puis connexion au monde de l'hôte).
- Prochaine étape : test réel à deux PC (nouvel installeur des deux côtés), puis la session « agent Java » pour ouvrir le monde en LAN automatiquement.

### 2026-10-11 (Windows : jouer à plusieurs, tests de charge)
- Page renommée « Jouer ensemble » à la demande de Zeleff, textes de l'hôte au pluriel.
- Tests de charge (résultats dans [TEST-DUO.md](TEST-DUO.md)) : serveur Cobbleverse (136 mods) sur le PC de Zeleff, un vrai jeu par un vrai tunnel Tandem, faux joueurs Carpet qui sprintent. Terrain neuf : hôte + 4 amis confortable, saturation CPU vers 8 joueurs, RAM serrée dès 2. Monde pré-généré : environ 20 joueurs tiennent, limite = fil principal du serveur. Génération pure 77 chunks/s, environ 11 Ko par chunk. Tunnel : 0,3 à 1 Mbit/s par joueur, jamais la limite.
- Outils : exemple `duo tunnel <port>` (serveur vu à travers une vraie session, débit mesuré), `TANDEM_MEMORY_MB` pour `play`, scripts locaux de montage et de pilotage du test.
- Prochaine étape : session dédiée à l'optimisation (6c ci-dessus), et le test réel à deux PC des nouveautés (nouvel installeur des deux côtés).

### 2026-10-11 (Windows : optimiser « Jouer ensemble », mesures)
- Profilage du serveur avec Java Flight Recorder (spark écarté : biaisé et coûteux sous Windows). Deux packs : Cobbleverse (Fabric 1.21.1) et Prominence II (Fabric 1.20.1), mondes pré-générés, 2 à 18 joueurs. Résultats dans [TEST-DUO.md](TEST-DUO.md#optimisation--où-part-le-temps-du-serveur-11-octobre-2026).
- Trouvé : apparitions vanilla calculées puis jetées quand un pack bloque les mobs (40 % du tick sur Cobbleverse) ; IA des entités ensuite (distance d'activation : −15 à 20 % sur Prominence) ; Architectury et Balm qui passent par la réflexion à chaque appel. Distance de simulation et VMP : peu d'effet.
- Prototype de mini-mod Fabric (local, `.claude/notes/tandem-host`) : 47 → 28 ms par tick à 18 joueurs sur Cobbleverse, −8 à 13 % sur Prominence, même jar pour 1.20.1 et 1.21.1.
- Prochaine étape : décider avec Zeleff du mini-mod (ajouté par Tandem pendant une session d'hôte), y ajouter la distance d'activation, puis NeoForge.
- Suite : pistes de niveau 1 à 3 étudiées. Niveau 1 (cœurs inutilisés) : plafond 12 à 30 %, surtout un cache de la recherche d'apparition. Niveau 2 : réseau sans intérêt CPU ; génération du terrain par les amis faisable (génération non reproductible de toute façon, injection de chunks dans l'hôte en marche prouvée). Niveau 3 écarté (casse les mods).
- Décision avec Zeleff (D40) : priorités notées dans la 6c (mini-mod branché, vrai jeu de l'hôte mesuré, pré-génération en un clic), génération coopérative mise de côté ; rien de commencé.

### 2026-10-11 (Windows : plus de CurseForge)
- Choix de Zeleff (D41) : Tandem n'utilise plus CurseForge. Retiré : module `content::curseforge`, commandes de la clé et des téléchargements à la main, dialogue de clé, réglage, panneau de l'instance. Un `.zip` CurseForge est refusé avec un message qui oriente vers « Depuis un autre launcher ». Gardé : l'import d'instances déjà installées (dont l'app CurseForge), qui ne fait que copier des fichiers locaux.
- Vérifié : fmt, Clippy, 134 tests (dont le refus d'un pack CurseForge), TypeScript, import `.mrpack` et Réglages dans l'aperçu (backend simulé).
- Cadrage de la session « mini-mod hôte » écrit (6d ci-dessus) : décisions à prendre, travail de la v1 Fabric, critères de fin.
- Ajouté au plan : Phase 4.8 « Soutenir les créateurs » (liens de soutien Modrinth mis en valeur), pour plus tard.

### 2026-10-11 (Tandem Optimizations, soutien aux créateurs)
- 6d faite : **Tandem Optimizations** (D42), mod ajouté au lancement des instances Fabric et Quilt 1.20 à 1.21.11, jamais écrit dans l'instance, coupable dans les Réglages et par instance. Distance d'activation ajoutée (vanilla seulement, après avoir mesuré que Cobblemon accumulait 75 % de Pokémon en plus), vérification de construction contre 20 versions (a attrapé un plantage en 1.21.9+ et un cas qui ne compilait pas en 1.21.5+). Mesures dans TEST-DUO : jusqu'à −30 % de temps de tick sur Prominence avant la restriction aux créatures vanilla.
- Phase 4.8 faite : bouton « Soutenir l'auteur » dans les fiches, onglet « Créateurs » (instance et modpack), temps de jeu par instance et rappel unique après 10 h.
- Vérifié : fmt, Clippy, 139 tests, TypeScript, Réglages, onglet Créateurs et fiche dans l'aperçu (backend simulé).
- Reste : test en vrai (solo et « Jouer ensemble ») avec le mod sur 3 gros packs ; remesurer Prominence avec la version « vanilla seulement » si utile.
- Prochaine étape : cadrage et début des idées « monde dans un processus à part » et « monde qui appartient au groupe » (6e, 6f).

### 2026-10-11 (recadrage de « Jouer ensemble »)
- Exploration poussée à la demande de Zeleff : mini-mod (fait), soutien aux créateurs (fait), monde à part (base faite et branchée, bêta), cadrage d'un monde partagé par le groupe. Bilan honnête : le monde à part apporte du confort, pas de vitesse ; le plafond de joueurs est le PC de l'hôte.
- Décision de Zeleff (D44) : on arrête de chercher de nouveaux angles et de repousser la limite de joueurs. Les sections 6c à 6f sont regroupées en une feuille de route de cinq chantiers, à finir un par un.
- Prochaine session : chantier 1 (terminer Tandem Optimizations v1), dans une nouvelle conversation.

### 2026-10-12 (chantier 1 : Tandem Optimizations v1 terminé)
- Fait : tests en vrai du mod tel que Tandem l'ajoute, en solo et en monde à part avec un invité, sur Cobbleverse, Prominence II et Fabulously Optimized, plus une instance Quilt réelle. Aucun plantage dû au mod. Mesure Prominence « créatures vanilla seulement » : ≈ −25 % de temps de tick, même niveau que la version complète.
- Corrigé : sur Quilt, Tandem installait Fabric API à côté de QFAPI et le jeu ne démarrait plus.
- Noté : course dans Essential à l'ouverture directe du monde (1 essai sur 2, compte hors ligne), conflit d'écriture d'une config TreeChop en monde à part (chantier 2).
- Prochaine étape : chantier 2 (terminer le monde à part), qui commence par un test à deux PC : nouvel installeur à compiler avec l'accord de Zeleff.

### 2026-10-12 (chantier 2 : monde à part, partie sans ami)
- Corrigé : le serveur du monde à part et le jeu de l'hôte écrivaient les mêmes fichiers (dossier `config` lié) ; les dossiers de l'instance sont maintenant recopiés à chaque démarrage (D45). Première copie de Prominence 4,6 s, ensuite rien à refaire.
- Fait : 1 Go de moins pour un jeu lancé directement sur un serveur (hôte en monde à part, invités), le serveur récupère la place (D45). Mesuré une fois sur Cobbleverse et Prominence.
- Vérifié : instance Quilt réelle hébergée à part, invité connecté.
- Reste : test à deux PC (Zeleff + un ami, nouvel installeur) et décision bêta / par défaut.

### 2026-10-12 (chantier 3 : préparer le monde en un clic)
- Fait (D46) : bouton « Préparer » dans l'onglet Mondes (Fabric et Quilt) : trois tailles de zone avec temps et place estimés, progression en direct (pourcentage, chunks, vitesse, temps restant), annulation qui garde le travail fait. Le monde tourne dans le serveur du monde à part avec Chunky ajouté au lancement (téléchargé depuis Modrinth, jamais copié dans l'instance).
- Vérifié : exemple `pregen` sur Fabulously Optimized (1 521 chunks en 16 s) et Prominence II (annulation puis reprise rapide), puis dans la vraie app sur l'instance Quilt de test (monde de 14 à 62 Mo) ; interface aussi testée avec le faux backend.
- Pas remesuré : l'effet sur le tick est déjà mesuré avec le même outil (TEST-DUO).
- Prochaine étape : chantier 4 (indicateur de charge pour l'hôte), et toujours le test à deux PC du chantier 2.

### 2026-10-12 (chantiers 2 et 4)
- Chantier 2 terminé : scénario complet simulé sur un PC avec le vrai tunnel (Fabulously Optimized) : l'ami entre par le code, l'hôte ferme son jeu, l'ami reste connecté, l'hôte revient. Outil : `duo host` avec `TANDEM_WORLD_PORT`.
- Chantier 4 fait (D47) : carte « Santé de la partie » dans « Jouer ensemble » (monde : fluide / chargé / en retard ; mémoire du PC ; un conseil quand ça sature). Tandem Optimizations 1.1 écrit la durée des ticks toutes les 100 ticks quand le monde est partagé ; sinon Tandem se rabat sur le « Can't keep up! » de Minecraft.
- Vérifié : relevés sur de vrais serveurs 1.21.4 (FO) et 1.20.1 (Prominence), silence en solo, mod vérifié contre les 20 versions, carte testée avec le faux backend.
- Pas vérifié en vrai : le relevé dans un monde ouvert en LAN depuis le jeu (impossible à ouvrir sans écran) ; même code, condition « monde publié » de Minecraft.
- Prochaine étape : chantier 5 (monde du groupe, 6f) à recadrer avec Zeleff, ou passer au peaufinage complet de l'app.

### 2026-10-12 (fin de « Jouer ensemble », cadrage de la refonte)
- Chantier 5 (monde du groupe) abandonné (D48) : le groupe laisse héberger le meilleur PC. La feuille de route « Jouer ensemble » est terminée.
- Refonte de l'interface cadrée (Phase 7, [REFONTE.md](REFONTE.md)) : retour de Zeleff et de son frère (« pas très propre ni agréable »), direction « le Minecraft moderne » choisie parmi plusieurs pistes, sombre, animations sobres, police carrée libre pour les grands titres.
- Prochaine étape : Phase 7, étape 1 (fondations).

### 2026-10-12 (refonte, étape 1 : fondations)
- Fait (D49) : nouvelles couleurs graphite, boutons pleins qui s'enfoncent, champs, panneaux, onglets, listes, puces, messages, barres de défilement ; échelle de texte réduite sans 10-11 px ; icônes en pixels entiers. Toute l'app change d'habillage d'un coup (mêmes noms de classes).
- Police des grands titres : Jersey 10, choisie par Zeleff sur une page de comparaison ; base des boutons et surfaces validée par lui.
- Vérifié : TypeScript, page `/?fondations` et pages Accueil, Instances, fiche d'instance avec le faux backend.
- Reste visible jusqu'aux étapes suivantes : mises en page de chaque page (titres dorés en capitales, cartes d'instance serrées), à refaire page par page.
- Prochaine étape : étape 2 (cadre de l'app : barre de titre, navigation, compte).

### 2026-10-12 (refonte, étape 2 : cadre de l'app)
- Fait : barre de titre et barre latérale d'un seul tenant (même gris, la ligne de séparation ne court que côté contenu), titre « Tandem » en Jersey 10, navigation à grandes icônes (24 px) avec page active surlignée, compte en carte claire, « Récentes » sans capitales espacées, Réglages au même style.
- Arrivée d'un ami : pastille verte avec le nombre sur « Jouer ensemble » tant que la page n'a pas été ouverte (sinon le point doré de partie en cours).
- Icône « Jouer ensemble » redessinée en deux personnages (on la confondait avec celle d'Instances).
- Vérifié : TypeScript, aperçu avec le faux backend (navigation, menu du compte). Pas vérifié : la pastille d'arrivée en vrai (il faut une vraie partie).
- Prochaine étape : étape 3 (Accueil).

### 2026-10-12 (refonte, étape 3 : Accueil)
- Fait : la dernière instance possède l'écran : grand visuel (image mise en avant de la galerie du modpack Modrinth, sinon dernière capture du joueur, sinon le ciel dessiné), nom en Jersey 10, « Joué il y a… » dans la ligne d'infos (plus d'étiquette dorée au-dessus), Jouer puis « Inviter des amis » (devient « Partie ouverte » / « N amis connectés » pendant une session), détails et dossier en boutons carrés.
- « Rejoindre un ami » directement sur l'accueil : code, puis la page « Jouer ensemble » prend le relais ; sans compte Microsoft, un bouton pour l'ajouter.
- « Tes autres instances » (trois au plus) et Nouveautés à droite, en colonne sous 1 100 px.
- Vérifié : TypeScript, détecteur, aperçu à 1 280 × 800 avec le faux backend (capture comme visuel, connexion Microsoft simulée puis code rejoint depuis l'accueil). Pas vérifié : une vraie image de galerie Modrinth dans l'app.
- Prochaine étape : étape 4 (Jouer ensemble).

### 2026-10-12 (refonte, étape 4 : Jouer ensemble)
- Fait : en-tête simple (titre en Jersey 10, phrase, compte utilisé) à la place de la bannière de nuit ; « Inviter un ami » et « Rejoindre un ami » en vrais titres, boutons pleins (clair pour inviter, vert pour rejoindre) ; code d'invitation agrandi dans un creux, copie en un clic ; étapes de l'hôte à coches vertes ; invités, santé de la partie, préparation de l'instance de l'ami et sa barre de progression au nouveau style ; « Partie de Léo » comme titre côté invité. Interrupteurs et cases à cocher un peu plus grands, coins presque carrés.
- Plus de textes explicatifs en 12 px : 13 px partout sur la page.
- Vérifié avec le faux backend à 1 280 × 800 : compte Microsoft simulé, invitation, arrivée d'un ami (pastille sur « Jouer ensemble » depuis l'accueil, effacée à l'ouverture de la page, « 2 amis connectés » sur l'accueil), puis côté invité avec un code.
- Prochaine étape : étape 5 (Instances et fiche d'instance).

### 2026-10-12 (refonte, étape 5 : Instances et fiche d'instance)
- Fait : un seul visuel d'instance partout (`InstanceVisual`) : image mise en avant du modpack, sinon dernière capture, sinon dessin ; toujours recadré au même format, en fondu, une recherche par instance et par session (effacée quand une partie se termine).
- Cartes d'instance avec ce visuel (vignette), icône en bas à gauche, nom, version, date ; plus de biseau blanc d'inventaire. Page Instances : titre en Jersey 10, grille plus aérée.
- Fiche : en-tête avec le visuel, fil d'Ariane en haut, nom en Jersey 10, version du pack s'il y en a une, boutons carrés (supprimer en rouge discret, dossier) et Jouer.
- Onglets : panneaux d'alerte, de conflit et de mises à jour au même style (bord doré, rouge ou vert, sans liseré latéral), console et journaux dans un creux arrondi, petits cadres aux coins presque carrés. Icônes de blocs à un nombre entier de pixels.
- Vérifié avec le faux backend à 1 280 × 800 : page Instances, fiche et ses sept onglets.
- Prochaine étape : étape 6 (Découvrir et fiches de projet).

### 2026-10-12 (refonte, étape 6 : Découvrir et fiches de projet)
- Fait : titre en Jersey 10 et phrase lisible pour Découvrir ; résultats qui s'éclairent au survol, infos en 13 px ; fiche de projet avec l'image mise en avant de la galerie derrière l'en-tête (assombrie pour lire), titre en Jersey 10, plus de dégradé à la couleur du projet ; types de version sans capitales.
- Vérifié avec le faux backend à 1 280 × 800 : recherche de mods, onglet Modpacks, fiche d'un modpack.
- Prochaine étape : étape 7 (Réglages, Comptes, fenêtres, notifications, états vides et d'erreur).

### 2026-10-12 (refonte, étape 7 : Réglages, Comptes, fenêtres, états)
- Fait : titre des Réglages en Jersey 10 ; libellés des champs des fenêtres (nouvelle instance, modpack, changement de version, import) en 13 px demi-gras ; textes d'aide en 13 px ; accueil d'un compte (« Bienvenue, … ») en Jersey 10 ; avertissement des comptes aux coins presque carrés ; « Cette instance n'existe plus » avec un bouton vers les instances au lieu d'une ligne grise.
- Plus aucun titre en police pixel, capitales espacées ou ombre décalée hors des codes d'invitation et du gros bouton Jouer.
- Les notifications Windows restent celles du système (rien à styler).
- Vérifié avec le faux backend à 1 280 × 800 : Réglages, fenêtre Nouvelle instance, fenêtre Comptes.
- Prochaine étape : étape 8 (finition : revue complète, `DESIGN.md`, décision qui remplace D11).

### 2026-10-12 (refonte, étape 8 : finition, Phase 7 terminée)
- Revue complète par un relecteur indépendant (outil impeccable) sur les captures de toutes les pages : 8 corrections, toutes faites : tailles de texte réduites à des jetons (`meta` 13, `title` 44, `hero` 64), ciel dessiné des cartes refait (puis varié jour / couchant / nuit selon l'instance), plus d'étiquette au-dessus du titre d'une fiche de projet, « Supprimer » sorti de l'en-tête vers l'onglet Réglages, icône de secours visible, point « partie en cours » en vert, résultats de Découvrir en une seule liste. Icônes de loader et têtes de compte ramenées à des tailles entières.
- `DESIGN.md` écrit à partir du code livré, D50 remplace D11.
- Vérifié : TypeScript, détecteur sur tout `src`, captures avec le faux backend. Pas encore vu dans la vraie app (`pnpm tauri dev`) ni sur de vrais visuels Modrinth.
- Prochaine étape : retour aux petits ajouts ; test de la refonte par Zeleff dans la vraie app.

### 2026-10-13 (retour de Zeleff sur les fiches de projet)
- « Soutenir l'auteur » allongeait l'en-tête des fiches et laissait un grand vide sous le titre : il passe dans la colonne de droite, en bas de « Créateurs » ; « Voir sur Modrinth » devient le premier des « Liens ». L'en-tête ne garde que l'instance, Installer et la version.
- Vérifié avec le faux backend (fiche de Sodium).
- Barre latérale repliable (demande de Zeleff) : bouton à gauche de la barre de titre ou Ctrl+B (⌘B sur Mac). Repliée (80 px), elle ne garde que les icônes, la tête du compte et celles des instances récentes, avec leur nom en info-bulle ; la pastille d'ami arrivé et le point « en cours » restent visibles. L'état est retenu sur ce PC.

### 2026-10-13 (lancement du jeu)
- Fait : le bouton Jouer devient une piste qui se remplit en vert pendant le téléchargement (étape et pourcentage), un balayage lumineux pendant les étapes sans quantité (vérification, loader, démarrage), « Démarrage du jeu… » jusqu'à l'écran titre (ligne « Sound engine started » du journal, toutes versions et loaders), puis « En jeu » après un bref éclat. Arrêter est un bouton carré à côté, plus jamais le bouton principal. Largeur fixe d'un état à l'autre ; sans animation si Windows les désactive.
- Plus de « 0 % » trompeur pendant les étapes sans quantité (fiche, barre latérale, barre de titre).
- Vérifié avec le faux backend (séquence complète, maintenant plus réaliste : étape du loader puis écran titre 3 s après le démarrage). Pas vérifié : un vrai jeu, en particulier la durée de « Démarrage du jeu… » sur un gros modpack.

### 2026-10-13 (onglet Contenu désencombré)
- Fait : les encadrés empilés au-dessus de la liste (optimisations recommandées, conflits, mises à jour) deviennent une seule bande « À regarder » avec un résumé (« 1 conflit · 2 mises à jour · 3 optimisations conseillées ») et « Tout mettre à jour » directement. Repliée par défaut : une ligne de 48 px. Seuls les conflits, qui peuvent empêcher le jeu de démarrer, affichent toujours leurs lignes et leurs boutons « Désactiver ». La bande disparaît quand il n'y a rien.
- Vérifié avec le faux backend (instance avec un conflit, deux mises à jour et trois optimisations : liste des mods remontée d'environ 300 px).
- Retouches demandées par Zeleff : toute la partie gauche d'une ligne (icône, nom, version) ouvre la fiche, plus seulement le nom ; vraie barre de recherche en haut de l'onglet (40 px, icône, toujours là dès qu'il y a du contenu) à la place du petit filtre qui n'apparaissait qu'au-delà de 15 éléments ; phrases qui répétaient l'évidence retirées (« x nouvelles versions… marquées dans la liste », description des optimisations, compteur « 4 éléments · 1 désactivé » passé dans la recherche).
- Même passe sur les autres onglets : boutons d'action agrandis (36 px, texte 13 px) dans Mondes, Captures, Créateurs, Traduction et Réglages ; retirés : « 2 mondes » en tête (la liste le dit), la note sur les 5 sauvegardes gardées (passée en info-bulle de l'interrupteur), « Une image (carrée de préférence) ou un bloc », « Taille de la fenêtre au lancement », « Une confirmation est demandée », la phrase sur l'ordre de traduction ; textes de Créateurs et des quêtes traduites raccourcis.

### 2026-10-14 (Phase 8, chantier 1 : Découvrir)
- Cadrage de la Phase 8 avec Zeleff : rendre Tandem attirant pour le solo (découverte, reprise de partie, choix d'un modpack).
- Fait : Découvrir a un tri (« Les plus téléchargés » ou « Pertinence » selon la recherche, « Les plus suivis », « Nouveautés », « Mis à jour récemment »), des catégories à cocher propres à chaque type, une version du jeu pour les modpacks. Sans recherche ni filtre, trois rayons défilants (12 projets chacun, « Tout voir » applique le tri) au-dessus de la liste des plus téléchargés. Changer de type revient aux rayons.
- Nouveautés et mises à jour exigent 10 abonnés quand on ne cherche rien (D51) : sinon la liste est pleine de tests et de packs de serveurs privés.
- Vérifié : tests Rust (facettes, ordre automatique), facettes testées sur la vraie API Modrinth, page avec le faux backend (rayons, filtres, « Tout voir », modpacks). Pas vérifié : la vraie app avec les vrais résultats.
- Prochaine étape : chantier 2, « Continuer » en un clic dans le dernier monde.
- Retour de Zeleff (« un peu le bordel ») : Découvrir remis en ordre. Les types deviennent les onglets de la page (soulignés), avec l'instance (ou « Importer un fichier » pour les modpacks) à droite ; tri, version et catégories passent dans une colonne de filtres à gauche (cases à cocher) ; la recherche mène la colonne principale. Un seul rayon en images (« Nouveautés » : image mise en avant du projet, sinon son icône sur sa couleur), puis « Mis à jour récemment » et « Les plus suivis » en deux listes courtes de 5, côte à côte quand la place le permet. Phrase d'introduction et encadré des modpacks retirés (l'explication passe en info-bulle du bouton d'import).

### 2026-10-14 (Phase 8, chantier 2 : « Continuer » en un clic)
- Fait : sur l'Accueil, le grand bouton devient « Continuer » et lance le jeu directement dans le dernier monde joué de l'instance (`--quickPlaySingleplayer`, que le moteur savait déjà faire pour « Jouer ensemble »). Au-dessus, le monde choisi (icône, nom, mode) ouvre un petit menu : les autres mondes avec leur date, ou « Écran titre du jeu » pour revenir au bouton « Jouer » habituel. La liste est relue après chaque partie. Avant la 1.20 (le jeu ne sait pas ouvrir un monde au lancement) ou sans monde, rien ne change.
- Onglet Mondes : bouton « Jouer » sur chaque monde (1.20+), désactivé quand le jeu tourne.
- Les autres boutons Jouer (cartes d'instance, barre latérale, fiche) ouvrent toujours l'écran titre : l'Accueil sert à reprendre, le reste à choisir.
- Vérifié avec le faux backend (choix du monde, appel `launch_instance` avec le bon dossier depuis l'Accueil et l'onglet Mondes). Pas vérifié dans un vrai jeu cette fois ; le même chemin sert déjà à « Jouer ensemble » (Fabric, Forge, NeoForge).
- Prochaine étape : chantier 3, carnet de partie par monde.

### 2026-10-14 (Phase 8, chantier 3 : carnet de partie)
- Fait : module `journal` (Rust) qui lit ce que le jeu enregistre déjà, sans rien modifier : `level.dat` (jour), `stats/` (temps de jeu, distance, créatures vaincues, blocs minés, morts ; format 1.13+ et ancien), `advancements/` (9 grandes étapes vanilla, avec leur date), et la progression FTB Quests (`saves/<monde>/ftbquests/` face au livre `config/ftbquests/quests/chapters/`, titres des chapitres pris dans le fichier ou dans `lang/`, chapitres et quêtes invisibles écartés). Petit lecteur SNBT en arbre ajouté à `translate/snbt.rs`.
- Onglet Mondes : cliquer sur un monde (icône et nom) déplie son carnet : chiffres, grandes étapes (cochées, datées, « Prochaine » sur la suivante), quêtes avec une barre d'XP et le détail par chapitre.
- Accueil : à côté du monde choisi, un résumé (« Jour 418 · 112 h de jeu · Quêtes 64 / 230 », ou la prochaine étape sans quêtes) qui ouvre le carnet dans l'onglet Mondes. La route d'instance accepte maintenant un onglet et un monde.
- Correction : `search_content` dépassait la limite de 7 arguments de clippy (commit du chantier 1) ; tri et filtres regroupés en un seul argument.
- Vérifié : tests Rust (dates des critères, statistiques nouvel et ancien format, quêtes par chapitre, arbre SNBT) ; exemple `journal <instance>` sur de vrais mondes (DawnCraft 3/230 quêtes, Prominence II 1 752 quêtes lues en 0,5 s en debug, Fabulously Optimized, Cobbleverse) ; interface avec le faux backend.
- Prochaine étape : chantier 4, fiches de modpack « est-ce pour moi ? ».

### 2026-10-14 (Phase 8, chantier 4 : fiches de modpack « est-ce pour moi ? »)
- Fait : sur la fiche d'un modpack, sous l'en-tête, une bande de quatre cases pour la version qu'« Installer » choisirait : Mémoire (« Ton PC suit », « Un peu juste » ou « Trop lourd pour ton PC », d'après la mémoire automatique de D18 face à celle de la machine), Contenu (nombre de mods et taille à télécharger), Quêtes (livre de quêtes ou pas), Surtout (deux thèmes dominants des mods inclus). Dans la colonne de droite, « Dans le même esprit » : 5 modpacks qui partagent ses catégories et sa version du jeu, les plus suivis d'abord.
- Moteur : `project::pack_overview` (mods inclus déjà résolus pour l'onglet Contenu, tailles des fichiers en lots de 100, catégories des mods) ; les résumés de projets gardent maintenant leurs catégories.
- Vérifié : tests Rust (thèmes sans bibliothèques, mods de quêtes reconnus) ; exemple `project` sur de vrais packs (Prominence II : 446 mods, 1,7 Go, quêtes, 0,4 s ; Fabulously Optimized : 49 mods, 45 Mo ; Cobbleverse : 150 mods, 940 Mo) ; interface avec le faux backend.
- Phase 8 terminée. Prochaine étape : Zeleff teste les quatre chantiers dans la vraie app.


### 2026-10-14 (harmonie de l'interface)
- Demande de Zeleff : moins de textes trop explicatifs, éléments mieux placés, plus de cohérence.
- Fait : tour de toutes les pages dans le navigateur. Textes d'aide raccourcis ou retirés partout (Accueil, Instances, réglages d'instance, Réglages, Jouer ensemble, Traduction, Glossaire, comptes, imports, dialogues) ; sous-titres de page retirés. Onglets d'instance : Contenu par défaut, Console déplacée en fin. Réglages d'instance : Dupliquer et Supprimer réunis dans « Gestion », listes de même largeur. Accueil : les 3 autres instances sur une ligne, bouton « Détails » plus clair que l'icône de console. Réglages : l'aide du service de traduction passe sous « Service ». Jouer ensemble : « Tu joues en tant que » seulement avec un compte Microsoft. Références à l'ancien onglet « Informations » corrigées en « Réglages ». Règle notée en D54.
- Vérifié : types (`tsc`), pages revues avec le faux backend.
- Prochaine étape : Zeleff regarde dans la vraie app et signale ce qui détonne encore.

### 2026-10-20 (mises à jour automatiques, une seule fenêtre)
- Fait : installeur de test compilé pour Zeleff. Mises à jour de Tandem (D55) : module `updates` (Rust), pastille « Tandem x.y » dans la barre de titre, fenêtre avec les notes et la progression, « Vérifier les mises à jour » dans Réglages ; refus pendant un jeu ou une partie à plusieurs. `release.yml` signe les fichiers et écrit `latest.json` ; les 0.x ne sont plus des pré-versions. Une seule fenêtre à la fois (`single-instance`).
- Vérifié : clippy, types, interface avec le faux backend (pastille, fenêtre, progression, « à jour »), installeur signé produit en local (`.sig`).
- Pas vérifié : une vraie mise à jour de bout en bout (il faut une release publiée avec `latest.json`).
- Prochaine étape : Zeleff ajoute le secret GitHub et teste l'installeur ; ensuite premier lancement guidé.

### 2026-10-20 (page Apparence)
- Fait : page Apparence ouverte depuis le menu du compte (D56). Aperçu 3D à tourner (skinview3d), capes du compte, bibliothèque sur le PC (import PNG, glisser-déposer, skin d'un joueur par pseudo, renommer, modèle bras larges / fins, retirer avec annulation), skins par défaut du jeu, « Porter ce skin ». La tête du compte suit le nouveau skin.
- Vérifié : tests Rust (modèle deviné, fichiers refusés, noms des capes), exemple `skins` sur de vraies données (18 skins par défaut lus dans un jar, modèle bien deviné sur 6 d'entre eux, skin d'un joueur par pseudo, pseudo inconnu), clippy, types, page avec le faux backend.
- Pas vérifié : l'envoi réel d'un skin et le changement de cape sur un vrai compte (à faire par Zeleff dans `tauri dev`).
- Prochaine étape : Zeleff essaie la page avec son compte ; ensuite premier lancement guidé.

### 2026-10-20 (premier lancement guidé)
- Fait : écran de bienvenue au tout premier lancement (D57). Compte Microsoft, parties des autres launchers à cocher, première partie (Minecraft tel quel, Minecraft optimisé avec Fabulously Optimized, ou un modpack). Erreur « pas de release publiée » des mises à jour traitée comme « à jour ».
- Vérifié : types, parcours complet avec le faux backend (`localStorage.setItem("mock-first-run", "1")`) : connexion, import, « Plus tard » / « Pas maintenant », création de la première instance, réglage posé.
- Vérifié aussi par Zeleff dans la vraie app sur un dossier vide : connexion Microsoft, instances des autres launchers trouvées, « Minecraft optimisé » installé. Retouches demandées : pas de navigation ni de séparateur dans la barre de titre pendant l'écran, paysage qui défile (plans à vitesses différentes) au lieu du décor fixe.
- Prochaine étape : au choix de Zeleff (signature SignPath après la première release, packs de skins d'événement, autre).

### 2026-10-20 (Pack Tandem)
- Vérifié : les packs d'événement de Mojang ne peuvent pas être redistribués (contrat de licence) ; Modrinth App les copie dans son code.
- Fait : six skins originaux avec relief (deuxième couche) et ombrage, dessinés par un script versionné (`tools/skins/generate.py`), choisis avec Zeleff sur aperçu 3D. Section « Pack Tandem » dans la page Apparence (D58).
- Vérifié : test Rust (les six PNG sont des skins valides, bras larges ou fins bien déclarés), clippy, types, page avec le faux backend (vrais PNG).
- Pas vérifié : un skin du pack porté sur un vrai compte.

### 2026-10-20 (instances dans Jouer ensemble, amis)
- Fait : « Créer une instance » dans les listes d'instances de Jouer ensemble, et un encadré clair quand il n'y en a aucune (le monde se crée dans le jeu). Bouton « Jouer avec des amis » sur chaque instance (icône seule dans une fenêtre étroite), qui ouvre Jouer ensemble avec l'instance choisie.
- Fait : système d'amis sans serveur (D59) : moteur, service dans l'app, panneau Amis.
- Vérifié : `friends selftest` (deux launchers, vrais relais : code, demande, acceptation, statut, invitation, hors ligne, retrait), tests Rust, clippy, types, interface avec le faux backend.
- Pas vérifié : deux vrais PC.

### 2026-10-20 (menu clic droit)
- Fait : menu contextuel réutilisable (`ContextMenu`, clic droit, touche Menu ou Maj+F10, flèches, Échap, toujours dans la fenêtre). Sur les instances récentes de la barre latérale et les cartes d'instance (Accueil, Instances) : version, Jouer / Arrêter, Jouer avec des amis, Ouvrir, Mondes, Ajouter du contenu, Ouvrir le dossier, Dupliquer, Exporter en .mrpack, Supprimer (désactivés pendant que le jeu tourne). Les fenêtres Supprimer et Dupliquer sont maintenant communes à toute l'app.
- Vérifié : types, menu et actions avec le faux backend (clavier, suppression, duplication, position près des bords).


### 2026-10-20 (page Réglages en sections)
- Fait : Réglages découpés en sections avec un menu à gauche (Jeu, Performances, Agent IA, Launcher, Journal), une seule affichée à la fois. Agent IA : services en cartes (« Sur ton PC » / « En ligne », le service enregistré marqué « Actif »), puis la connexion du service choisi (clé, modèle, test), puis la langue. Les boutons de l'onglet Traduction ouvrent directement la section Agent IA.
- Vérifié : types, navigation au clavier, choix d'un service + clé + test + enregistrement avec le faux backend, lien depuis l'onglet Traduction.

### 2026-10-20 (README et notes de version)
- Fait : README et `docs/releases/v0.1.0.md` mis à jour pour la v0.1 réelle : jouer ensemble sans serveur, amis, monde à part (bêta), Tandem Optimizations, apparence, premier lancement guidé, mises à jour automatiques ; « Pas encore là » revu (installeurs signés, Forge/NeoForge, reprise si l'hôte part).
- Site : mis à jour et poussé dans un dépôt à part (`RandomZeleff.github.io`), en ligne le jour de la v0.1.

### 2026-10-20 (publication de la v0.1.0)
- Fait : v0.1.0 publiée (https://github.com/RandomZeleff/tandem/releases/tag/v0.1.0) : installeurs Windows (.exe, .msi) et macOS (.dmg universel), `latest.json` signé pour les mises à jour automatiques. Site en ligne : https://randomzeleff.github.io (dépôt `RandomZeleff.github.io`, GitHub Pages).
- Corrigé avant : le test `mirrors_instance_folders_into_real_files` échouait sur macOS depuis plusieurs commits (un ancien lien vers un dossier y est un lien symbolique, à retirer comme un fichier) ; `release.yml` cherchait les installeurs à la racine alors que les artefacts gardent leurs sous-dossiers (premier passage du workflow).
- Prochaine étape : demander la signature SignPath (Phase 9), tester une vraie mise à jour 0.1.0 → 0.1.1, test à deux PC des amis.
