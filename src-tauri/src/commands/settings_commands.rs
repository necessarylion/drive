use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::config::backup_config::BackupConfig;
use crate::controllers::settings_controller::SettingsController;
use crate::models::backup::{BackupProvider, SettingsView};
use crate::models::error::AppResult;

type Settings<'a> = State<'a, Arc<SettingsController>>;

#[tauri::command]
pub async fn settings_load(settings: Settings<'_>) -> AppResult<SettingsView> {
    Ok(settings.view())
}

#[tauri::command]
pub async fn settings_config_for(settings: Settings<'_>, provider: BackupProvider) -> AppResult<BackupConfig> {
    Ok(settings.config_for(provider))
}

#[tauri::command]
pub async fn settings_set_storage_root(app: AppHandle, settings: Settings<'_>, path: String) -> AppResult<()> {
    // ponytail: the app starts again to use the new folder. That is one line, and no service can keep
    // the old folder by mistake. Swap the folder inside the running services if the restart gets in the way.
    if settings.set_storage_root(&path) {
        app.restart();
    }
    Ok(())
}
