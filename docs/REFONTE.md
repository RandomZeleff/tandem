# Refonte de l'interface (cadrage, 12 octobre 2026)

Retour de Zeleff et de son frère : l'interface « ne fait pas très propre ni agréable ». Objectif : un launcher au style moderne, toujours clairement Minecraft. Ce document cadre la refonte ; rien n'est encore codé.

## Ce qui ne va pas aujourd'hui (D11 « Deepslate »)

- **Trop sombre et trop plat** : fond presque noir (`#08090a` à `#171a1e`), panneaux à peine distincts du fond, bordures d'un pixel. Rien ne guide l'œil.
- **Le pixel partout** : police pixel sur tous les titres, coins en escalier sur chaque bouton et panneau. À petite taille, ça fait « fait main » plutôt que soigné.
- **Texte petit et serré** : beaucoup de 11 à 13 px, des titres en capitales dorées espacées qui se ressemblent tous.
- **Trop de styles de titres et de tailles** : la hiérarchie se perd, chaque page invente un peu.
- **Accueil chargé** : bannière, cartes, nouveautés au même niveau ; « Jouer » n'est pas l'évidence.

## Direction choisie : « le Minecraft moderne »

Tandem parle la langue des menus récents de Mojang (Bedrock, nouveau launcher officiel) : surfaces gris anthracite franches, boutons pleins et épais avec un bord plus sombre en bas qui s'enfonce au clic, vert Minecraft pour l'action principale, grands visuels de modpack. Propre et lisible d'abord ; le côté Minecraft vient des boutons, des couleurs, des têtes de skin et des visuels, pas d'un décor pixel partout.

Gardé des autres pistes étudiées :
- À l'accueil, une seule instance possède l'écran et **Jouer** est l'action évidente.
- Une arrivée d'ami reste signalée tant qu'on ne l'a pas vue.
- Peu de tailles de texte ; la hiérarchie passe par la graisse.
- Les lignes en direct (téléchargements, invités, progression) changent sur place, sans sauter.
- Les éléments en pixels (têtes de skin, icônes de blocs) restent nets, à des tailles entières.
- Les visuels d'instance ont toujours le même format.

Risque assumé : c'est la direction la plus proche du launcher officiel et de Lunar. La différence doit venir de la finition et des fonctions propres à Tandem (Jouer ensemble, préparation du monde, santé de la partie), pas d'un décor.

## Périmètre et étapes

Toute l'application, page par page, chacune finie avant la suivante (même règle que pour « Jouer ensemble »). Les fonctions et les textes restent ; seul l'habillage et la mise en page changent, sauf amélioration de texte validée.

1. **Fondations** : couleurs, échelle de texte réduite, espacements, boutons, champs, panneaux, onglets, listes, icônes, états (survol, focus, désactivé, chargement, erreur). Remplace les classes `.btn*`, `.panel`, `.slot`, `.px-corners*` de D11.
2. **Cadre de l'app** : barre de titre, navigation, compte.
3. **Accueil** : reprendre sa partie en un clic, inviter, rejoindre.
4. **Jouer ensemble** : inviter, rejoindre, amis dans la partie, santé de la partie.
5. **Instances** : liste, fiche d'instance et ses onglets (Console, Contenu, Mondes, Captures, Traduction, Créateurs, Réglages).
6. **Découvrir** et fiches de projet Modrinth.
7. **Réglages, Comptes**, fenêtres, notifications, états vides et d'erreur.
8. **Finition** : revue complète, puis `DESIGN.md` écrit à partir de ce qui est construit (remplace D11 par une nouvelle décision).

Vérification à chaque étape : aperçu avec le faux backend, captures, puis l'app réelle sur les données de test.

## Choix tranchés (Zeleff, 12 octobre 2026)

- **Titres** : une police libre au dessin carré, dans l'esprit Minecraft, réservée aux grands titres ; tout le reste dans une police moderne lisible. Les polices de Mojang (Minecraft Ten / Seven) ne sont pas libres : exclues. Police exacte à choisir à l'étape 1 (licence libre vérifiée, embarquée).
- **Thème** : sombre seulement.
- **Animations** : sobres (boutons qui s'enfoncent, transitions de page douces, progression fluide).
- **Grand visuel de l'accueil** : pour un modpack Modrinth, son image de galerie mise en avant ; sinon la dernière capture d'écran du joueur dans l'instance ; à défaut, un ciel Minecraft généré.

Non concerné : le moteur (`tandem-core`), les commandes Tauri, les fonctions.
