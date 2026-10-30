//! All global keys and limits of the app.

use crate::models::backup::BackupProvider;

pub struct AppConstants;

impl AppConstants {
    pub const RECENT_LIMIT: usize = 50;
    pub const MENU_RECENT_LIMIT: usize = 5;
    /// How long the app waits for the user to finish the Google sign-in in the browser.
    pub const GOOGLE_SIGN_IN_SECONDS: u64 = 300;
    /// An item stays in the trash this long. Then Drive deletes it forever. The window shows the same number.
    pub const TRASH_KEEP_DAYS: i64 = 30;
    pub const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1000;
    /// How often the app looks for old items in the trash while it runs.
    pub const TRASH_CHECK_SECONDS: u64 = 60 * 60;
    /// How often the app syncs with the backup while it runs, to get the changes made somewhere else.
    pub const SYNC_CHECK_SECONDS: u64 = 5 * 60;

    /// A big file goes to the backup in parts of this size. Google wants a multiple of 256 KB,
    /// and S3 wants 5 MB or more.
    pub const UPLOAD_PART_BYTES: u64 = 8 * 1024 * 1024;
    /// How long one part may take. A slower network than about 30 KB per second needs a bigger number.
    pub const UPLOAD_PART_SECONDS: u64 = 300;
    /// How many times in a row a part may fail before the upload gives up.
    pub const UPLOAD_PART_TRIES: u64 = 5;

    /// S3 takes no more parts than this for one file.
    pub const UPLOAD_MAX_PARTS: u64 = 10_000;

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

    // Each provider has its own bucket and folder.
    pub fn bucket_key(provider: BackupProvider) -> String {
        format!("{}.bucket", provider.raw())
    }

    pub fn prefix_key(provider: BackupProvider) -> String {
        format!("{}.prefix", provider.raw())
    }
}
