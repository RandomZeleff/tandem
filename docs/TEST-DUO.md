# Test du jeu à deux (Phase 6a)

Deux joueurs, deux PC, idéalement **deux réseaux différents** (chacun chez soi). Chacun a un compte Microsoft qui possède Minecraft: Java Edition.

## Installation (l'ami)

1. Lancer `Tandem_0.1.0_x64-setup.exe`. L'installeur n'est pas signé : si Windows affiche « Windows a protégé votre ordinateur », cliquer **Informations complémentaires** puis **Exécuter quand même**.
2. Ouvrir Tandem, cliquer sur la carte du compte en haut à gauche → **Se connecter avec Microsoft**.
3. Créer une instance **identique** à celle de l'hôte : même version de Minecraft, même loader, mêmes mods. Le plus simple : installer le même modpack depuis Découvrir, ou pour une instance Fabric/Quilt, l'hôte l'exporte (onglet Contenu → Exporter) et l'ami l'importe (Instances → Importer un modpack). Pour un premier test, une instance **vanilla** de la même version suffit.

## La partie

**Hôte**
1. Jouer ensemble → choisir l'instance (et le monde, 1.20+) → **Inviter et lancer le jeu** → envoyer le code aux amis.
2. Lancer l'instance, entrer dans un monde, puis Échap → **Ouvrir au réseau local** → **Démarrer le monde en LAN**.
3. La deuxième étape se coche toute seule (« Monde ouvert »).

**Invité**
1. Jouer ensemble → taper le code → **Rejoindre** (Tandem choisit l'instance ; sinon **Préparer l'instance**).
2. Quand « Monde ouvert » s'affiche → **Jouer** : le jeu se lance directement dans le monde de l'hôte.

## À noter pendant le test

- Temps entre « Rejoindre » et l'affichage de la partie.
- Le ping et « direct » ou « relais » affichés (des deux côtés).
- La partie est-elle fluide ? Combien de temps ?
- L'invité quitte puis revient avec le même code ; l'hôte exclut l'invité ; l'hôte arrête l'invitation.
- En cas de problème : la phrase affichée, et le journal du launcher (Réglages → Journal du launcher, ou `%APPDATA%\Tandem\logs`) des deux côtés.

## Cadre du test et résultats à remplir

Build Windows x64 0.1.0 du 10 octobre 2026, source `e0c00f0` (Phase 6a).
L'EXE et le MSI installent la même application : choisir un seul format, de préférence l'EXE. Le même installateur convient aux deux joueurs. Fermer Tandem avant l'installation.

- Utiliser deux comptes Microsoft distincts possédant Minecraft: Java Edition ; sélectionner le compte Microsoft dans Tandem des deux côtés.
- Commencer avec un nouveau monde de test et deux instances vanilla de même version, puis recommencer avec le même modpack et la même version de celui-ci.
- Garder Tandem ouvert des deux côtés pendant toute la partie. L'hôte doit aussi garder son monde ouvert. Aucune synchronisation automatique des mods ni transfert de l'hébergement dans cette version.
- Pour le premier essai, lancer chaque instance une fois en solo pour terminer les téléchargements, puis fermer le jeu de l'invité avant de rejoindre.
- Un chemin « relais » est un résultat valide ; noter son ping et la fluidité. Un essai sur le même réseau ne valide pas le fonctionnement entre deux domiciles.

| Étape | Résultat attendu | Résultat observé / durée |
| --- | --- | --- |
| Installation sur chaque PC | Tandem démarre, connexion Microsoft possible | |
| Invitation avant ouverture du monde | L'invité rejoint la session et attend le monde | |
| Ouverture LAN par l'hôte | « Monde ouvert » apparaît des deux côtés | |
| L'invité clique sur Jouer | Il arrive dans le monde avec son propre compte | |
| Partie de 15 à 30 minutes | Déplacements, blocs et inventaires restent synchronisés | |
| L'invité quitte le jeu et la session, puis rejoint le même code | Il peut revenir tant que l'invitation reste active | |
| L'hôte exclut l'invité | La liaison de l'invité se ferme | |
| L'hôte arrête l'invitation | L'invité est déconnecté ; l'ancien code ne permet plus de jouer | |
| Nouvelle invitation après arrêt | Une nouvelle session peut démarrer | |
| Inversion des rôles | Le second PC peut aussi héberger | |
| Même modpack des deux côtés | La partie fonctionne également avec les mods | |

## Si une étape bloque

- Monde non détecté : vérifier que l'hôte a bien utilisé « Ouvrir au réseau local » dans le monde en cours. Relever toute demande ou erreur du pare-feu, sans le désactiver.
- Invitation introuvable : vérifier le code et que l'hôte a toujours son invitation active ; essayer une nouvelle invitation.
- Connexion au monde refusée : relever le message exact, vérifier les comptes Microsoft sélectionnés, les versions du jeu, du loader et des mods.
- Pour le retour : indiquer qui héberge, les versions, réseau identique ou différent, direct/relais, ping, étape bloquée et heure. Joindre les journaux des deux launchers ; si le jeu échoue, joindre aussi son `logs/latest.log` dans le dossier de l'instance. Relire les journaux avant partage pour masquer les informations personnelles éventuelles.

### Fiche de retour

- Date / heure :
- Hôte / invité :
- Version Minecraft / loader / modpack :
- Deux réseaux différents : oui / non
- Liaison et ping de chaque côté :
- Durée avant arrivée dans le monde :
- Durée de jeu et fluidité :
- Dernière étape réussie :
- Message exact et journaux en cas d'échec :

Les vérifications automatiques sur un seul PC ne remplacent pas ce test réel à deux.

## Vérifications de cette livraison (10 octobre 2026)

- TypeScript : `pnpm exec tsc --noEmit` OK.
- Rust : formatage et Clippy sans avertissement ; `cargo test --workspace` : 134 tests réussis.
- `duo selftest` : annonce LAN détectée, invitation publiée/résolue, connexion en environ 1,13 s, ping via tunnel réussi, code inconnu refusé, arrêt de l'hôte signalé. Les deux extrémités tournent sur le même PC : cela ne valide pas deux réseaux distants.
- Release compilée ; installateurs NSIS EXE et Windows Installer MSI générés.
- Exécutable release démarré avec des données isolées : backend prêt en 350 ms, interface prête en 490 ms. Aucun compte ni monde personnel utilisé pour ce contrôle.
- Installation sur un autre PC et vraie partie authentifiée à deux : à vérifier avec la grille ci-dessus.

## Résultats du premier test réel (10 octobre 2026)

Zeleff et un ami, deux PC, deux comptes Microsoft, installeurs de la livraison ci-dessus.

- Inviter → rejoindre → jouer : **marche**, l'invité étant en **4G** (autre réseau, derrière l'opérateur mobile) comme en Wi-Fi.
- Ping affiché : **≈ 30 ms en 4G**, **≈ 3 ms en Wi-Fi** (même Wi-Fi : équivalent d'un LAN classique).
- Exclure l'invité : marche.
- Inversion des rôles (chacun héberge à son tour) : marche.
- Longue partie en cours au moment du retour, fluide.
- Liaison : affichée **« direct »** (relevé par Zeleff lors du second essai).
- Instances différentes : les mods manquants s'affichent en jaune ; avec la bonne instance, plus aucun avertissement. Comportement attendu.
- Pas relevé : durée exacte avant l'arrivée dans le monde.
- Retour de Zeleff : rien à corriger côté fonctionnement ; à peaufiner plus tard pour l'accessibilité et réduire les frictions.


## Test de charge : combien d'amis sur un PC moyen (11 octobre 2026)

**Montage.** PC de Zeleff : Ryzen 5 8400F (6 cœurs, 12 threads), 15,5 Go de RAM, avec Chrome, Discord et Claude ouverts (environ 3 Go). Modpack **Cobbleverse 1.7.42** (Fabric 1.21.1, 136 mods, le pack Cobblemon le plus joué ; aucun pack Cobblemon populaire n'atteint 200 mods en 1.21.1). Serveur Fabric dédié à 3 Go de mémoire pour le monde de l'hôte, plus le vrai jeu (3 Go) qui le rejoint **par un vrai tunnel Tandem** : 6 Go au total, comme Tandem en donnerait au jeu de l'hôte. Toutes les 2 minutes, un invité de plus (faux joueur Carpet, côté serveur). **Tous les joueurs explorent en ligne droite à la vitesse d'un sprint**, le cas le plus lourd : chunks neufs en permanence, apparitions de Pokémon autour de chacun. Distance de vue 12. Outils : `.claude/notes/scripts/loadtest_setup.py` et `loadtest_run.py`.

| Joueurs | Temps par tick (moy. / pic P95) | CPU du PC | RAM libre min. | Échanges disque (pages/s) | Débit vers un invité |
|---|---|---|---|---|---|
| 2 | 7 ms / 15 ms | 38 % | 521 Mo | 557 | 1,0 Mbit/s |
| 3 | 14 ms / 33 ms | 59 % | 433 Mo | 2 124 | 0,4 Mbit/s |
| 4 | 20 ms / 38 ms | 58 % | 575 Mo | 737 | 0,5 Mbit/s |
| 5 | 30 ms / 58 ms | 73 % | 979 Mo | 289 | 0,5 Mbit/s |
| 6 | 37 ms / 75 ms | 82 % | 623 Mo | 1 963 | 0,5 Mbit/s |
| 7 | 46 ms / 92 ms | 86 % | 368 Mo | 1 607 | 0,5 Mbit/s |
| 8 | 41 ms / 76 ms | 83 % | 351 Mo | 1 616 | 0,5 Mbit/s |
| 9 | 45 ms / 79 ms | 83 % | 368 Mo | 559 | 0,4 Mbit/s |
| 10 | 43 ms / 113 ms | 88 % | 315 Mo | 11 170 | 0,3 Mbit/s |
| 11 | 51 ms / 103 ms | 88 % | 584 Mo | 4 226 | 0,3 Mbit/s |

Le serveur doit finir chaque tick en moins de 50 ms pour tenir ses 20 TPS.

**Lecture.**
- **Ce qui serre en premier : la RAM.** Dès le premier ami, il reste moins de 600 Mo libres et Windows échange avec le disque. Ça ne casse rien, mais le PC n'a plus de marge. (Le vrai jeu de l'hôte fait tout dans un seul processus : un peu moins gourmand que ce montage à deux processus.)
- **Ce qui casse ensuite : le processeur.** Le temps par tick monte d'environ 7 ms par explorateur. Confortable jusqu'à 5 joueurs (hôte + 4 amis) ; à 6-7 joueurs, les pics dépassent 50 ms (petits ralentissements) ; à partir de 8, le CPU plafonne vers 85 % et la génération du monde prend du retard. Le temps par tick ne monte plus, mais le débit envoyé à chaque joueur baisse (moins de chunks prêts), et à l'arrêt **6 500 à 8 000 chunks** étaient encore en attente.
- **Le tunnel Tandem n'est pas la limite.** Un joueur qui explore reçoit 0,3 à 1 Mbit/s et en envoie environ 0,01. Héberger 4 amis demande donc environ 2 à 4 Mbit/s d'envoi : la fibre et la plupart des box ADSL ou 4G suffisent. Le ping dans le tunnel (même PC) passe de 3 à 8 ms, à cause de la charge du PC, pas de Tandem.
- **Limites du test.** Les invités ajoutés sont des faux joueurs côté serveur : ils chargent le monde comme de vrais joueurs mais ne passent pas par le réseau (un seul vrai jeu passait par le tunnel). Tout le monde explore sans arrêt : des amis qui jouent autour d'une base chargent bien moins le serveur. Les images par seconde du jeu de l'hôte n'ont pas été mesurées.

**En une phrase :** sur un PC moyen avec un modpack Cobblemon, **l'hôte et 4 amis tiennent sans problème** même en explorant tous en même temps ; à 6-7 ça commence à ralentir par moments ; 8 (le maximum d'un monde ouvert en LAN) passe encore si tout le monde n'explore pas en même temps.

### Comparaison : monde déjà généré (11 octobre 2026)

Même montage, mais la zone autour du point d'apparition est générée à l'avance (Chunky, 1 500 × 1 500 blocs) et les joueurs y tournent en cercles à la vitesse d'un sprint, sans jamais voir de terrain neuf. 4 invités ajoutés toutes les 2 minutes.

| Joueurs | Terrain neuf : tick moy. / pic | CPU du PC | Monde pré-généré : tick moy. / pic | CPU du PC |
|---|---|---|---|---|
| 2 | 7 / 15 ms | 38 % | 5 / 15 ms | 28 % |
| 6 | 37 / 75 ms | 82 % | 18 / 28 ms | 32 % |
| 10 | 43 / 113 ms (CPU saturé) | 88 % | 28 / 48 ms | 34 % |
| 14 | — | — | 38 / 62 ms | 37 % |
| 18 | — | — | 43 / 65 ms | 35 % |
| 22 | — | — | 45 / 74 ms | 37 % |

Génération pure sur une zone vierge, PC entier dessus et personne en jeu : **77 chunks/s**, environ **11 Ko par chunk** sur le disque.

**Lecture.**
- **La génération du terrain fait la majorité de la charge.** À 6 joueurs, le temps par tick est divisé par deux et le CPU passe de 82 % à 32 %. Un joueur qui sprinte en terrain neuf demande environ 9 à 10 chunks/s : à 77 chunks/s, 8 explorateurs suffisent à saturer le PC, ce que montrait le premier test.
- **Sur un monde déjà généré, la limite devient le fil principal du serveur** : il n'utilise qu'un cœur (10 % du CPU du PC), et chaque joueur lui ajoute environ 2 ms par tick (Pokémon, entités autour de chacun). Sur ce processeur, ça tient jusqu'à **une vingtaine de joueurs**. Au-delà, seul un processeur plus rapide par cœur aide.
- **La RAM ne change pas** : 300 à 700 Mo libres dans les deux cas.
- **Mais un monde ouvert en LAN est limité à 8 joueurs** par Minecraft lui-même. Dépasser 8 demanderait de modifier le jeu (l'agent Java prévu), ou de faire tourner le monde de l'hôte comme un serveur à part.
- **Coût d'une pré-génération** : une zone de 4 000 × 4 000 blocs (62 500 chunks) prend environ 13 min sur ce PC seul et pèse environ 700 Mo. Partagée entre 5 PC : environ 3 min chacun, plus l'envoi d'environ 140 Mo par ami vers l'hôte. Pré-générer à l'avance sur le seul PC de l'hôte donne déjà l'essentiel du gain.

## Optimisation : où part le temps du serveur (11 octobre 2026)

**Montage.** Même PC. Deux modpacks très différents, chacun sur un monde pré-généré (Chunky), joueurs qui tournent en cercles à la vitesse d'un sprint, 4 invités de plus toutes les 90 s :
- **Cobbleverse** (Fabric 1.21.1, 136 mods), serveur à 3 Go + un vrai jeu par le tunnel Tandem, comme les tests précédents ;
- **Prominence II** (Fabric 1.20.1, RPG, 678 mods côté client, 443 côté serveur), serveur à 6 Go, sans vrai jeu (pas assez de RAM pour les deux), temps du tick mesuré par Carpet (`tick query` n'existe pas avant 1.20.3).

**Profilage : Java Flight Recorder**, pas spark. Sous Windows, spark n'a que son échantillonneur Java : il ne voit le code qu'aux « safepoints » (résultats faux, très différents de JFR) et ralentit le serveur en permanence (10 joueurs : 46 ms par tick avec lui, 32 sans). JFR est inclus dans Java 21, sans ce biais. Chaque échantillon du fil du serveur est rangé par sous-système et par mod (classes lues dans les jars, Mixins reconnus à leur nom).

### Temps par tick (ms) selon le nombre de joueurs

Cobbleverse :

| Variante | 2 | 6 | 10 | 14 | 18 |
|---|---|---|---|---|---|
| Référence (distance de simulation 12) | 9,3 | 20,3 | 32,4 | 39,3 | 47,1 |
| Distance de simulation 8 | 7,4 | 19,8 | 31,2 | 38,5 | 43,3 |
| Apparition vanilla coupée (`doMobSpawning false`) | 6,9 | 13,6 | 18,0 | 24,1 | 25,6 |
| … + VMP | 6,6 | 15,0 | 18,3 | 22,7 | 24,4 |
| … + VMP + ServerCore (réglages par défaut) | 6,8 | 12,4 | 15,8 | 19,3 | 21,6 |
| … + distance d'activation (ServerCore, 32 blocs) | 6,3 | 11,8 | 15,3 | 18,4 | 24,3 |
| **Mini-mod Tandem seul** (sans toucher aux règles du monde) | 8,1 | 16,0 | 20,9 | 24,4 | 28,0 |

Prominence II :

| Variante | 2 | 6 | 10 | 14 | 18 |
|---|---|---|---|---|---|
| Référence | 18,6 | 38,0 | 51,5 | 60,8 | (arrêté) |
| Mini-mod Tandem | 13,0 | 34,1 | 48,1 | 53,1 | 72,1 |
| Mini-mod + ServerCore (distance d'activation, villageois bloqués ralentis) | 11,4 | 27,7 | 40,4 | 49,2 | 56,8 |

Le serveur tient ses 20 TPS tant que le tick reste sous 50 ms. Écart entre deux passages identiques : environ ±2 ms.

### Ce qui coûte, et ce qui se généralise

1. **Apparitions vanilla qui ne servent à rien.** Cobbleverse bloque toutes les créatures vanilla (MobsBeGone), mais seulement au moment où elles entrent dans le monde : le serveur fait d'abord toute la recherche d'emplacement (biome, structures, chunks voisins), pour chaque chunk autour de chaque joueur, à chaque tick. **40 % du tick** à 10-18 joueurs, pour rien. Le cas est courant (packs qui remplacent les mobs vanilla, catégories vides). Correctif sans effet en jeu : sauter les catégories où rien ne peut apparaître (mini-mod ci-dessous). Gain : 47 → 28 ms à 18 joueurs. Sur Prominence, les mobs vanilla existent : le mini-mod ne saute qu'une catégorie vide, comme prévu.
2. **IA des entités.** Après ça, la charge principale des deux packs : Pokémon (53 % du tick sur Cobbleverse), mobs vanilla et moddés, recherche de chemin, villageois qui cherchent leurs points d'intérêt (Prominence). La **distance d'activation** (entités loin des joueurs mises au ralenti, comme Paper) gagne 15 à 20 % sur Prominence, presque rien sur Cobbleverse (les Pokémon apparaissent près des joueurs). ServerCore la propose mais ne marche que sur un serveur dédié, pas dans le monde de l'hôte : à refaire dans le mini-mod pour en profiter.
3. **Bibliothèques partagées mal optimisées.** Architectury (présente dans les deux packs, et dans des centaines d'autres) reconstruit un `MethodHandle` par réflexion à chaque appel de chaque écouteur d'événement ; Balm relit une annotation et recrée un identifiant à chaque lecture de config, ce que Forgiving Void fait pour chaque entité à chaque tick. Architectury : 2 à 3 % du tick ; Forgiving Void (au-dessus de Balm) : jusqu'à 6 %, dont seule la lecture de config est corrigée pour l'instant dans le mini-mod.
4. **Coûts propres à certains mods** (à signaler à leurs auteurs plutôt qu'à corriger nous-mêmes) : fonction de datapack qui redonne un effet à tous les joueurs à chaque tick (Cobbleverse « No Hunger » : un paquet réseau par joueur et par tick), baies de Cobblemon qui se marquent « modifiées » à chaque tick, calcul d'agressivité de Fight or Flight à chaque tick, Pehkui (4,5 %), BlockSwap (3,7 %) sur Prominence.
5. **Ce qui ne change presque rien ici** : distance de simulation 12 → 8 (−8 % : apparitions et IA se font de toute façon à moins de 64-128 blocs des joueurs), VMP (pensé pour le réseau de nombreux vrais joueurs, que nos faux joueurs n'ont pas).
6. **Pré-génération** : 77 chunks/s sur Cobbleverse (qui a c2me), 35 chunks/s sur Prominence (sans c2me). c2me double la vitesse de génération : à proposer pour « préparer le monde ».

### Le mini-mod Tandem (prototype, Fabric)

Un jar de 7 Ko, actif seulement côté serveur (le monde de l'hôte), rien à installer chez les amis. Compilé directement contre les noms « intermédiaires » de Fabric, stables d'une version à l'autre : le même jar marche en 1.20.1 et 1.21.1.
- Catégories d'apparition mortes : calculées au démarrage à partir des listes des biomes et des structures, et du mod bloqueur s'il y en a un ; sautées ensuite. Les marchands ambulants, patrouilles, générateurs de monstres continuent (contrairement à `doMobSpawning false`).
- Cache de `ConfigReflection.getIdentifier` (Balm) et appel direct des écouteurs Architectury.

Limites : Fabric seulement pour l'instant (NeoForge utilise d'autres noms à l'exécution), pas encore branché dans Tandem, faux joueurs côté serveur (le coût réseau de vrais joueurs n'est pas mesuré).

## Tandem Optimizations v1 : mesures (11 octobre 2026)

Le mini-mod devenu **Tandem Optimizations** (D42, [sources](../mods/tandem-optimizations/README.md)), ajouté par Tandem au lancement. Même banc que plus haut (faux joueurs Carpet en cercle sur monde pré-généré, +4 joueurs toutes les 90 s), mais mémoire du serveur réduite (Prominence 4 Go, Cobbleverse 3 Go) : avec Chrome, Discord et Steam ouverts, 5 à 6 Go faisaient paginer le PC et faussaient tout. Temps par tick en ms (au-delà de 50, le serveur prend du retard).

Prominence II (1.20.1) :

| Variante | 2 | 6 | 10 | 14 |
|---|---|---|---|---|
| Sans le mod | 15,0 | 42,6 | 55,2 | 69,5 |
| Mod sans distance d'activation | 14,5 | 41,6 | 55,8 | 71,5 |
| Mod complet (distance d'activation sur toutes les créatures) | 12,4 | 29,9 | 46,8 | 52,3 |

Cobbleverse (1.21.1), mod sans puis avec distance d'activation : 5,3 / 11,5 / 15,3 / 20,7 / 23,4 contre 5,4 / 11,5 / 15,2 / 18,9 / 23,5 (2 à 18 joueurs) : pas de gain, attendu (les Pokémon apparaissent près des joueurs).

**Problème trouvé et corrigé.** Avec la distance d'activation, Cobbleverse avait **75 % de Pokémon en plus** à chaque palier (634 contre 363 à 18 joueurs) : Cobblemon fait vieillir ses Pokémon avec un compteur avancé dans leur propre tick, ralentis ils disparaissaient quatre fois plus tard. Aucun moyen de savoir quel mod fait pareil : la distance d'activation ne touche plus que les créatures vanilla (et pas celles dont un minuteur se voit : poules, zombies-villageois, piglins, hoglins, tatous). Vérifié ensuite : nombre de Pokémon revenu dans la marge du hasard (132 / 250 / 326 contre 91 / 220 / 282 sans distance d'activation). Le gain sur Prominence avec cette version n'est pas remesuré : il sera plus faible que le tableau (ses créatures moddées ne sont plus ralenties).

Ce que le mod complet change sur Prominence : jusqu'à 30 % de temps de tick en moins, et le serveur tient ses 20 TPS jusqu'à 10 joueurs au lieu de 6. Le reste du mod (catégories d'apparition mortes, Architectury, Balm) ne gagne presque rien sur ce pack ; son gros effet est sur les packs qui bloquent les mobs vanilla (Cobbleverse : 47 → 28 ms à 18 joueurs, plus haut).

Versions : construit et vérifié contre les 20 versions de 1.20 à 1.21.11, démarré sur de vrais serveurs 1.20.1, 1.21.1, 1.21.4 et 1.21.11 (les bébés grandissent au même rythme, aucune erreur).

### Validation en vrai (12 octobre 2026)

Le mod tel que Tandem l'ajoute (exemples `play` et `server`, mêmes arguments que l'app), sur un monde existant, compte hors ligne :

| Pack | Solo (monde du jeu) | « Jouer ensemble » (monde à part + un invité) |
|---|---|---|
| Cobbleverse (Fabric 1.21.1) | 4 min, OK | invité connecté, OK |
| Prominence II (Fabric 1.20.1) | OK (voir plus bas) | invité 2 min 30, OK |
| Fabulously Optimized (Fabric 1.21.4) | OK | invité 2 min 15, OK |
| Instance Quilt 1.20.1 (QSL, Architectury, Balm, Waystones, Friends&Foes, Lithium) | OK, chargé par `-Dloader.addMods` | non testé (même code que Fabric) |

Le mod s'annonce partout (`Tandem Optimizations 1.0.0 active`), aucune erreur ni avertissement Mixin de sa part. Les catégories d'apparition sautées sont les bonnes : toutes les vanilla sur Cobbleverse (le pack bloque toutes les créatures vanilla), seulement `MISC` ailleurs.

**Prominence et la version « créatures vanilla seulement »** (une mesure, même banc que le tableau ci-dessus, serveur 4 Go) : 8,3 / 31,4 / 46,8 / 52,5 ms à 2 / 6 / 10 / 14 joueurs. C'est le niveau de la version complète (12,4 / 29,9 / 46,8 / 52,3) : sur ce pack, la charge vient surtout des créatures vanilla. Le gain réel par rapport à sans le mod (15,0 / 42,6 / 55,2 / 69,5) est donc d'environ **25 % de temps de tick en moins**, et le serveur tient ses 20 TPS jusqu'à 10 joueurs. Prudence : la ligne « sans le mod » date d'une autre séance, le chiffre à 2 joueurs varie beaucoup d'une séance à l'autre.

Trouvé au passage (pas dans le mod) :
- **Quilt** : Tandem installait Fabric API en dépendance à côté de QFAPI, qui le contient déjà, et le jeu refusait de démarrer (« Duplicate mandatory mod ids »). Corrigé : sur Quilt, l'un des deux suffit.
- **Essential** (Prominence II) : un plantage sur deux essais à l'ouverture directe du monde (`gg.essential.sps.WorldsManager`, état pas encore prêt), notre mod absent de la pile ; pas reproduit au second essai ni sans le mod. Course dans Essential avec le compte hors ligne, à surveiller avec un vrai compte (l'hôte arrive aussi directement dans son monde avec « Jouer ensemble »).
- **Monde à part** : le serveur de Prominence n'a pas pu écrire `config/treechop-common.toml` (fichier utilisé par un autre processus) : le dossier était lié à celui de l'instance, que le jeu tenait ouvert. Corrigé par une copie (D45), revérifié sans erreur.

**Mémoire du jeu de l'hôte en monde à part** (une mesure par pack, mémoire réellement utilisée après un nettoyage forcé, `jcmd GC.run`) : Cobbleverse 3,1 Go en solo contre 2,6 Go en rejoignant le monde à part, Prominence II 2,7 contre 2,4 Go. D'où 1 Go de moins pour le jeu dans ce cas (D45) ; vérifié sur Prominence (7 Go au lieu de 8) : même mémoire utilisée, aucune erreur. Le pic vu par Windows varie surtout avec le ramasse-miettes d'un essai à l'autre : le gain sûr est le plafond.

## Repousser les limites : pistes de niveau 1 à 3 (11 octobre 2026)

Trois niveaux étudiés, des plus sûrs aux plus ambitieux, en écartant ce qui casserait les mods.

**Niveau 1 : les cœurs inutilisés de l'hôte.** Le serveur n'utilise qu'un cœur sur 12. Part du tick qui n'est que de la « recherche » (calculable à côté puis appliquée) :

| Recherche | Cobbleverse (18 joueurs) | Prominence (14 joueurs) |
|---|---|---|
| Chemins des mobs | 1,7 % | 5,5 % |
| Entités proches (capteurs, cibles) | 3,8 % | 3,9 % |
| Points d'intérêt des villageois | 0,5 % | 2,9 % |
| Emplacements d'apparition vanilla | 2,7 % (déjà réduit par le mini-mod) | 17,1 % |
| Emplacements d'apparition d'un mod | 3,0 % | 0 % |

Plafond : 12 à 30 % du tick. Le reste (décider, bouger, collisions) modifie le monde et doit rester sur un seul fil. Le plus gros morceau, la recherche d'apparition de Prominence, est surtout des lectures de biome et de chunks voisins à chaque essai : un cache suffit, sans parallélisme. Calculer les chemins en parallèle toucherait l'IA des mobs moddés : risqué.

**Niveau 2 : les amis comme coprocesseurs.**
- *Réseau* : sur 62 s avec un vrai joueur, les fils réseau (compression comprise) totalisent 1 échantillon contre 2 739 pour le fil du serveur. Déplacer la compression dans le tunnel Rust ne gagnerait rien. Un relais entre amis n'aiderait qu'un hôte à très petit débit montant (environ 0,5 Mbit/s par ami).
- *Génération du terrain par les amis* : c'est la piste qui tient. Deux constats :
  - La génération de Minecraft **n'est pas reproductible**, même seul : deux mondes de même graine générés dans le même ordre diffèrent de 0,6 % (Cobbleverse) à 1,7 % (Prominence) des blocs, à cause des décorations des chunks voisins posées en parallèle (feuilles, arbres, mousse, argile, poches de minerai, herbe). Le relief, les biomes et les structures sont identiques. Vérifier qu'un ami génère « la même chose » que l'hôte n'a donc pas de sens : un chunk généré par un ami est aussi valable que celui de l'hôte.
  - **Injection dans le monde de l'hôte en marche, prouvée** : 289 chunks générés dans un autre monde (même graine, à x = 20 000), envoyés en fichiers de 6,8 Ko par chunk, ont été pris par le mini-mod en 9 s et chargés par le serveur au lieu d'être générés. Il ne reste que les différences de la vie du monde après chargement (eau qui coule, varech qui pousse : 0,1 % des blocs).
  - Gain attendu : celui mesuré entre terrain neuf et terrain pré-généré (6 joueurs : 37 → 18 ms par tick, CPU du PC 82 → 32 %). Un explorateur demande environ 10 chunks/s, soit environ 70 Ko/s à envoyer et un cœur et demi chez l'ami.
  - Reste à faire : faire tourner le générateur chez l'ami (un serveur sans joueurs du même modpack, 2 à 3 Go de RAM), décider où générer (devant chaque joueur), envoyer par le tunnel, entités nées à la génération, autres dimensions, NeoForge.

**Niveau 3 : déléguer la simulation des mobs à l'ami le plus proche.** Plafond théorique élevé (les entités font 40 à 60 % du tick), mais l'ami aurait besoin d'une copie vivante du monde côté serveur autour de lui, synchronisée dans les deux sens, et les mobs moddés touchent des données globales du serveur (combats et équipes Cobblemon, inventaires, progrès). C'est exactement le genre de piste qui casse les mods : **écartée**. Ses équivalents locaux (distance d'activation, IA allégée loin des joueurs) donnent une partie du gain sans ce risque.
