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
use std::time::Duration;

use tauri::{App, AppHandle, Emitter, Manager, RunEvent, Window, WindowEvent};

use crate::app_menu::AppMenu;
use crate::commands::{backup_commands, drive_commands, settings_commands, update_commands};
use crate::config::app_config::AppConfig;
use crate::constants::AppConstants;
use crate::controllers::backup_controller::BackupController;
use crate::controllers::drive_controller::DriveController;
use crate::controllers::settings_controller::SettingsController;
use crate::controllers::update_controller::UpdateController;
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
            drive_service.clone(),
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

        let drive = Arc::new(DriveController::new(
            drive_service,
            backup_service.clone(),
            settings.clone(),
        ));
        app.manage(drive.clone());
        tauri::async_runtime::spawn(Self::keep_trash_clean(drive));
        let backup = Arc::new(BackupController::new(backup_service, settings.clone(), observer));
        app.manage(backup.clone());
        tauri::async_runtime::spawn(Self::keep_in_sync(backup));
        app.manage(Arc::new(SettingsController::new(settings, default_root)));
        app.manage(Arc::new(UpdateController::new(app.handle().clone())));
        AppMenu::install(app)?;
        TrayMenu::install(app)?;
        Ok(())
    }

    /// Deletes old items from the trash when the app starts, and then again and again while it runs.
    /// The app can run for days, because the close button only hides the window.
    async fn keep_trash_clean(drive: Arc<DriveController>) {
        let mut timer = tokio::time::interval(Duration::from_secs(AppConstants::TRASH_CHECK_SECONDS));
        loop {
            // The first tick comes at once.
            timer.tick().await;
            drive.empty_old_trash();
        }
    }

    /// Syncs with the backup again and again, so changes made on another computer come here too.
    /// The window starts the first sync itself.
    async fn keep_in_sync(backup: Arc<BackupController>) {
        loop {
            tokio::time::sleep(Duration::from_secs(AppConstants::SYNC_CHECK_SECONDS)).await;
            backup.auto_back_up();
        }
    }

    /// The close button only hides the main window. Drive keeps running, and the window comes back as it was.
    /// "Quit Drive" in the menu or in the tray is what stops the app.
    fn keep_alive(window: &Window, event: &WindowEvent) {
        if window.label() != AppConstants::MAIN_WINDOW {
            return;
        }
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
    }

    /// A click on the Dock icon (Mac) shows the hidden window again.
    // ponytail: Windows and Linux bring the window back from the tray ("Open Drive").
    // Add tauri-plugin-single-instance if starting Drive a second time should do it too.
    // Tauri gives the event by value, so the shape of this function is fixed.
    #[allow(clippy::needless_pass_by_value)]
    fn reopen(app: &AppHandle, event: RunEvent) {
        #[cfg(target_os = "macos")]
        if let RunEvent::Reopen { .. } = event {
            AppMenu::show_window(app);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(AppContainer::install)
        .on_window_event(AppContainer::keep_alive)
        .invoke_handler(tauri::generate_handler![
            drive_commands::drive_list,
            drive_commands::drive_columns,
            drive_commands::drive_overview,
            drive_commands::drive_folder_chain,
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
            update_commands::update_check,
            update_commands::update_install,
        ])
        .build(tauri::generate_context!())
        .expect("Drive could not start")
        .run(AppContainer::reopen);
}
