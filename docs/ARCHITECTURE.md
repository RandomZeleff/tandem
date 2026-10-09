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
│   ├─ translate  extraction, LLM, resource pack       │
│   └─ p2p        détection LAN, tunnel iroh           │
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
├─ cache/               réponses API, traductions
└─ logs/                tandem.YYYY-MM-DD.log (rotation journalière, 7 fichiers)
```

## Flux clés

- **Lancement** : résoudre version + loader → lister fichiers requis → télécharger ce qui manque dans `store` → hardlink → construire la commande → spawn → streamer les logs via events.
- **Multi P2P** : hôte détecte l'annonce LAN → ouvre un endpoint iroh → code d'invitation ; invité résout le code → connexion QUIC → proxy TCP local ↔ flux QUIC.
- **Traduction** : scan des jars → extraction des chaînes → masquage des codes de format → LLM par lots → validation → resource pack → cache par hash.

## Logs

`tracing` → 3 sorties : stderr, fichier journalier, buffer mémoire (2000 entrées) poussé au frontend via l'event `log://entry`.
Le frontend s'abonne puis récupère l'historique (`get_logs`) ; le champ `seq` élimine les doublons.
Filtre par défaut `info,tandem*=debug`, surchargeable avec `RUST_LOG`.
