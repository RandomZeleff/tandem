# Tandem Optimizations

Petit mod côté serveur que Tandem ajoute au lancement des instances **Fabric et Quilt, Minecraft 1.20 à 1.21.11** (D42). Il n'est jamais copié dans l'instance : Tandem passe `-Dfabric.addMods=<jar>` (`-Dloader.addMods` pour Quilt), le jar vit dans `<données Tandem>/mods/`. Il agit sur le serveur du jeu : le monde solo (serveur intégré) et celui qu'on ouvre à ses amis avec « Jouer ensemble ».

## Ce qu'il fait

| Optimisation | Effet en jeu | Réglage |
|---|---|---|
| **Distance d'activation** : une créature vanilla inactive à plus de 32 blocs (à l'horizontale) de tout joueur est simulée 1 tick sur 4 | Les créatures au loin bougent plus lentement. Jamais pour les créatures ajoutées par des mods (un mod peut compter le temps dans le tick de sa créature : Cobblemon fait vieillir ses Pokémon ainsi, ralentis ils s'accumulaient), ni pour : en l'air, dans l'eau ou la lave, en feu, blessée, avec une cible, un effet, une monture ou un passager, apprivoisée, amoureuse, villageois et marchands, pillards, boss, poules (œufs), zombies-villageois (soin), piglins et hoglins (zombification), tatous (écailles). Le compteur de disparition et la croissance des bébés continuent au même rythme | Réglages de Tandem, ou `-Dtandem.optimizations.activationRange=<blocs>` (0 coupe) |
| **Catégories d'apparition mortes** : saute la recherche d'apparition naturelle d'une catégorie où rien ne peut apparaître (listes vides, ou toutes les créatures bloquées par MobsBeGone) | Aucun | `-Dtandem.optimizations.spawnGroups=false` |
| **Architectury** : appel direct des écouteurs d'événements (au lieu d'un `MethodHandle` recréé à chaque appel) | Aucun | `-Dtandem.optimizations.libraries=false` |
| **Balm** : cache de l'identifiant de configuration (lu par annotation à chaque accès) | Aucun | idem |
| **Santé du monde** : durée moyenne et maximale des ticks, une ligne `[Tandem] world health: mspt=… max=… players=…` toutes les 100 ticks, seulement quand le monde est partagé (serveur dédié ou monde ouvert en LAN) ; Tandem la lit pour l'indicateur de charge de « Jouer ensemble » | Aucun | `-Dtandem.optimizations.health=false` |

Le journal du jeu le dit au démarrage : `Tandem Optimizations 1.1.0 active: activation range 32 blocks, ...`. Avec `-Dtandem.optimizations.debug=true`, il compte toutes les 10 s les ticks sautés.

Les arguments JVM de l'instance passent après ceux de Tandem : un joueur peut changer un réglage (par exemple `-Dtandem.optimizations.activationRange=48`).

## Tolérance aux versions

Le code utilise les noms *intermédiaires* de Fabric (`class_1308`, `method_5773`), stables d'une version à l'autre et ceux que Fabric utilise en jeu de 1.14 à 1.21.11. À partir de 26.1, Minecraft n'est plus obfusqué et Fabric n'a plus de noms intermédiaires : il faudra un second jar, aux noms de Mojang, qui servira aussi pour NeoForge.

Trois filets :
1. `build.py --check` compile les sources contre chaque version, cherche les cibles des Mixins dans chacune (`TARGETS`) et vérifie que chaque méthode utilisée par le jar compilé existe partout.
2. Les Mixins sont facultatifs (`required: false`, `defaultRequire: 0`) : une cible absente ne fait rien.
3. À l'exécution, une méthode manquante (`LinkageError`) coupe l'optimisation concernée au lieu de faire planter le jeu.

## Construire

Pas de Gradle : `javac` compile directement contre un jar de Minecraft aux noms intermédiaires (celui que Fabric écrit au premier lancement dans `.fabric/remappedJars/`). Il faut un JDK 17 ou plus récent.

```bash
python mods/tandem-optimizations/build.py --mc <minecraft-1.21.1>/server-intermediary.jar --check <dossier des autres versions> --libs <dossier libraries de Tandem> --jdk <JDK>
```

Le jar produit (`dist/tandem-optimizations-fabric.jar`) est reproductible (dates fixes) et versionné dans le dépôt : Tandem l'embarque à la compilation (`crates/tandem-core/src/optimizations.rs`), la CI n'a pas besoin de Java. Avant de committer un nouveau jar, lancer `--check` contre toutes les versions de 1.20 à 1.21.11 (pour obtenir un jar de serveur par version : lancer `fabric-server-launch.jar` de cette version une fois, sans `eula.txt`).
