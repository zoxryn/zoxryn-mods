//! Update trust and transaction policy; independent of game detection and Tauri.
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use url::Url;
pub const MAX_MANIFEST: usize = 64 * 1024;
pub const MAX_PACKAGE: usize = 256 * 1024 * 1024;
pub const MAX_METADATA_AGE_SECONDS: i64 = 90 * 24 * 60 * 60;

pub fn now_unix() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

pub fn approved_redirect(url: &Url, repository: Option<&str>, qa: bool) -> bool {
    if qa {
        return is_loopback(url);
    }
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(443)
    {
        return false;
    }
    match url.host_str() {
        Some("github.com") => repository.is_some_and(|repo| {
            valid_repository(repo).is_ok() && url.path().starts_with(&format!("/{repo}/releases/"))
        }),
        Some("release-assets.githubusercontent.com" | "objects.githubusercontent.com") => true,
        _ => false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub repository: Option<String>,
    pub feed_url: Option<String>,
    pub recovery_url: Option<String>,
}
impl Channel {
    pub fn validate(&self, qa: bool) -> Result<Option<Url>> {
        let Some(feed) = &self.feed_url else {
            return Ok(None);
        };
        let url = Url::parse(feed)?;
        if qa && is_loopback(&url) {
            return Ok(Some(url));
        }
        let repo = self
            .repository
            .as_deref()
            .context("GitHub release repository is missing")?;
        valid_repository(repo)?;
        let expected = format!("https://github.com/{repo}/releases/latest/download/latest.json");
        if feed != &expected {
            bail!("Update feed must be the configured GitHub Releases latest.json URL");
        }
        Ok(Some(url))
    }
}
pub fn valid_repository(repo: &str) -> Result<()> {
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 100
                || p.starts_with('.')
                || !p
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        })
        || ["owner", "repo", "your-owner", "your-repo"]
            .iter()
            .any(|v| parts.iter().any(|p| p.eq_ignore_ascii_case(v)))
    {
        bail!("Set a real GitHub OWNER/REPOSITORY before enabling the update feed");
    }
    Ok(())
}
pub fn is_loopback(url: &Url) -> bool {
    url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.username().is_empty()
        && url.password().is_none()
}
pub fn verify_signed_manifest(
    bytes: &[u8],
    signature: &str,
    public_key: &str,
) -> Result<serde_json::Value> {
    if bytes.len() > MAX_MANIFEST || signature.len() > 8192 {
        bail!("Update metadata exceeds its size limit");
    }
    let key = String::from_utf8(
        STANDARD
            .decode(public_key.trim())
            .context("Invalid update public key encoding")?,
    )?;
    let sig = String::from_utf8(
        STANDARD
            .decode(signature.trim())
            .context("Invalid manifest signature encoding")?,
    )?;
    let signature = Signature::decode(&sig)?;
    PublicKey::decode(&key)?
        .verify(bytes, &signature, false)
        .context("Update manifest signature verification failed")?;
    let raw: serde_json::Value =
        serde_json::from_slice(bytes).context("Signed update manifest is invalid JSON")?;
    let version = raw["version"]
        .as_str()
        .context("Signed manifest version is missing")?;
    if !signature
        .trusted_comment()
        .split('\t')
        .any(|part| part == format!("version:{version}"))
    {
        bail!("The manifest signature does not authenticate its release version");
    }
    Ok(raw)
}
#[derive(Debug, Clone)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub size: Option<u64>,
    pub url: Url,
    pub signature: String,
    pub expires_at: i64,
    pub sha256: Option<String>,
}
pub fn validate_metadata(raw: &serde_json::Value, channel: &Channel, qa: bool) -> Result<Release> {
    validate_metadata_at(raw, channel, qa, now_unix())
}
fn validate_metadata_at(
    raw: &serde_json::Value,
    channel: &Channel,
    qa: bool,
    now: i64,
) -> Result<Release> {
    let object = raw
        .as_object()
        .context("Update metadata must be an object")?;
    if object.keys().any(|key| {
        !["version", "notes", "pub_date", "expires_at", "platforms"].contains(&key.as_str())
    }) {
        bail!("Update manifest contains unsupported fields");
    }
    let version = raw["version"]
        .as_str()
        .context("Update version is missing")?;
    let version = semver::Version::parse(version)?;
    if version.pre.is_empty() == false {
        bail!("Prerelease updates are not enabled on this channel");
    }
    let date = |field: &str| -> Result<i64> {
        Ok(time::OffsetDateTime::parse(
            raw[field]
                .as_str()
                .with_context(|| format!("Update {field} is missing"))?,
            &time::format_description::well_known::Rfc3339,
        )
        .with_context(|| format!("Update {field} must be an RFC 3339 timestamp"))?
        .unix_timestamp())
    };
    let issued = date("pub_date")?;
    let expires_at = date("expires_at")?;
    if issued > now + 300 {
        bail!("Release metadata is dated in the future. Check the Windows clock and retry.");
    }
    if expires_at <= now {
        bail!("Release metadata has expired. Wait for a refreshed signed manifest or use the recovery installer.");
    }
    if expires_at <= issued || expires_at - issued > MAX_METADATA_AGE_SECONDS {
        bail!("Release metadata validity must be positive and no longer than 90 days");
    }
    let platform = raw["platforms"]
        .get("windows-x86_64-nsis")
        .or_else(|| raw["platforms"].get("windows-x86_64"))
        .context("No Windows x64 NSIS update is available")?;
    let url = Url::parse(
        platform["url"]
            .as_str()
            .context("Update installer URL is missing")?,
    )?;
    if !(qa && is_loopback(&url)) {
        let repo = channel
            .repository
            .as_deref()
            .context("GitHub repository is missing")?;
        valid_repository(repo)?;
        let prefix = format!("https://github.com/{repo}/releases/download/v{version}/");
        if !url.as_str().starts_with(&prefix)
            || !url.path().ends_with(".exe")
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("Installer must be a versioned NSIS asset in the configured GitHub release repository");
        }
    }
    let signature = platform["signature"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() < 8192)
        .context("Update artifact signature is missing")?
        .to_string();
    let size = platform
        .get("size")
        .map(|s| {
            s.as_u64()
                .context("Update size must be an unsigned byte count")
        })
        .transpose()?;
    if size.is_some_and(|n| n == 0 || n > MAX_PACKAGE as u64) {
        bail!("Update installer exceeds the allowed download size");
    }
    let notes = match raw.get("notes") {
        None => String::new(),
        Some(value) => value
            .as_str()
            .context("Release notes must be text")?
            .to_string(),
    };
    if notes.len() > 32 * 1024 {
        bail!("Release notes exceed the size limit");
    }
    let sha256 = platform
        .get("sha256")
        .map(|value| {
            let value = value.as_str().context("Installer SHA-256 must be text")?;
            if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                bail!("Installer SHA-256 is malformed");
            }
            Ok(value.to_ascii_lowercase())
        })
        .transpose()?;
    Ok(Release {
        version: version.to_string(),
        notes,
        size,
        url,
        signature,
        expires_at,
        sha256,
    })
}

pub fn validate_release(
    raw: &serde_json::Value,
    channel: &Channel,
    installed: &str,
    qa: bool,
) -> Result<Release> {
    let release = validate_metadata(raw, channel, qa)?;
    if semver::Version::parse(&release.version)? <= semver::Version::parse(installed)? {
        bail!("Release is not newer than the installed version");
    }
    Ok(release)
}

#[derive(Default)]
struct GateState {
    transactions: usize,
    updating: bool,
}
#[derive(Clone, Default)]
pub struct OperationGate(Arc<Mutex<GateState>>);
pub struct OperationLease {
    gate: OperationGate,
    update: bool,
}
impl OperationGate {
    pub fn transaction(&self) -> Result<OperationLease> {
        let mut s = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Operation guard failed"))?;
        if s.updating {
            bail!("Application update is active. Mod/file operations are temporarily blocked.");
        }
        s.transactions += 1;
        Ok(OperationLease {
            gate: self.clone(),
            update: false,
        })
    }
    pub fn update(&self) -> Result<OperationLease> {
        let mut s = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Operation guard failed"))?;
        if s.updating || s.transactions > 0 {
            bail!("Finish the active mod/file operation before updating Zoxryn Mods.");
        }
        s.updating = true;
        Ok(OperationLease {
            gate: self.clone(),
            update: true,
        })
    }
    pub fn active(&self) -> bool {
        self.0
            .lock()
            .map(|s| s.updating || s.transactions > 0)
            .unwrap_or(true)
    }
}
impl Drop for OperationLease {
    fn drop(&mut self) {
        if let Ok(mut s) = self.gate.0.lock() {
            if self.update {
                s.updating = false
            } else {
                s.transactions = s.transactions.saturating_sub(1)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authentic_manifest_passes_and_tampered_metadata_is_rejected() {
        let bytes = include_bytes!("../fixtures/updater/signed-manifest.json");
        let sig = include_str!("../fixtures/updater/signed-manifest.json.sig");
        let key = include_str!("../updater-public-key.pub");
        assert_eq!(
            verify_signed_manifest(bytes, sig, key).unwrap()["version"],
            "0.1.7"
        );
        let forged = String::from_utf8(bytes.to_vec())
            .unwrap()
            .replace("QA signed", "QA forged");
        assert!(verify_signed_manifest(forged.as_bytes(), sig, key).is_err());
        assert!(verify_signed_manifest(bytes, "invalid-signature", key).is_err());
        assert!(verify_signed_manifest(&vec![0; MAX_MANIFEST + 1], sig, key).is_err());
    }
    fn channel() -> Channel {
        Channel {
            repository: Some("ZoxrynExample/mods".into()),
            feed_url: Some(
                "https://github.com/ZoxrynExample/mods/releases/latest/download/latest.json".into(),
            ),
            recovery_url: None,
        }
    }
    fn release() -> serde_json::Value {
        let issued = time::OffsetDateTime::now_utc();
        let format = &time::format_description::well_known::Rfc3339;
        serde_json::json!({"version":"0.1.7","notes":"A release","pub_date":issued.format(format).unwrap(),"expires_at":(issued+time::Duration::days(30)).format(format).unwrap(),"platforms":{"windows-x86_64-nsis":{"url":"https://github.com/ZoxrynExample/mods/releases/download/v0.1.7/Zoxryn-Mods-0.1.7-Setup.exe","signature":"signed-by-release-tool","size":2000}}})
    }
    #[test]
    fn updates_and_transactions_are_mutually_exclusive() {
        let g = OperationGate::default();
        let t = g.transaction().unwrap();
        assert!(g.update().is_err());
        drop(t);
        let u = g.update().unwrap();
        assert!(g.transaction().is_err());
        assert!(g.update().is_err());
        drop(u);
        assert!(g.transaction().is_ok());
    }
    #[test]
    fn a_queued_transaction_also_blocks_the_update() {
        let g = OperationGate::default();
        let a = g.transaction().unwrap();
        let b = g.transaction().unwrap();
        drop(a);
        assert!(g.update().is_err());
        drop(b);
        assert!(g.update().is_ok());
    }
    #[test]
    fn gate_unwinds_after_failures() {
        let g = OperationGate::default();
        {
            let _u = g.update().unwrap();
        }
        assert!(!g.active());
        assert!(g.transaction().is_ok());
    }
    #[test]
    fn disabled_channel_performs_no_network_work() {
        let c = Channel {
            repository: None,
            feed_url: None,
            recovery_url: None,
        };
        assert!(c.validate(false).unwrap().is_none());
    }
    #[test]
    fn only_a_real_pinned_github_feed_is_accepted() {
        assert!(channel().validate(false).is_ok());
        for feed in [
            "http://github.com/ZoxrynExample/mods/releases/latest/download/latest.json",
            "https://evil.test/latest.json",
            "http://127.0.0.1:8999/latest.json",
        ] {
            let mut c = channel();
            c.feed_url = Some(feed.into());
            assert!(c.validate(false).is_err());
        }
        assert!(valid_repository("OWNER/REPO").is_err());
    }
    #[test]
    fn loopback_transport_is_test_only() {
        let c = Channel {
            repository: None,
            feed_url: Some("http://127.0.0.1:8999/latest.json".into()),
            recovery_url: None,
        };
        assert!(c.validate(true).is_ok());
        assert!(c.validate(false).is_err());
    }
    #[test]
    fn release_metadata_is_pinned_and_downgrades_are_rejected() {
        assert!(validate_release(&release(), &channel(), "0.1.6", false).is_ok());
        assert!(validate_release(&release(), &channel(), "0.1.7", false).is_err());
        let mut r = release();
        r["platforms"]["windows-x86_64-nsis"]["url"] =
            serde_json::json!("https://evil.test/app.exe");
        assert!(validate_release(&r, &channel(), "0.1.6", false).is_err());
    }
    #[test]
    fn missing_signatures_unsupported_targets_and_oversized_packages_fail() {
        for change in 0..3 {
            let mut r = release();
            if change == 0 {
                r["platforms"]["windows-x86_64-nsis"]["signature"] = serde_json::json!("");
            } else if change == 1 {
                r["platforms"] = serde_json::json!({});
            } else {
                r["platforms"]["windows-x86_64-nsis"]["size"] = serde_json::json!(MAX_PACKAGE + 1);
            }
            assert!(validate_release(&r, &channel(), "0.1.6", false).is_err());
        }
    }

    #[test]
    fn expired_future_and_malformed_metadata_cannot_offer_updates() {
        let raw = release();
        let issued = time::OffsetDateTime::parse(
            raw["pub_date"].as_str().unwrap(),
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap()
        .unix_timestamp();
        assert!(validate_metadata_at(&raw, &channel(), false, issued + 31 * 86400).is_err());
        assert!(validate_metadata_at(&raw, &channel(), false, issued - 301).is_err());
        for field in ["pub_date", "expires_at", "version", "notes"] {
            let mut invalid = raw.clone();
            invalid[field] = serde_json::json!(42);
            assert!(
                validate_metadata(&invalid, &channel(), false).is_err(),
                "{field}"
            );
        }
        let mut too_long = raw.clone();
        too_long["expires_at"] = serde_json::json!((time::OffsetDateTime::now_utc()
            + time::Duration::days(100))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap());
        assert!(validate_metadata(&too_long, &channel(), false).is_err());
        let mut prerelease = raw;
        prerelease["version"] = serde_json::json!("0.1.8-beta.1");
        assert!(validate_metadata(&prerelease, &channel(), false).is_err());
    }

    #[test]
    fn production_channel_and_redirects_stay_in_the_approved_repository() {
        let actual: Channel = serde_json::from_str(include_str!("../update-channel.json")).unwrap();
        assert_eq!(actual.repository.as_deref(), Some("zoxryn/zoxryn-mods"));
        assert_eq!(
            actual.validate(false).unwrap().unwrap().as_str(),
            "https://github.com/zoxryn/zoxryn-mods/releases/latest/download/latest.json"
        );
        for good in [
            "https://github.com/zoxryn/zoxryn-mods/releases/download/v0.1.7/latest.json",
            "https://release-assets.githubusercontent.com/asset?token=cdn",
        ] {
            assert!(approved_redirect(
                &Url::parse(good).unwrap(),
                actual.repository.as_deref(),
                false
            ));
        }
        for bad in [
            "http://github.com/zoxryn/zoxryn-mods/releases/latest",
            "https://github.com/other/repo/releases/download/app.exe",
            "https://github.com:444/zoxryn/zoxryn-mods/releases/asset",
            "https://evil.test/app.exe",
            "https://user:pass@release-assets.githubusercontent.com/asset",
        ] {
            assert!(!approved_redirect(
                &Url::parse(bad).unwrap(),
                actual.repository.as_deref(),
                false
            ));
        }
    }
}
