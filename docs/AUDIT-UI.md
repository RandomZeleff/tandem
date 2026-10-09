# Audit de l'UI — 2026-10-09

Audit technique de l'interface (`src/`), écran par écran, avant la v0.1. Lecture du code de tous les écrans et composants, détecteur Impeccable (0 alerte), recherche de motifs répétés. Aucune modification de code.

## Score

| # | Dimension | Score | Constat principal |
|---|-----------|-------|-------------------|
| 1 | Accessibilité | 2/4 | Anneau de focus invisible sur la plupart des boutons (rogné par `clip-path`), cartes d'instance inaccessibles au clavier, dialogues sans gestion du focus |
| 2 | Performance | 3/4 | Sain après D24/D25 ; restent des détails (`For` sur des listes de booléens/nombres) |
| 3 | Tailles de fenêtre | 3/4 | Mise en page fixe pensée pour ≥ 1280 px ; l'accueil à 3 colonnes est serré à la largeur minimale (1100 px), à vérifier |
| 4 | Thème / tokens | 2/4 | Tokens présents dans `index.css`, mais 135 couleurs en dur dans les `.tsx` et 6 copies du bandeau d'erreur |
| 5 | Cohérence d'implémentation | 3/4 | Système pixel cohérent et propre au produit ; manque de composants partagés (erreur, onglets, menu) |
| **Total** | | **13/20** | **Acceptable : base saine, finitions à faire** |

**Verdict de cohérence : réussi.** L'app a une identité claire (coins en escalier, boutons biseautés, emplacements d'inventaire, barre d'XP, icônes pixel dessinées à la main) appliquée partout. Les problèmes sont des finitions et de l'accessibilité, pas un manque de direction.

## Constats par priorité

### P1 — à corriger avant la v0.1

1. **Anneau de focus invisible sur les boutons à coins pixel**
   - Où : `.px-corners*` (`index.css`), 64 usages (boutons, panneaux, carte du compte)
   - Impact : `clip-path` rogne aussi le contour (`outline`) dessiné à l'extérieur : au clavier, on ne voit pas où on est sur la majorité des boutons.
   - Correctif : anneau de focus dessiné à l'intérieur (`box-shadow` inset or / `outline-offset` négatif) pour les éléments `.px-corners*`.

2. **Cartes d'instance inaccessibles au clavier**
   - Où : `InstanceCard.tsx` (`<article onClick>`)
   - Impact : impossible d'ouvrir une instance depuis l'accueil ou la page Instances sans souris (seul le bouton Jouer est atteignable).
   - Correctif : lien/bouton qui couvre la carte, ou `tabindex` + Entrée/Espace.

3. **Dialogues sans gestion du focus**
   - Où : `Dialog.tsx`, visionneuse de captures
   - Impact : le focus reste derrière la fenêtre : Tab parcourt la page cachée, Entrée peut déclencher un bouton du fond ; le focus n'est pas rendu à l'élément d'origine à la fermeture.
   - Correctif : focus initial dans le dialogue, Tab bouclé dedans, focus restauré à la fermeture.

4. **Menu du compte**
   - Où : `Sidebar.tsx` (`AccountCard`)
   - Impact : Échap ne le ferme pas ; le bouton « Retirer » n'apparaît qu'au survol (invisible au clavier) ; retirer un compte se fait sans confirmation.
   - Correctif : Échap + clic extérieur, bouton visible au focus (`group-focus-within`), confirmation ou annulation possible.

5. **Pas d'historique de navigation** (déjà au plan)
   - Où : `lib/store.ts` (`route` = un simple signal)
   - Impact : pas de retour arrière (souris 4/5, Alt+←) ; l'onglet d'une instance revient sur « Console » et Découvrir perd recherche et filtre à chaque passage.

6. **5 menus déroulants natifs + 1 case à cocher native** (déjà au plan)
   - Où : `LogView`, `NewInstanceDialog` (×2 + case « Snapshots »), `Discover`, `MemoryPicker`
   - Impact : liste blanche/système de Windows au milieu d'une UI pixel ; rendu différent sur Mac.

### P2 — prochaine passe

7. **Écrans vides affichés pendant le chargement.** Contenu, Mondes et Captures montrent « Aucun… » le temps que la liste arrive : faux message, puis saut de mise en page. Il faut un état « chargement » distinct de « vide ».
8. **Suppressions sans filet.** Retirer un mod (corbeille dans Contenu) est immédiat et sans annulation ; idem pour un compte. Proposer une confirmation légère ou un « Annuler » quelques secondes.
9. **Bandeau d'erreur dupliqué 6 fois** (`bg-[#2A1414]…`), jamais refermable, et qui reste affiché en changeant d'onglet. Un composant `Alert` (erreur / info / succès) avec bouton fermer.
10. **Retour des actions incohérent.** L'export affiche une notice, la plupart des autres actions réussies ne disent rien. Un système de notifications courtes (« toasts ») unifierait.
11. **Erreurs avalées** : « Ouvrir le dossier » (accueil, page d'instance) appelle `void api.openInstanceFolder(…)` sans `catch` : en cas d'échec, rien ne s'affiche.
12. **Découvrir sans état « aucun résultat ».** Une recherche vide affiche juste « 0 résultats ».
13. **Réglages quasi vides.** La page ne contient que la version, le dossier et les logs ; le seul vrai réglage global (sauvegarde auto des mondes) est caché dans l'onglet Mondes. Prévoir de vrais réglages (sauvegardes, mémoire par défaut, Java, dossier de données, comportement à la fermeture du jeu…).
14. **Nombres au format anglais.** `formatBytes` et la mémoire affichent « 2.6 Mo », « 1.5 Go » : en français, « 2,6 Mo ». Passer par `Intl.NumberFormat("fr-FR")`.
15. **Onglets sans clavier.** Les `role="tablist"` (instance, Découvrir, filtres d'instances) ne gèrent pas les flèches et n'ont pas de `tabpanel` relié.
16. **Couleurs en dur.** 135 hex dans les composants (rouge d'erreur, vert XP `#8BE04E`, gris `#C9CCD1`…) au lieu des tokens : à ramener dans `@theme` au fil des corrections.
17. **Accueil à la largeur minimale** (1100 px) : 3 cartes + colonne de 292 px laissent ≈ 155 px par carte. À vérifier et passer à 2 colonnes si besoin.

### P3 — si le temps le permet

18. Pas de prise en compte de `prefers-reduced-motion` (peu d'animations, impact faible).
19. `XpBar` et `GameStatsPanel` utilisent `For` sur des booléens/nombres recréés à chaque mise à jour : `Index` est fait pour ça.
20. Pas de raccourcis clavier globaux (Ctrl+F pour chercher dans Découvrir, Ctrl+N pour une nouvelle instance…).

## Problèmes systémiques

- **Composants de base manquants** : alerte, menu déroulant, onglets, popover. Chaque écran refait les siens, d'où les écarts (focus, clavier, couleurs).
- **Le focus clavier n'a jamais été traité de bout en bout** : anneau rogné, cartes non focusables, dialogues et menus sans piège ni restauration.
- **États de chargement non distingués des états vides** dans les onglets d'instance.

## Ce qui marche bien

- Identité visuelle forte et cohérente, propre au produit (pas une UI générique).
- Bons libellés accessibles sur les boutons-icônes (`aria-label` partout où il n'y a pas de texte), `role="progressbar"` et `role="switch"` corrects.
- Textes clairs et chaleureux en français, messages d'erreur explicites (crash, loader indisponible, Iris manquant).
- Images paresseuses (icônes, captures), interface légère, démarrage et console optimisés (D24, D25).
- Cas particuliers pensés : instance Vanilla dans Découvrir, shaders sans Iris, aucune instance, hors ligne.

## Ordre proposé

1. Composants de base : `Select` (menus déroulants maison), `Alert`, onglets accessibles, `Dialog` avec focus, popover.
2. Accessibilité clavier : anneau de focus, cartes, menus (P1 1-4).
3. Navigation avec historique (P1 5).
4. États chargement/vide, suppressions avec filet, toasts, erreurs avalées (P2 7-12).
5. Réglages, format français des nombres, tokens, accueil à 1100 px (P2 13-17).
6. P3 si le temps le permet.
