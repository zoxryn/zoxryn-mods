use crate::app_updates;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tauri::{Manager, State};
#[cfg(not(feature = "updater-qa"))]
use zoxryn_core::game;
use zoxryn_core::{model::*, scanner, Engine};
type Shared = Arc<Mutex<Engine>>;

async fn run<T: Send + 'static>(
    state: State<'_, Shared>,
    task: impl FnOnce(&mut Engine) -> anyhow::Result<T> + Send + 'static,
) -> Result<T, String> {
    let shared = state.inner().clone();
    let _lease = app_updates::gate()
        .transaction()
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = _lease;
        let mut engine = shared
            .lock()
            .map_err(|_| "Manager lock failed".to_string())?;
        task(&mut engine).map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn snapshot(state: State<'_, Shared>) -> Result<Snapshot, String> {
    run(state, |e| e.snapshot()).await
}
#[tauri::command]
async fn detect_games() -> Result<Vec<Game>, String> {
    #[cfg(feature = "updater-qa")]
    {
        Ok(vec![])
    }
    #[cfg(not(feature = "updater-qa"))]
    {
        tauri::async_runtime::spawn_blocking(game::detect)
            .await
            .map_err(|e| e.to_string())
    }
}
#[tauri::command]
async fn select_game(
    state: State<'_, Shared>,
    path: String,
    launcher: String,
) -> Result<Game, String> {
    run(state, move |e| e.select_game(Path::new(&path), &launcher)).await
}
#[tauri::command]
async fn import_archive(state: State<'_, Shared>, path: String) -> Result<ModRecord, String> {
    run(state, move |e| e.import(Path::new(&path))).await
}
#[tauri::command]
async fn prepare_operation(
    state: State<'_, Shared>,
    id: String,
    operation: String,
) -> Result<Plan, String> {
    run(state, move |e| e.prepare(&id, &operation)).await
}
#[tauri::command]
async fn prepare_restore(state: State<'_, Shared>, id: String) -> Result<Plan, String> {
    run(state, move |e| e.prepare_restore(&id)).await
}
#[tauri::command]
async fn prepare_clean(state: State<'_, Shared>) -> Result<Plan, String> {
    run(state, |e| e.prepare_clean()).await
}
#[tauri::command]
async fn scan_installed(state: State<'_, Shared>) -> Result<ScanInventory, String> {
    let shared = state.inner().clone();
    let lease = app_updates::gate()
        .transaction()
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || -> Result<ScanInventory, String> {
        let _lease = lease;
        let (game, owners, revision) = {
            let e = shared.lock().map_err(|_| "Manager lock failed")?;
            e.scan_context().map_err(|e| format!("{e:#}"))?
        };
        let result = scanner::scan(&game, &owners).map_err(|e| format!("{e:#}"))?;
        let mut e = shared.lock().map_err(|_| "Manager lock failed")?;
        e.persist_scan(result, revision)
            .map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn prepare_adoption(state: State<'_, Shared>, id: String) -> Result<Plan, String> {
    run(state, move |e| e.prepare_adoption(&id)).await
}
#[tauri::command]
async fn organization_snapshot(
    state: State<'_, Shared>,
) -> Result<zoxryn_core::organization::OrganizationSnapshot, String> {
    run(state, |e| e.organization_snapshot()).await
}
#[tauri::command]
async fn save_collection(
    state: State<'_, Shared>,
    id: Option<String>,
    name: String,
    description: String,
    category: String,
    members: Vec<zoxryn_core::organization::MemberRef>,
) -> Result<String, String> {
    run(state, move |e| {
        e.save_collection(id.as_deref(), &name, &description, &category, &members)
    })
    .await
}
#[tauri::command]
async fn remove_collection(state: State<'_, Shared>, id: String) -> Result<(), String> {
    run(state, move |e| e.remove_collection(&id)).await
}
#[tauri::command]
async fn assign_preview(
    state: State<'_, Shared>,
    kind: String,
    id: String,
    path: Option<String>,
) -> Result<(), String> {
    run(state, move |e| {
        e.assign_preview(&kind, &id, path.as_deref().map(Path::new))
    })
    .await
}
#[tauri::command]
async fn read_preview(state: State<'_, Shared>, key: String) -> Result<String, String> {
    run(state, move |e| {
        use base64::Engine as _;
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(e.read_preview(&key)?)
        ))
    })
    .await
}
#[tauri::command]
async fn rename_suit(
    state: State<'_, Shared>,
    id: String,
    name: String,
    category: String,
) -> Result<(), String> {
    run(state, move |e| e.rename_suit(&id, &name, &category)).await
}
#[tauri::command]
async fn prepare_suit(
    state: State<'_, Shared>,
    id: String,
    operation: String,
) -> Result<Plan, String> {
    run(state, move |e| e.prepare_suit(&id, &operation)).await
}
#[tauri::command]
async fn apply_plan(
    state: State<'_, Shared>,
    token: String,
    approved: bool,
    compatibility_reviewed: bool,
    ownership_reviewed: Option<bool>,
) -> Result<Report, String> {
    run(state, move |e| {
        e.apply_with_ownership(
            &token,
            approved,
            compatibility_reviewed,
            ownership_reviewed.unwrap_or(false),
        )
    })
    .await
}
#[tauri::command]
async fn retry_recovery(state: State<'_, Shared>) -> Result<(), String> {
    run(state, |e| e.recover()).await
}
#[tauri::command]
async fn open_location(state: State<'_, Shared>, kind: String) -> Result<(), String> {
    run(state, move |e| {
        let path = match kind.as_str() {
            "game" => e
                .selected_game()?
                .map(|g| g.path)
                .ok_or_else(|| anyhow::anyhow!("Select a game first"))?,
            "data" => e.data_path().to_string_lossy().into(),
            _ => anyhow::bail!("Unknown location"),
        };
        hidden_command("explorer.exe").arg(path).spawn()?;
        Ok(())
    })
    .await
}
fn hidden_command(program: &str) -> std::process::Command {
    let mut c = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c
}
#[tauri::command]
fn open_official(id: String) -> Result<(), String> {
    let url = match id.as_str() {
        "shv" | "asi" => "https://www.dev-c.com/gtav/scripthookv/",
        "shvdn" => "https://github.com/scripthookvdotnet/scripthookvdotnet/releases",
        "openiv" => "https://openiv.com/",
        "rph" | "lspdfr" => "https://www.lcpdfr.com/lspdfr/legacy/install/",
        "gta5mods" => "https://www.gta5-mods.com/",
        "lcpdfr" => "https://www.lcpdfr.com/",
        _ => return Err("Only verified publisher pages can be opened".into()),
    };
    hidden_command("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn run_app() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(not(feature = "updater-qa"))]
            let data = {
                let args: Vec<_> = std::env::args_os().collect();
                if let Some(i) = args.iter().position(|a| a == "--data-dir") {
                    std::path::PathBuf::from(args.get(i + 1).ok_or("--data-dir requires a path")?)
                } else {
                    app.path().app_local_data_dir()?
                }
            };
            #[cfg(feature = "updater-qa")]
            let data = crate::updater_qa::root()?.join("data");
            let engine = match Engine::open(&data) {
                Ok(e) => e,
                Err(e) => {
                    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                    app.dialog()
                        .message(format!(
                            "Could not open the Zoxryn Mods workspace.\n\n{e:#}"
                        ))
                        .title("Zoxryn Mods")
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                    return Err(e.into());
                }
            };
            let shared = Arc::new(Mutex::new(engine));
            app.manage(shared.clone());
            app.manage(app_updates::UpdateService::new(
                app.handle(),
                shared.clone(),
            )?);
            #[cfg(feature = "updater-qa")]
            crate::updater_qa::start(app.handle().clone(), shared);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_updates::update_status,
            app_updates::check_app_update,
            app_updates::approve_app_update,
            app_updates::open_recovery_page,
            snapshot,
            detect_games,
            select_game,
            import_archive,
            prepare_operation,
            prepare_restore,
            prepare_clean,
            scan_installed,
            prepare_adoption,
            organization_snapshot,
            save_collection,
            remove_collection,
            assign_preview,
            read_preview,
            rename_suit,
            prepare_suit,
            apply_plan,
            retry_recovery,
            open_location,
            open_official
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        let _ = std::fs::write(
            std::env::temp_dir().join("zoxryn-mods-startup-error.txt"),
            e.to_string(),
        );
    }
}
