# Digital Balance 1.3.5 release build

- Run `build-installer.bat` on the Windows development PC.
- The script deletes stale release outputs before building.
- It runs the frontend production build first.
- It builds both NSIS and MSI packages when available.
- It copies fresh installers into `release-artifacts`.
- It generates `SHA256SUMS.txt` so the file can be verified after transferring it to another PC.
- The installer uses an embedded WebView2 bootstrapper to reduce first-run dependency surprises.
