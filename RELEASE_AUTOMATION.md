# Controlled release automation

Production activation is **disabled**. Nothing in this version publishes on an ordinary commit, tag, push or pull request. The implementation has offline policy and publication-fixture tests; the dedicated signing VM, credential broker, hosted staging and unattended production operation remain unconfigured and unauthorized. Publishing these workflow files makes only the explicit unsigned candidate workflow available; it does not activate signing or publishing. The v0.1.9 manual release has separate owner authorization.

## Trust boundaries

1. An operator reviews the source, version, release notes and full main commit, then authorizes its normal fast-forward source push separately.
2. The protected coordinator on a dedicated Windows signing VM receives an exact `vVERSION@FULL_COMMIT` approval. It verifies repository numeric identity, current main, increasing stable version and absence of the version/tag before dispatching the candidate workflow with a unique request ID.
3. A fresh GitHub-hosted Windows job installs frozen dependencies, audits source and Windows dependencies, runs backend/UI/native updater/policy tests, and builds an **unsigned** production NSIS installer. It has contents-read permission and no production signing secret or release-write permission.
4. A second fresh hosted job downloads the immutable candidate as data and attests the installer and candidate metadata. Only this job has OIDC and attestations-write permission; it does not check out or execute project code. Official Actions and the attestation action's inner action are pinned to reviewed commit hashes.
5. The coordinator waits for the entire dispatched run to succeed. It rejects reruns, mismatched heads, ambiguous runs, expired/extra artifacts, changed keys, failed required checks, invalid versions and unexpected files. Patched, pinned `gh attestation verify` must validate the exact certificate SAN, source and signer digests, main ref and hosted runner identity for both files.
6. The candidate executable is never run on the signer. A separately installed, pinned official native Tauri CLI signs it through a protected bridge. No npm install, Cargo build, downloaded code or checkout script runs while signing credentials are present. The bridge checks the dedicated host/SID, protected owners/ACLs, reparse paths and original public identity, and passes the decrypted DPAPI password only in the signer process environment. GitHub credentials are removed from that child environment. Signing output is suppressed.
7. The coordinator creates a 30-day update manifest, signs it, invokes the pinned native public-key verifier, generates checksums and repeats remote/source/expiry checks. It creates a draft, uploads the exact six assets, validates the draft, then publishes it as stable/latest.
8. Every public asset is downloaded and compared byte-for-byte by SHA-256. The native verifier checks public installer/manifest signatures again. The public latest feed must exactly match the new signed manifest. Success is recorded only after these checks.

The six files are `Zoxryn-Mods-VERSION-Setup.exe`, its `.sig`, `latest.json`, `latest.json.sig`, `SHA256SUMS.txt` and `RELEASE_NOTES.md`. Candidate metadata, QA programs, source archives, signing keys, policies, credentials and logs are not release assets. No existing version is overwritten and no force-push is implemented.

## Required one-time setup and approval

Review and explicitly authorize this configuration before enabling it:

- A dedicated, isolated Windows VM and dedicated local service identity, separate from the everyday/gaming PC. It is a release coordinator, **not a self-hosted GitHub Actions runner**. Do not grant untrusted users interactive or remote job access. Protect the VM, administrator accounts, disk, runtime/tool directories, vault and recovery backups.
- Deploy an audited copy of `coordinator.py` and `sign-approved.ps1` to a protected control directory outside any checkout/download directory. Install a pinned Python runtime and use `python -I`; protect its DLLs and standard library too. Install reviewed native Tauri signer, native `verify_release_signatures` built with the established public key, and patched GitHub CLI. Record each executable/script SHA-256 in the external policy. No tool may be taken from a candidate checkout at signing time.
- Make policy/control/vault/tool files owned by the dedicated identity, SYSTEM or trusted Administrators; disable inherited access on signing paths and deny other principals write/delete/permission-change/ownership access. The bridge rejects unexpected writable principals, owners and reparse paths. Review parent volume/directory ACLs and the complete installed runtime/tool trees during provisioning.
- Securely provision the **existing** encrypted updater private key and its original password into that VM identity's vault, then protect the password with that VM identity's CurrentUser DPAPI. Copying the current PC's DPAPI blob alone does not work. Any transfer/export/re-protection requires explicit owner authorization; no transfer, decryption export, key rotation or replacement identity has been performed here. Never put this key/password in GitHub secrets or workflow artifacts.
- Create a GitHub App restricted to `zoxryn/zoxryn-mods`, with repository Contents read/write, Actions read/write, metadata read and artifact-attestation read as required. Do not grant repository administration, secrets, organization-wide or unrelated repository permissions. The App's private credential belongs in the VM's protected credential broker; provide a fresh short-lived installation token as `GH_TOKEN` only to the coordinator. Refresh/broker integration and token lifetime must be tested on that host before unattended operation is claimed.
- Review branch protection/rules, write/dispatch access, fork permissions, GitHub-hosted runner availability and attestation support. Push the reviewed source/workflow only after separate source/publication authorization. Run the candidate workflow there without production signing first and validate actual certificate identity/output against the coordinator's exact policy.
- Fill in the **external** production policy with the approved host, SID, absolute protected paths, tool hashes, original public key and verified repository ID. The checked-in example has `enabled: false` and unprovisioned host/tool fields. Do not activate by merely changing this repository example.
- Perform an authorized end-to-end staging exercise on the dedicated host, including wrong source/workflow/key, tampered candidate, concurrent trigger, network/token expiry, failed upload and public verification failures. Review credential exposure and host logs. Only after this passes should the owner authorize the external policy's `enabled: true` and production use.

The dedicated signing host, vault provisioning, token broker and production activation above remain unconfigured. The candidate workflow alone cannot publish a release.

## Future approved trigger

After one-time setup, a reviewed, authorized source commit must already be on main. On the dedicated host, invoke the protected pinned Python runtime in isolated mode:

```text
python -I coordinator.py --policy PROTECTED_POLICY --version VERSION --commit FULL_40_CHARACTER_SHA --approve-release vVERSION@FULL_40_CHARACTER_SHA
```

The same trigger runs tests, builds, signs, uploads, publishes and verifies without separate manual asset-upload/publish steps. Approval is tied to that exact version/commit; unfinished development commits do not trigger it.

Failures stop the flow and retain the release work directory plus a `FAILED` marker. A failed upload leaves a draft for inspection. A failure after publication can leave a public release whose verification has not completed; inspect it before any recovery action. No automatic deletion, overwriting or rollback occurs. Existing versions/drafts, a stale mutex after host failure, token expiry or changed main require deliberate operator recovery and a new approved request. The controller removes only its own empty mutex after closing it.

## Remaining limits

Offline tests cover policy gates, assets, checksums, expiration, exact attestation arguments, stable draft/publication order, collision locking, failed upload and synthetic public downloads. They do not replace real GitHub attestation validation, protected-host provisioning, credential-broker testing or a live production publication test. The implementation therefore remains **prepared, not operational** for unattended publishing.

The existing updater's signing identity is preserved. Tauri updater signatures are not Windows Authenticode certificates. Metadata expires; an approved refresh/new release is needed before expiry. Hosted build provenance identifies source/workflow, but does not prove dependencies are defect-free or provide reproducible-build equality.

Primary references: [GitHub secure workflow use](https://docs.github.com/en/actions/reference/security/secure-use), [GitHub attestation verification](https://cli.github.com/manual/gh_attestation_verify), [exact-workflow verification advisory](https://github.com/cli/cli/security/advisories/GHSA-wjmr-j3rp-mh2g), [Tauri signer](https://v2.tauri.app/reference/cli/#signer-sign).
