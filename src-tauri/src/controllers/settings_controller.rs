use std::path::PathBuf;
use std::sync::Arc;

use crate::config::backup_config::BackupConfig;
use crate::constants::AppConstants;
use crate::models::backup::{BackupProvider, SettingsView};
use crate::repositories::settings_repository::SettingsRepository;

/// Reads and saves what the Settings dialog shows, but for the backup. That part is in `BackupController`.
pub struct SettingsController {
    settings: Arc<SettingsRepository>,
    default_storage_root: PathBuf,
}

impl SettingsController {
    pub fn new(settings: Arc<SettingsRepository>, default_storage_root: PathBuf) -> Self {
        Self {
            settings,
            default_storage_root,
        }
    }

    pub fn view(&self) -> SettingsView {
        SettingsView {
            storage_root: self.settings.get(AppConstants::STORAGE_ROOT_KEY).unwrap_or_default(),
            default_storage_root: self.default_storage_root.clone(),
            config: BackupConfig::saved(&self.settings),
        }
    }

    /// The saved settings of one provider. Each provider has its own bucket and folder.
    pub fn config_for(&self, provider: BackupProvider) -> BackupConfig {
        BackupConfig::saved_for(provider, &self.settings)
    }

    /// Sets where the files are kept. An empty path means the default folder.
    /// The change takes effect when the app starts the next time.
    pub fn set_storage_root(&self, path: &str) {
        self.settings.set(AppConstants::STORAGE_ROOT_KEY, path);
    }
}
