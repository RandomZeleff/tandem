# Tandem

**A fast, modern Minecraft: Java Edition launcher built for singleplayer — and for playing with a friend without renting a server.**

> 🚧 Early development: v0.1 is a first preview (see [Releases](https://github.com/RandomZeleff/tandem/releases)). The French UI comes first; co-op without a server is the next big step.

## Why Tandem?

Most launchers make playing together painful: you sign up for a hosting site, pay, upload your world, configure it. Tandem wants it to be as simple as clicking **"Invite a friend"**.

## What works today

- **Instances** — isolated setups for vanilla, Fabric, Quilt, Forge and NeoForge, the right Java downloaded automatically, several instances running side by side, memory picked from your machine and mod count.
- **Content** — search and install mods, resource packs, shaders and modpacks from Modrinth; `.mrpack` install, import and export; one-click updates; warnings before disabling a library other mods need.
- **Playing** — live console, memory and CPU graphs, crash analysis that names the likely culprit mod.
- **Worlds and screenshots** — automatic backups after each session with undoable restore, screenshot gallery per instance.
- **Fast** — starts in well under a second, parallel resumable downloads, stays smooth with hundreds of mods.

## Planned features

- **Co-op without a server** — open your world to LAN, share a short invite code, and your friend joins through a direct peer-to-peer connection (relay fallback). No mods, works on any version or loader.
- **Instances** — isolated game setups, fast version switching, multiple instances running side by side.
- **CurseForge import** for modpacks.
- **AI modpack translation** — translate mods, quests and guide books into your language as a toggleable resource pack.
- **Smooth accounts** — sign in with your Microsoft account, switch accounts in one click.

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
