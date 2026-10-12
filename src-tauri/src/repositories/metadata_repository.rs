use std::fs;
use std::path::PathBuf;

use crate::models::drive::DriveMetadata;
use crate::models::error::{AppError, AppResult};
use crate::repositories::atomic_file::AtomicFile;

/// Reads and writes the metadata JSON file.
#[derive(Debug)]
pub struct MetadataRepository {
    file: PathBuf,
}

impl MetadataRepository {
    pub fn new(file: PathBuf) -> Self {
        Self { file }
    }

    pub fn load(&self) -> DriveMetadata {
        let Ok(text) = fs::read_to_string(&self.file) else {
            return DriveMetadata::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self, metadata: &DriveMetadata) -> AppResult<()> {
        let text = serde_json::to_string(metadata).map_err(|error| AppError::Io(error.into()))?;
        AtomicFile::write(&self.file, &text)
    }
}
