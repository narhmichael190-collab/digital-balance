# Liba Bilancia v1.4.4

## Visual polish + Windows branding migration

- Added a compact online/offline status control with clear state styling.
- Separated app names and categories in Overview → Usage so the category reads as secondary metadata.
- Refined dashboard spacing, panel density, score/streak presentation, and hover feedback for a calmer productivity-game feel.
- Redesigned the collapsed sidebar header so the logo and menu control form a consistent vertical stack.
- Added a Windows NSIS installer migration hook that removes legacy **Digital Balance** Start Menu/Desktop shortcuts before and after installation.
- Kept the stable application identifier and `digitalbalance` deep-link scheme so updater/auth compatibility remains intact.
- Kept the Liba Bilancia icon as the Windows bundle icon.
- Bumped application/package version to **1.4.4**.

### Update test target
Install 1.4.3 first, then publish 1.4.4 through the existing GitHub release workflow. The updater should discover 1.4.4, show the download progress UI, install, restart, and leave a **Liba Bilancia** Start Menu entry using the current icon.
