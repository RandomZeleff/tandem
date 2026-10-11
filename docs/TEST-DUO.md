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

Raphaël et un ami, deux PC, deux comptes Microsoft, installeurs de la livraison ci-dessus.

- Inviter → rejoindre → jouer : **marche**, l'invité étant en **4G** (autre réseau, derrière l'opérateur mobile) comme en Wi-Fi.
- Ping affiché : **≈ 30 ms en 4G**, **≈ 3 ms en Wi-Fi** (même Wi-Fi : équivalent d'un LAN classique).
- Exclure l'invité : marche.
- Inversion des rôles (chacun héberge à son tour) : marche.
- Longue partie en cours au moment du retour, fluide.
- Liaison : affichée **« direct »** (relevé par Raphaël lors du second essai).
- Instances différentes : les mods manquants s'affichent en jaune ; avec la bonne instance, plus aucun avertissement. Comportement attendu.
- Pas relevé : durée exacte avant l'arrivée dans le monde.
- Retour de Raphaël : rien à corriger côté fonctionnement ; à peaufiner plus tard pour l'accessibilité et réduire les frictions.


## Test de charge : combien d'amis sur un PC moyen (11 octobre 2026)

**Montage.** PC de Raphaël : Ryzen 5 8400F (6 cœurs, 12 threads), 15,5 Go de RAM, avec Chrome, Discord et Claude ouverts (environ 3 Go). Modpack **Cobbleverse 1.7.42** (Fabric 1.21.1, 136 mods, le pack Cobblemon le plus joué ; aucun pack Cobblemon populaire n'atteint 200 mods en 1.21.1). Serveur Fabric dédié à 3 Go de mémoire pour le monde de l'hôte, plus le vrai jeu (3 Go) qui le rejoint **par un vrai tunnel Tandem** : 6 Go au total, comme Tandem en donnerait au jeu de l'hôte. Toutes les 2 minutes, un invité de plus (faux joueur Carpet, côté serveur). **Tous les joueurs explorent en ligne droite à la vitesse d'un sprint**, le cas le plus lourd : chunks neufs en permanence, apparitions de Pokémon autour de chacun. Distance de vue 12. Outils : `.claude/notes/scripts/loadtest_setup.py` et `loadtest_run.py`.

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
