# Liba Bilancia v1.4.3 redesign — implementation and test notes

## Implemented

- Rebranded the interface, HTML titles, tray labels, and app icon as Liba Bilancia.
- Set the Tauri Windows executable name to `liba-bilancia.exe` while preserving the existing app identifier and deep-link scheme.
- Added a generated 512px Liba Bilancia icon for the frontend and Windows bundle.
- Added Light, Dark, and System appearance options. System follows the OS `prefers-color-scheme` value. Reward themes control accent and typography only; appearance controls app surfaces and text.
- Added a separate updater download phase with an always-visible determinate or indeterminate progress bar, byte feedback, retryable download/install errors, and a separate Install & Restart action.
- Changed Focus Protection discovery to enumerate visible top-level windows and only list eligible user applications currently open. The list uses friendly names and can be refreshed.
- Kept the saved Focus blocklist even when an app is not open at refresh time. That means an app selected earlier can still be blocked if it is launched during the Focus session.
- Applied a case-insensitive Windows system-process denylist in the native tracker/protection path so shell processes such as LockApp and ShellHost are not tracked or offered as block targets.
- Cleaned up Top Apps/Insights to show one friendly app name, category, and duration rather than repeating the executable filename.
- Removed the monitor-status block from the sidebar and softened the main overview's visual density.
- Updated release and updater documentation for the 1.4.3 workflow.

## Theme design decision

Appearance is an accessibility/display preference (Light, Dark, or System). Reward themes are cosmetic accents (accent colours and typography). They no longer decide the background or panel brightness. This prevents equipping a reward from overriding a user's chosen appearance.

## Compatibility choices

- `identifier` remains `com.digitalbalance.desktop`.
- Deep-link scheme remains `digitalbalance`.
- Updater endpoint and public signing key remain unchanged.
- Cargo/package internal name remains `digital-balance`.
- `mainBinaryName` is now `liba-bilancia`, producing `liba-bilancia.exe` on Windows. Test the first in-place upgrade from 1.4.2 before distributing widely.

## Validation performed in the editing environment

- Frontend TypeScript/TSX syntax transpilation: passed for all source files.
- JSON parsing: passed for package, lockfile, Tauri config, and capability config files.
- CSS brace-balance check: passed.
- Version consistency check: `package.json`, both package-lock root versions, `Cargo.toml`, and `tauri.conf.json` all report `1.4.3`.
- Confirmed `get_running_apps` is registered as a Tauri command and the updater progress component is present.

## Still required on Windows

The editing environment could not complete `npm ci` (dependency download stalled) and does not have the Windows Rust/Tauri toolchain. Therefore a full TypeScript/Vite production build and Windows installer build have **not** been verified here. Before tagging the release, run these commands from the project folder on the Windows development PC:

```powershell
npm.cmd ci
npm.cmd run build
npm.cmd run tauri:build
```

Then test the installer on a separate PC that has 1.4.2 installed. Verify that it updates in place, opens as Liba Bilancia with the new logo, and that the old `digital-balance.exe` is no longer left behind. Also verify Focus Protection with several ordinary apps open and confirm that only visible user applications are listed. Test Light, Dark, and System against Windows appearance changes.
