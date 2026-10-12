use std::sync::Arc;

use tauri::State;

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
pub async fn settings_set_storage_root(settings: Settings<'_>, path: String) -> AppResult<()> {
    settings.set_storage_root(&path);
    Ok(())
}
