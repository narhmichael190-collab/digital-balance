# Liba Bilancia automatic updates

Liba Bilancia uses the Tauri 2 updater. Each release must have matching versions in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Update all three before tagging. The project lockfiles should remain synchronized with the package versions.

## Signing keys

The signing public key is configured in `src-tauri/tauri.conf.json`. The matching private key and its password must remain in GitHub repository secrets named `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Never commit or paste the private key.

## Create a release

1. Run the production frontend build and the Windows Tauri build.
2. Commit the release source, including the version bumps.
3. Create a new version tag (for example `v1.4.3`) on that commit.
4. Push the tag to GitHub. The `Release Liba Bilancia` workflow creates the Windows installers and signed `latest.json`.
5. Verify `latest.json` reports the same version as the installer filenames.

Example for a new release (replace the tag with the version actually committed):

```powershell
git tag v1.4.3
git push origin v1.4.3
```

Do not move or reuse an existing release tag for a different build unless intentionally correcting a broken release. A new version tag is safer.

## In-app experience

When an update is available, Liba Bilancia offers a separate **Download update** step with progress feedback, followed by **Install & restart**. Users can keep using the app while the download is in progress. On Windows, the updater closes the running app while applying the installer.

## Compatibility

Do not change the updater signing key, app identifier, or deep-link scheme after publishing an updater-enabled release. The Windows executable filename has been changed to `liba-bilancia.exe`; Tauri's updater/bundler should handle the binary rename, but this migration must be verified on a test machine upgrading from 1.4.2 before broad distribution.
