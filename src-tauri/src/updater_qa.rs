//! Compiled only into the isolated updater-qa binary; never enabled in releases.
use crate::app_updates::{self, UpdateService};
use anyhow::{bail, Result};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Listener, Manager};
use zoxryn_core::{safety::check_tree, Engine};
pub fn root() -> Result<PathBuf> {
    let path = PathBuf::from(env!("ZOXRYN_UPDATER_QA_ROOT"));
    check_tree(&path)?;
    if std::fs::read_to_string(path.join("SYNTHETIC-UPDATER-QA"))?.trim() != "ZOXRYN-SYNTHETIC-ONLY"
    {
        bail!("Updater QA marker is missing");
    }
    Ok(path)
}
fn write(name: &str, value: serde_json::Value) -> Result<()> {
    let root = root()?;
    std::fs::create_dir_all(root.join("evidence"))?;
    std::fs::write(
        root.join("evidence").join(name),
        serde_json::to_vec_pretty(&value)?,
    )?;
    Ok(())
}
fn mode(value: &str) -> Result<()> {
    std::fs::write(root()?.join("scenario.txt"), value)?;
    Ok(())
}
pub fn start(app: AppHandle, engine: Arc<Mutex<Engine>>) {
    tauri::async_runtime::spawn(async move {
        let version = app.package_info().version.to_string();
        let result=async{
            let versions:serde_json::Value=serde_json::from_slice(&std::fs::read(root()?.join("versions.json"))?)?;
            if version==versions["to"].as_str().unwrap_or(""){
                let service=app.state::<UpdateService>();
                let restored_last_check=service.status().last_checked_at;
                mode("valid")?;
                let current=app_updates::check(&app,&service).await?;
                if current.phase!="up_to_date" || !current.metadata_verified {bail!("Installed stable release was not recognized");}
                let e=engine.lock().unwrap();let snapshot=e.snapshot()?;
                let status=app.state::<UpdateService>().status();
                write("relaunch.json",serde_json::json!({"version":version,"identifier":app.config().identifier,"executable":std::env::current_exe()?,"data":e.data_path(),"mods":snapshot.mods.len(),"backups":snapshot.backups.len(),"launched":true,"last_checked_at":status.last_checked_at,"last_check_restored_before_check":restored_last_check,"up_to_date_verified":true}))?;
                return Ok::<_,anyhow::Error>(());
            }
            if version!=versions["from"].as_str().unwrap_or(""){bail!("Unexpected QA source version");}
            let progress=Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
            let recorded=progress.clone();
            app.listen("app-update-status", move |event| {
                if let Ok(status)=serde_json::from_str::<serde_json::Value>(event.payload()) {
                    let mut events=recorded.lock().unwrap();
                    events.push(serde_json::json!({"phase":status["phase"],"downloaded_bytes":status["downloaded_bytes"],"total_bytes":status["total_bytes"]}));
                    let _=write("progress.json",serde_json::json!(*events));
                }
            });
            let service=app.state::<UpdateService>();
            if !service.status().error.as_deref().unwrap_or("").contains("previous update did not finish") {bail!("Incomplete receipt recovery was not detected");}
            let mut outcomes=Vec::new();
            outcomes.push(serde_json::json!({"case":"incomplete_receipt","blocked":true}));
            for case in ["network_error","bad_manifest","expired_metadata","downgrade","prerelease","invalid_manifest","tampered_installer","interrupted_download","wrong_artifact_version"]{
                mode(case)?;
                let checked=app_updates::check(&app,&service).await;
                if ["tampered_installer","interrupted_download","wrong_artifact_version"].contains(&case){
                    let status=checked?;let err=app_updates::approve(&app,&service,status.approval_token.unwrap(),true).await.err().ok_or_else(||anyhow::anyhow!("Tampered installer was accepted"))?;
                    outcomes.push(serde_json::json!({"case":case,"blocked":true,"error":err.to_string()}));
                }else{let err=checked.err().ok_or_else(||anyhow::anyhow!("Invalid update feed was accepted"))?;outcomes.push(serde_json::json!({"case":case,"blocked":true,"error":err.to_string()}));}
            }
            mode("missing_release")?;let missing=app_updates::check(&app,&service).await?;
            if missing.phase!="not_published" || missing.approval_token.is_some(){bail!("Missing GitHub release was not handled");}
            outcomes.push(serde_json::json!({"case":"missing_release","blocked":true}));
            mode("valid")?;let status=app_updates::check(&app,&service).await?;
            let token=status.approval_token.unwrap();
            if app_updates::approve(&app,&service,token.clone(),false).await.is_ok(){bail!("Unapproved update was accepted");}
            let transaction=app_updates::gate().transaction()?;
            if app_updates::approve(&app,&service,token.clone(),true).await.is_ok(){bail!("Update ran during a transaction");}
            drop(transaction);
            outcomes.push(serde_json::json!({"case":"unapproved","blocked":true}));outcomes.push(serde_json::json!({"case":"active_transaction","blocked":true}));
            write("before-install.json",serde_json::json!({"version":version,"identifier":app.config().identifier,"checks":outcomes,"approval_required":true}))?;
            // This approval is restricted to this marker-protected, compiled QA binary.
            // The official updater now verifies, installs, exits and restarts v0.1.7.
            app_updates::approve(&app,&service,token,true).await?;Ok(())
        }.await;
        if let Err(e) = result {
            let _ = write(
                "failed.json",
                serde_json::json!({"error":format!("{e:#}"),"version":version}),
            );
        }
        app.exit(0);
    });
}
