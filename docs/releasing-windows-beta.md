# Releasing a Windows beta

How a beta reaches testers on other Windows PCs, and how those PCs find the next one. The pipeline is `.github/workflows/beta-windows.yml`; the macOS and Linux releases are separate and are not covered here.

## What a beta is

- A tag such as `v0.7.0-beta.1` builds one **NSIS installer** (`*-setup.exe`) on a GitHub runner and publishes it as a **pre-release**. No MSI: a pre-release version cannot be packaged as an MSI.
- The installer is **not code-signed** unless an Authenticode certificate is configured, so Windows SmartScreen warns on first run. Testers choose **More info**, then **Run anyway**. It installs for the current user without administrator rights.
- The in-app update check reads `latest.json` from a rolling release tagged `beta-manifest`. GitHub's own `releases/latest` URL ignores pre-releases, which is why the manifest has a fixed home.

## One-time setup

1. **Generate the fork's updater key pair**, outside the repository:

   ```powershell
   npx tauri signer generate -w "$env:USERPROFILE\.tauri\fork.key"
   ```

   Choose a passphrase and back the file up. If the private key is lost, installed copies can no longer update and testers must reinstall. **Never commit the private key**; the matching `.pub` file is the public key.

2. **Store the private key and its passphrase as repository secrets** (Settings > Secrets and variables > Actions, or with the GitHub CLI):

   ```powershell
   Get-Content "$env:USERPROFILE\.tauri\fork.key" -Raw | gh secret set TAURI_SIGNING_PRIVATE_KEY --repo <owner>/mynota
   gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo <owner>/mynota
   ```

3. **Point the app at the fork**, in one commit, in `app/src-tauri/tauri.conf.json`:
   - `plugins.updater.pubkey`: the content of the `.pub` file;
   - `plugins.updater.endpoints`: `["https://github.com/<owner>/mynota/releases/download/beta-manifest/latest.json"]`.

   Doing only one of the two breaks update checks: the signature would not match, or there would be no manifest.

4. Optionally add an Authenticode certificate (`WINDOWS_CERT_PATH` and `WINDOWS_CERT_PASSWORD`, or `WINDOWS_CERT_THUMBPRINT`) to remove the SmartScreen warning.

## Releasing a beta

1. Make sure the verification gate is green (see [windows-recipe.md](windows-recipe.md) and the hand-over notes).
2. Set the version in both places, which must agree (the manifest script fails on a mismatch with the tag): `app/src-tauri/tauri.conf.json` (`version`) and the workspace `Cargo.toml` (`version`). Example: `0.7.0-beta.1`.
3. Validate a **packaged** build locally first. The development build hides real problems (the packaged app resolves model and template paths differently and has a stricter content security policy):

   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\fork.key" -Raw
   .\scripts\release-windows.ps1 -Bundles nsis
   ```

   Install the produced `target\release\bundle\nsis\*-setup.exe` on a clean user profile and run the recipe checks that apply.
4. Commit, then tag and push:

   ```powershell
   git tag v0.7.0-beta.1
   git push origin v0.7.0-beta.1
   ```

5. The workflow builds, publishes the pre-release and refreshes `beta-manifest`. Check the run, then the release page.

## What testers do

1. Download `*-setup.exe` from the pre-release and install it (SmartScreen: More info, Run anyway).
2. On first launch, download the models (about 0.7 GB, once), then pick an export folder in **Settings > Markdown export**.
3. Run the checks in [windows-recipe.md](windows-recipe.md) that fit their setup (audio devices, headset type, a real call) and send back the **report template** at its end. No hardware identifiers are required: the device category (wired, USB, Bluetooth) and the Windows version are enough.
4. Later betas arrive through **Settings > Updates**, once the update check is enabled; the user clicks **Update**.

## Known limits

- Unsigned installer unless a certificate is configured.
- One architecture (`windows-x86_64`).
- The `v*` stable tags still trigger the upstream `release.yml` (macOS plus Windows). A stable channel for the fork needs its own decision.
- The product is still named "Myna" with the upstream identifier, so it replaces an upstream install on the same PC. Decide a distinct name and identifier before inviting people who also use upstream.
