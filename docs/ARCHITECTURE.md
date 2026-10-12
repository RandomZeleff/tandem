# Architecture

## Vue d'ensemble

```
┌──────────────── Frontend (WebView) ────────────────┐
│  UI uniquement : état, affichage, appels invoke()   │
└───────────────▲──────────────────┬─────────────────┘
        events  │                  │ commands
┌───────────────┴──────────────────▼─────────────────┐
│  src-tauri (app Tauri) : commandes fines, events    │
├─────────────────────────────────────────────────────┤
│  crates/tandem-core (Rust pur, sans Tauri)        │
│   ├─ meta       piston-meta, loaders                 │
│   ├─ download   moteur parallèle, store par hash     │
│   ├─ java       détection / install JRE              │
│   ├─ instance   CRUD, config, lancement, process     │
│   ├─ auth       Microsoft → Xbox → XSTS → MC         │
│   ├─ content    Modrinth, fiches, mrpack, dépendances│
│   ├─ launchers  import Modrinth App, Prism, CF, Mojang│
│   ├─ translate  extraction, LLM, resource pack       │
│   ├─ duo        détection LAN, tunnel iroh           │
│   └─ optimizations  mod Tandem ajouté au lancement   │
├─────────────────────────────────────────────────────┤
│  SQLite (sqlx) · système de fichiers · keyring       │
└─────────────────────────────────────────────────────┘
```

## Données sur disque

Racine surchargeable avec la variable d'environnement `TANDEM_DATA_DIR` (utile en dev/tests).

```
%APPDATA%/Tandem/
├─ launcher.db          SQLite (instances, comptes, réglages, cache meta)
├─ store/<sha1[0..2]>/<sha1>   fichiers dédupliqués (mods, packs — Phase 3)
├─ libraries/           jars Maven partagés
├─ versions/<id>/       <id>.json, <id>.jar, natives/
├─ assets/              index + objets (format Mojang, partagés)
├─ java/<version>/      runtimes téléchargés
├─ instances/<id>/      .minecraft de chaque instance (id = slug du nom)
├─ mods/                jar de Tandem Optimizations (ajouté au lancement, D42)
├─ cache/               réponses API, traductions
└─ logs/                tandem.YYYY-MM-DD.log (rotation journalière, 7 fichiers)
```

## Flux clés

- **Lancement** : résoudre version + loader → lister fichiers requis → télécharger ce qui manque dans `store` → hardlink → construire la commande → spawn → streamer les logs via events.
- **Optimisations** : pour Fabric et Quilt 1.20 à 1.21.11, le lancement ajoute `-Dfabric.addMods=<données>/mods/tandem-optimizations-fabric-<v>.jar` (jar embarqué dans l'app, sources dans `mods/tandem-optimizations/`), rien n'est écrit dans l'instance.
- **Préparer un monde** (D46) : le serveur du monde à part démarre avec Chunky ajouté par `addMods` (`<données>/mods/chunky/`, téléchargé depuis Modrinth), Tandem envoie `chunky radius/start` sur la console et lit la progression ; annuler = `chunky pause` puis arrêt du serveur.
- **Multi P2P** : hôte détecte l'annonce LAN → ouvre un endpoint iroh → code d'invitation ; invité résout le code → connexion QUIC → proxy TCP local ↔ flux QUIC.
- **Traduction** : scan des jars → extraction des chaînes → masquage des codes de format → LLM par lots → validation → resource pack → cache par hash.

## Logs

`tracing` → 3 sorties : stderr, fichier journalier, buffer mémoire (2000 entrées) poussé au frontend via l'event `log://entry`.
Le frontend s'abonne puis récupère l'historique (`get_logs`) ; le champ `seq` élimine les doublons.
Filtre par défaut `info,tandem*=debug`, surchargeable avec `RUST_LOG`.
