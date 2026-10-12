use std::sync::Arc;

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::config::backup_config::BackupConfig;
use crate::controllers::backup_controller::BackupController;
use crate::models::backup::{BackupProvider, BackupStatus};
use crate::models::error::{AppError, AppResult};

type Backup<'a> = State<'a, Arc<BackupController>>;

#[tauri::command]
pub async fn backup_status(backup: Backup<'_>) -> AppResult<BackupStatus> {
    Ok(backup.status())
}

#[tauri::command]
pub async fn backup_auto(backup: Backup<'_>) -> AppResult<()> {
    backup.auto_back_up();
    Ok(())
}

#[tauri::command]
pub async fn backup_toggle_pause(backup: Backup<'_>) -> AppResult<()> {
    backup.toggle_pause();
    Ok(())
}

/// Returns how many files are only in the backup. See `BackupController::back_up`.
#[tauri::command]
pub async fn backup_now(backup: Backup<'_>) -> AppResult<usize> {
    backup.back_up().await
}

#[tauri::command]
pub async fn backup_finish(backup: Backup<'_>, remove_extras: bool) -> AppResult<()> {
    backup.finish_backup(remove_extras).await
}

#[tauri::command]
pub async fn backup_restore(backup: Backup<'_>) -> AppResult<usize> {
    backup.restore().await
}

#[tauri::command]
pub async fn backup_save_settings(backup: Backup<'_>, config: BackupConfig) -> AppResult<()> {
    backup.save_settings(&config);
    Ok(())
}

#[tauri::command]
pub async fn backup_save_keys(
    backup: Backup<'_>,
    access_key: String,
    secret_key: String,
    provider: BackupProvider,
) -> AppResult<()> {
    backup.save_keys(&access_key, &secret_key, provider)
}

#[tauri::command]
pub async fn backup_remove_keys(backup: Backup<'_>, provider: BackupProvider) -> AppResult<()> {
    backup.remove_keys(provider);
    Ok(())
}

#[tauri::command]
pub async fn backup_key_hint(backup: Backup<'_>, provider: BackupProvider) -> AppResult<Option<String>> {
    Ok(backup.key_hint(provider))
}

#[tauri::command]
pub async fn backup_test_connection(backup: Backup<'_>, config: BackupConfig) -> AppResult<usize> {
    backup.test_connection(&config).await
}

/// Opens the Google sign-in page in the browser, and waits until the user is done there.
#[tauri::command]
pub async fn backup_google_sign_in(app: AppHandle, backup: Backup<'_>, config: BackupConfig) -> AppResult<()> {
    let sign_in = backup.begin_google_sign_in(&config).await?;
    let opened = app.opener().open_url(&sign_in.url, None::<&str>);
    opened.map_err(|error| AppError::Io(std::io::Error::other(error.to_string())))?;
    backup.finish_google_sign_in(&sign_in, &config).await
}
