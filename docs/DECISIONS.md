# Journal des décisions

Format : date · décision · pourquoi · statut (proposé / validé / abandonné)

---

### D1 · 2026-10-08 · Tauri 2 + Rust
- Pourquoi : binaire léger, faible RAM vs Electron, Rust idéal pour I/O parallèles et P2P.
- Statut : validé

### D2 · 2026-10-08 · Frontend SolidJS + Tailwind
- Pourquoi : réactivité fine sans virtual DOM, bundle léger, cohérent avec l'objectif performance.
- Statut : validé (Tailwind v4 via `@tailwindcss/vite`)

### D3 · 2026-10-08 · Logique métier dans une crate `core` séparée
- Pourquoi : testable sans Tauri, réutilisable (CLI, serveur relais éventuel).
- Statut : validé

### D4 · 2026-10-08 · Store de fichiers par hash + hardlinks
- Pourquoi : déduplication entre instances, changement de version quasi instantané.
- Risque : hardlinks impossibles entre volumes différents → repli sur copie.
- Statut : validé

### D5 · 2026-10-08 · Multi via tunnel du LAN vanilla (sans mod) avec `iroh`
- Pourquoi : marche sur toutes versions/loaders, pas de serveur à payer, NAT traversal + relais intégrés.
- Statut : validé

### D6 · 2026-10-08 · Modrinth en source principale, CurseForge en secondaire
- Pourquoi : API ouverte et propre ; CurseForge exige une clé et certains mods interdisent la distribution tierce.
- Statut : validé

### D7 · 2026-10-08 · Accès API Minecraft : app Azure + validation Mojang
- Les nouvelles apps Azure reçoivent un 403 sur `api.minecraftservices.com` tant que leur Client ID n'est pas approuvé.
- Étapes : app Azure (comptes Microsoft personnels uniquement, tenant `consumers`, public client flows activés) → formulaire https://aka.ms/mce-reviewappid avec le Client ID.
- En attendant : développement avec compte offline.
- Statut : en cours
