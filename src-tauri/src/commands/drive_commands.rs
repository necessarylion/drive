use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::controllers::drive_controller::DriveController;
use crate::models::drive::{DriveSection, FolderColor, FolderRef, ItemSorter, ItemView, Overview, SortField};
use crate::models::error::{AppError, AppResult};

type Drive<'a> = State<'a, Arc<DriveController>>;

#[tauri::command]
pub async fn drive_list(
    drive: Drive<'_>,
    section: DriveSection,
    folder: Option<PathBuf>,
    query: String,
    sort_field: SortField,
    ascending: bool,
) -> AppResult<Vec<ItemView>> {
    let sorter = ItemSorter {
        field: sort_field,
        ascending,
    };
    Ok(drive.list(section, folder.as_deref(), &query, sorter))
}

#[tauri::command]
pub async fn drive_columns(drive: Drive<'_>, path: Vec<PathBuf>) -> AppResult<Vec<Vec<ItemView>>> {
    Ok(drive.columns(&path))
}

#[tauri::command]
pub async fn drive_overview(drive: Drive<'_>) -> AppResult<Overview> {
    Ok(drive.overview())
}

#[tauri::command]
pub async fn drive_folder_chain(drive: Drive<'_>, folder: PathBuf) -> AppResult<Vec<FolderRef>> {
    Ok(drive.folder_chain(&folder))
}

#[tauri::command]
pub async fn drive_create_folder(
    drive: Drive<'_>,
    name: String,
    folder: PathBuf,
    color: FolderColor,
) -> AppResult<PathBuf> {
    drive.create_folder(&name, &folder, color)
}

#[tauri::command]
pub async fn drive_create_document(drive: Drive<'_>, name: String, folder: PathBuf) -> AppResult<PathBuf> {
    drive.create_document(&name, &folder)
}

#[tauri::command]
pub async fn drive_add(drive: Drive<'_>, sources: Vec<PathBuf>, folder: PathBuf) -> AppResult<()> {
    drive.add(&sources, &folder).await
}

#[tauri::command]
pub async fn drive_duplicate(drive: Drive<'_>, paths: Vec<PathBuf>) -> AppResult<()> {
    drive.duplicate(&paths).await
}

#[tauri::command]
pub async fn drive_export(drive: Drive<'_>, paths: Vec<PathBuf>, folder: PathBuf) -> AppResult<()> {
    drive.export(&paths, &folder).await
}

#[tauri::command]
pub async fn drive_rename(drive: Drive<'_>, path: PathBuf, name: String) -> AppResult<()> {
    drive.rename(&path, &name)
}

#[tauri::command]
pub async fn drive_set_starred(drive: Drive<'_>, paths: Vec<PathBuf>, starred: bool) -> AppResult<()> {
    drive.set_starred(&paths, starred);
    Ok(())
}

#[tauri::command]
pub async fn drive_set_pinned(drive: Drive<'_>, paths: Vec<PathBuf>, pinned: bool) -> AppResult<()> {
    drive.set_pinned(&paths, pinned);
    Ok(())
}

#[tauri::command]
pub async fn drive_set_color(drive: Drive<'_>, paths: Vec<PathBuf>, color: FolderColor) -> AppResult<()> {
    drive.set_color(color, &paths);
    Ok(())
}

#[tauri::command]
pub async fn drive_trash(drive: Drive<'_>, paths: Vec<PathBuf>) -> AppResult<()> {
    drive.trash(&paths)
}

#[tauri::command]
pub async fn drive_restore(drive: Drive<'_>, paths: Vec<PathBuf>) -> AppResult<()> {
    drive.restore(&paths)
}

#[tauri::command]
pub async fn drive_delete_forever(drive: Drive<'_>, paths: Vec<PathBuf>) -> AppResult<()> {
    drive.delete_forever(&paths)
}

/// Opens a file in the app the system uses for it.
#[tauri::command]
pub async fn drive_open(app: AppHandle, drive: Drive<'_>, path: PathBuf) -> AppResult<()> {
    // The window sends the path. Only open what is ours.
    if !drive.is_in_storage(&path) {
        return Err(AppError::OutsideDrive);
    }
    let opened = app.opener().open_path(path.to_string_lossy(), None::<&str>);
    opened.map_err(|error| AppError::Io(std::io::Error::other(error.to_string())))
}

/// Shows an item in the file manager of the system. Without a path, it shows the drive folder.
#[tauri::command]
pub async fn drive_reveal(app: AppHandle, drive: Drive<'_>, path: Option<PathBuf>) -> AppResult<()> {
    let shown = match path {
        Some(path) if drive.is_in_storage(&path) => app.opener().reveal_item_in_dir(path),
        Some(_) => return Err(AppError::OutsideDrive),
        None => app
            .opener()
            .open_path(drive.root_folder().to_string_lossy(), None::<&str>),
    };
    shown.map_err(|error| AppError::Io(std::io::Error::other(error.to_string())))
}
