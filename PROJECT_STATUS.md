# Liba Bilancia — Project status

Current source version: **1.4.4** (visual polish + Windows branding migration).

## Product identity
- Display name: Liba Bilancia
- Windows executable: `liba-bilancia.exe`
- Stable app identifier: `com.digitalbalance.desktop`
- Stable deep-link scheme: `digitalbalance`
- Cargo package remains `digital-balance` internally.

## Current feature areas
- Local Windows foreground-app tracking and daily usage totals
- App limits and Focus Protection (Casual, Smart, Deep Focus)
- Focus sessions, local rewards, coins, themes, frames, and effects
- Supabase sign-in, profile, activity rewards, and leaderboard
- Signed Tauri updater with GitHub Releases metadata
- Appearance preference: Light, Dark, or System

## Compatibility notes
The bundle identifier, deep-link scheme, updater endpoint, and signing public key are deliberately unchanged. Keep the matching updater private signing key and password only in GitHub repository secrets; never put them in this repository or share them in chat.

## Build on Windows
```powershell
npm.cmd ci
npm.cmd run build
npm.cmd run tauri:build
```

Installers are created under `src-tauri\target\release\bundle\nsis` and `src-tauri\target\release\bundle\msi`.
