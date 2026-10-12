use std::fs;
use std::path::Path;

use crate::models::error::AppResult;

/// Writes a file in one step, so a crash in the middle never leaves half a file.
pub struct AtomicFile;

impl AtomicFile {
    pub fn write(file: &Path, text: &str) -> AppResult<()> {
        let draft = file.with_extension("tmp");
        fs::write(&draft, text)?;
        Ok(fs::rename(&draft, file)?)
    }
}
