# Test du jeu à deux (Phase 6a)

Deux joueurs, deux PC, idéalement **deux réseaux différents** (chacun chez soi). Chacun a un compte Microsoft qui possède Minecraft: Java Edition.

## Installation (l'ami)

1. Lancer `Tandem_0.1.0_x64-setup.exe`. L'installeur n'est pas signé : si Windows affiche « Windows a protégé votre ordinateur », cliquer **Informations complémentaires** puis **Exécuter quand même**.
2. Ouvrir Tandem, cliquer sur la carte du compte en haut à gauche → **Se connecter avec Microsoft**.
3. Créer une instance **identique** à celle de l'hôte : même version de Minecraft, même loader, mêmes mods. Le plus simple : installer le même modpack depuis Découvrir, ou pour une instance Fabric/Quilt, l'hôte l'exporte (onglet Contenu → Exporter) et l'ami l'importe (Instances → Importer un modpack). Pour un premier test, une instance **vanilla** de la même version suffit.

## La partie

**Hôte**
1. Jouer à deux → choisir l'instance → **Créer une invitation** → envoyer le code à l'ami.
2. Lancer l'instance, entrer dans un monde, puis Échap → **Ouvrir au réseau local** → **Démarrer le monde en LAN**.
3. La deuxième étape se coche toute seule (« Monde ouvert »).

**Invité**
1. Jouer à deux → taper le code → choisir l'instance → **Rejoindre**.
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

