use std::sync::Arc;

use tauri::State;

use crate::controllers::update_controller::UpdateController;
use crate::models::error::AppResult;

type Updates<'a> = State<'a, Arc<UpdateController>>;

/// The version of a newer release, or nothing when the app is up to date.
#[tauri::command]
pub async fn update_check(updates: Updates<'_>) -> AppResult<Option<String>> {
    updates.check().await
}

/// Installs the newer release and starts the app again.
#[tauri::command]
pub async fn update_install(updates: Updates<'_>) -> AppResult<()> {
    updates.install().await
}
