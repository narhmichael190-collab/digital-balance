# Liba Bilancia v1.4.3

## Improvements
- Updated the brand mark and Windows application icon for Liba Bilancia.
- Added Light, Dark, and System appearance preferences. Reward themes now control accents and typography independently of the chosen appearance.
- Added an update download panel with a visible determinate or indeterminate progress bar and a separate Install & Restart step.
- Focus Protection now lists only currently open, visible user applications and shows friendly app names instead of executable filenames.
- Excluded Windows shell/system processes from tracked app sessions and app-protection targets.
- Removed the redundant executable-name column from Insights Top Apps and improved list spacing.
- Removed the sidebar monitor-status block.

## Compatibility
- Kept the existing application identifier, app-data filename, and `digitalbalance` deep-link scheme so existing local data and sign-in callbacks remain compatible.

## Validation status

Frontend TSX syntax and CSS structural checks were completed in the editing environment. A full dependency-backed TypeScript/Vite build and Windows Rust/Tauri installer build still need to be run on the Windows development PC before this release is tagged.
