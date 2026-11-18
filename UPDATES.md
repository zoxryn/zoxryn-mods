# Application updates

Zoxryn Mods 0.1.7 extends the official Tauri 2 updater plugin (Rust 2.14.1) introduced in 0.1.6. The application identifier remains `com.zoxryn.mods`; the executable, current-user NSIS installation model and existing data directory remain unchanged.

The configured production channel is `https://github.com/zoxryn/zoxryn-mods/releases/latest/download/latest.json`, with a required detached `latest.json.sig`. Source and release publication require explicit approval. Before release assets are published, the app reports **No release published**. Network failures do not prevent opening or using the app. The existing installed application is not replaced by preparing this build.

## User flow

The installed app checks once at startup and on **Settings → Check for Updates**. Checks fetch only small release metadata. A verified offer shows the installed/available version, plain-text notes and size when supplied. **Later** dismisses it without downloading. **Update & Restart** explicitly approves that offer, downloads it with progress, verifies the official Tauri artifact signature and signed version, runs the NSIS installer at the registered current installation location, then restarts the app. There is no background service, polling timer or silent package download.

The backend rejects installation while a scan or queued/running manager operation holds a transaction lease. It also rejects unfinished journals, unresolved recovery, a corrupt database, overlapping installation/data/game directories, or executables outside the registered current-user installation. Portable/build copies require the recovery installer before in-place updates are available.

Before download, SQLite is checkpointed and a consistent database copy is saved under `app-update-backups/`. Existing archives, blobs, mod backups, external detections, ownership and settings stay in the same data directory. Updater receipts record downloading/installing/completed/failed/incomplete states. An app update does not call game detection, mod installation, restore or enable/disable routines. It never targets GTA files.

Network and verification failures keep the existing application and data. Retry from Settings. For an interrupted installer, use the matching recovery installer at the same application location and retain app data. A database checkpoint is retained for deliberate manual recovery; the app never automatically rolls the database backward or deletes user data. Arbitrary power-loss/locked-file repair is not guaranteed.

## Signing custody

The release private key is encrypted outside the project at `%LOCALAPPDATA%\ZoxrynModsSigning\updater-release.key`. A random password is protected separately using Windows CurrentUser DPAPI, with a user-only ACL on the vault. Neither private key nor password is included in source, installers, manifests, reports or logs. Only `src-tauri/updater-public-key.pub` is distributed.

Keep a secure recovery backup of the original signing identity. DPAPI recovery is tied to the Windows identity/machine; copying the encrypted key alone is insufficient if its password is lost. Never regenerate the key for an established distribution. The initialization script refuses that implicit identity change. An intentional key rotation and cross-account CI secret provisioning remain release-operator tasks.

`with-signing.ps1` supplies signing credentials only to the child Tauri CLI and clears the environment afterward. `build.ps1` uses this wrapper. Tauri's updater signatures are separate from Windows Authenticode signing; no Authenticode certificate was supplied, so first-run Windows trust warnings remain possible.

## Prepare a real GitHub Releases channel

The public repository is `zoxryn/zoxryn-mods`. Private repository authentication is not implemented. Do not put a GitHub token in the desktop application. Authenticated push permission must be established by the release operator before the separately approved first push.

1. Keep the original signing vault. For a later release, raise the stable version consistently in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` and write UTF-8 release notes. The interface reads its version from package metadata.
2. Run `./scripts/prepare-github-release.ps1 -NotesFile ./release-notes/0.1.7.md` using PowerShell 7. This validates channel/identity/version, audits public source, runs backend/desktop/frontend tests, builds and signs NSIS, packages public source/portable files, signs the manifest and verifies artifact signatures, authenticated versions, checksums and tampering rejection. It creates local release assets only.
3. `-VerifyExistingBuild` can repackage an already tested production build only when its retained provenance matches current source inputs and installer/executable hashes. Routine future releases should use the full workflow.
4. Review the public source audit and staged changes. After explicit source-publishing approval, commit with a public-safe Git author identity and push without force. An empty remote has no history to merge; if remote commits appear later, fetch and reconcile them before proposing a push.
5. Only after explicit release approval, create the matching stable `vVERSION` release and upload `Zoxryn-Mods-VERSION-Setup.exe`, its `.sig`, `latest.json`, and `latest.json.sig` together, plus the prepared checksums and notes. Do not mark a test/prerelease as the stable latest channel. Do not alter signed JSON bytes after signing.
6. Download the public manifest, detached signature and installer independently and verify them before approving bootstrap installation. Only then install 0.1.7 manually once over the registered current-user installation. Later approved stable releases can update through Settings.

The feed is the fixed HTTPS URL `https://github.com/zoxryn/zoxryn-mods/releases/latest/download/latest.json`. It uses Tauri's supported static manifest with `platforms.windows-x86_64-nsis`, the raw NSIS `.exe`, and its official `.sig`. Size and SHA-256 are optional additional manifest fields. The entire JSON additionally has a detached `latest.json.sig`, so version, notes, URLs and size are authenticated before display/selection. Official `restore_update` consumes those verified bytes, avoiding a second unsigned metadata fetch, and enforces a version-bound artifact signature.

Production URL policy pins the feed and installer to the configured GitHub repository, requires HTTPS, rejects downgrades/prereleases, caps metadata at 64 KiB and packages at 256 MiB, and restricts redirects to the same repository or approved GitHub asset CDN hosts. Signed metadata carries RFC 3339 publication and expiry times; the release tool defaults to 30 days, with a 90-day maximum. Expired/future-dated or malformed metadata is rejected, including metadata for the installed version. Offers also expire after 30 minutes. Recheck time is retained separately from SQLite. No arbitrary URL or install path is accepted from the UI. Raw plugin updater commands are not exposed to frontend capabilities.

The local release directory contains a real signed manifest and installer awaiting publication. Older placeholder files under release-templates are examples only. Native QA uses a loopback-only feed with a distinct test product/executable. Its HTTP exception and test orchestration are compiled only with the explicit `updater-qa` feature, excluded from production builds. GitHub Actions signing is not configured: Windows DPAPI custody is not portable to hosted CI, and no CI secret transfer was authorized.

## Verification

Run `scripts/build.ps1 -TestsOnly` for the Rust tests and `pnpm test:ui` for headless interface tests. The native test setup is `scripts/prepare-updater-qa.ps1`: it refuses existing roots and authors only a marker-protected synthetic workspace. QA builds must set `ZOXRYN_UPDATER_QA_ROOT` to that root and use the generated version-specific config with the `updater-qa` feature. They use `Zoxryn Mods Updater QA` registration, `zoxryn-mods-updater-qa.exe`, an isolated WebView profile, and a protected uninstaller hook. Never substitute a production installer or executable for the QA artifacts.

`scripts/verify-updater-qa.py capture ROOT` stores a baseline; `verify ROOT` checks the real updater restart, completion receipt, consistent SQLite backup, all logical tables and retained file hashes. Run `scripts/test-native-updater.ps1` for a fresh isolated signed-upgrade test. It retains its local verification report and failure evidence outside public source.

Official references: [Tauri updater](https://v2.tauri.app/plugin/updater/), [verified metadata restoration](https://docs.rs/tauri-plugin-updater/2.14.1/tauri_plugin_updater/struct.Updater.html#method.restore_update), [artifact verification and installation](https://docs.rs/tauri-plugin-updater/2.14.1/tauri_plugin_updater/struct.Update.html).
