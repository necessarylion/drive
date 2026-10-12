use serde::{Deserialize, Serialize};

use crate::constants::AppConstants;
use crate::models::backup::BackupProvider;
use crate::models::error::{AppError, AppResult};
use crate::repositories::settings_repository::SettingsRepository;

/// The backup settings. The user edits them in the Settings dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupConfig {
    pub provider: BackupProvider,
    /// The bucket. For Azure, this is the container. For Google Drive, this is the name of the backup folder.
    pub bucket: String,
    /// The Azure storage account. Other providers do not use it.
    pub account: String,
    /// The OAuth client ID of the Google Cloud project. Only Google Drive uses it.
    pub client_id: String,
    /// The client secret of a "Desktop app" client. Google says this one is not a real secret,
    /// because every copy of a desktop app has it. So it is kept with the settings.
    pub client_secret: String,
    pub region: String,
    pub prefix: String,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            provider: BackupProvider::S3,
            bucket: String::new(),
            account: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            region: AppConstants::AWS_DEFAULT_REGION.to_owned(),
            prefix: String::new(),
        }
    }
}

impl BackupConfig {
    /// The settings the backup uses now.
    pub fn saved(settings: &SettingsRepository) -> Self {
        let raw = settings.get(AppConstants::BACKUP_PROVIDER_KEY).unwrap_or_default();
        Self::saved_for(BackupProvider::from_raw(&raw).unwrap_or_default(), settings)
    }

    /// The settings of one provider. Each provider remembers its own bucket and folder.
    pub fn saved_for(provider: BackupProvider, settings: &SettingsRepository) -> Self {
        let mut config = Self {
            provider,
            ..Self::default()
        };
        config.account = Self::clean(settings, AppConstants::AZURE_ACCOUNT_KEY).unwrap_or(config.account);
        config.client_id = Self::clean(settings, AppConstants::GOOGLE_CLIENT_ID_KEY).unwrap_or(config.client_id);
        config.client_secret =
            Self::clean(settings, AppConstants::GOOGLE_CLIENT_SECRET_KEY).unwrap_or(config.client_secret);
        config.bucket = Self::clean(settings, &AppConstants::bucket_key(provider)).unwrap_or(config.bucket);
        config.region = Self::clean(settings, AppConstants::AWS_REGION_KEY).unwrap_or(config.region);
        config.prefix = Self::clean(settings, &AppConstants::prefix_key(provider)).unwrap_or(config.prefix);
        config
    }

    /// Makes these settings the ones the backup uses.
    pub fn save(&self, settings: &SettingsRepository) {
        settings.set(AppConstants::BACKUP_PROVIDER_KEY, self.provider.raw());
        settings.set(&AppConstants::bucket_key(self.provider), &self.bucket);
        settings.set(&AppConstants::prefix_key(self.provider), &self.prefix);
        // Write only what this provider uses, or saving one provider resets a field of another one.
        match self.provider {
            BackupProvider::S3 => settings.set(AppConstants::AWS_REGION_KEY, &self.region),
            BackupProvider::Azure => settings.set(AppConstants::AZURE_ACCOUNT_KEY, &self.account),
            BackupProvider::GoogleDrive => {
                settings.set(AppConstants::GOOGLE_CLIENT_ID_KEY, &self.client_id);
                settings.set(AppConstants::GOOGLE_CLIENT_SECRET_KEY, &self.client_secret);
            }
            BackupProvider::Gcs => {}
        }
    }

    pub fn host(&self) -> String {
        match self.provider {
            BackupProvider::S3 => format!("{}.s3.{}.amazonaws.com", self.bucket, self.region),
            // Google Cloud Storage answers S3 requests at this address.
            BackupProvider::Gcs => format!("{}.storage.googleapis.com", self.bucket),
            BackupProvider::Azure => format!("{}.blob.core.windows.net", self.account),
            BackupProvider::GoogleDrive => "www.googleapis.com".to_owned(),
        }
    }

    /// The region that goes into the request signature. Google has no regions here and takes "auto".
    pub fn signing_region(&self) -> &str {
        if self.provider == BackupProvider::Gcs {
            "auto"
        } else {
            &self.region
        }
    }

    /// Where the backup goes, like `s3://bucket/folder/`.
    pub fn destination(&self) -> String {
        let prefix = self.key_prefix();
        match self.provider {
            BackupProvider::S3 => format!("s3://{}/{prefix}", self.bucket),
            BackupProvider::Gcs => format!("gs://{}/{prefix}", self.bucket),
            BackupProvider::Azure => format!("https://{}/{}/{prefix}", self.host(), self.bucket),
            BackupProvider::GoogleDrive => format!("Google Drive/{}/{prefix}", self.bucket),
        }
    }

    /// "" for the top of the bucket, or "folder/" for a folder in it.
    pub fn key_prefix(&self) -> String {
        let trimmed = self.prefix.trim_matches(['/', ' ']);
        if trimmed.is_empty() {
            String::new()
        } else {
            format!("{trimmed}/")
        }
    }

    /// The bucket, region and account become part of the server address, so only safe letters are allowed.
    pub fn validate(&self) -> AppResult<()> {
        if self.bucket.is_empty() {
            return Err(AppError::NotConfigured);
        }
        if self.provider == BackupProvider::GoogleDrive {
            // The folder name is never part of an address, so any name is fine. The client ID is.
            let name = self.client_id.strip_suffix(".apps.googleusercontent.com").unwrap_or("");
            if !Self::is_safe(name, true) {
                return Err(Self::bad(
                    "The client ID looks wrong. It ends with “.apps.googleusercontent.com”.",
                ));
            }
            return Ok(());
        }
        // ponytail: bucket names with dots or “_” are refused. They need another address style. Add it if you have one.
        let length_fits = (3..=63).contains(&self.bucket.len());
        if !length_fits
            || !Self::is_safe(&self.bucket, true)
            || self.bucket.starts_with('-')
            || self.bucket.ends_with('-')
        {
            return Err(Self::bad(
                "The bucket name can only have small letters, numbers and “-”.",
            ));
        }
        let account_fits = (3..=24).contains(&self.account.len()) && Self::is_safe(&self.account, false);
        if self.provider == BackupProvider::Azure && !account_fits {
            return Err(Self::bad(
                "The storage account name can only have small letters and numbers.",
            ));
        }
        if self.provider == BackupProvider::S3 && !Self::is_safe(&self.region, true) {
            return Err(Self::bad("The region looks wrong. An example is “ap-southeast-1”."));
        }
        Ok(())
    }

    /// True when the text is not empty and has only small letters, numbers and, if allowed, "-".
    fn is_safe(text: &str, allow_dash: bool) -> bool {
        if text.is_empty() {
            return false;
        }
        for byte in text.bytes() {
            let plain = byte.is_ascii_lowercase() || byte.is_ascii_digit() || (allow_dash && byte == b'-');
            if !plain {
                return false;
            }
        }
        true
    }

    fn bad(message: &str) -> AppError {
        AppError::BadSettings(message.to_owned())
    }

    fn clean(settings: &SettingsRepository, key: &str) -> Option<String> {
        let value = settings.get(key)?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    }
}
