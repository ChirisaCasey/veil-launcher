# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Veil Launcher**: a Windows tray-first Tauri 2 app (Rust backend + SvelteKit/Svelte 5 SPA) that swaps VALORANT's main-menu background video (`<live>\ShooterGame\Content\Movies\Menu\*Homescreen*.mp4`) for a user video **after** Riot's file verification and restores the original **after** the game exits, so normal launches never trigger a repair. The original spec is [instructions.md](instructions.md).

**Hard rule:** file swapping only. Never inject, hook, read game memory, or overlay anything.

## Commands

Rust lives in `%USERPROFILE%\.cargo\bin` (may need adding to `PATH` in a fresh shell).

```sh
npm install
npm run dev                 # UI only in a browser at :1420, backed by src/lib/mock.ts
npm run tauri dev           # full app (Rust + UI, hot reload)
npm run check               # svelte-check (TypeScript + Svelte)
npm run tauri build         # release exe + NSIS installer in src-tauri/target/release/bundle/nsis/

cd src-tauri
cargo test                  # all Rust unit tests
cargo test detect::         # one module's tests (filter by path/name)
cargo clippy --all-targets  # CI runs this with -D warnings
cargo run --example probe   # READ-ONLY: prints detected install + candidate files on this machine
```

`tauri-build` requires the frontend output dir `build/` to exist: run `npm run build` once (or create a placeholder `build/index.html`) before plain `cargo build`/`cargo test`.

Binary modes (see `src-tauri/src/cli.rs`): `--background` (start hidden, used by autostart and the scheduled task), `--minimized`, `--helper <op> <hex-json>` (elevated one-shot file op, runs without Tauri), `--cleanup` (used by the NSIS uninstall hook: restore original, remove startup entries).

## Architecture

- **Contract:** [src/lib/api.ts](src/lib/api.ts) is the single source of truth for commands, types and events. Rust mirrors it in `src-tauri/src/commands.rs` + `engine.rs` with `#[serde(rename_all = "camelCase")]`. Change both sides together, and add any new command to the mock in `src/lib/mock.ts` too, so browser dev keeps working.
- **`engine.rs`** is the central controller (`Engine`, cloned via `Arc`): owns settings, install info, detection, library and swap journal, and pushes changes out through an `EventSink` (Tauri impl in `lib.rs` emits `status-changed`, `settings-changed`, `notify`, `progress`, and updates the tray). Commands are thin wrappers over it.
- **`core/`** contains the pure logic, all unit-tested on temp dirs:
  - `paths`: install discovery (Riot metadata yaml → registry → drive scan → user override).
  - `detect`: scores `*.mp4`, 0–100. Homescreen ≫ version prefix ≫ mtime; cinematics are never auto-selected; the user can pin a file.
  - `swap`: backup, atomic replace, sha256 verify, and the `state.json` journal used for crash recovery.
  - `watcher`: a process state machine over a `ProcessSource` trait (`SysinfoSource` in production, a scripted fake in tests).
  - `elevate`: UAC `runas` helper and the `schtasks` logon task.
  - `ffmpeg`: download, probe, transcode.
  - `library`: the user's video library.
- **Swap lifecycle:** Riot Client UI (`Riot Client.exe`) → game (`VALORANT-Win64-Shipping.exe`) → trigger (`riotClientClosed` default, or `gameStarted`) + delay → swap → game exits → restore. `RiotClientServices.exe` stays alive in the background and is deliberately ignored.
- **Backup invariant:** the backup is refreshed only when the target differs from the custom video (i.e. it is genuine, possibly a new patch). A custom video must never overwrite the backup.
- **Elevation:** writes are attempted unelevated first; access denied → `--helper` via `runas`. The helper only writes `.mp4` files inside a `Movies\Menu` folder. Background/startup mode uses the HKCU Run key when the Menu folder is user-writable, otherwise an `ONLOGON /RL HIGHEST` scheduled task (`startup.rs`).
- **Data dir:** `%APPDATA%\VeilLauncher\` (config.json, state.json, backups\, videos\, ffmpeg\, logs\). Config is plain JSON (not tauri-plugin-store), because helper/cleanup modes run without Tauri.
- **UI:** `src/routes/+page.svelte` shell; views in `src/lib/views/`; rune stores in `src/lib/stores/*.svelte.ts` (wired up in `stores/index.ts` `initApp()`); `src/lib/platform.ts` wraps Tauri APIs with browser fallbacks. The theme is CSS variables in `src/app.css`, with accent tints derived via `color-mix()`. The window is frameless (custom `TitleBar`), and close hides it to the tray.
- **Installer:** NSIS only. MSI is intentionally disabled because only the NSIS hook (`src-tauri/windows/hooks.nsh`) runs `--cleanup` on uninstall. Install mode is perMachine (the elevated scheduled task must point at a non-user-writable exe).

## Safety when developing

Never write to the real VALORANT install from tests or experiments. Use temp dirs, as the existing tests do; engine tests disable real discovery. Don't create real scheduled tasks or Run keys outside the app's own settings flow.

## Release

`.github/workflows/release.yml` builds on `v*` tags via `tauri-action` into a draft GitHub release; `ci.yml` runs check/test/clippy. Brand sources are in `assets/` (`assets/scripts/build-brand.cjs` regenerates icon/logo/banner SVGs; then `npx tauri icon assets/icon-1024.png`).
