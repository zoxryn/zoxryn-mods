//! Offline public-key verification. Reads release artifacts only; never runs them.
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use minisign_verify::{PublicKey, Signature};
use std::{fs, path::PathBuf};
fn main() -> Result<()> {
    let artifact = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("Pass a signed installer")?,
    );
    let expected = std::env::args()
        .nth(2)
        .context("Pass its release version")?;
    let key =
        String::from_utf8(STANDARD.decode(include_str!("../updater-public-key.pub").trim())?)?;
    let sig = fs::read_to_string(format!("{}.sig", artifact.display()))?;
    let sig = Signature::decode(&String::from_utf8(STANDARD.decode(sig.trim())?)?)?;
    let bytes = fs::read(&artifact)?;
    let public = PublicKey::decode(&key)?;
    public.verify(&bytes, &sig, false)?;
    if !sig
        .trusted_comment()
        .split('\t')
        .any(|p| p == format!("version:{expected}"))
    {
        bail!("The authenticated signature does not bind the expected release version");
    }
    let mut tampered = bytes.clone();
    *tampered.last_mut().context("Empty installer")? ^= 1;
    if public.verify(&tampered, &sig, false).is_ok() {
        bail!("Tampering was accepted");
    }
    let mut manifest_verified = false;
    if let Some(path) = std::env::args_os().nth(3) {
        use sha2::{Digest, Sha256};
        let path = PathBuf::from(path);
        let manifest = fs::read(&path)?;
        let detached = fs::read_to_string(format!("{}.sig", path.display()))?;
        let encoded_key = include_str!("../updater-public-key.pub");
        let raw =
            zoxryn_core::update_policy::verify_signed_manifest(&manifest, &detached, encoded_key)?;
        let channel = serde_json::from_str(include_str!("../update-channel.json"))?;
        let release = zoxryn_core::update_policy::validate_metadata(&raw, &channel, false)?;
        let checksum = format!("{:x}", Sha256::digest(&bytes));
        if release.version != expected
            || release.size != Some(bytes.len() as u64)
            || release.sha256.as_deref() != Some(checksum.as_str())
        {
            bail!("Manifest version, size or SHA-256 does not match the installer");
        }
        let mut altered = manifest;
        *altered.last_mut().context("Empty manifest")? ^= 1;
        if zoxryn_core::update_policy::verify_signed_manifest(&altered, &detached, encoded_key)
            .is_ok()
        {
            bail!("Modified metadata was accepted");
        }
        manifest_verified = true;
    }
    println!(
        "{}",
        serde_json::json!({"artifact":artifact.file_name().map(|n| n.to_string_lossy()),"bytes":bytes.len(),"version":expected,"public_key_verified":true,"signed_version_verified":true,"tampered_bytes_rejected":true,"manifest_and_sha256_verified":manifest_verified,"executed":false})
    );
    Ok(())
}
