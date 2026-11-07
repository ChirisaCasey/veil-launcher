# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

Greenfield. The only source of truth so far is [instructions.md](instructions.md) (the owner's spec). No stack, build system, or code exists yet. Once one is chosen, add the build/run/test/installer commands here.

The UI framework is undecided: the owner suggested Dear ImGui or Tauri and asked to be consulted (via AskUserQuestion) on this and other significant design decisions as the project progresses. App name and icon are also still to be chosen (spec asks for a one-word sleek name).

## What the app does

A Windows system-tray utility that swaps VALORANT's main-menu background video for a user-chosen `.mp4`.

- Target directory: `<install>\VALORANT\live\ShooterGame\Content\Movies\Menu`
- Target file: an `.mp4` whose name contains `Homescreen`, often with a prefix/postfix (e.g. `13_06_Homescreen.mp4`). The prefix/postfix, or even the whole filename, can change between game patches, and several versions with different prefixes may coexist. Detection must be heuristic ("smart"), with a user override to select a specific file.
- Swap = back up the original, then write the custom video under the exact same filename.

## Core timing constraint (drives the architecture)

Riot's client verifies game files at launch and will re-download a modified video. So:

1. Custom video must be swapped in **after the Riot Client closes** (post-verification), while VALORANT is starting/running.
2. The **original must be restored after VALORANT exits**, so a normal launch doesn't trigger a repair/update.

This means the app needs process monitoring of the Riot Client and VALORANT lifecycle, not just a one-shot file copy. Backup integrity is critical: never lose or overwrite the original backup.

## Hard rules

- **No injection, hooking, or overlay into the game process.** File swapping only; this must stay a plain utility.
- Request admin elevation only when needed (e.g. writing into the game directory), not for the whole app by default.

## Required features (from spec)

- Tray-first app with menu options (launch game, change video, settings, etc.) plus a full modern, animated desktop UI designed with UX in mind.
- Theme accent color picker; light and dark themes.
- Opt-in: run in background so the custom video applies even when the game is launched outside the app.
- Opt-in: start with Windows.
- An installer.
- Published on GitHub with releases and a polished, informative README.md. The README should link to https://github.com/ChirisaCasey/Val-Patcher as a lightweight `.bat` alternative. Optionally (only after the main app is built), that repo may be cloned and improved.
