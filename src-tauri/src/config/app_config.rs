use std::path::{Path, PathBuf};

use crate::constants::AppConstants;
use crate::repositories::settings_repository::SettingsRepository;

/// Where the app keeps its data on disk.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub storage_root: PathBuf,
}

impl AppConfig {
    pub fn new(storage_root: PathBuf) -> Self {
        Self { storage_root }
    }

    /// The folder picked in Settings, or the default one.
    /// Set `DRIVE_STORAGE_ROOT` to try the app with another folder, away from the real files.
    // ponytail: if the picked folder is missing (a disk that is not plugged in), the drive shows empty
    // and actions fail with an error. Add a check at start if this becomes a real case.
    pub fn live(default_root: &Path, settings: &SettingsRepository) -> Self {
        if let Ok(custom) = std::env::var(AppConstants::STORAGE_ROOT_ENV) {
            return Self::new(PathBuf::from(custom));
        }
        match settings.get(AppConstants::STORAGE_ROOT_KEY) {
            Some(saved) if !saved.is_empty() => Self::new(PathBuf::from(saved)),
            _ => Self::new(default_root.to_path_buf()),
        }
    }

    pub fn files_root(&self) -> PathBuf {
        self.storage_root.join("Files")
    }

    pub fn trash_root(&self) -> PathBuf {
        self.storage_root.join("Trash")
    }

    pub fn metadata_file(&self) -> PathBuf {
        self.storage_root.join("metadata.json")
    }

    pub fn backup_state_file(&self) -> PathBuf {
        self.storage_root.join("backup-state.json")
    }
}
