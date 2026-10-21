# Tandem

**A fast, modern Minecraft: Java Edition launcher built for singleplayer — and for playing with your friends without renting a server.**

> v0.1 is the first public version (see [Releases](https://github.com/tandem-launcher/tandem/releases)). The interface is in French for now. Website: [tandem-launcher.github.io](https://tandem-launcher.github.io).

## Why Tandem?

Most launchers make playing together painful: you sign up for a hosting site, pay, upload your world, configure it. With Tandem you click **"Invite"**, send a short code, and your friend is in your world.

## What works today

- **Play together without a server** — share an `XXXX-XXXX` code and your friends join your world through a direct peer-to-peer connection (public relay as a fallback), with no Tandem server involved. Guests get a copy of the host's instance with the exact same mods, and the game launches straight into the world. Live ping, arrivals, kick, and a health card for the host. Optional *separate world* (beta, Fabric/Quilt): the world runs in a hidden server so friends keep playing if the host's game crashes.
- **Friends** — permanent friend codes (no Tandem account), see who is online, playing or hosting, invite or join in one click, appear offline.
- **Instances** — isolated setups for vanilla, Fabric, Quilt, Forge and NeoForge, the right Java downloaded automatically, several instances running side by side, memory picked from your machine and mod count. Import from the Modrinth App, Prism, the CurseForge app, the official launcher or any instance folder (originals are never modified).
- **Content** — search and install mods, resource packs, shaders and modpacks from Modrinth; `.mrpack` install, import and export; one-click updates; warnings before disabling a library other mods need.
- **AI modpack translation** — translate mods, quests and guide books into your language with a local model (Ollama, LM Studio) or your own API key; authors' translations always win, reversible in one click.
- **Playing** — live console, memory and CPU graphs, crash analysis that names the likely culprit mod.
- **Worlds and screenshots** — automatic backups after each session with undoable restore, a world logbook, screenshot gallery per instance.
- **Accounts and skins** — Microsoft sign-in, several accounts, skin and cape changer with a 3D preview and the original *Tandem Pack* skins.
- **Fast** — starts in well under a second, parallel resumable downloads, stays smooth with hundreds of mods. *Tandem Optimizations* (Fabric/Quilt 1.20–1.21) trims useless world work, added at launch without touching the instance.
- **Self-updating** — signed updates checked at startup, installed in one click.

## Planned

- Signed installers (Windows SmartScreen, Apple notarization).
- Tandem Optimizations and the separate world on Forge and NeoForge.
- Keep the world going when the host leaves.

## Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org).

- Committers and reviewers: [members of tandem-launcher](https://github.com/orgs/tandem-launcher/people)
- Approvers: [owners of tandem-launcher](https://github.com/orgs/tandem-launcher/people?query=role%3Aowner)

Installers are built from this repository by GitHub Actions ([`release.yml`](.github/workflows/release.yml)); each release is approved by hand before signing.

Privacy policy: [tandem-launcher.github.io/confidentialite.html](https://tandem-launcher.github.io/confidentialite.html) (in French). In short: no Tandem server, no telemetry; Tandem only talks to the services the features need (Microsoft/Xbox/Mojang, Modrinth, loader servers, GitHub for updates, the public iroh relays for playing together and friends, and the AI service you pick, if any).

## Privacy & security

- Sign-in uses the official Microsoft OAuth flow in your own browser.
- Tokens are stored only on your machine, in the OS credential store.
- Tandem has no account servers: your credentials never leave your computer except to talk to Microsoft, Xbox and Minecraft services.
- Game files are downloaded from Mojang's official servers.

## Tech stack

- [Tauri 2](https://tauri.app) + Rust backend (`crates/tandem-core` holds all launcher logic)
- SolidJS + Tailwind CSS frontend

## Development

Requirements: Rust (stable), Node.js 22+, pnpm.

```bash
pnpm install
pnpm tauri dev
```

Roadmap and design notes live in [`docs/`](docs/) (in French).

## Credits

Developed by [Zeleff](https://github.com/RandomZeleff), with the help of [Claude](https://claude.com) (Anthropic) as an AI pair programmer.

## License

[MIT](LICENSE)

## Disclaimer

Tandem is not an official Minecraft product. It is not approved by or associated with Mojang or Microsoft. You need a legitimate copy of Minecraft: Java Edition to play.
