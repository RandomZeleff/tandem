# Test sous charge — fiche à suivre

But : voir comment Tandem se comporte avec de vrais gros modpacks, avant d'optimiser. À faire avec un build release (`pnpm tauri build`, puis `target/release/tandem.exe`) **sans `pnpm tauri dev` ouvert** (il recompile en fond et fausse les temps, voir D24).

Note pour chaque étape : le temps (chrono du téléphone suffit), ce qui coince, et une capture si quelque chose est bizarre. À la fin, envoie le fichier du jour de `%APPDATA%\Tandem\logs\` (`tandem.AAAA-MM-JJ.log`).

## Les packs

Dans **Découvrir → Modpacks** (triés par téléchargements) :

1. **Un gros pack Fabric** : 200 mods ou plus.
2. **Un pack NeoForge 1.21.1**.
3. **Un pack Forge 1.20.1 avec FTB Quests** : utile aussi pour la future traduction.

## Pour chaque pack

### 1. Installation
- [ ] Clic sur « Installer » → temps jusqu'à « prêt à jouer ».
- [ ] La barre de progression avance-t-elle régulièrement ? Des pauses longues ?
- [ ] Pendant l'installation, l'interface reste-t-elle fluide (changer de page, ouvrir Réglages) ?
- [ ] Couper le Wi-Fi au milieu, puis le remettre et relancer l'installation : reprend-elle sans tout retélécharger ?

### 2. Premier lancement
- [ ] Temps entre « Jouer » et le menu principal du jeu.
- [ ] Console : défile-t-elle sans figer l'interface pendant le chargement (des milliers de lignes) ?
- [ ] Panneau RAM/CPU : valeurs cohérentes avec le Gestionnaire des tâches ?
- [ ] Onglet Informations : quelle mémoire « Automatique » a été choisie ? Le jeu en a-t-il manqué ?

### 3. Le launcher pendant le jeu
- [ ] Gestionnaire des tâches → mémoire et processeur de `tandem.exe` (et de `msedgewebview2.exe`) pendant que le jeu tourne.
- [ ] Réglage « Réduire le launcher » : se réduit-il au lancement, revient-il à la fermeture du jeu ?

### 4. Onglet Contenu
- [ ] Temps d'affichage de la liste (lignes grises puis mods).
- [ ] Défilement fluide avec toutes les icônes ?
- [ ] « Vérifier les mises à jour » : temps de réponse.
- [ ] Désactiver un mod, le réactiver ; retirer un mod puis « Annuler ».

### 5. Crash volontaire
- [ ] Désactiver un mod dont d'autres dépendent (ex. une bibliothèque comme Fabric API ou Architectury), lancer.
- [ ] Le panneau de crash désigne-t-il le bon coupable ? Le bouton « Désactiver » / « Voir le rapport » marche-t-il ?

### 6. Mondes et captures
- [ ] Créer un monde, jouer 2 minutes, prendre 3 captures (F2), quitter.
- [ ] Sauvegarde automatique créée ? Taille et temps raisonnables ?
- [ ] Les captures apparaissent-elles en direct dans l'onglet pendant la partie ?

### 7. Export
- [ ] Contenu → « Exporter » en `.mrpack` (Fabric uniquement) : temps, taille du fichier.

## Démarrage du launcher
- [ ] Avec les 3 packs installés, fermer et rouvrir Tandem 3 fois : les lignes `backend ready ms=` et `UI ready ms=` du log donnent les temps.
- [ ] Une fois après un redémarrage de Windows (démarrage à froid).
