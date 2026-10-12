---
name: Tandem
description: A modern Minecraft launcher that speaks the language of Mojang's recent menus.
colors:
  grass: "#3c8527"
  grass-hover: "#47962f"
  grass-edge: "#1d4d13"
  grass-light: "#5dbb3f"
  xp: "#7fd14f"
  xp-text: "#8fd16a"
  xp-deep: "#4e9a1e"
  stone: "#d0d1d4"
  stone-hover: "#e2e3e5"
  stone-edge: "#8c8d91"
  stone-ink: "#1b1c1e"
  bedrock: "#101113"
  slate-900: "#151619"
  slate-800: "#1b1c1f"
  slate-750: "#222327"
  slate-700: "#26272b"
  slate-650: "#2e3034"
  slate-600: "#383a3f"
  slate-500: "#45474d"
  line: "#303237"
  line-strong: "#46484e"
  divider: "#2b2d31"
  outline: "#0c0d0e"
  chalk: "#f2f1ed"
  chalk-2: "#dddbd5"
  chalk-3: "#c9ccd1"
  muted: "#afb2b8"
  faint: "#8d9198"
  gold: "#f2c744"
  gold-deep: "#6b5518"
  redstone: "#d93b2e"
  redstone-hover: "#e5493c"
  redstone-edge: "#7d1d16"
  redstone-text: "#f28a80"
  diamond: "#4fd6d0"
  amethyst: "#b99cf2"
  danger: "#2c1716"
  danger-line: "#6e2a26"
  warning: "#2b2213"
  success: "#182615"
  success-line: "#2e5a1a"
typography:
  display-hero:
    fontFamily: "Jersey 10, Geist Variable, sans-serif"
    fontSize: "64px"
    fontWeight: 400
    lineHeight: "58px"
    letterSpacing: "0.01em"
  display-title:
    fontFamily: "Jersey 10, Geist Variable, sans-serif"
    fontSize: "44px"
    fontWeight: 400
    lineHeight: "40px"
    letterSpacing: "0.01em"
  play-label:
    fontFamily: "Jersey 10, Geist Variable, sans-serif"
    fontSize: "28px"
    fontWeight: 400
    letterSpacing: "0.01em"
  headline:
    fontFamily: "Geist Variable, system-ui, sans-serif"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: "26px"
  title:
    fontFamily: "Geist Variable, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 650
    lineHeight: "22px"
  body-lead:
    fontFamily: "Geist Variable, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: "22px"
  body:
    fontFamily: "Geist Variable, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "20px"
    fontFeature: "tnum"
  meta:
    fontFamily: "Geist Variable, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: "18px"
  label:
    fontFamily: "Geist Variable, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: "16px"
  mono:
    fontFamily: "Geist Mono Variable, ui-monospace, monospace"
    fontSize: "12px"
    fontWeight: 400
rounded:
  xs: "2px"
  sm: "3px"
  md: "4px"
  lg: "6px"
spacing:
  2xs: "4px"
  xs: "6px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "20px"
  2xl: "24px"
components:
  button:
    backgroundColor: "{colors.slate-600}"
    textColor: "{colors.chalk}"
    typography: "{typography.body}"
    rounded: "{rounded.sm}"
    padding: "0 16px 3px"
    height: "38px"
  button-hover:
    backgroundColor: "{colors.slate-500}"
  button-primary:
    backgroundColor: "{colors.grass}"
    textColor: "#ffffff"
    rounded: "{rounded.sm}"
    padding: "0 16px 3px"
    height: "38px"
  button-primary-hover:
    backgroundColor: "{colors.grass-hover}"
  button-play:
    backgroundColor: "{colors.grass}"
    textColor: "#ffffff"
    typography: "{typography.play-label}"
    rounded: "{rounded.md}"
    padding: "0 32px"
    height: "54px"
  button-light:
    backgroundColor: "{colors.stone}"
    textColor: "{colors.stone-ink}"
    rounded: "{rounded.sm}"
    padding: "0 16px 3px"
    height: "38px"
  button-light-hover:
    backgroundColor: "{colors.stone-hover}"
  button-danger:
    backgroundColor: "{colors.redstone}"
    textColor: "#ffffff"
    rounded: "{rounded.sm}"
    padding: "0 16px 3px"
    height: "38px"
  button-danger-hover:
    backgroundColor: "{colors.redstone-hover}"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    rounded: "{rounded.sm}"
    padding: "0 16px"
    height: "38px"
  button-ghost-hover:
    textColor: "{colors.chalk}"
  panel:
    backgroundColor: "{colors.slate-700}"
    rounded: "{rounded.md}"
    padding: "16px"
  dialog:
    backgroundColor: "{colors.slate-750}"
    rounded: "{rounded.lg}"
    padding: "24px"
    width: "460px"
  field:
    backgroundColor: "{colors.slate-900}"
    textColor: "{colors.chalk}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "38px"
  slot:
    backgroundColor: "{colors.slate-900}"
    rounded: "{rounded.sm}"
  chip:
    backgroundColor: "rgb(255 255 255 / 0.07)"
    textColor: "{colors.chalk-2}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "0 8px"
    height: "24px"
  nav-item:
    textColor: "{colors.muted}"
    rounded: "{rounded.sm}"
    padding: "0 10px"
    height: "44px"
  nav-item-active:
    backgroundColor: "rgb(255 255 255 / 0.08)"
    textColor: "{colors.chalk}"
---

# Design System: Tandem

## Overview

**Creative North Star: "The Modern Minecraft Menu"**

Tandem borrows the grammar of Mojang's recent menus (Ore UI, the new launcher) rather than the pixel decor of the old title screen. Surfaces are solid anthracite in clear tonal steps; controls are thick, filled and physical, with a darker band along their bottom edge that collapses when pressed. Minecraft green is reserved for the one action that matters on a screen, which is almost always Play. The result should read at a glance as a Minecraft launcher, while staying clean enough to scan in the evening next to Discord.

The interface is dark only and moderately dense: a desktop window (1280×800 reference) with a fixed 240px sidebar, content capped at 1400px, and hierarchy carried by weight and tone more than by size. Pixel art appears only where Minecraft itself is pixel art (skin heads, block faces, loader marks, the 12-pixel icon set, drawn skies) and always at whole-pixel scales. Everywhere else the system is smooth: Geist for text, near-square corners, hairline edges.

Rejected, by the direction this build implements: the generic dark SaaS launcher (glass, blur, decorative gradients, large rounded cards) and the old all-pixel launcher look (pixel borders everywhere, pixel body text).

**Key Characteristics:**
- Graphite surfaces stepped ground → panel → raised, no glass, no blur.
- Buttons with a 1px near-black outline and a 4px darker bottom edge that sinks on press.
- One green primary action per view; gold only for caution; redstone for stop and destructive actions.
- Square display face (Jersey 10) for big titles and the Play label only; Geist for everything else.
- Pixel art at whole-pixel sizes only, where Minecraft is natively pixel art.
- No browser focus ring: keyboard focus reuses the hover look (D26).

## Colors

A cool, near-neutral graphite ramp lit by one saturated Minecraft green, with gold, redstone, diamond and amethyst held back as semantic signals.

### Primary
- **Grass Green** (grass): the face of the primary button and the Play button; the checked state of toggles and checkboxes; the notification count badge. Its hover lift is **Grass Hover** and its pressed band is **Grass Edge** (dark forest).
- **Bright Grass** (grass-light): the 2px focus/open ring inside fields and the 3px underline of the selected page tab.
- **Experience Green** (xp, xp-deep): progress fills (the segmented XP bar), the "running" dot, the caret colour, the selection tint. **XP Text** (xp-text) is the readable green for "En cours", success copy, links in author markdown and inline actions in toasts.

### Secondary
- **Stone** (stone, stone-hover, stone-edge, stone-ink): the light Ore UI button, a strong secondary action placed beside green ("Inviter des amis", "Ajouter mon compte"). Dark ink on a pale face.

### Tertiary
- **Gold** (gold, gold-deep): caution only. Warning alerts and panels, crash and missing-mod headings, beta versions, untranslated counts, a medium ping, the account-required gate.
- **Redstone** (redstone, redstone-hover, redstone-edge, redstone-text): Stop and destructive buttons; error text and error panels.
- **Diamond** (diamond): "already translated by the authors" and DEBUG log lines. **Amethyst** (amethyst): the AI translation sparkle. Both are narrow, data-specific accents.

### Neutral
- **Bedrock** (bedrock): the deepest well, behind the XP bar.
- **Slate 900** (slate-900): title bar, sidebar, sunken wells (fields, slots, segmented tab track).
- **Slate 800** (slate-800): the app ground behind page content.
- **Slate 750** (slate-750): floating surfaces: dialogs, menus, toasts.
- **Slate 700** (slate-700): panels and cards, one step above the ground. **Slate 650** is their hover.
- **Slate 600 / 500** (slate-600, slate-500): the neutral button face and its hover; the selected segmented tab; scrollbar thumb.
- **Line, Line Strong, Divider** (line, line-strong, divider): hairlines for skeletons and markdown, field strokes, and the 1px seams between title bar, sidebar and content.
- **Outline** (outline): the 1px near-black ring around every built button.
- **Chalk** (chalk, chalk-2, chalk-3): warm off-white text: primary, secondary body copy, tertiary.
- **Muted, Faint** (muted, faint): meta lines, inactive nav and tabs, placeholders, separators.
- **Message grounds** (danger, danger-line, warning, success, success-line): tinted dark grounds and strokes for alerts and toasts.

### Named Rules
**The One Green Rule.** A view has one green-faced button: the action the player came for. Everything else is neutral slate, light stone, or ghost.

**The Gold Means Caution Rule.** Gold signals something that needs attention. It is never decoration, never a brand accent, never a primary button.

## Typography

**Display Font:** Jersey 10 (with Geist Variable, sans-serif), self-hosted, OFL.
**Body Font:** Geist Variable (with system-ui, sans-serif).
**Label/Mono Font:** Geist Mono Variable (with ui-monospace) for codes, percentages and versions.

**Character:** A square, blocky display face carries the Minecraft voice in a few large words; Geist keeps everything else quiet, legible and tabular (tabular numerals are on by default).

### Hierarchy
- **Display hero** (Jersey 10 400, 64px/58px): the Home picture's title (last played instance, or the welcome line). One per screen at most.
- **Display title** (Jersey 10 400, 44px/40px): page titles (Instances, Découvrir, Jouer ensemble, Réglages, an instance or project name).
- **Play label** (Jersey 10 400, 28px, uppercase): the large Play button and the welcome "Créer une instance" button only.
- **Headline** (Geist 600, 20px/26px): dialog titles.
- **Title** (Geist 650, 16px/22px): panel and section titles; card names use 15px semibold.
- **Body** (Geist 400, 14px/20px): default text, buttons (at 600), list rows. **Body lead** (15px/22px) for prominent sentences.
- **Meta** (Geist 400, 13px/18px): metadata lines, help text, segmented tabs, secondary labels.
- **Label** (Geist 500, 12px/16px): chips, timestamps, small captions, sidebar group labels.

The full Geist scale (xs 12, meta 13, sm 14, base 15, lg 17, xl 20, 2xl 24, 3xl 32, 4xl 40) is defined, but the shipped interface lives almost entirely between 12 and 15px; larger Geist sizes are rare (invite code in mono, a guest session heading).

### Named Rules
**The Weight Not Size Rule.** Within Geist, rank with weight (400 / 500 / 600 / 650) and tone (chalk → chalk-2 → muted → faint), not with new sizes.

**The Square Face Is Rare Rule.** Jersey 10 appears only at title (44), hero (64) and the Play label (28), plus the title-bar wordmark. Never in body copy, buttons other than Play/Create, labels or numbers. It always carries its short hard drop shadow (0 3px 0, 35% black), the way Minecraft draws title text.

## Layout

A fixed desktop shell: a 40px title bar in slate-900; a 240px sidebar in slate-900 separated by a 1px divider; a scrolling content area on slate-800 with 24px padding and content capped at 1400px, centred. The instance page is the exception: it fills the area edge to edge with its own header and tabs.

Pages stack sections vertically with 20–24px gaps. Card grids are fluid auto-fill grids (minimum 220–240px columns, 16px gutters). The Home page opens with a large picture band (clamp(340px, 48vh, 460px)) whose title and actions sit bottom-left with 36px side padding.

Spacing rhythm is a 4px base with heavy use of 8, 12 and 16px (gap 8 is the default between siblings, 12 inside rows and cards, 16 between cards). Controls share a 38px height (buttons, fields, select); compact icon buttons are 32px; nav rows 44px; recent-instance rows 48px.

## Elevation & Depth

Depth is tonal first: each surface is a step lighter than what it sits on (slate-900 → 800 → 700 → 750 for floating layers). Shadows are structural, not ambient: inset hairlines draw edges, inset top shades sink wells, inset bottom bands build buttons. Only floating layers (menus, selects, toasts, dialogs) cast a real drop shadow.

### Shadow Vocabulary
- **Panel edge** (`inset 0 0 0 1px rgb(255 255 255 / 0.055), inset 0 1px 0 rgb(255 255 255 / 0.04), 0 1px 2px rgb(0 0 0 / 0.35)`): every panel and card.
- **Button build** (`0 0 0 1px #0c0d0e, inset 0 1px 0 <shine>, inset 0 -4px 0 <edge>`): every filled button; pressed becomes `0 0 0 1px #0c0d0e, inset 0 2px 0 rgb(0 0 0 / 0.18), inset 0 -1px 0 <edge>`.
- **Well** (`inset 0 0 0 1px rgb(0 0 0 / 0.35), inset 2px 2px 0 rgb(0 0 0 / 0.3), inset -1px -1px 0 rgb(255 255 255 / 0.05)`): inventory slots holding icons, heads and the invite code.
- **Field** (`inset 0 0 0 1px #46484e, inset 0 2px 0 rgb(0 0 0 / 0.3)`): text fields and selects.
- **Floating** (`0 12px 32px rgb(0 0 0 / 0.55)`): menus and select popovers; toasts use `0 10px 28px rgb(0 0 0 / 0.5)` with a tone-coloured 1px inset ring.
- **Title drop** (`text-shadow: 0 3px 0 rgb(0 0 0 / 0.35)`): display titles.

### Named Rules
**The Built Not Floating Rule.** At rest, nothing on the page floats. Panels sit one tone up with a hairline; buttons are built with an edge; only popovers, toasts and dialogs cast shadows.

## Shapes

Near-square everywhere: 3px on buttons, fields, chips, rows and slots; 4px on panels, cards and menus; 6px on dialogs and the Home picture band; 2px on checkboxes, toggle knobs and inline code. Edges are drawn with 1px inset rings rather than borders, and coloured state panels (warning, danger, success) are tinted fills with a matching ring, never a side stripe.

Pictures always fill a fixed box with object-cover, so every instance visual takes its container's format (16:8 on cards). Legibility over pictures comes from dark scrims (bottom-up, plus a left-side scrim on the Home band); gradients exist only in that role.

Pixel art keeps whole pixels: the icon set is drawn on a 12px grid and renders only at 12 or 24px (requests snap); block faces are 8×8 and render at multiples of 8; skies are flat colour bands, never smooth gradients; all pixel SVGs use crisp edges.

## Components

### Buttons
Tactile and built: a solid face, a near-black outline, a bright top hairline, and a darker bottom band that the face sinks into.
- **Shape:** near-square (3px), 38px tall, 16px side padding with 3px extra at the bottom for the band; label 14px semibold with an 8px icon gap.
- **Neutral:** slate-600 face, slate-500 hover, #26272b band.
- **Primary:** grass face, grass-hover, grass-edge band, white label with a 1px dark text shadow.
- **Light:** stone face and ink, stone-edge band, a 50% white top shine.
- **Danger:** redstone face, redstone-edge band, white label. Used for Stop and destructive confirmations.
- **Microsoft:** white face, #b9b9b9 band, on the same build, for the sign-in button only.
- **Ghost:** no build; muted text, 7% white wash on hover.
- **Hover / Focus:** the face lightens one step (90ms); keyboard focus shows exactly the hover look.
- **Pressed:** padding shifts down, the band collapses from 4px to 1px and a 2px top shade appears (70ms), so the face sinks.
- **Disabled:** label at 50%, face desaturated and dimmed.

### Play Button
The signature control. Large form: 54px tall, 4px corners, 32px side padding, Jersey 10 28px uppercase "JOUER" with an 18px play icon. Medium (38px) and icon-only (32×36) forms appear on cards and rows. It is stateful: green Play when idle, a disabled neutral button with an XP-green download icon and a mono percentage while installing, redstone Stop while running.

### Chips
- **Style:** 24px tall, 3px corners, 7% white wash, chalk-2 12px medium label, optional 6px-gap icon. No stroke.

### Cards / Containers
- **Corner Style:** 4px (panels, cards); 6px (dialogs).
- **Background:** slate-700; hover and focus slate-650 on clickable cards; slate-750 for floating panels.
- **Shadow Strategy:** panel edge (see Elevation & Depth).
- **State panels:** warning, danger and success panels mix 7–8% of gold, redstone or bright grass into slate-700 with a matching 1px ring.
- **Internal Padding:** 16px for panels, 12–14px for card bodies, 24px for dialogs.
- **Instance card:** a 16:8 picture box with a dark bottom scrim and the instance slot (40px) at bottom-left, then name (15px semibold), meta line with loader mark, and last-played time beside an icon Play button.

### Inputs / Fields
- **Style:** slate-900 well, 1px line-strong ring, 2px inner top shade, 3px corners, 38px tall, 12px padding.
- **Hover:** ring lifts to slate-500.
- **Focus / Open:** a 2px bright-grass inner ring (the one place focus has its own look, because typing needs a location).
- **Toggles and checkboxes:** square, inventory-style: slate-900 with a line-strong ring when off; grass with a darker bottom band when on; square white knob.
- **Select:** a field-styled trigger with a caret; options in a slate-750 floating panel, 32px rows.

### Navigation
- **Sidebar:** slate-900, 240px. Account card on top (slot with skin head, name, account kind). Nav rows 44px, 3px corners, 12px pixel icon + 14px label: muted medium at rest, 4% white wash on hover/focus, 8% white wash and chalk semibold when active. A "Récentes" group of 48px instance rows below; Réglages and version pinned at the bottom above a divider.
- **Tabs, underline:** 44px tabs, muted medium at rest, chalk semibold with a 3px bright-grass bottom bar when selected. For page sections.
- **Tabs, segmented:** a slate-900 well track (4px corners, 3px padding); the selected segment is a raised slate-600 chip. For compact filters.

### Inventory Slot
A recessed slate-900 well, shaded at the top-left, that frames every pixel object: block faces, skin heads, project icons, instance icons, the invite code. Block faces inside take about two thirds of the slot, rounded to a multiple of 8.

### Instance Visual
The picture for an instance follows a fixed chain: the modpack's featured gallery image, else the player's latest screenshot in that instance, else a drawn sky. The drawn sky is stable per instance and chosen among day, sunset and night (night and sunset with stars), over pixel hills in the instance's block colour. The image always covers its box and fades in over the drawing (500ms ease-out, none under reduced motion).

### XP Bar
Segmented progress (16 segments by default) on a bedrock track with a 2px inset; filled segments are xp green with an xp-deep bottom band and a pale top highlight.

### Alerts and Toasts
- **Alert:** inline, 3px corners, tinted ground (danger, warning, success) with its 1px line, text in the tone colour, optional close button.
- **Toast:** bottom-right stack, 360px wide, slate-750, 4px corners, a 12px tone icon, a tone-coloured inset ring and the floating shadow; slides up 8px over 180ms (ease-out-quart) and simply appears under reduced motion.

### Motion
Sober and functional. Buttons press in 70ms and recolour in 90ms; hover washes and tab changes take 100ms; images fade in over 500ms; toasts slide in over 180ms; skeletons breathe at 1.4s; the sign-in dots and current-step marker step with `steps(2)`. Every animation stops under `prefers-reduced-motion`.

## Do's and Don'ts

### Do:
- **Do** give every filled button the full build: 1px #0c0d0e outline, top shine, 4px darker bottom band that collapses to 1px on press.
- **Do** keep one green (grass) button per view, on the action the player came for, usually Play.
- **Do** let keyboard focus reuse the hover look on buttons, rows, nav and tabs (D26); fields alone get the 2px bright-grass ring.
- **Do** step surfaces by tone (slate-900 → 800 → 700 → 750) and draw edges with 1px inset rings.
- **Do** keep corners between 2px and 6px: 3px controls, 4px panels, 6px dialogs.
- **Do** render pixel icons at 12 or 24px and block faces at multiples of 8, with crisp edges.
- **Do** show an instance through its visual chain (featured gallery image → latest screenshot → drawn sky), always object-cover in a fixed box.
- **Do** rank Geist text with weight and tone before reaching for a larger size.
- **Do** stop every animation under reduced motion.

### Don't:
- **Don't** use glass, backdrop blur, decorative gradients or large rounded cards; gradients only appear as dark legibility scrims over pictures.
- **Don't** draw a browser focus ring or a glow on buttons and rows.
- **Don't** use Jersey 10 outside page titles (44px), the Home hero (64px), the Play label (28px) and the wordmark.
- **Don't** set body text, borders or pixel frames in pixel style; pixel art is for skin heads, block faces, loader marks, the icon set and drawn skies.
- **Don't** put small uppercase kicker labels above titles.
- **Don't** use gold for anything but caution, or green for anything but the main action, progress and success.
- **Don't** mark state panels with a coloured side stripe; tint the fill and ring the whole panel.
- **Don't** scale pixel art to fractional pixel sizes or smooth it.
