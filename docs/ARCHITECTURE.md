# Architecture

## Vue d'ensemble

```
┌──────────────── Frontend (WebView) ────────────────┐
│  UI uniquement : état, affichage, appels invoke()   │
└───────────────▲──────────────────┬─────────────────┘
        events  │                  │ commands
┌───────────────┴──────────────────▼─────────────────┐
│  crate `app` (Tauri) : commandes fines, events      │
├─────────────────────────────────────────────────────┤
│  crate `core` (Rust pur, testable sans Tauri)        │
│   ├─ meta       piston-meta, loaders                 │
│   ├─ download   moteur parallèle, store par hash     │
│   ├─ java       détection / install JRE              │
│   ├─ instance   CRUD, config, lancement, process     │
│   ├─ auth       Microsoft → Xbox → XSTS → MC         │
│   ├─ content    Modrinth, CurseForge, mrpack         │
│   ├─ translate  extraction, LLM, resource pack       │
│   └─ p2p        détection LAN, tunnel iroh           │
├─────────────────────────────────────────────────────┤
│  SQLite (sqlx) · système de fichiers · keyring       │
└─────────────────────────────────────────────────────┘
```

## Données sur disque

```
%APPDATA%/<nom>/
├─ launcher.db          SQLite (instances, comptes, réglages, cache meta)
├─ store/<sha1[0..2]>/<sha1>   fichiers dédupliqués (libs, mods, assets)
├─ assets/              index + objets (format Mojang, partagés)
├─ java/<version>/      runtimes téléchargés
├─ instances/<id>/      .minecraft de chaque instance (mods = hardlinks vers store)
└─ cache/               réponses API, traductions
```

## Flux clés

- **Lancement** : résoudre version + loader → lister fichiers requis → télécharger ce qui manque dans `store` → hardlink → construire la commande → spawn → streamer les logs via events.
- **Multi P2P** : hôte détecte l'annonce LAN → ouvre un endpoint iroh → code d'invitation ; invité résout le code → connexion QUIC → proxy TCP local ↔ flux QUIC.
- **Traduction** : scan des jars → extraction des chaînes → masquage des codes de format → LLM par lots → validation → resource pack → cache par hash.
