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
