use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use zoxryn_core::{
    safety::{check_tree, guard_tree},
    update_policy::*,
    Engine,
};
type Shared = Arc<Mutex<Engine>>;
static GATE: OnceLock<OperationGate> = OnceLock::new();
pub fn gate() -> &'static OperationGate {
    GATE.get_or_init(OperationGate::default)
}
const PUBLIC_KEY: &str = include_str!("../updater-public-key.pub");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub installed_version: String,
    pub configured: bool,
    pub phase: String,
    pub available_version: Option<String>,
    pub release_notes: String,
    pub size_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub error: Option<String>,
    pub can_install: bool,
    pub eligibility_message: String,
    pub approval_token: Option<String>,
    pub metadata_verified: bool,
    pub last_checked_at: Option<i64>,
}
struct Offered {
    update: Update,
    release: Release,
    token: String,
    checked: Instant,
}
struct Inner {
    status: UpdateStatus,
    offer: Option<Offered>,
}
pub struct UpdateService {
    inner: Mutex<Inner>,
    channel: Channel,
    engine: Shared,
    data: PathBuf,
    install: PathBuf,
}
#[derive(Serialize, Deserialize)]
struct Receipt {
    from_version: String,
    to_version: String,
    phase: String,
    database_backup: String,
    message: Option<String>,
}

fn channel() -> Result<Channel> {
    #[cfg(feature = "ui-qa")]
    {
        return Ok(Channel {
            repository: None,
            feed_url: None,
            recovery_url: None,
        });
    }
    #[cfg(feature = "updater-qa")]
    {
        let bytes = std::fs::read(crate::updater_qa::root()?.join("channel.json"))?;
        return Ok(serde_json::from_slice(&bytes)?);
    }
    #[cfg(not(any(feature = "updater-qa", feature = "ui-qa")))]
    Ok(serde_json::from_str(include_str!(
        "../update-channel.json"
    ))?)
}
pub fn qa_enabled() -> bool {
    cfg!(feature = "updater-qa")
}
fn installed_path() -> Result<PathBuf> {
    Ok(std::fs::canonicalize(
        std::env::current_exe()?
            .parent()
            .context("No application directory")?,
    )?)
}
fn eligible(install: &Path) -> Result<()> {
    if cfg!(feature = "ui-qa") {
        bail!("UI QA product cannot install application updates");
    }
    check_tree(install)?;
    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let product = if qa_enabled() {
            "Zoxryn Mods Updater QA"
        } else {
            "Zoxryn Mods"
        };
        let key=RegKey::predef(HKEY_CURRENT_USER).open_subkey(format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{product}")).context("Install the recovery installer once to enable in-place updates. Portable/build copies cannot replace another installation.")?;
        let location: String = key.get_value("InstallLocation")?;
        let location = PathBuf::from(location.trim_matches('"'));
        check_tree(&location)?;
        if !std::fs::canonicalize(location)?
            .to_string_lossy()
            .eq_ignore_ascii_case(&install.to_string_lossy())
        {
            bail!("This executable is not running from the registered installation. Use the recovery installer.");
        }
    }
    #[cfg(not(windows))]
    bail!("This release supports the Windows NSIS updater only");
    Ok(())
}
impl UpdateService {
    pub fn new(app: &AppHandle, engine: Shared) -> Result<Self> {
        let installed = app.package_info().version.to_string();
        let channel = channel()?;
        let configured = channel.validate(qa_enabled())?.is_some();
        let install = installed_path()?;
        let eligibility = eligible(&install);
        let can_install = eligibility.is_ok();
        let data = engine
            .lock()
            .map_err(|_| anyhow::anyhow!("Manager lock failed"))?
            .data_path()
            .to_path_buf();
        let last_checked_at = std::fs::read(data.join("app-updates/last-check.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<i64>(&bytes).ok());
        let mut status=UpdateStatus{installed_version:installed.clone(),configured,phase:if configured{"idle"}else{"not_configured"}.into(),available_version:None,release_notes:String::new(),size_bytes:None,downloaded_bytes:0,total_bytes:None,error:None,can_install,eligibility_message:eligibility.err().map(|e|e.to_string()).unwrap_or_else(||"Verified current-user installation; user data stays outside the application directory.".into()),approval_token:None,metadata_verified:false,last_checked_at};
        if let Ok(bytes) = std::fs::read(data.join("app-updates/receipt.json")) {
            if let Ok(mut receipt) = serde_json::from_slice::<Receipt>(&bytes) {
                if receipt.phase == "installing" {
                    if receipt.to_version == installed {
                        receipt.phase = "completed".into();
                    } else {
                        status.error=Some("The previous update did not finish. Check again or use the manual recovery installer; your data was retained.".into());
                        receipt.phase = "incomplete".into();
                    }
                    let _ = write_receipt(&data, &receipt);
                }
            }
        }
        Ok(Self {
            inner: Mutex::new(Inner {
                status,
                offer: None,
            }),
            channel,
            engine,
            data,
            install,
        })
    }
    pub fn status(&self) -> UpdateStatus {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .status
            .clone()
    }
    fn change(&self, app: &AppHandle, edit: impl FnOnce(&mut UpdateStatus)) {
        let value = {
            let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
            edit(&mut inner.status);
            inner.status.clone()
        };
        let _ = app.emit("app-update-status", &value);
    }
    fn failure(&self, app: &AppHandle, message: String) {
        self.change(app, |s| {
            s.phase = "error".into();
            s.error = Some(message);
        });
    }
}
fn write_receipt(data: &Path, receipt: &Receipt) -> Result<()> {
    let directory = data.join("app-updates");
    let _guards = guard_tree(&directory, true)?;
    let temp = directory.join(format!("receipt-{}.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&temp, serde_json::to_vec_pretty(receipt)?)?;
    std::fs::File::options()
        .write(true)
        .open(&temp)?
        .sync_all()?;
    std::fs::rename(temp, directory.join("receipt.json"))?;
    Ok(())
}
fn client(repository: Option<String>) -> Result<reqwest::Client> {
    // restore_update bypasses the plugin's check(), where ring is normally set.
    let _ = rustls::crypto::ring::default_provider().install_default();
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            let url = attempt.url();
            let permitted = approved_redirect(url, repository.as_deref(), qa_enabled());
            if permitted && attempt.previous().len() < 5 {
                attempt.follow()
            } else {
                attempt.error("Update redirect is not an approved secure release host")
            }
        }))
        .build()?)
}
async fn bounded_get(url: url::Url, limit: usize, channel: &Channel) -> Result<Vec<u8>> {
    let mut response = client(channel.repository.clone())?
        .get(url)
        .send()
        .await
        .context("Could not reach the update server")?
        .error_for_status()
        .context("The update server returned an error")?;
    if response.content_length().is_some_and(|n| n > limit as u64) {
        bail!("Update response exceeds its allowed size");
    }
    let mut result = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if result.len() + chunk.len() > limit {
            bail!("Update response exceeds its allowed size");
        }
        result.extend_from_slice(&chunk);
    }
    Ok(result)
}
pub async fn check(app: &AppHandle, service: &UpdateService) -> Result<UpdateStatus> {
    let Some(feed) = service.channel.validate(qa_enabled())? else {
        return Ok(service.status());
    };
    {
        let mut inner = service
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("Updater state failed"))?;
        if ["checking", "downloading", "verifying", "installing"]
            .contains(&inner.status.phase.as_str())
        {
            return Ok(inner.status.clone());
        }
        inner.offer = None;
        inner.status.approval_token = None;
        inner.status.phase = "checking".into();
        inner.status.available_version = None;
        inner.status.release_notes.clear();
        inner.status.size_bytes = None;
        inner.status.total_bytes = None;
        inner.status.downloaded_bytes = 0;
    }
    service.change(app, |s| {
        s.phase = "checking".into();
        s.error = None;
        s.metadata_verified = false;
        s.last_checked_at = Some(now_unix());
    });
    // A small separate state file preserves check history without a SQLite migration.
    let directory = service.data.join("app-updates");
    if let Ok(_guards) = guard_tree(&directory, true) {
        let temporary = directory.join(format!("last-check-{}.tmp", uuid::Uuid::new_v4()));
        if let Ok(bytes) = serde_json::to_vec(&service.status().last_checked_at) {
            if std::fs::write(&temporary, bytes).is_ok() {
                let _ = std::fs::rename(&temporary, directory.join("last-check.json"));
            }
            let _ = std::fs::remove_file(temporary);
        }
    }
    let result = async {
        let mut sig_url = feed.clone();
        sig_url.set_path(&(feed.path().to_string() + ".sig"));
        let bytes = match bounded_get(feed, MAX_MANIFEST, &service.channel).await {
            Ok(bytes) => bytes,
            Err(error)
                if error
                    .downcast_ref::<reqwest::Error>()
                    .and_then(|e| e.status())
                    == Some(reqwest::StatusCode::NOT_FOUND) =>
            {
                service.change(app, |s| {
                    s.phase = "not_published".into();
                    s.error = None;
                });
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let signature = String::from_utf8(bounded_get(sig_url, 8192, &service.channel).await?)?;
        let raw = verify_signed_manifest(&bytes, &signature, PUBLIC_KEY)?;
        let release = validate_metadata(&raw, &service.channel, qa_enabled())?;
        let installed = service.status().installed_version;
        let version = semver::Version::parse(
            raw["version"]
                .as_str()
                .context("Signed update version is missing")?,
        )?;
        let current = semver::Version::parse(&installed)?;
        if version < current {
            bail!("The release feed attempted a downgrade. The installed application was kept.");
        }
        if version == current {
            service.change(app, |s| {
                s.phase = "up_to_date".into();
                s.available_version = None;
                s.approval_token = None;
                s.metadata_verified = true;
                s.release_notes.clear();
            });
            return Ok(());
        }
        let update = app
            .updater_builder()
            .endpoints(vec![service
                .channel
                .validate(qa_enabled())?
                .context("Update feed is not configured")?])?
            .pubkey(PUBLIC_KEY.trim())
            .timeout(Duration::from_secs(60))
            .installer_args([format!("/D={}", service.install.display())])
            .build()?
            .restore_update(raw)?
            .context("No newer signed Windows update is available")?;
        if update.version != release.version
            || update.download_url != release.url
            || update.signature != release.signature
        {
            bail!("Updater metadata selection did not match the signed manifest");
        }
        let token = uuid::Uuid::new_v4().to_string();
        {
            let mut inner = service.inner.lock().unwrap();
            inner.offer = Some(Offered {
                update,
                release: release.clone(),
                token: token.clone(),
                checked: Instant::now(),
            });
        }
        service.change(app, |s| {
            s.phase = "available".into();
            s.available_version = Some(release.version);
            s.release_notes = release.notes;
            s.size_bytes = release.size;
            s.total_bytes = release.size;
            s.downloaded_bytes = 0;
            s.approval_token = Some(token);
            s.metadata_verified = true;
        });
        Ok::<_, anyhow::Error>(())
    }
    .await;
    if let Err(e) = result {
        service.failure(app, format!("Update check failed: {e:#}"));
        return Err(e);
    }
    Ok(service.status())
}
pub async fn approve(
    app: &AppHandle,
    service: &UpdateService,
    token: String,
    approved: bool,
) -> Result<()> {
    if cfg!(feature = "ui-qa") {
        bail!("UI QA product cannot download or install updates");
    }
    if !approved {
        bail!("Download and installation require your explicit approval");
    }
    let (update, release) = {
        let inner = service
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("Updater state failed"))?;
        if ["downloading", "verifying", "installing"].contains(&inner.status.phase.as_str()) {
            bail!("An update is already active");
        }
        let offer = inner
            .offer
            .as_ref()
            .context("Check for updates before approving an installation")?;
        if offer.token != token || offer.checked.elapsed() > Duration::from_secs(1800) {
            bail!("Update offer expired; check again before approving");
        }
        if offer.release.expires_at <= now_unix() {
            bail!("The signed release metadata expired. Check again before approving.");
        }
        (offer.update.clone(), offer.release.clone())
    };
    let _exclusive = gate().update()?;
    eligible(&service.install)?;
    let checkpoint = {
        let engine = service
            .engine
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Finish the active file operation before updating"))?;
        engine.checkpoint_for_app_update(&service.install)?
    };
    let from_version = service.status().installed_version;
    let mut receipt = Receipt {
        from_version,
        to_version: release.version.clone(),
        phase: "downloading".into(),
        database_backup: checkpoint.to_string_lossy().into(),
        message: None,
    };
    write_receipt(&service.data, &receipt)?;
    service.change(app, |s| {
        s.phase = "downloading".into();
        s.error = None;
        s.downloaded_bytes = 0;
        s.total_bytes = release.size;
    });
    let result = async {
        let mut response = client(service.channel.repository.clone())?
            .get(release.url.clone())
            .timeout(Duration::from_secs(180))
            .send()
            .await
            .context("Could not download the update; check your connection and retry")?
            .error_for_status()?;
        let total = response.content_length().or(release.size);
        if total.is_some_and(|n| n > MAX_PACKAGE as u64) {
            bail!("Update exceeds the maximum download size");
        }
        let mut bytes = Vec::new();
        let mut last = Instant::now();
        while let Some(chunk) = response.chunk().await.context("Update download was interrupted. The installed application and data were kept; check the connection and retry.")? {
            if bytes.len() + chunk.len() > MAX_PACKAGE {
                bail!("Update exceeded the allowed download size");
            }
            bytes.extend_from_slice(&chunk);
            if last.elapsed() >= Duration::from_millis(100) {
                service.change(app, |s| {
                    s.downloaded_bytes = bytes.len() as u64;
                    s.total_bytes = total;
                });
                last = Instant::now();
            }
        }
        if release.size.is_some_and(|n| n != bytes.len() as u64) {
            bail!("Downloaded update size does not match its signed manifest");
        }
        if release
            .sha256
            .as_ref()
            .is_some_and(|expected| format!("{:x}", Sha256::digest(&bytes)) != *expected)
        {
            bail!("Downloaded update SHA-256 does not match its signed manifest");
        }
        if release.expires_at <= now_unix() {
            bail!("Release metadata expired during download. Check again before retrying.");
        }
        service.change(app, |s| {
            s.phase = "verifying".into();
            s.downloaded_bytes = bytes.len() as u64;
            s.total_bytes = Some(bytes.len() as u64);
        });
        update.verify(&bytes).context(
            "Official Tauri update signature/version verification failed; installation was blocked",
        )?;
        receipt.phase = "installing".into();
        write_receipt(&service.data, &receipt)?;
        service.change(app, |s| s.phase = "installing".into());
        // On Windows the official updater launches NSIS with /UPDATE /R and exits.
        // The explicit final /D argument pins installation to the registered directory.
        update.install(&bytes).context(
            "The verified installer could not start. Use the recovery installer if retry fails",
        )?;
        Ok::<_, anyhow::Error>(())
    }
    .await;
    if let Err(e) = result {
        receipt.phase = "failed".into();
        receipt.message = Some(format!("{e:#}"));
        let _ = write_receipt(&service.data, &receipt);
        service.failure(app, format!("Update failed: {e:#}"));
        return Err(e);
    }
    Ok(())
}

#[tauri::command]
pub fn update_status(service: State<'_, UpdateService>) -> UpdateStatus {
    service.status()
}
#[tauri::command]
pub async fn check_app_update(
    app: AppHandle,
    service: State<'_, UpdateService>,
) -> Result<UpdateStatus, String> {
    check(&app, &service).await.map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn approve_app_update(
    app: AppHandle,
    service: State<'_, UpdateService>,
    token: String,
    approved: bool,
) -> Result<(), String> {
    approve(&app, &service, token, approved)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn open_recovery_page(service: State<'_, UpdateService>) -> Result<(), String> {
    let repo = service.channel.repository.as_deref().ok_or(
        "The release repository is not configured. Use the supplied local recovery installer.",
    )?;
    valid_repository(repo).map_err(|e| e.to_string())?;
    let url = format!("https://github.com/{repo}/releases/latest");
    let mut command = std::process::Command::new("rundll32.exe");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .args(["url.dll,FileProtocolHandler", &url])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn updater_http_client_initializes_its_crypto_provider() {
        assert!(super::client(Some("zoxryn/zoxryn-mods".into())).is_ok());
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }

    #[test]
    #[ignore = "Explicit live GitHub metadata test; never downloads installers or opens the application"]
    fn live_github_channel_uses_the_production_https_client() {
        use zoxryn_core::update_policy::*;
        let channel: Channel =
            serde_json::from_str(include_str!("../update-channel.json")).unwrap();
        let url = channel.validate(false).unwrap().unwrap();
        let result =
            tauri::async_runtime::block_on(super::bounded_get(url, MAX_MANIFEST, &channel));
        match result {
            Ok(bytes) => assert!(!bytes.is_empty()),
            Err(error) => {
                assert_eq!(
                    error
                        .downcast_ref::<reqwest::Error>()
                        .and_then(|e| e.status()),
                    Some(reqwest::StatusCode::NOT_FOUND),
                    "{error:#}"
                );
                println!("LIVE_GITHUB_METADATA: HTTPS succeeded; repository has no published latest.json (HTTP 404). No installer requested.");
            }
        }
    }
}
