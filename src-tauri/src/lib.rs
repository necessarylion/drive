//! Drive: manages the files in one folder and backs them up to the cloud.
//!
//! The code goes in one direction: command -> controller -> service -> repository -> model.

mod app_menu;
pub mod commands;
pub mod config;
pub mod constants;
pub mod controllers;
pub mod models;
pub mod repositories;
pub mod services;
mod tray;

use std::error::Error;
use std::sync::Arc;

use tauri::{App, AppHandle, Emitter, Manager};

use crate::app_menu::AppMenu;
use crate::commands::{backup_commands, drive_commands, settings_commands};
use crate::config::app_config::AppConfig;
use crate::constants::AppConstants;
use crate::controllers::backup_controller::BackupController;
use crate::controllers::drive_controller::DriveController;
use crate::controllers::settings_controller::SettingsController;
use crate::repositories::azure_repository::AzureRepository;
use crate::repositories::backup_state_repository::BackupStateRepository;
use crate::repositories::file_repository::FileRepository;
use crate::repositories::google_auth_repository::GoogleAuthRepository;
use crate::repositories::google_drive_repository::GoogleDriveRepository;
use crate::repositories::http_transport::HttpTransport;
use crate::repositories::metadata_repository::MetadataRepository;
use crate::repositories::s3_repository::S3Repository;
use crate::repositories::secret_repository::KeychainRepository;
use crate::repositories::settings_repository::SettingsRepository;
use crate::services::aws_signer::AwsSigner;
use crate::services::backup_service::{BackupObserver, BackupRepositories, BackupService};
use crate::services::drive_service::DriveService;
use crate::tray::TrayMenu;

/// Tells the window that the backup made a step. The window then asks for the new state.
struct WindowObserver {
    app: AppHandle,
}

impl BackupObserver for WindowObserver {
    fn backup_changed(&self) {
        let _ = self.app.emit(AppConstants::BACKUP_CHANGED_EVENT, ());
    }
}

/// The one place where the parts are built and wired together.
struct AppContainer;

impl AppContainer {
    fn install(app: &mut App) -> Result<(), Box<dyn Error>> {
        let default_root = app.path().data_dir()?.join(AppConstants::APP_FOLDER);
        let settings_file = app
            .path()
            .config_dir()?
            .join(AppConstants::APP_FOLDER)
            .join(AppConstants::SETTINGS_FILE);
        let settings = Arc::new(SettingsRepository::new(settings_file));
        let config = AppConfig::live(&default_root, &settings);

        let files = Arc::new(FileRepository::new());
        let signer = Arc::new(AwsSigner::new());
        let http = Arc::new(HttpTransport::new());
        let google_auth = Arc::new(GoogleAuthRepository::new(signer.clone(), http.clone()));
        let observer: Arc<dyn BackupObserver> = Arc::new(WindowObserver {
            app: app.handle().clone(),
        });

        let metadata_store = Arc::new(MetadataRepository::new(config.metadata_file()));
        let state_store = Arc::new(BackupStateRepository::new(config.backup_state_file()));
        let drive_service = Arc::new(DriveService::new(config.clone(), files.clone(), metadata_store));
        let repositories = BackupRepositories {
            s3: Arc::new(S3Repository::new(signer.clone(), http.clone())),
            azure: Arc::new(AzureRepository::new(signer.clone(), http.clone())),
            google: Arc::new(GoogleDriveRepository::new(google_auth.clone(), signer, http)),
        };
        let backup_service = Arc::new(BackupService::new(
            drive_service.root_folder().to_path_buf(),
            files.clone(),
            repositories,
            google_auth,
            Arc::new(KeychainRepository::new(AppConstants::KEYCHAIN_SERVICE)),
            state_store,
            observer.clone(),
        ));

        // The window may show files (pictures, PDFs) from the storage folder, and from nowhere else.
        let storage_root = files.prepared_folder(&config.storage_root);
        app.asset_protocol_scope().allow_directory(storage_root, true)?;

        app.manage(Arc::new(DriveController::new(
            drive_service,
            backup_service.clone(),
            settings.clone(),
        )));
        app.manage(Arc::new(BackupController::new(
            backup_service,
            settings.clone(),
            observer,
        )));
        app.manage(Arc::new(SettingsController::new(settings, default_root)));
        AppMenu::install(app)?;
        TrayMenu::install(app)?;
        Ok(())
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(AppContainer::install)
        .invoke_handler(tauri::generate_handler![
            drive_commands::drive_list,
            drive_commands::drive_columns,
            drive_commands::drive_overview,
            drive_commands::drive_folder_chain,
            drive_commands::drive_read_text,
            drive_commands::drive_create_folder,
            drive_commands::drive_create_document,
            drive_commands::drive_add,
            drive_commands::drive_duplicate,
            drive_commands::drive_export,
            drive_commands::drive_rename,
            drive_commands::drive_set_starred,
            drive_commands::drive_set_pinned,
            drive_commands::drive_set_color,
            drive_commands::drive_trash,
            drive_commands::drive_restore,
            drive_commands::drive_delete_forever,
            drive_commands::drive_open,
            drive_commands::drive_reveal,
            backup_commands::backup_status,
            backup_commands::backup_auto,
            backup_commands::backup_toggle_pause,
            backup_commands::backup_now,
            backup_commands::backup_finish,
            backup_commands::backup_restore,
            backup_commands::backup_save_settings,
            backup_commands::backup_save_keys,
            backup_commands::backup_remove_keys,
            backup_commands::backup_key_hint,
            backup_commands::backup_test_connection,
            backup_commands::backup_google_sign_in,
            settings_commands::settings_load,
            settings_commands::settings_config_for,
            settings_commands::settings_set_storage_root,
        ])
        .run(tauri::generate_context!())
        .expect("Drive could not start");
}
