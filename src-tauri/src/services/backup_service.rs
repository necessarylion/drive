use std::collections::{BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use parking_lot::Mutex;

use crate::config::backup_config::BackupConfig;
use crate::models::backup::{
    BackupConnection, BackupFile, BackupPlan, BackupProgress, BackupProvider, BackupState, Credentials,
};
use crate::models::drive::{DriveMetadata, SyncState};
use crate::models::error::{AppError, AppResult};
use crate::repositories::backup_repository::BackupRepository;
use crate::repositories::backup_state_repository::BackupStateRepository;
use crate::repositories::file_repository::FileRepository;
use crate::repositories::google_auth_repository::{GoogleAuthRepository, GoogleSignIn};
use crate::repositories::secret_repository::SecretRepository;
use crate::services::drive_service::DriveService;

/// Is told when the backup made a step, so the window can show it.
pub trait BackupObserver: Send + Sync {
    fn backup_changed(&self);
}

/// The backup repositories, one for each kind of provider.
pub struct BackupRepositories {
    pub s3: Arc<dyn BackupRepository>,
    pub azure: Arc<dyn BackupRepository>,
    pub google: Arc<dyn BackupRepository>,
}

/// Clears the progress and saves the state when a backup step ends: when it is done,
/// when it fails, and when it is stopped in the middle.
struct StepGuard<'a> {
    service: &'a BackupService,
}

impl Drop for StepGuard<'_> {
    fn drop(&mut self) {
        *self.service.progress.lock() = BackupProgress::default();
        self.service.save_state();
        self.service.observer.backup_changed();
    }
}

/// Keeps the drive and the backup the same, in both directions.
pub struct BackupService {
    root: PathBuf,
    /// Moves files to the trash when they were removed from the backup.
    drive: Arc<DriveService>,
    files: Arc<FileRepository>,
    repositories: BackupRepositories,
    google_auth: Arc<GoogleAuthRepository>,
    secrets: Arc<dyn SecretRepository>,
    state_store: Arc<BackupStateRepository>,
    observer: Arc<dyn BackupObserver>,
    /// Which files are known to be in the backup, and in which backup.
    state: Mutex<BackupState>,
    progress: Mutex<BackupProgress>,
}

impl BackupService {
    pub fn new(
        drive: Arc<DriveService>,
        files: Arc<FileRepository>,
        repositories: BackupRepositories,
        google_auth: Arc<GoogleAuthRepository>,
        secrets: Arc<dyn SecretRepository>,
        state_store: Arc<BackupStateRepository>,
        observer: Arc<dyn BackupObserver>,
    ) -> Self {
        let state = Mutex::new(state_store.load());
        Self {
            root: drive.root_folder().to_path_buf(),
            drive,
            files,
            repositories,
            google_auth,
            secrets,
            state_store,
            observer,
            state,
            progress: Mutex::new(BackupProgress::default()),
        }
    }

    pub fn progress(&self) -> BackupProgress {
        self.progress.lock().clone()
    }

    // ponytail: this is what the last check with the backup found. A file that was removed from the backup
    // by hand shows as synced until the next check.
    pub fn state_of(&self, path: &Path, modified: i64) -> SyncState {
        let key = DriveMetadata::key(path);
        if self.progress.lock().uploading_paths.contains(&key) {
            return SyncState::Uploading;
        }
        match self.state.lock().synced_dates.get(&key) {
            Some(date) if modified <= *date => SyncState::Synced,
            _ => SyncState::Pending,
        }
    }

    /// Tells the service where the backup goes now. What it knew about another place is dropped,
    /// because a file that is in one backup is not in the other.
    pub fn follow(&self, destination: &str) {
        let mut state = self.state.lock();
        if state.destination == destination {
            return;
        }
        *state = BackupState {
            destination: destination.to_owned(),
            ..BackupState::default()
        };
        drop(state);
        self.save_state();
    }

    /// Compares this computer with the backup. Changes nothing.
    /// A file that is on one side only is new there, unless the last sync saw it on both sides:
    /// then it was deleted on the other side. When a file changed on both sides, this computer wins.
    // ponytail: a file changed in the backup between an upload and the next sync counts as not changed,
    // because its backup date is only read at that next sync. Keep the date from the upload reply to close it.
    pub async fn plan(&self, config: &BackupConfig) -> AppResult<BackupPlan> {
        let _guard = StepGuard { service: self };
        let connection = self.connect(config)?;
        self.follow(&config.destination());
        self.set_progress("Checking the backup…");
        let mut remote = HashMap::new();
        for object in self.repository(config).list(&connection).await? {
            remote.insert(object.key.clone(), object);
        }
        let listed_keys: BTreeSet<String> = remote.keys().cloned().collect();

        let mut plan = BackupPlan {
            connection,
            uploads: Vec::new(),
            downloads: Vec::new(),
            remote_deletes: Vec::new(),
            local_deletes: Vec::new(),
        };
        let prefix = config.key_prefix();
        let mut seen_paths = BTreeSet::new();
        let mut state = self.state.lock();
        let BackupState {
            synced_dates,
            remote_dates,
            ..
        } = &mut *state;

        for file in self.files.all_files(&self.root) {
            let path_key = DriveMetadata::key(&file.path);
            seen_paths.insert(path_key.clone());
            let step = BackupFile {
                key: format!("{prefix}{}", self.relative_key(&file.path)),
                file: file.path,
                modified: file.modified,
            };
            let base = synced_dates.get(&path_key).copied();
            let local_changed = base.is_none_or(|date| file.modified > date);
            let Some(object) = remote.remove(&step.key) else {
                if local_changed {
                    plan.uploads.push(step);
                } else {
                    plan.local_deletes.push(step);
                }
                continue;
            };
            let remote_changed = remote_dates
                .get(&step.key)
                .is_some_and(|date| object.last_modified > *date);
            let is_current = if base.is_none() {
                // First time both sides have the file. Same rule as "aws s3 sync": upload when the size
                // differs or the local file is newer.
                object.size == file.size && file.modified <= object.last_modified
            } else {
                !local_changed && !remote_changed
            };
            if is_current {
                synced_dates.insert(path_key, file.modified);
                remote_dates.insert(step.key, object.last_modified);
            } else if local_changed {
                plan.uploads.push(step);
            } else {
                plan.downloads.push(BackupFile {
                    modified: object.last_modified,
                    ..step
                });
            }
        }

        for object in remote.into_values() {
            // Keys that end with "/" are folder markers made by the provider's website. Leave them.
            if object.key.ends_with('/') {
                continue;
            }
            // A key is outside data. Refuse it when it could write outside the drive.
            let Some(file) = object
                .key
                .strip_prefix(&prefix)
                .and_then(|relative| self.local_path(relative))
            else {
                continue;
            };
            let path_key = DriveMetadata::key(&file);
            seen_paths.insert(path_key.clone());
            let remote_changed = remote_dates
                .get(&object.key)
                .is_some_and(|date| object.last_modified > *date);
            let step = BackupFile {
                file,
                key: object.key,
                modified: object.last_modified,
            };
            if synced_dates.contains_key(&path_key) && !remote_changed {
                plan.remote_deletes.push(step);
            } else {
                plan.downloads.push(step);
            }
        }
        // Forget the files that are gone from both sides.
        synced_dates.retain(|key, _| seen_paths.contains(key));
        remote_dates.retain(|key, _| listed_keys.contains(key));
        Ok(plan)
    }

    pub async fn run(&self, plan: &BackupPlan) -> AppResult<()> {
        let _guard = StepGuard { service: self };
        let remote = self.repository(&plan.connection.config);
        let total = plan.uploads.len();
        if total > 0 {
            {
                let mut progress = self.progress.lock();
                progress.text = format!("Uploading 1 of {total}");
                progress.uploads_total = total;
            }
            let next = AtomicUsize::new(0);
            // ponytail: four files at the same time, written out. Make it a list of workers
            // if the number must come from a setting.
            tokio::try_join!(
                self.upload_files(plan, &next),
                self.upload_files(plan, &next),
                self.upload_files(plan, &next),
                self.upload_files(plan, &next),
            )?;
        }
        for (index, step) in plan.downloads.iter().enumerate() {
            self.set_progress(&format!("Downloading {} of {}", index + 1, plan.downloads.len()));
            // The local file changed after the plan was made. It goes up at the next sync instead.
            if !self.is_unchanged(&step.file) {
                continue;
            }
            self.download(&remote, &step.key, &step.file, step.modified, &plan.connection)
                .await?;
            self.remember(plan, step, true);
        }
        for (index, step) in plan.remote_deletes.iter().enumerate() {
            self.set_progress(&format!("Removing {} of {}", index + 1, plan.remote_deletes.len()));
            remote.delete(&step.key, &plan.connection).await?;
            self.remember(plan, step, false);
        }
        for step in &plan.local_deletes {
            // Only a file that did not change since the last sync. The trash keeps it for 30 days.
            if self.is_unchanged(&step.file) {
                self.drive.trash(std::slice::from_ref(&step.file))?;
                self.remember(plan, step, false);
            }
        }
        Ok(())
    }

    /// Takes the next file of the plan that no worker took yet and uploads it, until no file is left.
    async fn upload_files(&self, plan: &BackupPlan, next: &AtomicUsize) -> AppResult<()> {
        let remote = self.repository(&plan.connection.config);
        let total = plan.uploads.len();
        while let Some(upload) = plan.uploads.get(next.fetch_add(1, Ordering::Relaxed)) {
            let path_key = DriveMetadata::key(&upload.file);
            self.progress.lock().uploading_paths.insert(path_key.clone());
            self.observer.backup_changed();
            remote.upload(&upload.file, &upload.key, &plan.connection).await?;
            {
                let mut progress = self.progress.lock();
                progress.uploading_paths.remove(&path_key);
                progress.uploads_done += 1;
                progress.text = format!("Uploading {} of {total}", (progress.uploads_done + 1).min(total));
            }
            // The settings can change while a backup runs. Then this upload went to the old place.
            let mut state = self.state.lock();
            if plan.connection.config.destination() == state.destination {
                state.synced_dates.insert(path_key, upload.modified);
                // The backup gave the file a new date. The next sync reads it.
                state.remote_dates.remove(&upload.key);
            }
        }
        Ok(())
    }

    /// Downloads the files that are in the backup but not on this computer. Never replaces a local file.
    /// Returns how many files it downloaded.
    pub async fn restore(&self, config: &BackupConfig) -> AppResult<usize> {
        let _guard = StepGuard { service: self };
        let connection = self.connect(config)?;
        self.follow(&config.destination());
        self.set_progress("Checking the backup…");
        let remote = self.repository(config);
        let prefix = config.key_prefix();
        let mut count = 0;
        for object in remote.list(&connection).await? {
            let relative = object.key.strip_prefix(&prefix).unwrap_or(&object.key);
            // A key is outside data. Refuse it when it could write outside the drive.
            let Some(destination) = self.local_path(relative) else {
                continue;
            };
            if self.files.exists(&destination) {
                continue;
            }

            count += 1;
            self.set_progress(&format!("Downloading file {count}"));
            self.download(&remote, &object.key, &destination, object.last_modified, &connection)
                .await?;
            let mut state = self.state.lock();
            state
                .synced_dates
                .insert(DriveMetadata::key(&destination), object.last_modified);
            state.remote_dates.insert(object.key, object.last_modified);
        }
        Ok(count)
    }

    // MARK: - Access keys

    /// The saved access key ID with most of it hidden, like "AKIA…3XYZ". `None` when no keys are saved.
    pub fn saved_key_hint(&self, provider: BackupProvider) -> Option<String> {
        let key = self.secrets.load(provider)?.access_key;
        if provider == BackupProvider::GoogleDrive {
            return Some("Signed in".to_owned());
        }
        // An Azure SAS token has no key ID to show.
        if key.is_empty() {
            return Some("SAS token saved".to_owned());
        }
        let letters: Vec<char> = key.chars().collect();
        let start: String = letters.iter().take(4).collect();
        let end: String = letters.iter().skip(letters.len().saturating_sub(4)).collect();
        Some(format!("{start}…{end}"))
    }

    /// For Azure, `secret_key` is the SAS token and `access_key` is not used.
    pub fn save_keys(&self, access_key: &str, secret_key: &str, provider: BackupProvider) -> AppResult<()> {
        let mut access = access_key.trim();
        let mut secret = secret_key.trim();
        if provider == BackupProvider::Azure {
            access = "";
            // The Azure website shows the token with a "?" in front on some pages.
            secret = secret.strip_prefix('?').unwrap_or(secret);
        } else if access.is_empty() {
            return Err(AppError::MissingKeys);
        }
        if secret.is_empty() {
            return Err(AppError::MissingKeys);
        }
        self.secrets.save(&Credentials::new(access, secret), provider)
    }

    /// Starts a Google sign-in. Open `url` of the result in the browser, then call `finish_google_sign_in`.
    pub async fn begin_google_sign_in(&self, config: &BackupConfig) -> AppResult<GoogleSignIn> {
        self.google_auth.begin(config).await
    }

    /// Waits for the user to finish in the browser, and saves the sign-in in the keychain.
    pub async fn finish_google_sign_in(&self, sign_in: &GoogleSignIn, config: &BackupConfig) -> AppResult<()> {
        let refresh_token = self.google_auth.finish(sign_in, config).await?;
        self.secrets
            .save(&Credentials::new("", &refresh_token), BackupProvider::GoogleDrive)
    }

    pub fn remove_keys(&self, provider: BackupProvider) {
        self.secrets.delete(provider);
    }

    /// Asks the provider for the file list. Returns how many files are there.
    /// Fails when the settings or keys are wrong.
    pub async fn test_connection(&self, config: &BackupConfig) -> AppResult<usize> {
        let connection = self.connect(config)?;
        Ok(self.repository(config).list(&connection).await?.len())
    }

    // MARK: - Helpers

    /// "Docs/a.txt" for a file in the folder Docs of the drive. Keys use "/" on every system.
    fn relative_key(&self, path: &Path) -> String {
        let relative = path.strip_prefix(&self.root).unwrap_or(path);
        let mut parts = Vec::new();
        for component in relative.components() {
            parts.push(component.as_os_str().to_string_lossy().into_owned());
        }
        parts.join("/")
    }

    /// Where the key belongs on this computer. `None` when a part of the key is not a plain name,
    /// like "", ".", ".." or a drive letter. So nothing can be written outside the drive.
    fn local_path(&self, relative: &str) -> Option<PathBuf> {
        let mut path = self.root.clone();
        for part in relative.split('/') {
            let mut components = Path::new(part).components();
            let is_plain_name = matches!(components.next(), Some(Component::Normal(name)) if name == part);
            if !is_plain_name || components.next().is_some() {
                return None;
            }
            path.push(part);
        }
        Some(path)
    }

    /// Downloads next to the real place, under a hidden name, and gives it the real name when it is all there.
    /// So a stop in the middle never leaves half a file. Replaces a local file with the same name.
    async fn download(
        &self,
        remote: &Arc<dyn BackupRepository>,
        key: &str,
        destination: &Path,
        modified: i64,
        connection: &BackupConnection,
    ) -> AppResult<()> {
        if let Some(folder) = destination.parent() {
            self.files.create_folder(folder)?;
        }
        let name = destination.file_name().unwrap_or_default().to_string_lossy();
        let draft = destination.with_file_name(format!(".{name}.part"));
        if let Err(error) = remote.download(key, &draft, connection).await {
            let _ = self.files.delete(&draft);
            return Err(error);
        }
        self.files.move_item(&draft, destination)?;
        // Keep the backup date, so the next sync does not upload the file again.
        self.files.set_modified(modified, destination)
    }

    /// True when the file is missing, or did not change since the last sync.
    fn is_unchanged(&self, path: &Path) -> bool {
        let Some(item) = self.files.item(path) else {
            return true;
        };
        let state = self.state.lock();
        state
            .synced_dates
            .get(&DriveMetadata::key(path))
            .is_some_and(|date| item.modified <= *date)
    }

    /// Writes down that the file is now the same in both places, or gone from both.
    /// Not when the settings changed while the sync ran: then the step went to the old place.
    fn remember(&self, plan: &BackupPlan, step: &BackupFile, is_in_both: bool) {
        let mut state = self.state.lock();
        if plan.connection.config.destination() != state.destination {
            return;
        }
        let path_key = DriveMetadata::key(&step.file);
        if is_in_both {
            state.synced_dates.insert(path_key, step.modified);
            state.remote_dates.insert(step.key.clone(), step.modified);
        } else {
            state.synced_dates.remove(&path_key);
            state.remote_dates.remove(&step.key);
        }
    }

    fn set_progress(&self, text: &str) {
        self.progress.lock().text = text.to_owned();
        self.observer.backup_changed();
    }

    fn save_state(&self) {
        // The file only makes the icons right sooner. If it cannot be saved, the next check finds it all again.
        let state = self.state.lock().clone();
        let _ = self.state_store.save(&state);
    }

    fn connect(&self, config: &BackupConfig) -> AppResult<BackupConnection> {
        config.validate()?;
        let credentials = self.secrets.load(config.provider).ok_or(AppError::NoCredentials)?;
        Ok(BackupConnection {
            config: config.clone(),
            credentials,
        })
    }

    fn repository(&self, config: &BackupConfig) -> Arc<dyn BackupRepository> {
        match config.provider {
            BackupProvider::S3 | BackupProvider::Gcs => self.repositories.s3.clone(),
            BackupProvider::Azure => self.repositories.azure.clone(),
            BackupProvider::GoogleDrive => self.repositories.google.clone(),
        }
    }
}
