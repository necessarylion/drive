//! All global keys and limits of the app.

use crate::models::backup::BackupProvider;

pub struct AppConstants;

impl AppConstants {
    pub const RECENT_LIMIT: usize = 50;
    pub const MENU_RECENT_LIMIT: usize = 5;
    /// How much of a text file the preview reads.
    pub const TEXT_PREVIEW_BYTES: u64 = 256 * 1024;
    /// How long the app waits for the user to finish the Google sign-in in the browser.
    pub const GOOGLE_SIGN_IN_SECONDS: u64 = 300;
    /// An item stays in the trash this long. Then Drive deletes it forever. The window shows the same number.
    pub const TRASH_KEEP_DAYS: i64 = 30;
    pub const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1000;
    /// How often the app looks for old items in the trash while it runs.
    pub const TRASH_CHECK_SECONDS: u64 = 60 * 60;

    pub const APP_FOLDER: &'static str = "Drive";
    pub const SETTINGS_FILE: &'static str = "settings.json";
    pub const STORAGE_ROOT_ENV: &'static str = "DRIVE_STORAGE_ROOT";

    pub const BACKUP_PROVIDER_KEY: &'static str = "backup.provider";
    /// "false" turns the backup off. Anything else, also no value, means on.
    pub const BACKUP_ENABLED_KEY: &'static str = "backup.enabled";
    pub const AZURE_ACCOUNT_KEY: &'static str = "azure.account";
    pub const GOOGLE_CLIENT_ID_KEY: &'static str = "google.clientID";
    pub const GOOGLE_CLIENT_SECRET_KEY: &'static str = "google.clientSecret";
    pub const AWS_REGION_KEY: &'static str = "aws.region";
    pub const AWS_PAUSED_KEY: &'static str = "aws.paused";
    pub const STORAGE_ROOT_KEY: &'static str = "storage.root";
    /// The same name the Swift app used, so keys saved by it on a Mac are still found.
    pub const KEYCHAIN_SERVICE: &'static str = "com.zinkyaw.drive.aws";
    pub const AWS_DEFAULT_REGION: &'static str = "us-east-1";

    pub const BACKUP_CHANGED_EVENT: &'static str = "backup-changed";
    /// Tells the window to show the Settings dialog.
    pub const OPEN_SETTINGS_EVENT: &'static str = "open-settings";
    pub const MAIN_WINDOW: &'static str = "main";
    /// The label of a viewer window starts with this. `capabilities/default.json` uses the same start.
    pub const VIEWER_WINDOW_PREFIX: &'static str = "viewer-";

    // Each provider has its own bucket and folder.
    pub fn bucket_key(provider: BackupProvider) -> String {
        format!("{}.bucket", provider.raw())
    }

    pub fn prefix_key(provider: BackupProvider) -> String {
        format!("{}.prefix", provider.raw())
    }
}
