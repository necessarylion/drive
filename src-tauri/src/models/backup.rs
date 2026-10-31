use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::backup_config::BackupConfig;

/// Where the backup goes. The raw value is also the name of the keys in the keychain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BackupProvider {
    // "aws" is the name the keys had before other providers came. Keep it, or saved keys are lost.
    #[default]
    #[serde(rename = "aws")]
    S3,
    #[serde(rename = "gcs")]
    Gcs,
    #[serde(rename = "azure")]
    Azure,
    #[serde(rename = "gdrive")]
    GoogleDrive,
}

impl BackupProvider {
    pub const ALL: [Self; 4] = [Self::S3, Self::Gcs, Self::Azure, Self::GoogleDrive];

    pub fn raw(self) -> &'static str {
        match self {
            Self::S3 => "aws",
            Self::Gcs => "gcs",
            Self::Azure => "azure",
            Self::GoogleDrive => "gdrive",
        }
    }

    pub fn from_raw(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|provider| provider.raw() == raw)
    }
}

/// For Azure, `access_key` is empty and `secret_key` holds the SAS token.
/// For Google Drive, `access_key` is empty and `secret_key` holds the refresh token.
// No Debug on purpose, so a secret can never get into a log line.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub access_key: String,
    pub secret_key: String,
    #[serde(default)]
    pub session_token: Option<String>,
}

impl Credentials {
    pub fn new(access_key: &str, secret_key: &str) -> Self {
        Self {
            access_key: access_key.to_owned(),
            secret_key: secret_key.to_owned(),
            session_token: None,
        }
    }
}

/// Everything one backup request needs to know.
#[derive(Clone)]
pub struct BackupConnection {
    pub config: BackupConfig,
    pub credentials: Credentials,
}

/// One file in the backup. The date is milliseconds since 1970.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteObject {
    pub key: String,
    pub size: u64,
    pub last_modified: i64,
}

impl RemoteObject {
    /// Used when the server sent no date we can read. Then the local file always counts as newer.
    pub const UNKNOWN_DATE: i64 = i64::MIN;
}

/// One file of a plan: where it is on this computer, and where it is in the backup.
#[derive(Clone)]
pub struct BackupFile {
    pub file: PathBuf,
    pub key: String,
    /// The change date when the plan was made: of the local file for an upload or a local delete,
    /// of the backup copy for a download.
    pub modified: i64,
}

/// What one sync will do. `BackupService::plan` makes it and `BackupService::run` does it.
#[derive(Clone)]
pub struct BackupPlan {
    pub connection: BackupConnection,
    /// Files that are new or changed on this computer.
    pub uploads: Vec<BackupFile>,
    /// Files that are new or changed in the backup.
    pub downloads: Vec<BackupFile>,
    /// Files that were deleted or renamed on this computer. They are removed from the backup.
    pub remote_deletes: Vec<BackupFile>,
    /// Files that were removed from the backup. They go to the trash on this computer.
    pub local_deletes: Vec<BackupFile>,
}

/// What is known to be in the backup. It is saved to a file, so the backup icons are right when the app starts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupState {
    /// Where the backup goes, like `s3://bucket/folder/`. The dates are only true for this place.
    pub destination: String,
    /// File path -> the change date that is known to be in the backup.
    /// A path here was in both places after the last sync. So a path that is missing on one side now
    /// was deleted there, and is not new on the other side.
    pub synced_dates: BTreeMap<String, i64>,
    /// Backup key -> the date the backup gave that file at the last sync. A later date means
    /// it was changed in the backup. Missing right after an upload, until the next sync reads it.
    pub remote_dates: BTreeMap<String, i64>,
}

/// The current step of a backup, for the window to show.
#[derive(Debug, Clone, Default)]
pub struct BackupProgress {
    pub text: String,
    pub uploads_done: usize,
    pub uploads_total: usize,
    /// The files that are on their way now. Some files go up at the same time.
    pub uploading_paths: BTreeSet<String>,
}

impl BackupProgress {
    #[allow(clippy::cast_precision_loss)]
    pub fn fraction(&self) -> f64 {
        if self.uploads_total == 0 {
            0.0
        } else {
            self.uploads_done as f64 / self.uploads_total as f64
        }
    }
}

/// The state of the backup, the way the window shows it.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub is_configured: bool,
    pub is_running: bool,
    pub is_paused: bool,
    pub is_uploading: bool,
    pub fraction: f64,
    pub summary: String,
    /// `s3://bucket/folder/`
    pub destination: String,
}

/// What the Settings dialog shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    /// The folder the user picked. Empty when the default folder is used.
    pub storage_root: String,
    pub default_storage_root: PathBuf,
    pub config: BackupConfig,
}
