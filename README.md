# Tandem

**A fast, modern Minecraft: Java Edition launcher built for singleplayer — and for playing with a friend without renting a server.**

> 🚧 Early development. Nothing is ready to play yet.

## Why Tandem?

Most launchers make playing together painful: you sign up for a hosting site, pay, upload your world, configure it. Tandem wants it to be as simple as clicking **"Invite a friend"**.

## Planned features

- **Co-op without a server** — open your world to LAN, share a short invite code, and your friend joins through a direct peer-to-peer connection (relay fallback). No mods, works on any version or loader.
- **Instances** — isolated game setups, fast version switching, multiple instances running side by side.
- **Content** — search and install modpacks, mods, shaders and resource packs from Modrinth (CurseForge import too).
- **Performance** — shared deduplicated file store, parallel downloads, automatic Java management, sensible JVM defaults.
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

## Disclaimer

Tandem is not an official Minecraft product. It is not approved by or associated with Mojang or Microsoft. You need a legitimate copy of Minecraft: Java Edition to play.
