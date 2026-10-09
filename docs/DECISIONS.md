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
- Statut : validé

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
- Statut : validé

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
