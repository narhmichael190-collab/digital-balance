# Digital Balance — recovered + V1.1 feature build

This is a rebuilt source project for the Windows Digital Balance app.

## What is included

- Windows foreground-app tracking with local history
- Casual / Smart / Deep Focus modes
- Global leaderboard via Supabase + Google OAuth
- Secure server-side leaderboard points
- Balance Coins for local cosmetic purchases
- Reward Lab with multiple themes, avatar frames, and Focus effects
- App Control Center with daily limits and three actions:
  - Warn
  - Close once when the cap is reached
  - Block until the next day
- Deep Focus process blocklist
- Command palette (`Ctrl+K`)
- Start-with-Windows support
- Local-first privacy model

## Important browser limitation

Digital Balance works at the Windows application/process level. It can limit `chrome.exe`, `msedge.exe`, Discord, Spotify, etc. It cannot know that a specific browser tab is Instagram/TikTok without a separate browser extension.

## Restore setup

1. Copy `.env.example` to `.env`.
2. Put your existing Supabase Project URL and Publishable key into `.env`.
3. In a terminal in this folder run:

```powershell
npm.cmd install
npm.cmd run tauri dev
```

## Production installer

```powershell
npm.cmd run build
npm.cmd run tauri build
```

The Windows installers appear under:

`src-tauri/target/release/bundle/`

This project is configured to produce both NSIS and MSI Windows installers.

## Publishing note

For public distribution, publish the installer from a stable download/release page rather than passing the raw `.exe` through chat apps. The NSIS installer performs an integrity self-check; a damaged or modified download can trigger an “Installer integrity check has failed” message.

For a public product, code-sign the Windows executable/installer as well.
