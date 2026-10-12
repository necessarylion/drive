use std::fs;
use std::path::PathBuf;

use crate::models::backup::BackupState;
use crate::models::error::{AppError, AppResult};
use crate::repositories::atomic_file::AtomicFile;

/// Reads and writes the backup state JSON file.
#[derive(Debug)]
pub struct BackupStateRepository {
    file: PathBuf,
}

impl BackupStateRepository {
    pub fn new(file: PathBuf) -> Self {
        Self { file }
    }

    /// A file that cannot be read, also one from the Swift app, gives an empty state.
    /// Then the next check with the backup finds it all again.
    pub fn load(&self) -> BackupState {
        let Ok(text) = fs::read_to_string(&self.file) else {
            return BackupState::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self, state: &BackupState) -> AppResult<()> {
        let text = serde_json::to_string(state).map_err(|error| AppError::Io(error.into()))?;
        AtomicFile::write(&self.file, &text)
    }
}
