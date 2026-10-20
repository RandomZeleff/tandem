# Journal des décisions

Format : date · décision · pourquoi · statut (proposé / validé / abandonné)

---

### D1 · 2026-10-08 · Tauri 2 + Rust
- Pourquoi : binaire léger, faible RAM vs Electron, Rust idéal pour I/O parallèles et P2P.
- Statut : validé

### D2 · 2026-10-08 · Frontend SolidJS + Tailwind
- Pourquoi : réactivité fine sans virtual DOM, bundle léger, cohérent avec l'objectif performance.
- Statut : validé (Tailwind v4 via `@tailwindcss/vite`)

### D3 · 2026-10-08 · Logique métier dans une crate `core` séparée
- Pourquoi : testable sans Tauri, réutilisable (CLI, serveur relais éventuel).
- Statut : validé

### D4 · 2026-10-08 · Store de fichiers par hash + hardlinks
- Pourquoi : déduplication entre instances, changement de version quasi instantané.
- Risque : hardlinks impossibles entre volumes différents → repli sur copie.
- Statut : validé

### D5 · 2026-10-08 · Multi via tunnel du LAN vanilla (sans mod) avec `iroh`
- Pourquoi : marche sur toutes versions/loaders, pas de serveur à payer, NAT traversal + relais intégrés.
- Statut : validé

### D6 · 2026-10-08 · Modrinth en source principale, CurseForge en secondaire
- Pourquoi : API ouverte et propre ; CurseForge exige une clé et certains mods interdisent la distribution tierce.
- Statut : remplacé par D41 (Modrinth seule source)

### D7 · 2026-10-08 · Accès API Minecraft : app Azure + validation Mojang
- Les nouvelles apps Azure reçoivent un 403 sur `api.minecraftservices.com` tant que leur Client ID n'est pas approuvé.
- Étapes : app Azure (comptes Microsoft personnels uniquement, tenant `consumers`, public client flows activés) → formulaire https://aka.ms/mce-reviewappid avec le Client ID.
- Client ID : `47ec2f94-9a22-4089-95c2-e2cbbc4afd44` (dans `tandem-core/src/auth.rs`). Formulaire Mojang : à envoyer une fois le nom choisi.
- En attendant : développement avec compte offline.
- Statut : en cours

### D8 · 2026-10-08 · Nom du projet : Tandem
- Pourquoi : « jouer à deux » = le différenciateur P2P ; court, bilingue, sans « Minecraft/Mojang/Craft ».
- Vérifs : aucun launcher/mod Minecraft homonyme (Modrinth, GitHub, web). `tandem.app`, `playtandem.com` et `tandemlauncher.com` déjà pris ; libres au 2026-10-08 : `tandemlauncher.app`, `tandemlauncher.net`, `tandemmc.com`.
- Identifiants : crates `tandem` / `tandem-core`, bundle `dev.tandem.launcher`, données `%APPDATA%/Tandem`.
- Statut : validé

### D9 · 2026-10-08 · Engagements pris auprès de Mojang (formulaire App ID)
- Connexion Microsoft via le **navigateur système** (pas de webview intégrée), tenant `consumers`, scopes `XboxLive.signin offline_access`.
- Tokens stockés **uniquement** dans le gestionnaire d'identifiants de l'OS (Windows Credential Manager).
- Aucun serveur Tandem ne reçoit d'identifiants ni de données de compte.
- Fichiers de jeu téléchargés uniquement depuis les serveurs officiels Mojang ; pas de contournement de la vérification de possession.
- Statut : validé (formulaire envoyé le 2026-10-08) — ne pas s'en écarter.

### D10 · 2026-10-08 · Fichiers vanilla partagés par chemin, store par hash réservé aux mods
- Libraries (layout Maven), `versions/<id>/` et `assets/objects` (déjà rangés par hash par Mojang) sont partagés par toutes les instances via leur chemin : pas besoin de hardlinks.
- Le store content-addressed + hardlinks (D4) sert en Phase 3 pour mods / resource packs / shaders copiés dans chaque instance.
- Statut : validé

### D11 · 2026-10-08 · Direction artistique « Deepslate »
- Sombre, angles en escalier de pixels (jamais de border-radius), reliefs biseautés, cases d'inventaire, barre d'XP, ciels en bandes nettes (un seul voile dégradé pour la lisibilité).
- Pixel uniquement pour titres, étiquettes et icônes (Pixelify Sans) ; texte en Geist, données en Geist Mono. Polices embarquées via Fontsource (aucun appel réseau).
- Rôles des couleurs : Herbe = action principale, XP = progression, Or = social/étiquettes, Améthyste = IA et snapshots, Redstone = danger.
- Référence : planche « Fondations » du canevas de design ; tokens dans `src/index.css` (`@theme`) et classes `.btn*`, `.panel`, `.slot`, `.field`, `.px-corners*`.
- Statut : remplacée par D50 (2026-10-12)

### D12 · 2026-10-08 · Fabric / Quilt : profil fusionné dans le JSON vanilla, version figée
- Les profils publiés par meta.fabricmc.net / meta.quiltmc.org (`inheritsFrom`) sont appliqués sur le JSON vanilla : `mainClass` remplacée, arguments ajoutés, libraries du loader en tête du classpath et prioritaires sur une library vanilla de même `group:artifact[:classifier]`.
- La version du loader est choisie (dernière stable par défaut) et **enregistrée à la création** de l'instance : jamais de mise à jour implicite au lancement.
- Les profils sont mis en cache dans `versions/<loader>-loader-<version>-<mc>/` → lancement hors ligne possible.
- Forge / NeoForge (installers + processors) restent à faire ; `Loader` les connaît déjà mais renvoie `LoaderNotSupported`.
- Statut : validé

### D13 · 2026-10-08 · Contenu des instances suivi en base, fichiers dans le store
- Chaque projet installé = une ligne `instance_content` (projet, version, fichier, SHA-1, dépendance oui/non). Le fichier est téléchargé dans `store/<sha1[0..2]>/<sha1>` puis hardlinké dans `mods/`, `resourcepacks/` ou `shaderpacks/`.
- Version choisie : la plus récente compatible (version de jeu + loaders ; Quilt accepte `quilt` et `fabric`), release en priorité. Dépendances `required` installées récursivement si absentes ; une version épinglée par la dépendance est respectée.
- Retirer un projet ne retire pas ses dépendances (elles peuvent servir à d'autres mods) ; nettoyage des orphelins plus tard.
- Les fichiers ajoutés à la main dans le dossier de l'instance ne sont pas suivis (pas de scan pour l'instant).
- Statut : validé

### D14 · 2026-10-08 · Icônes de loaders dessinées en pixel art maison
- Vanilla, Fabric, Quilt, Forge, NeoForge ont une icône 12×12 dans le style de la DA (`LoaderIcon` / `LoaderTag` dans `src/components/pixel.tsx`), inspirée des logos officiels sans les copier : cohérent avec D11 et sans question de licence.
- Statut : validé

### D15 · 2026-10-08 · Mises à jour et désactivation du contenu
- Désactiver = renommer le fichier en `<nom>.disabled` (convention de Prism / app Modrinth, ignorée par le jeu et par Iris) ; l’état est aussi en base (`enabled`).
- Une mise à jour cherche la dernière version compatible via le SHA-1 du fichier installé : releases d’abord, bêtas seulement s’il n’existe aucune release compatible, et jamais une version publiée avant celle installée.
- Une mise à jour conserve les choix du joueur (activé/désactivé, marqué dépendance) et la date d’installation ; ses nouvelles dépendances requises sont installées.
- Modifier le contenu est refusé tant que le jeu tourne (fichiers verrouillés sous Windows).
- Statut : validé

### D16 · 2026-10-08 · Modpacks `.mrpack`
- Installer un modpack crée toujours une **nouvelle instance** (nom = titre du projet, loader et version du loader imposés par le pack). Si l’installation échoue, l’instance est supprimée : pas d’instance à moitié installée.
- Sécurité : téléchargements limités aux domaines de la spec (`cdn.modrinth.com`, `github.com`, `raw.githubusercontent.com`, `gitlab.com`, en HTTPS), chemins relatifs sans `..` ni racine, overrides extraits via `enclosed_name`.
- Fichiers du pack reconnus par Modrinth (SHA-1) = contenu normal de l’instance (mises à jour, activation). Les autres fichiers sont installés mais pas suivis.
- Seuls Fabric et Quilt sont proposés dans Découvrir (facette `categories`) tant que Forge/NeoForge ne se lancent pas ; un pack Forge/NeoForge importé est refusé avec un message clair.
- Fichiers optionnels (`env.client = optional`) installés par défaut ; `unsupported` ignorés.
- Export : contenu Modrinth activé → `files` (URL + sha1/sha512 de l’API), le reste → `overrides/` ; exclus : `saves`, `logs`, `crash-reports`, `screenshots`, caches, `*.disabled`.
- Statut : validé

### D17 · 2026-10-08 · Forge / NeoForge : rejouer l’installeur officiel sans son interface
- On télécharge l’installeur officiel, on lit `version.json` (profil fusionné comme Fabric) et `install_profile.json`, puis on exécute nous-mêmes ses processors côté client avec la JVM Mojang de la version. Pas d’exécution de l’installeur en mode GUI/headless : on garde la main sur la progression, les erreurs et le cache.
- Les processors ne tournent qu’une fois par version de loader (marqueur `versions/<loader>-<mc>-<version>/.processed`, sorties vérifiées par SHA-1 quand l’installeur les fournit).
- Le jar client vanilla reste nommé `<mc>.jar` : il correspond au `${version_name}.jar` de l’`ignoreList` de Forge, ce qui évite de le copier.
- Versions stockées sans le préfixe Minecraft pour Forge (`47.4.10`, comme dans les `.mrpack`) ; versions Forge à suffixe (≤ 1.7.10) non proposées.
- Statut : validé

### D18 · 2026-10-09 · Mémoire automatique et flags G1 par défaut
- Sans réglage, la mémoire suit le nombre de mods activés de l’instance (jars dans `mods/`) : 2 Go en vanilla, 4 Go jusqu’à 50 mods, 6 Go jusqu’à 150, 8 Go au-delà.
- Plafond pour laisser respirer l’OS : la moitié de la RAM sur les petites machines, tout sauf 6 Go sur les grosses (`max(RAM/2, RAM − 6 Go)`), plancher 1 Go. Ex. : 8 Go → 4 Go max, 16 Go → 8 Go pour un gros pack.
- Le joueur peut fixer une taille par instance (onglet Informations) ou revenir en « Automatique ».
- G1 réglé pour un client (pauses courtes, grande young gen, base Aikar sans `AlwaysPreTouch`) ajouté à chaque lancement, Java 8 compris ; pas ajouté si le joueur choisit son propre GC (`-XX:+Use…GC` dans ses arguments JVM).
- Statut : validé

### D19 · 2026-10-09 · Suggestion de mods de performance
- Catalogue court et figé (ids Modrinth), par impact : Sodium (à défaut Embeddium), Lithium, FerriteCore, ModernFix, ImmediatelyFast, Entity Culling, Dynamic FPS. Uniquement des mods connus pour être sûrs côté client, pas de liste « fourre-tout ».
- Compatibilité vérifiée en direct sur Modrinth pour la version et le loader de l’instance ; rien pour une instance vanilla.
- Un seul moteur de rendu par instance : Sodium, Embeddium et Rubidium s’excluent, et rien n’est proposé si l’un d’eux ou un jar OptiFine est déjà là.
- Panneau dans l’onglet Contenu (installation à l’unité ou « Tout installer », dépendances comprises) ; « Masquer » est retenu par instance (réglage `perf_suggestions_hidden.<id>`). Hors ligne, le panneau ne s’affiche pas.
- Statut : validé

### D20 · 2026-10-09 · Apple Silicon : 1.18.x en natif, le reste avant 1.19 sous Rosetta
- 1.19+ : natif (Mojang fournit Java et LWJGL arm64).
- 1.18.x : natif en remplaçant LWJGL 3.2 et `java-objc-bridge` par ceux de la 1.19.2 (LWJGL 3.3.1, natives arm64 seulement : Forge ne garde qu’un module `org.lwjgl.natives`), sur le Java 17 arm64 de Mojang (`java-runtime-gamma`). Chargement ≈ 4-7 s au lieu de ≈ 19 s sous Rosetta.
- 1.17.x : reste sous Rosetta. Avec LWJGL 3.3, macOS refuse l’icône de fenêtre et 1.17 en fait une erreur fatale (1.18 ne pose pas d’icône sur Mac).
- ≤ 1.16 : Rosetta (aucun Java 8 arm64 chez Mojang ; un Java tiers comme Azul Zulu reste possible plus tard).
- Statut : validé

### D21 · 2026-10-09 · Analyse de crash locale, par heuristiques
- À chaque crash, Tandem lit le rapport de crash, sinon le log de crash de la JVM (`hs_err_pid*.log`), sinon `logs/latest.log`, et en tire : description, première exception, cause connue et mods suspects. Tout reste local, aucun envoi.
- Suspects, par ordre de confiance : mods nommés par Forge/NeoForge (`Suspected Mods`, frames `TRANSFORMER/modid@…`), Mixins en échec (`from mod x`), mods refusés par Fabric (« incompatible »), sinon le premier mod rencontré dans la pile d’appels, reconnu par les packages de ses classes (jars de `mods/` lus avec leurs métadonnées `fabric.mod.json` / `quilt.mod.json` / `mods.toml`). Le jeu, la JVM et les loaders ne sont jamais accusés.
- Causes connues : manque de mémoire (lien vers le réglage), mauvaise version de Java, pilote graphique / OpenGL, mods incompatibles, plantage natif de la JVM.
- Un suspect installé via Tandem peut être désactivé en un clic depuis le panneau ; « Voir le rapport » l’ouvre dans le Finder / l’Explorateur.
- Statut : validé

### D22 · 2026-10-09 · Mondes et sauvegardes
- Onglet « Mondes » par instance : nom (lu dans `level.dat`, NBT gzip), mode de jeu, version, taille, dernière partie, icône.
- Sauvegardes en zip dans `backups/<instance>/<monde>/<ms>-<type>.zip`, hors du dossier de l’instance (supprimer une instance ne supprime pas ses sauvegardes) ; `session.lock` exclu.
- Sauvegarde automatique à la fermeture du jeu des mondes joués pendant la session (`level.dat` modifié), activée par défaut (réglage `auto_backup_worlds`) ; 5 automatiques gardées par monde, les manuelles jamais supprimées. Faite avant de libérer l’instance, pour qu’on ne puisse pas relancer pendant la copie.
- Restaurer remplace le monde après en avoir sauvegardé l’état actuel (« avant restauration ») : une restauration s’annule toujours. Sauvegarde et restauration refusées pendant que le jeu tourne (fichiers en cours d’écriture).
- Pas de suppression de monde depuis Tandem pour l’instant (risque de perte, le Finder / l’Explorateur suffit).
- Statut : validé

### D23 · 2026-10-09 · Captures d’écran par instance
- Onglet « Captures » : les images de `screenshots/` (png/jpg), les plus récentes d’abord, datées par la date du fichier.
- Les images passent par un protocole maison `tandem-shot://localhost/<instance>/<fichier>` plutôt que le protocole `asset` de Tauri : le webview ne reçoit jamais de chemin arbitraire (instance et nom de fichier validés côté Rust), et la grille ne charge que ce qui est à l’écran (`loading="lazy"`), le navigateur limitant lui-même les requêtes en parallèle.
- Miniatures JPEG de 480 px de large générées à la demande (crate `image`, png + jpeg seulement) dans `cache/thumbnails/<instance>/`, refaites si la capture est plus récente, supprimées avec l’instance. URLs versionnées par la date de la capture → cache navigateur `immutable`.
- Suppression définitive (pas de corbeille) après confirmation ; « Afficher dans le dossier » pour le reste (copier, partager).
- Statut : validé

### D24 · 2026-10-09 · Démarrage : mesurer d’abord, fond de fenêtre sombre
- Le démarrage est loggé à chaque lancement : `backend ready ms=…` (fin de `setup`) et `UI ready ms=…` (premier rendu, signalé par le frontend via `ui_ready`), comptés depuis l’entrée dans `run`. Sert à repérer les régressions.
- Mesure de référence (release, Windows, à chaud) : ≈ 240-300 ms jusqu’à la première image, objectif < 1 s atteint. L’essentiel (≈ 150-170 ms) est la création de la fenêtre et du WebView2 par Tauri avant `setup` ; notre init en prend ≈ 5-8 ms. Pas d’optimisation spéculative au-delà.
- Fenêtre affichée tout de suite (retour visuel immédiat) mais avec `backgroundColor` = fond de l’app (#111316) : plus de fenêtre blanche pendant l’init du webview. Préféré à une fenêtre cachée jusqu’au premier rendu, qui retarderait le moment où l’app paraît lancée.
- Statut : validé

### D25 · 2026-10-09 · Flux à haut débit envoyés par paquets
- La sortie du jeu part du backend par paquets (`game://output` porte `lines: string[]`) : envoyés au plus 50 ms après leur première ligne, ou dès 500 lignes. Un événement par ligne coûtait ≈ 1,5 ms chacun dans l’UI (mise à jour + mise en page), soit des dizaines de secondes de gel au démarrage d’un gros modpack.
- Côté UI, un paquet = une seule mise à jour du store (lignes en trop retirées d’un coup) et un seul défilement. Les logs du launcher sont regroupés côté UI de la même façon (50 ms).
- Pas de virtualisation des listes pour l’instant : console plafonnée à 5000 lignes, contenu à 250 mods affiché en ≈ 80 ms. À revoir si une vue dépasse ≈ 100 ms au premier affichage.
- Statut : validé

### D26 · 2026-10-09 · Pas d'anneau de focus, composants de base maison
- Pas d'anneau de focus du navigateur (bordure jaune jugée trop « web » pour une app) : `outline: none` partout. Le focus clavier reprend l'état de survol de chaque contrôle (luminosité des boutons, fond des lignes, texte des onglets), pour que la navigation au clavier reste possible.
- Contrôles natifs du webview remplacés par des composants maison, identiques sur Windows et Mac : `Select` (liste rendue dans un portail en position fixe, fermée si la page défile ou la fenêtre change de taille), `Checkbox`, `Alert`, `Tabs`.
- Fenêtres modales : focus initial sur l'élément `autofocus` sinon le premier contrôle, Tab bouclé dedans, focus rendu à la fermeture ; seule la modale du dessus gère Tab. Menus et listes : Échap et clic extérieur, en phase de capture pour qu'un menu ouvert dans un dialogue se ferme sans fermer le dialogue.
- Statut : validé

### D27 · 2026-10-09 · Historique de navigation en mémoire
- Pile d'historique dans le store (50 pages max), pas l'API History du navigateur : WebView2 traite déjà lui-même les boutons de souris et Alt+← sur ses propres entrées, ce qui aurait déclenché deux retours. Tandem intercepte ces gestes (`preventDefault`) et gère seul sa pile, identique sur Windows et Mac.
- Gestes : boutons latéraux de la souris, Alt+←/→, ⌘[ / ⌘] sur Mac, flèches dans la barre de titre.
- Chaque entrée garde l'état de sa page via `remembered(clé, défaut)` et la position de défilement de la zone principale. La page est recréée à chaque navigation (même entre deux pages du même type) et repart de cet état.
- Une page d'instance supprimée depuis est sautée par Retour/Avance.
- Statut : validé

### D28 · 2026-10-09 · Retours d'action et suppressions annulables
- Une action réussie se confirme par une notification courte en bas à droite (4 s, 6 s avec un bouton) ; les erreurs liées à un écran restent dans un bandeau `Alert` refermable sur cet écran.
- Retirer un mod ou un compte n'ouvre pas de confirmation : l'élément disparaît tout de suite et la suppression n'est faite qu'après ~6 s, sauf clic sur « Annuler » dans la notification. Si le launcher est fermé entre-temps, rien n'est supprimé. Les suppressions lourdes (instance, capture) gardent leur boîte de confirmation.
- Une liste qui charge affiche des lignes fantômes, jamais un faux « Aucun… ».
- Réglage « Quand le jeu démarre » : rester ouvert (défaut) ou réduire le launcher, qui revient au premier plan quand le dernier jeu lancé ainsi se ferme.
- Statut : validé

### D29 · 2026-10-09 · Arrêter un jeu arrête tout ce qu'il a lancé
- Le jeu est placé dans un Job Object à son lancement (Windows) ou démarré comme chef de son propre groupe de processus (Unix). « Arrêter » termine tout le groupe : les assistants livrés par certains modpacks (Crash Assistant lance sa propre JVM via un processus intermédiaire, hors de portée de `taskkill /T`) ne survivent plus au jeu.
- Le Job Object ne tue pas le jeu quand Tandem se ferme (pas de `KILL_ON_JOB_CLOSE`) : fermer le launcher en pleine partie laisse la partie continuer.
- Une fois le jeu terminé, on n'attend la fin de sa sortie que 2 s : un processus enfant qui garde le tuyau ouvert ne peut plus bloquer la fin de partie (sauvegarde auto, panneau de crash).
- Client HTTP en fenêtre HTTP/2 adaptative : la fenêtre par défaut (64 Ko) limitait les téléchargements à ~7 Mo/s.
- Java lancé avec sa sortie en UTF-8 (`stdout.encoding` / `sun.stdout.encoding`), pour que la console affiche les accents.
- Statut : validé

### D30 · 2026-10-09 · Dépendances entre mods lues dans les jars
- Les dépendances viennent des métadonnées des jars (`fabric.mod.json`, `quilt.mod.json`, `mods.toml`, `neoforge.mods.toml`), pas de Modrinth : ça marche hors ligne, pour les mods ajoutés à la main, et ça colle à ce que le loader vérifiera. Ids du jeu, du loader et de Java ignorés ; ids des jars imbriqués (jar-in-jar, `META-INF/jarjar`) comptés comme fournis.
- Un mod est « nécessaire » si un mod activé exige un de ses ids et qu'aucun autre jar activé ne le fournit. Désactiver ou retirer un mod nécessaire demande confirmation (liste des mods concernés) ; un mod dont personne n'a besoin passe sans question.
- Lecture coûteuse sur les gros packs (1-3 s à chaud, 20+ s disque froid) : lecture parallèle et cache par jar (taille + date), préchauffé en arrière-plan à l'ouverture de l'onglet Contenu ; ensuite ~10 ms.
- Forge et NeoForge ne plantent pas quand une dépendance manque (écran d'erreur dans le jeu) : Tandem lit leur rapport dans la sortie du jeu et propose de réactiver le mod (arrêt, réactivation, relance) ou de le chercher sur Modrinth. Fabric quitte, le panneau de crash (D21) couvre déjà ce cas.
- Statut : validé

### D31 · 2026-10-09 · Pas d'installeur avant la connexion Microsoft
- Sans connexion Microsoft, Tandem ne fonctionne qu'en comptes hors ligne : distribuer un installeur reviendrait à laisser jouer sans posséder le jeu, contraire à D9.
- Donc : aucun build distribuable (jobs de build de la CI désactivés, aucun tag `v*`) tant que Mojang n'a pas répondu à la demande d'App ID et que la Phase 2 n'est pas faite. La CI continue le lint et les tests sur Windows et macOS.
- À la reprise : comptes hors ligne permis seulement après un compte Microsoft qui possède le jeu (comme Prism Launcher), réactiver les jobs de build, compléter la section « Comptes » de `docs/releases/v0.1.0.md`, puis pousser le tag `v0.1.0` (brouillon de Release à relire et publier à la main).
- Statut : validé

### D32 · 2026-10-09 · Fiches de contenu : HTML nettoyé côté Rust, cache en mémoire
- Les descriptions et journaux Modrinth (Markdown mêlé de HTML) sont rendus et nettoyés en Rust (`pulldown-cmark` + `ammonia`), testables sans interface ; le frontend affiche le HTML tel quel. Autorisé : la mise en forme courante, `align`, `details`. Retiré : scripts, styles, formulaires, cadres. Les vidéos YouTube deviennent une vignette qui ouvre la vidéo dans le navigateur ; les autres cadres, un simple lien.
- Les liens vers un projet Modrinth s'ouvrent dans Tandem, les autres dans le navigateur.
- Compatibilité calculée en Rust : contenu = version du jeu de l'instance (+ loader pour les mods, Quilt accepte Fabric) ; modpack = loader que Tandem sait lancer. Version proposée : la plus récente stable compatible, sinon la plus récente compatible.
- Cache mémoire de 10 min (pages, versions, résumés de projets), pas de cache disque : les fiches changent souvent et la mémoire reste petite. La liste des versions d'un gros modpack pèse ~5 Mo, d'où le cache.
- Changer la version d'un contenu déjà installé passe par le retirer puis l'installer (pas de « rétrograder » pour l'instant).
- Statut : validé

### D33 · 2026-10-10 · Traduction IA : par texte, réversible, sans clé à nous
- Sources : langues des mods activés, des packs de ressources activés (ordre de `options.txt`, le dernier gagne), KubeJS, FTB Quests, Patchouli. Les textes déjà traduits par les auteurs sont gardés ; une « traduction » identique à une phrase anglaise (3 mots et plus) compte comme manquante.
- Unité de cache : le texte anglais (SHA-1) + la langue, en SQLite. Un même texte n'est traduit qu'une fois pour tous les mods, instances et versions. Les corrections du joueur (`manual`) ne sont jamais remplacées par l'IA.
- Envoi au modèle : lots par mod (30 textes, ~1 800 caractères), codes du jeu remplacés par des jetons `⟦n⟧` et vérifiés au retour (manquant, en double ou inventé = rejet, puis une seconde tentative en petits lots). Clé de traduction envoyée comme identifiant (indice du sens). Glossaire envoyé seulement pour les termes présents dans le lot : noms officiels de Minecraft (lus dans les fichiers du jeu), termes courts déjà traduits par les auteurs du pack, termes du joueur. Ordre : noms d'objets, quêtes, interface, livres ; on peut arrêter à tout moment.
- Mesuré avec qwen2.5:7b sur RTX 4060 : ~40-50 tokens/s, 90 textes en 29 s. Prominence II : ~50 000 textes, ~710 000 tokens en sortie, soit des heures en local : d'où l'estimation affichée, la priorité et le choix des sources.
- Sortie réversible : pack `resourcepacks/tandem-translation-<langue>/` inséré sous les packs du joueur (leurs traductions humaines gagnent) et accepté dans `incompatibleResourcePacks` ; fichiers de quêtes `lang/<langue>.snbt` créés à côté ; quêtes écrites dans les chapitres traduites sur place avec l'original dans `.tandem/originals/`. Un manifeste (`.tandem/translation.json`) liste ce qui a été écrit (SHA-1) : désactiver restaure tout, y compris la langue du jeu d'avant. Un fichier modifié depuis par quelqu'un d'autre n'est jamais écrasé.
- Services : API compatible OpenAI uniquement (Ollama, LM Studio et la plupart des services en ligne la proposent). Tandem n'embarque aucune clé ; celle du joueur va dans le coffre du système (crate `keyring`), jamais en base. Pas de prix affichés (ils changent) : tokens et durée estimée à partir de la vitesse mesurée au dernier passage.
- Statut : validé


### D34 · 2026-10-10 · Import CurseForge : clé du joueur, fichiers bloqués retrouvés ailleurs
- Le dépôt est public : une clé CurseForge embarquée serait lisible par tous (et ses conditions la réservent au projet qui l'a demandée). Comme pour la traduction (D33), le joueur colle sa propre clé, vérifiée auprès de l'API (`/games/432`) avant d'être rangée dans le coffre du système. Elle n'est demandée qu'au premier import d'un `.zip` CurseForge, dans un dialogue, ou dans Réglages.
- Un seul bouton « Importer un modpack » : le fichier est reconnu à son contenu (`manifest.json` → CurseForge, sinon `.mrpack`), pas à son extension.
- Fichiers dont l'auteur interdit la distribution par des tiers (pas d'URL dans l'API) : d'abord cherchés sur Modrinth par SHA-1 (même fichier, au bit près) ; le reste est listé dans l'instance (`.tandem/manual-downloads.json`) avec un lien vers la page exacte du fichier. Tandem surveille le dossier Téléchargements (toutes les 3 s pendant que le panneau est affiché, et au retour dans la fenêtre) et y reprend chaque fichier reconnu par taille puis SHA-1, même renommé par le navigateur.
- Statut : abandonné le 2026-10-11 (D41), code retiré

### D35 · 2026-10-10 · Import depuis d'autres launchers : copie, jamais de lien vers l'original
- Sources lues à leur place habituelle : Modrinth App (sa base `app.db` ouverte en lecture seule, `immutable`, sans prendre son verrou ; schéma actuel `instances` + `instance_content_sets` + `instance_links`, ancien schéma `profiles` en secours), Prism / PolyMC (`instance.cfg` + `mmc-pack.json`, dossier d'instances personnalisé respecté), app CurseForge (`minecraftinstance.json`), launcher officiel (`launcher_profiles.json`, version et loader lus dans `versions/<id>/<id>.json` → `inheritsFrom`). MultiMC portable ou dossier ailleurs : « Choisir un dossier… » reconnaît un dossier de launcher, une instance, ou un simple dossier de jeu (version et loader devinés d'après les mods, identifiés sur Modrinth par SHA-1, et la version des mondes).
- L'instance est **copiée** (jamais déplacée ni liée) : l'autre launcher continue de marcher, rien n'est perdu si l'import échoue (l'instance Tandem est alors supprimée). Mods, packs de ressources et shaders passent par le store (dédupliqués avec les autres instances), puis sont identifiés sur Modrinth pour les mises à jour. Le modpack Modrinth d'origine (Modrinth App, Prism) est repris si Modrinth connaît encore la version : la mise à jour du modpack marche ensuite dans Tandem.
- Pas copiés : journaux, rapports de crash, caches, et dans un `.minecraft` officiel les fichiers du launcher (`versions`, `libraries`, `assets`, `launcher_*`). Une instance vanilla ne prend pas le dossier `mods`.
- Mesuré : DawnCraft (CurseForge, Forge 1.18.2, 298 mods) : 5 305 fichiers, 1,1 Go copiés en 16 s, 162 fichiers reconnus sur Modrinth ; le jeu importé atteint le menu (69 s de chargement).
- Statut : validé

### D36 · 2026-10-10 · Connexion Microsoft : navigateur + PKCE, jetons découpés dans le coffre, hors ligne après possession
- Connexion dans le **navigateur système** (D9) : code d'autorisation + PKCE (S256), réponse reçue par un petit serveur local d'un seul usage sur `http://localhost:<port aléatoire>` (l'application Azure accepte `http://localhost` sur tout port). Pas de code à recopier ; `state` vérifié ; page « Connexion réussie » dans l'onglet. Annulable, abandonnée après 15 min.
- Chaîne : Microsoft → Xbox Live → XSTS → `login_with_xbox` → droits (`entitlements/mcstore`) + profil. Un compte qui ne possède pas le jeu est refusé (D9) ; erreurs XSTS expliquées (pas de profil Xbox, compte enfant, pays, vérification d'âge) avec le lien utile.
- Jetons (refresh Microsoft + jeton Minecraft et son expiration) dans le coffre du système **uniquement**, découpés en morceaux de 1 000 caractères (Windows limite un secret à 2 560 octets). La base ne garde que le pseudo, l'UUID et la tête du skin. Jeton renouvelé avant le lancement s'il expire dans moins de 10 min (une seule renouvellement à la fois : Microsoft fait tourner le refresh token) ; sans réseau, l'ancien jeton sert (le solo marche) ; refresh refusé → « session expirée » et la fenêtre Comptes s'ouvre.
- Tête du joueur : skin officiel téléchargé depuis `textures.minecraft.net` uniquement, visage 8×8 + calque du chapeau découpés en local (pas de service tiers type Crafatar qui recevrait l'UUID).
- Comptes hors ligne : ajout et lancement permis seulement si un compte Microsoft (qui possède forcément le jeu) est connecté, comme Prism. Les outils de développement (exemples) n'appliquent pas cette règle.
- Interface : fenêtre « Comptes » unique (liste, connexion pas à pas, hors ligne), carte de la barre latérale avec la vraie tête et changement rapide, bouton blanc « Se connecter avec Microsoft » au logo officiel (seul logo de marque utilisé, comme le demandent les règles de Microsoft pour ce bouton).
- Statut : validé

### D37 · 2026-10-10 · Jeu à deux : tunnel du LAN vanilla, code court sans serveur Tandem
- Complète D5. L'hôte ouvre son monde au réseau local dans le jeu ; Tandem repère l'annonce LAN de **ce** PC (le jeu répond sur `127.0.0.1`) et relaie chaque connexion TCP du jeu dans un flux QUIC iroh (ALPN `tandem/duo/1`). Chez l'invité, un faux serveur local transmet au flux, et une annonce LAN fait apparaître la partie dans « Parties en réseau local ». Tandem peut aussi lancer le jeu directement dessus.
- Code d'invitation `XXXX-XXXX` (40 bits, alphabet de Crockford : pas de I, L, O ni U ; les lettres qui ressemblent à des chiffres sont lues comme ces chiffres). L'hôte signe « mon endpoint iroh est X » avec une clé dérivée du code (SHA-256) et le publie sur le relais pkarr public d'iroh (`dns.iroh.link`, adossé au DHT Mainline). L'invité recalcule la clé et lit l'enregistrement : sans le code, impossible de trouver l'hôte. Il prouve ensuite qu'il connaît le code (empreinte envoyée dans `hello`).
- Le code suffit pour entrer ; l'hôte voit les arrivées et peut exclure. Relais et mise en relation : serveurs publics et gratuits de n0, pas de serveur Tandem.
- Comptes Microsoft obligatoires des deux côtés (le monde LAN vérifie les comptes auprès de Mojang) ; pas de contournement.
- Mesuré (preuve de concept, iroh 1.3, deux points sur un même PC) : code résolu en 0,2 s, connexion en 0,7 s, chemin direct choisi automatiquement.
- Statut : validé (périmètre 6a avec Zeleff)


### D38 · 2026-10-10 · Jeu à deux : moins de clics, protocole compatible
- Retours du premier test réel (Zeleff) : ce qui gêne, c'est d'avoir le même modpack, le nombre de clics et l'étape « Ouvrir au réseau local ».
- L'invité ne choisit plus d'instance avant de rejoindre : après l'accueil, Tandem prend celle qui a la version et le loader de l'hôte, en préférant le même modpack Modrinth (même version d'abord), puis le moins de mods différents. Elle reste modifiable. Le jeu se lance tout seul dès que le monde est ouvert, seulement si les instances correspondent (réglage `duo_auto_launch`, activé par défaut).
- Protocole : nouveau message `instance` (invité → hôte) et champ facultatif `pack` dans le résumé d'instance. `VERSION` reste à 1 : un hôte plus ancien ignore le message, un résumé sans `pack` se lit toujours (testé).
- Hôte : un seul bouton crée l'invitation et lance le jeu directement dans le monde choisi (`--quickPlaySingleplayer`, 1.20+ et versions numérotées par année ; avant, menu du jeu). On ne peut pas ouvrir le monde en LAN à la place du joueur sans mod (`/publish` exige les cheats) : Tandem repère l'entrée dans le monde dans le journal du jeu (`Starting integrated minecraft server`) et envoie une notification système de rappel ; arrivées et départs aussi notifiés, seulement quand le launcher n'est pas au premier plan (`tauri-plugin-notification`).
- Statut : validé

### D39 · 2026-10-10 · Jeu à deux : l'invité reçoit une copie de l'instance de l'hôte (6b)
- Jamais de modification des instances du joueur : la synchro ne s'applique qu'à une **copie** créée par Tandem (nommée « <instance> (<hôte>) », retrouvée par le réglage `duo_copy:<hôte>:<instance>` et mise à jour la fois suivante).
- Création : la version exacte du modpack Modrinth de l'hôte si son instance en vient (`pack` du résumé, D38) ; sinon une instance vide de même version de Minecraft, même loader et même version du loader.
- Mods : identiques au fichier près (SHA-1). Modrinth d'abord (`version_files`, plus rapide, épargne l'envoi de l'hôte), le reste demandé à l'hôte par un flux `M` (il ne sert que les fichiers de son résumé, l'invité vérifie l'empreinte). Mods en trop renommés `.disabled`.
- Fichiers de pack : les mods ne suffisent pas toujours (scripts KubeJS/CraftTweaker qui ajoutent des objets, datapacks globaux, configs partagées). Dossiers recopiés quand ils diffèrent : `config`, `defaultconfigs`, `kubejs`, `scripts`, `global_packs`, `datapacks`. `kubejs` et `scripts` sont rendus identiques (fichiers en trop supprimés). Flux `F` (liste) puis `P` (un fichier par chemin) ; l'hôte ne sert que les chemins listés dans ces dossiers, sans `..` ni chemin absolu.
- Pas de sauvegardes, d'options du joueur ni de packs de ressources : la copie sert à rejoindre, pas à cloner.
- Compatibilité : `VERSION` reste 1 ; un hôte plus ancien ferme les flux inconnus, l'invité le dit (« la version de Tandem de l'hôte ne sait pas encore… ») et garde ce qu'il a pu obtenir.
- Statut : validé

### D40 · 2026-10-11 · Optimiser l'hébergement : mini-mod côté hôte, génération coopérative mise de côté
- Constat (profilage JFR sur deux packs, [TEST-DUO.md](TEST-DUO.md#optimisation--où-part-le-temps-du-serveur-11-octobre-2026)) : une fois le gaspillage retiré, le tick est surtout du vrai travail (IA des entités). Les gains restants viennent de corrections ciblées côté serveur, pas de réglages (distance de simulation, VMP : peu d'effet).
- Retenu : un **mini-mod ajouté par Tandem seulement pendant une session d'hôte**, côté serveur (rien chez les amis). Fabric d'abord, compilé contre les noms intermédiaires (un même jar de 1.20 à 1.21), NeoForge ensuite. Contenu : apparitions qui ne peuvent rien produire sautées, bibliothèques lentes corrigées (Architectury, Balm), puis cache de la recherche d'apparition et distance d'activation des entités. Uniquement des changements sans effet visible en jeu, ou réglables.
- Priorité ensuite au ressenti des groupes courants (2 à 5 amis) : mémoire et fluidité du jeu de l'hôte, pré-génération en un clic.
- Mis de côté : génération du terrain par les amis (faisable : la génération de Minecraft n'est de toute façon pas reproductible, et l'injection de chunks dans l'hôte en marche est prouvée), gain trop étroit sous la limite LAN de 8 joueurs face à son coût. Écartés : simulation partagée entre PC, mobs confiés aux amis (cassent les mods), compression ou relais réseau (coût CPU nul).
- Statut : validé ; mini-mod réalisé en D42 (ajouté à chaque lancement, pas seulement pendant une session d'hôte)

### D41 · 2026-10-11 · Pas de CurseForge : Modrinth seule source de contenu
- Choix du mainteneur : Tandem n'utilise pas l'API CurseForge et ne télécharge rien depuis CurseForge. L'import de modpacks CurseForge (D34 : clé du joueur, panneau des téléchargements à la main) est retiré.
- Considérations aussi pesées : leurs conditions d'API (pas de cache des données, clé non partageable, pas de produit concurrent, accès révocable sans motif) et l'approbation demandée, mal adaptées à un lanceur public.
- Modrinth suffit comme source : très bien fournie en Fabric, Quilt et NeoForge récent (le choix par défaut à partir de 1.21) ; le manque se limite aux vieux modpacks Forge exclusifs à CurseForge. FTB écarté aussi (son app passe par Overwolf, ses packs récupèrent leurs mods sur CurseForge).
- Gardé : l'import depuis les autres launchers (D35), y compris l'app CurseForge. Tandem copie des fichiers déjà présents sur le PC du joueur, sans contacter CurseForge. Un `.zip` CurseForge choisi dans « Importer un modpack » est refusé avec un message qui oriente vers cet import.
- Principe (tranché par Zeleff) : **Tandem ne récupère jamais rien lui-même depuis CurseForge, mais les joueurs restent libres** d'importer ce qu'ils ont, d'où que ça vienne. Restreindre ce que le joueur fait de ses propres fichiers rendrait le launcher frustrant. Donc, dans « Jouer ensemble », l'hôte continue d'envoyer à ses amis les fichiers de son instance que Modrinth ne connaît pas (D39), même s'ils viennent à l'origine de CurseForge : ce sont ses fichiers, Tandem ne fait que les transmettre entre amis.
- Statut : validé


### D42 · 2026-10-11 · Tandem Optimizations : un mod ajouté au lancement, Fabric et Quilt d'abord
- Suite de D40. Le mini-mod devient **Tandem Optimizations** (`mods/tandem-optimizations/`) : il sert aussi en solo (le monde solo tourne sur un serveur intégré), pas seulement quand on héberge. Activé par défaut pour toutes les instances Fabric et Quilt de 1.20 à 1.21.11, coupable dans les Réglages et par instance.
- Ajouté par `-Dfabric.addMods=<jar>` (`-Dloader.addMods` pour Quilt) : rien n'est écrit dans l'instance, rien à nettoyer après un plantage, et l'export ou le partage d'une instance ne l'emporte pas. Jar embarqué dans l'app, écrit dans `<données>/mods/`. Réglages passés en propriétés système ; les arguments JVM du joueur viennent après et peuvent les changer.
- Construction : `javac` contre un jar de Minecraft aux noms intermédiaires, sans Gradle ni Loom (`build.py`). Le jar est reproductible et versionné dans le dépôt : la CI et la compilation de Tandem n'ont pas besoin de Java. Avant chaque nouveau jar, `build.py --check` le compile contre chaque version de 1.20 à 1.21.11, cherche les cibles des Mixins et vérifie que chaque méthode utilisée existe partout (cette vérification a trouvé deux cas qui faisaient planter ou ne compilaient pas en 1.21.5+ et 1.21.9+). En jeu, une méthode manquante coupe l'optimisation concernée au lieu de faire planter.
- Contenu de la v1 : catégories d'apparition mortes, correctifs Architectury et Balm, **distance d'activation limitée aux créatures vanilla**. Les créatures des mods tickent toujours normalement : un mod peut compter le temps dans le tick de sa créature (mesuré : Cobblemon fait vieillir ses Pokémon ainsi, ralentis ils restaient quatre fois plus longtemps, +75 % de Pokémon). Exclues aussi les créatures vanilla dont un minuteur se voit en jeu (poules, zombies-villageois, piglins, hoglins, tatous) et tout ce qui est occupé (combat, eau, monture, apprivoisé…). Seul effet visible : les créatures inactives au loin bougent plus lentement, d'où un réglage à part.
- Écarté : cache de la recherche d'apparition (le profil montre des lectures à des positions aléatoires, un cache exact ne retire que 1 à 2 %).
- 26.1 et plus : Minecraft n'est plus obfusqué, Fabric n'a plus de noms intermédiaires. Il faudra un second jar aux noms de Mojang, qui servira aussi pour NeoForge (mêmes noms depuis 1.20.5). Forge 1.20.1 (noms SRG) seulement si le besoin existe.
- Statut : validé (feu vert de Zeleff pour la session, 2026-10-11)

### D43 · 2026-10-11 · Héberger le monde à part, puis un monde qui appartient au groupe
- Après le mini-mod, les gains restants de « Jouer ensemble » ne sont plus dans le temps de tick mais dans la fiabilité : la partie s'arrête si l'hôte part ou si son jeu plante, l'hôte doit ouvrir le monde en LAN, 8 joueurs au plus.
- 6e : le monde tourne dans un serveur invisible lancé par Tandem quand on héberge (Fabric/Quilt d'abord), le jeu de l'hôte le rejoint comme les amis. Coût mesuré : +0,7 à +1,5 Go de mémoire, d'où un réglage facultatif au début.
- 6f : le monde est recopié chez les amis pendant la partie, avec un verrou publié sur le relais pkarr ; reprise par un autre si l'hôte part, ouverture sans l'hôte. Toujours sans serveur Tandem.
- Écarté pour l'instant : un PC par dimension (les mods qui relient les dimensions casseraient), cache de chunks chez les amis (à reconsidérer après avoir mesuré le trafic réel).
- Statut : validé ; recadré par D44 (6e à terminer, 6f plus tard)

### D44 · 2026-10-11 · Fin de l'exploration : une feuille de route, un chantier à la fois
- L'exploration voulue « jusqu'à l'excès » a ouvert plusieurs chantiers en parallèle (mini-mod, monde à part, monde du groupe) et cherché à repousser le nombre de joueurs. Bilan : les gains de vitesse sont acquis (mini-mod) ; le monde à part apporte surtout du confort ; au-delà de 20 à 30 joueurs sur un pack moyen, il faudrait répartir le monde entre plusieurs PC, ce que les modpacks ne supportent pas.
- Décision de Zeleff : plus de nouvel angle d'optimisation ni de course au nombre de joueurs. On termine dans l'ordre : Tandem Optimizations v1, monde à part, pré-génération en un clic, indicateur de charge, puis éventuellement le monde du groupe (6f). Chaque chantier est testé en vrai et documenté avant le suivant, pour éviter le surdimensionnement.
- Mis de côté : monde réparti entre plusieurs PC, un PC par dimension, génération coopérative, cache de chunks chez les amis, NeoForge et 26.x (après la feuille de route).
- Statut : validé

### D45 · 2026-10-12 · Monde à part : dossiers recopiés, moins de mémoire pour le jeu
- Les dossiers de l'instance (configs, scripts, données de mods) étaient liés dans le serveur : le serveur et le jeu de l'hôte écrivaient alors les mêmes fichiers en même temps (TreeChop ne pouvait plus enregistrer sa config sur Prominence II, risque de fichier abîmé). Ils sont maintenant recopiés à chaque démarrage du serveur : petits fichiers copiés, fichiers de 1 Mo et plus en lien physique (gratuits en disque). L'instance fait foi : un fichier qui diffère est recopié, ceux que le serveur crée lui-même restent. Les mods restent liés comme avant.
- Un jeu lancé directement sur un serveur (hôte qui rejoint son monde à part, invités) n'a pas de serveur intégré : 1 Go de moins que la mémoire automatique (pas si le joueur a choisi la sienne). Mesuré avec un joueur : le serveur intégré occupe 0,5 Go réellement utilisés sur Cobbleverse, 0,35 sur Prominence II ; le jeu garde au moins deux fois ce qu'il utilise. Sur 16 Go, le serveur reçoit ainsi ses 4 Go au lieu de 3,5.
- Choix de Zeleff : le monde à part reste une bêta facultative (désactivée par défaut), même au-delà de quelques amis.
- Statut : validé

### D46 · 2026-10-12 · Préparer le monde : Chunky dans le serveur du monde à part
- La génération du terrain neuf est le plus gros coût quand plusieurs joueurs explorent (TEST-DUO : à 6 joueurs, 37 ms de tick sur terrain neuf contre 18 sur une zone générée à l'avance). « Préparer » génère un carré de 1 000, 2 000 ou 4 000 blocs de côté autour du point d'apparition, jeu fermé.
- Moteur : le serveur du monde à part (D43, Fabric et Quilt), avec Chunky ajouté au lancement par `addMods` comme Tandem Optimizations. Chunky (GPL-3.0) est téléchargé depuis Modrinth à la première utilisation, pas embarqué dans Tandem ; il n'est pas ajouté si l'instance l'a déjà. Il faut Fabric API dans l'instance (dépendance de Chunky), sinon un message clair. Tandem pilote Chunky par la console et lit ses lignes de progression.
- Annuler met la tâche en pause et arrête proprement le serveur ; une nouvelle préparation repasse vite sur ce qui existe déjà.
- Écarté : écrire notre propre générateur dans Tandem Optimizations (Chunky est la référence, stable sur toutes les versions), et Forge/NeoForge pour l'instant (pas de serveur à part pour eux).
- Statut : validé

### D47 · 2026-10-12 · Indicateur de charge : relevé du mod, repli sur Minecraft
- L'hôte voit pendant une session si son PC suit : santé du monde (fluide sous 30 ms par tick, chargé jusqu'à 45, en retard au-delà ou dès que Minecraft signale du retard) et mémoire libre du PC (juste sous 2 Go, presque pleine sous 1 Go), avec un seul conseil à la fois (fermer des programmes, rester moins éloignés, préparer le monde).
- Mesure : Tandem Optimizations 1.1 chronomètre chaque tick du serveur et écrit une ligne toutes les 100 ticks, seulement quand le monde est partagé (serveur dédié ou monde ouvert en LAN) pour ne pas remplir les journaux en solo. Tandem lit la console du monde à part et la sortie du jeu. Sans le mod (Forge, NeoForge, mod coupé) : le message « Can't keep up! » de Minecraft, qui n'arrive qu'une fois le retard pris.
- Écarté : `tick query` (seulement depuis 1.20.3, et pas dans un monde LAN), profiler le serveur (trop lourd pour un indicateur).
- Statut : validé

### D48 · 2026-10-12 · Pas de monde partagé par le groupe (6f abandonné)
- Le chantier 5 (copie du monde chez les amis, reprise si l'hôte part, ouverture sans l'hôte) est abandonné, pas seulement repoussé.
- Raison (Zeleff) : un groupe laisse héberger celui qui a le meilleur PC, c'est le fonctionnement naturel. Le gain (jouer sans l'hôte, sauvegarde chez les amis) est rare ; le coût est le plus élevé de la feuille de route : synchroniser un monde de plusieurs Go pendant la partie, verrou pour éviter deux versions du monde, risque de monde abîmé, connexion de l'hôte et disque des amis.
- La feuille de route « Jouer ensemble » (D44) est donc terminée. Reste possible, si le besoin revient : un simple « Envoyer mon monde à un ami » (copie manuelle).
- Statut : validé


### D49 · 2026-10-12 · Fondations de la refonte : « le Minecraft moderne », titres en Jersey 10
- Couleurs : surfaces graphite en paliers nets (fond `#1b1c1f`, panneaux `#26272b`, boutons `#383a3f`), plus claires que « Deepslate » ; vert Minecraft `#3c8527` pour l'action principale, bouton clair « pierre » `#d0d1d4` pour l'action secondaire forte, rouge plein pour supprimer et arrêter.
- Boutons à la manière des menus récents de Mojang : face pleine, contour sombre, bande plus foncée en bas qui disparaît quand on appuie (le bouton s'enfonce). Coins presque carrés (2 à 6 px) à la place des coins en escalier ; les classes `.btn*`, `.panel`, `.field`, `.slot`, `.chip`, `.px-corners*` gardent leur nom, l'habillage change partout d'un coup.
- Texte : Geist pour tout, échelle courte (12, 13, 14, 15, 17, 20, 24, 32, 40, 52 px), plus de 10 ou 11 px. Grands titres seulement en **Jersey 10** (OFL, embarquée), choisie par Zeleff parmi Jersey 10, Russo One, Chakra Petch et Bungee : la plus proche des titres de Mojang, nette en grand. Pixelify Sans est retirée.
- Icônes en pixels affichées à 12 ou 24 px seulement (pixels entiers).
- Page de démonstration hors app : `pnpm dev` puis `/?fondations`.
- D11 reste en vigueur pour ce que les étapes suivantes n'ont pas encore refait ; une décision finale la remplacera à l'étape 8 avec `DESIGN.md`.
- Statut : validé

### D50 · 2026-10-12 · Système visuel « le Minecraft moderne » (remplace D11)
- La refonte de la Phase 7 est terminée ; les jetons du système sont dans `src/index.css` (sa description `DESIGN.md` reste hors du dépôt, avec les fichiers de l'outil de design).
- Repris de D49 : surfaces graphite en paliers, boutons pleins qui s'enfoncent, vert pour l'action principale, coins presque carrés, Geist partout et Jersey 10 seulement pour les grands titres.
- Ajouté pendant la refonte : jetons de texte `meta` (13), `title` (44, titres de page) et `hero` (64, accueil) ; un seul visuel d'instance partout (image mise en avant du modpack, sinon dernière capture, sinon un ciel dessiné de jour, de couchant ou de nuit selon l'instance) ; jamais d'étiquette au-dessus d'un titre ; l'or seulement pour les avertissements ; « Supprimer l'instance » dans l'onglet Réglages, plus dans l'en-tête ; pixel art (icônes, blocs, têtes, loaders) seulement à des tailles entières.
- Revue indépendante (outil impeccable) : 8 corrections demandées, toutes faites, puis 2 retouches.
- Page de démonstration des composants : `pnpm dev` puis `/?fondations`.
- Statut : validé

### D51 · 2026-10-14 · Découvrir : tri, catégories et rayons tirés de Modrinth
- Tout vient de la recherche Modrinth (`index` pour l'ordre, facettes pour les catégories et la version) : pas de serveur ni de calcul à Tandem.
- Pas de « tendances » ni de note : Modrinth n'en donne pas, et les calculer demanderait de suivre les chiffres dans le temps. « Les plus suivis » sert de « plus appréciés ».
- Plancher de 10 abonnés pour « Nouveautés » et « Mis à jour récemment » sans recherche (facette `follows>=10`) : sans lui, les premiers résultats sont des tests et des packs de serveurs privés. Une recherche tapée n'a pas de plancher.
- Les catégories proposées sont écrites dans l'app (`src/lib/discover.ts`), par type : la liste de Modrinth bouge peu et cela évite une requête.
- Rayons gardés 10 minutes en mémoire, comme les fiches (D32).
- Statut : validé

### D52 · 2026-10-14 · Carnet de partie : seulement ce que le jeu enregistre déjà
- Sources : `level.dat`, `stats/`, `advancements/` et FTB Quests. Rien n'est ajouté au jeu (pas de mod, pas de suivi par Tandem) : le carnet marche aussi pour les mondes joués avant Tandem.
- Pas de « X progrès sur Y » : les mods déclarent des centaines de progrès (souvent des recettes déguisées) et le total n'est connu que du jeu. À la place, 9 grandes étapes vanilla choisies, datées par leurs critères.
- Quêtes : on compte les quêtes du livre (chapitres `always_invisible` et quêtes `invisible` écartés) faites par l'équipe la plus avancée du monde. Seulement le format FTB Quests 1.16+ (`chapters/*.snbt`) ; les autres livres (Heracles, Better Questing, FTB Quests 1.12) ne sont pas lus.
- Joueur retenu : celui qui a le plus de temps de jeu dans le monde (un monde peut avoir connu plusieurs comptes).
- Statut : validé

### D53 · 2026-10-14 · Fiche de modpack : des repères honnêtes, sans télécharger le pack
- Tout vient de Modrinth et de la machine, sans télécharger le `.mrpack` (jusqu'à des dizaines de Mo) juste pour afficher une fiche.
- Nombre de mods : les mods inclus publiés sur Modrinth. Un pack peut en ajouter d'autres directement dans ses fichiers (souvent ceux de CurseForge) : l'info-bulle le dit.
- Mémoire : la règle de la mémoire automatique (D18) appliquée à ce nombre de mods, comparée à ce qu'elle donnerait sur ce PC. « Un peu juste » à partir de 75 % du souhaité, « Trop lourd » en dessous. Modrinth ne donne aucune exigence officielle.
- Quêtes : catégorie « Quêtes » du pack, ou un mod de quêtes parmi les inclus (FTB Quests n'est pas sur Modrinth, ses compagnons si). Sinon « Pas annoncées », jamais « Aucune ».
- Packs proches : recherche par ses deux premières catégories et sa version du jeu (une seule catégorie s'il y a moins de 3 résultats), pas par mods en commun (trop coûteux).
- Statut : validé


### D54 · 2026-10-14 · Textes de l'interface : une ligne, seulement si elle apprend quelque chose
- Un réglage ou une section n'a une phrase d'aide que si elle dit ce que le libellé et l'option ne disent pas déjà (« Pour les joueurs avancés. », « Mondes compris. »). Une ligne, pas de justification technique.
- Pas de sous-titre sous les titres de page (Instances, Jouer ensemble, Réglages…) : le titre et les boutons suffisent.
- Les détails (pourquoi un relais, comment marche un mod) vont dans une info-bulle ou nulle part.
- Une information apparaît là où on agit dessus : l'aide d'un service de traduction sous le choix du service, Dupliquer et Supprimer ensemble dans « Gestion ».
- Onglets d'instance : Contenu d'abord (onglet par défaut), Console en avant-dernier, Réglages en dernier.
- Statut : validé

### D55 · 2026-10-20 · Mises à jour de Tandem et une seule fenêtre
- Mises à jour par le plugin `updater` de Tauri, côté Rust (commandes `check_update` / `install_update`) : aucune requête réseau depuis l'interface.
- Source : `latest.json` attaché à chaque GitHub Release par `release.yml`, lu via le lien « dernière release » de GitHub. Ce lien ignore les brouillons et les pré-versions : les versions 0.x ne sont donc plus marquées « pré-version », sinon personne ne recevrait la mise à jour. Publier le brouillon = la proposer à tous.
- Fichiers signés (minisign). Clé privée hors du dépôt (`~/.tauri/tandem.key` chez Zeleff, secret `TAURI_SIGNING_PRIVATE_KEY` sur GitHub) ; clé publique dans `tauri.conf.json`. Perdre la clé privée = plus aucune mise à jour possible pour les launchers déjà installés : la sauvegarder.
- Vérification discrète 5 s après le démarrage (pas de message si elle échoue), et bouton « Vérifier » dans Réglages. Une pastille dans la barre de titre annonce la nouvelle version ; rien ne s'installe sans clic.
- Installation refusée pendant qu'un jeu tourne ou s'installe, ou pendant une partie avec des amis : l'installeur ferme Tandem. Windows : installeur NSIS en mode « passive » (barre de progression, pas de questions). macOS : `.app.tar.gz` universel.
- Les builds de test de la CI ne sont pas signées (`createUpdaterArtifacts` coupé) : seules les releases le sont.
- Une seule fenêtre : plugin `single-instance`, chargé en premier. Relancer Tandem ramène la fenêtre ouverte : deux launchers partageraient la même base et les mêmes instances.
- Statut : validé

### D56 · 2026-10-20 · Page Apparence : skins et capes
- Ouverte depuis le menu du compte (le skin appartient au compte), pas dans la navigation principale. Comptes Microsoft seulement ; un compte hors ligne garde le skin par défaut.
- API officielle de Mojang : profil (skin et capes), envoi d'un skin (PNG + modèle « classic » ou « slim »), cape active, retour au skin par défaut. Textures téléchargées seulement depuis `textures.minecraft.net`, côté Rust, et données à l'interface en data URL.
- Bibliothèque sur le PC : `<données>/skins/<sha1>.png`, noms et modèles dans le réglage `skin_library`. Un même skin n'est gardé qu'une fois. Ajouts : fichier PNG (bouton ou glisser-déposer sur la fenêtre), skin actuel d'un joueur par son pseudo (API publique de Mojang). Modèle deviné d'après la largeur du bras, modifiable.
- Skins par défaut (les 9 du jeu, deux modèles) : lus dans un jar du jeu 1.19.3+ déjà installé, copiés une fois dans `skins/defaults`. Rien d'embarqué dans Tandem ; sans jar récent, un bouton « Remettre le skin par défaut ».
- Pas de recherche dans NameMC ou les autres galeries : aucune n'a d'API publique, et lire leurs pages irait contre leurs conditions et casserait au moindre changement. Packs de skins d'événement de Mojang (MINECON…) : pas avant d'avoir vérifié la provenance des fichiers et les règles d'utilisation.
- Aperçu 3D avec skinview3d (MIT, three.js), chargé seulement à l'ouverture de la page.
- Statut : validé

### D57 · 2026-10-20 · Premier lancement guidé
- Affiché une seule fois, sur une installation neuve : aucun compte et aucune instance. Quelqu'un qui passe d'une ancienne version de Tandem ne le voit pas (réglage `onboarding_done` posé d'office). « Passer » toujours visible ; la barre de titre reste accessible.
- Trois étapes au plus : compte Microsoft (connexion habituelle, « Plus tard » possible), parties des autres launchers (seulement s'il y en a, à cocher, copiées), première partie. L'import d'au moins une partie termine le parcours : il y a déjà de quoi jouer.
- Première partie : Minecraft dernière version tel quel, « Minecraft optimisé » (Fabulously Optimized, installé en arrière-plan avec la progression dans la barre de titre), ou Découvrir sur les modpacks.
- Pas d'étape mémoire : la mémoire automatique (D18) convient, la changer reste dans les réglages de l'instance.
- Statut : validé

### D58 · 2026-10-20 · Un pack de skins à nous plutôt que ceux de Mojang
- Les packs d'événement de Mojang (MINECON Earth 2017, Builders & Biomes…) ne sont pas proposés : le contrat de licence de Minecraft interdit de distribuer ce que Mojang a fait sans son accord, contenu du Marketplace compris. Modrinth App les embarque quand même ; pas Tandem.
- À la place, le « Pack Tandem » : six skins originaux (Dino, Astronaute, Pirate, Robot, Mineur, Exploratrice), dessinés par `tools/skins/generate.py` (Pillow), sous la licence du projet. Ombrage par face et deuxième couche (capuches, casques, bottes, ceintures) pour le relief des skins récents.
- Embarqués dans `tandem-core` (`assets/skins`, `include_bytes!`) : disponibles hors ligne, environ 27 Ko en tout. Identifiants `pack:<id>`, portés comme les autres skins.
- Statut : validé

### D59 · 2026-10-20 · Amis, toujours sans serveur Tandem
- Choix de Zeleff : ajout par **code d'ami permanent**, statut complet (en ligne, en jeu sur telle instance, partie ouverte) avec « Apparaître hors ligne », et « Mes amis peuvent rejoindre sans invitation » au choix de l'hôte. Pas de recherche par pseudo : il faudrait un annuaire, donc un serveur.
- Identité : une clé iroh durable par dossier de données, dans le coffre du système (un dossier de test est un autre joueur). Son identifiant de point d'accès est l'adresse du joueur.
- Code d'ami (`XXXX-XXXX`, comme les invitations) publié sur le relais pkarr public d'iroh, republié toutes les 30 min (le relais le garde 2 h). Il ne sert qu'à se trouver la première fois : chaque ami garde ensuite l'identifiant de l'autre.
- Quand Tandem est ouvert : un point d'accès iroh (`tandem/friends/1`) et une connexion par ami en ligne, gardée ouverte (keep-alive QUIC). Statut, demandes, acceptations, retraits et invitations y passent en direct : rien de public à part le code, réactif sans interroger le relais en boucle. Amis hors ligne réessayés toutes les 30 s.
- Le statut est calculé par l'app toutes les 3 s (partie hébergée, sinon jeu lancé, sinon en ligne) ; l'invitation transmet le code d'invitation de la partie, et l'ami rejoint par le chemin habituel (préparation de l'instance, lancement automatique).
- Ce qu'un inconnu peut faire : envoyer une demande, rien d'autre ; sa connexion est coupée au bout d'une minute s'il n'est pas accepté. Un ami retiré pendant qu'on était hors ligne l'apprend à la connexion suivante.
- Compte Microsoft nécessaire (comme pour jouer ensemble).
- Statut : validé

