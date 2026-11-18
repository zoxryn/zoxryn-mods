# Update manifest example

`latest.template.json` documents the required fields and the configured GitHub repository. Its generated-value markers are deliberately invalid. It is unsigned and must never be uploaded as `latest.json`.

The production channel is configured for `zoxryn/zoxryn-mods`. Configuration does not publish a release or replace an installed application.

Use PowerShell 7 and `scripts/prepare-github-release.ps1 -NotesFile ./release-notes/0.1.7.md`. This runs checks, builds with the original signing identity, and prepares a real `latest.json`, detached manifest signature, signed installer, checksums and notes locally. Publishing requires separate approval. The installer, installer signature, manifest and detached manifest signature must be uploaded together into the corresponding stable `vVERSION` release. Metadata expires after 30 days by default; renew and re-sign metadata before expiry even when no binary update is needed.

Use `scripts/test-native-updater.ps1` for a fresh marker-protected QA installation and loopback feed. Its transport exception and automatic test approval exist only in the separate QA build. Test binaries must never be distributed as production releases.
