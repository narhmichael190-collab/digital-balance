# Digital Balance automatic updates

This release adds the Tauri updater. Existing 1.3.6 installs cannot self-update because the updater is not present in 1.3.6. Users on 1.3.6 therefore need one manual install of 1.3.7. After that, future signed releases can be installed from inside the app.

## 1. Create a Tauri signing key (ON YOUR PC ONLY)

```powershell
npm.cmd run tauri signer generate -- -w ~/.tauri/digital-balance.key
```

Keep the private key file and its password private. Tauri requires signed updater artifacts and says losing the private key prevents publishing future updates to already-installed users.

The command generates a public key. Copy the **public** key into `src-tauri/tauri.conf.json` replacing `REPLACE_WITH_TAURI_PUBLIC_KEY`. Never commit the private key.

## 2. Set the GitHub endpoint

Replace `YOUR_GITHUB_USERNAME/YOUR_REPOSITORY` in `src-tauri/tauri.conf.json` with the GitHub repository that contains this project.

## 3. GitHub repository secrets

Add these repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY` = the private key content/path you use for signing
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = the signing password

Do not put the private key in `.env`, source code, or chat.

## 4. Make a release

After committing and pushing the updater-enabled project, bump the app version for each release (`1.3.7`, `1.3.8`, `1.3.9`, ...), then push a tag such as:

```powershell
git tag v1.3.7
git push origin v1.3.7
```

The included GitHub Action builds Windows installers, signs updater artifacts, uploads a `latest.json`, and publishes the GitHub Release.

## 5. How users receive updates

The app checks for updates at startup. When a newer signed release exists, users see **Update now**. They do not need a new installer for later versions. On Windows, the updater installation exits the current app as part of the install process.

## 6. Important

Do not change the Tauri updater signing key after the first updater-enabled release. Keep it backed up securely.
