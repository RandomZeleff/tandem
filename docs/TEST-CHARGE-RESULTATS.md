# Test sous charge — résultats du 2026-10-09

Fait par Claude sur le PC de Raphaël (Windows 11, Ryzen 5 8400F, 16 Go), avec les outils `modpack` et `play` de `tandem-core` en release, dans un dossier de données séparé (`C:\Users\rapha\TandemLoadTest`, partant de zéro : Java, Minecraft et assets téléchargés aussi). L'interface n'a pas été pilotée : les points UI de la [fiche](TEST-CHARGE.md) restent à faire par un humain.

## Mesures

| | Prominence II | Create+ | Cobblemon [NeoForge] |
|---|---|---|---|
| Loader / version | Fabric 1.20.1 | Forge 1.19.2 (dernière version stable ; les 1.21.1 sont en alpha) | NeoForge 1.21.1 |
| Mods chargés | 678 | 257 | 73 + gros overrides |
| Taille du `.mrpack` | 423 Mo | 27 Mo | 101 Mo |
| Mods téléchargés | 563 fichiers, 1,28 Go | 256 fichiers, 224 Mo | 73 fichiers, 222 Mo |
| Installation | 171 s (avant correctif réseau) | 23 s | 26 s |
| Préparation au 1er lancement | 20 s (Java 17 + assets) | 30 s (installeur Forge) | 32 s (installeur NeoForge + Java 21) |
| Préparation ensuite | 1,0 s | 1,4 s | — |
| Fenêtre du jeu (« Setting user ») | 23 s | 18 s (à chaud) | 54 s (1er lancement) |
| Menu | ~80 s | ~38 s (à chaud) | ~70 s (1er lancement) |
| Mémoire au menu | ~4,0 Go | ~4,0-4,2 Go | ~3,8-3,9 Go |
| Processeur au menu | 3 % | 6 % | 1 % |
| Lignes de sortie jusqu'au menu | ~3 500 | ~3 900 | ~3 200 |
| Export `.mrpack` | 13 s (2 121 fichiers inclus) | 2 s | 4 s |

La mémoire automatique (D18) a donné ~8 Go à Prominence ; le jeu en utilise ~4 Go au menu, sans manque.

## Problèmes trouvés et corrigés

1. **Téléchargements 4 à 6× trop lents.** Un gros fichier descendait à 6,7 Mo/s alors que `curl` tirait 43 Mo/s du même CDN Modrinth. Cause : la fenêtre de contrôle de flux HTTP/2 par défaut (64 Ko), qui plafonne le débit à 64 Ko par aller-retour. Corrigé avec la fenêtre adaptative de `reqwest` : **26 à 36 Mo/s**. Profite à tous les téléchargements (mods, assets, Java). Un tampon d'écriture plus grand a été essayé avant : aucun effet, non gardé.
2. **Assistants de modpack qui survivent au jeu.** Crash Assistant (présent dans Prominence et Create+) lance sa propre JVM via un processus intermédiaire. Quand Tandem arrête le jeu, elle restait vivante (25 minutes et plus) et gardait le tuyau de sortie du jeu ouvert. Résultat dans l'app : l'instance pouvait rester « en cours » tout ce temps. `taskkill /T` ne suffit pas (le lien de parenté est cassé par l'intermédiaire). Corrigé : le jeu tourne dans un **Job Object** Windows (un groupe de processus sous Unix) et « Arrêter » tue tout le groupe ; de plus, après la fin du jeu, la lecture de sa sortie n'attend plus que 2 s. Vérifié : plus aucun Java ne reste, fin détectée immédiatement.
3. **Accents illisibles dans la console.** Java écrit `System.out` dans l'encodage de Windows (cp1252), affiché en `�`. Le jeu est maintenant lancé avec `stdout.encoding` / `sun.stdout.encoding` en UTF-8 : il ne reste que 3 lignes avec `�` sur ~3 500 (probablement écrites par un mod dans un autre encodage).
4. **`.mrpack` gardé en cache après installation** (423 Mo pour Prominence). Supprimé une fois l'installation réussie ; gardé après un échec pour qu'une nouvelle tentative ne le retélécharge pas.
5. **Outil `play`** : il lisait la sortie du jeu en UTF-8 strict et s'arrêtait au premier octet invalide, ce qui figeait le jeu (tuyau plein). Lecture tolérante, comme l'app.

## Pistes (pas encore faites)

- **Dépendances manquantes** : désactiver une bibliothèque (ex. Architectury dans Create+) ne fait pas planter le jeu : Forge affiche son propre écran d'erreur et reste ouvert, donc l'analyse de crash ne se déclenche pas. Le log est pourtant très clair (« Missing or unsupported mandatory dependencies… requested by pandalib »). Deux améliorations possibles : prévenir **avant** de désactiver un mod dont d'autres dépendent, et détecter ce message pendant le lancement pour proposer « Réactiver Architectury ».
- **Choix de la version d'un modpack** : Create+ apparaît dans Découvrir avec le filtre NeoForge 1.21.1, mais Tandem installe la dernière version *stable* (Forge 1.19.2). Montrer la version qui sera installée, et permettre d'en choisir une autre (alpha/bêta, autre version du jeu).
- **Temps de chargement** : 80 s pour 678 mods (Prominence), c'est le jeu lui-même ; les suggestions de mods de perf (D19) sont la bonne réponse côté Tandem (ModernFix réduit nettement ce temps).

## Reste à tester par un humain

Interface pendant une installation réelle, coupure du Wi-Fi au milieu, onglet Contenu avec 563 mods et leurs icônes, réglage « Réduire le launcher », captures en direct, et démarrage du launcher à froid après redémarrage de Windows.
