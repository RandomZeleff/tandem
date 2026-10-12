# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Application de bureau : webview Tauri 2, Windows d'abord, macOS aussi. Pas un site, pas de mobile.)

## Users

Des groupes d'amis, surtout des ados et jeunes adultes, qui jouent à des modpacks Minecraft ensemble et ne sont pas des techniciens. Ils ouvrent Tandem pour reprendre leur partie vite et jouer avec leurs amis sans louer ni configurer de serveur. Les joueurs avertis sont bienvenus, mais la simplicité passe d'abord : les réglages avancés existent et restent en retrait.

## Product Purpose

Tandem est un launcher Minecraft centré sur le solo et les petits groupes, la performance et la simplicité. Il installe et lance des instances Vanilla, Fabric, Quilt, Forge et NeoForge, installe modpacks et contenus depuis Modrinth, et permet de jouer entre amis avec un code d'invitation, sans serveur. Réussite : on reprend sa dernière partie en un clic, et un ami invité rejoint en deux minutes sans aide.

## Positioning

- « Jouer ensemble » sans serveur : tunnel pair à pair (iroh), code d'invitation, instance de l'hôte copiée chez l'ami (mods et fichiers de pack), monde à part en option.
- Optimisations côté serveur ajoutées au lancement (Tandem Optimizations), préparation du monde pour jouer à plusieurs, indicateur de charge pour l'hôte.
- Traduction de modpacks par IA (français d'abord).

## Operating Context

- Ouvert sur un PC de jeu, souvent à côté de Discord et d'un navigateur, plutôt le soir ; le jeu tourne en plein écran pendant que Tandem reste en arrière-plan et notifie (ami arrivé, monde ouvert).
- Une session type : ouvrir Tandem, reprendre la dernière instance, ou inviter des amis / entrer le code d'un ami. Plus rarement : parcourir Modrinth, installer un modpack, gérer mods, mondes, sauvegardes, captures.
- Compte Microsoft nécessaire pour jouer en ligne avec des amis (Mojang vérifie les comptes).

## Capabilities and Constraints

- Pages : Accueil, Instances (liste, fiche avec onglets Console, Contenu, Mondes, Captures, Traduction, Créateurs, Réglages), Découvrir (Modrinth), fiches de projet, Jouer ensemble, Réglages, Comptes.
- Modrinth est la seule source de contenu (D41 : pas d'intégration CurseForge, import seulement depuis les fichiers locaux de l'app CurseForge).
- Toutes les entrées-sorties en Rust ; le frontend affiche et appelle des commandes Tauri typées. Polices embarquées (aucun appel réseau pour l'interface).
- Un faux backend de développement (`src/dev/mock.ts`) fait tourner l'interface dans un navigateur.

## Brand Commitments

- Nom : Tandem (reste).
- Français uniquement pour l'instant (textes de l'interface, tutoiement).
- Lien visible avec Minecraft : on doit sentir un launcher Minecraft, sans forcément du pixel partout.
- Pas d'anneau de focus qui fasse « web » (D26) : le focus clavier reprend l'état de survol.

## Evidence on Hand

- Vrais modpacks testés : Cobbleverse, Prominence II, Fabulously Optimized (données de test, pas des preuves marketing).
- Retour de l'auteur et de son frère (2026-10-12) : l'interface actuelle « ne fait pas très propre ni agréable ».
- Pas encore d'utilisateurs, de témoignages ni de chiffres de téléchargement : rien de tel ne doit être inventé.

## Product Principles

1. Jouer d'abord : le chemin le plus court entre l'ouverture de Tandem et le jeu.
2. Des amis sans friction : inviter et rejoindre ne demandent aucune connaissance technique.
3. Simple par défaut, puissant à la demande.
4. Honnête : dire ce qui marche, ce qui est en bêta, ce qui coûte de la mémoire.
