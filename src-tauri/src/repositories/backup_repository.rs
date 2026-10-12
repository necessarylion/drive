use std::path::Path;

use async_trait::async_trait;

use crate::models::backup::{BackupConnection, RemoteObject};
use crate::models::error::AppResult;

/// One place that keeps the backup. S3, Azure and Google Drive each have one.
#[async_trait]
pub trait BackupRepository: Send + Sync {
    /// Every object under the configured folder.
    async fn list(&self, connection: &BackupConnection) -> AppResult<Vec<RemoteObject>>;
    async fn upload(&self, file: &Path, key: &str, connection: &BackupConnection) -> AppResult<()>;
    /// Downloads the object into the file at `destination`.
    async fn download(&self, key: &str, destination: &Path, connection: &BackupConnection) -> AppResult<()>;
    async fn delete(&self, key: &str, connection: &BackupConnection) -> AppResult<()>;
}
