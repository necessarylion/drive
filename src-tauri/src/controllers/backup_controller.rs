use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::Notify;

use crate::config::backup_config::BackupConfig;
use crate::constants::AppConstants;
use crate::models::backup::{BackupPlan, BackupProvider, BackupStatus};
use crate::models::error::{AppError, AppResult};
use crate::repositories::google_auth_repository::GoogleSignIn;
use crate::repositories::settings_repository::SettingsRepository;
use crate::services::backup_service::{BackupObserver, BackupService};

#[derive(Default)]
struct RunState {
    is_running: bool,
    last_result: String,
    /// A plan that waits for the user to say what happens with the extra files in the backup.
    pending_plan: Option<BackupPlan>,
    /// True when files changed while a backup was running.
    needs_another_run: bool,
}

/// Marks the backup as running. When it is dropped, also by a stop in the middle, the backup is free again.
struct RunGuard<'a> {
    controller: &'a BackupController,
}

impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        self.controller.run.lock().is_running = false;
        self.controller.observer.backup_changed();
    }
}

/// Runs backup and restore for the window, and keeps their state.
pub struct BackupController {
    service: Arc<BackupService>,
    settings: Arc<SettingsRepository>,
    observer: Arc<dyn BackupObserver>,
    run: Mutex<RunState>,
    /// Stops the work that runs now. Used by Pause.
    cancel: Notify,
}

impl BackupController {
    const UP_TO_DATE: &'static str = "All files are up to date";

    pub fn new(
        service: Arc<BackupService>,
        settings: Arc<SettingsRepository>,
        observer: Arc<dyn BackupObserver>,
    ) -> Self {
        service.follow(&BackupConfig::saved(&settings).destination());
        Self {
            service,
            settings,
            observer,
            run: Mutex::new(RunState::default()),
            cancel: Notify::new(),
        }
    }

    pub fn status(&self) -> BackupStatus {
        let config = self.config();
        let run = self.run.lock();
        let progress = self.service.progress();
        let is_configured = config.is_on();
        let is_paused = self.is_paused();
        let summary = if !is_configured {
            "Backup is off"
        } else if run.is_running && progress.text.is_empty() {
            "Working…"
        } else if run.is_running {
            &progress.text
        } else if is_paused {
            "Backup is paused"
        } else if run.last_result.is_empty() {
            "Waiting for the first backup"
        } else {
            &run.last_result
        };
        BackupStatus {
            is_configured,
            is_running: run.is_running,
            is_paused,
            is_uploading: run.is_running && progress.uploads_total > 0,
            fraction: progress.fraction(),
            summary: summary.to_owned(),
            destination: if is_configured {
                config.destination()
            } else {
                "Backup is off".to_owned()
            },
        }
    }

    /// Uploads new and changed files without asking. It never removes anything from the backup.
    // ponytail: each run lists the whole backup folder. Fine for a personal drive; keep a local index if it gets slow.
    pub fn auto_back_up(self: &Arc<Self>) {
        if !self.config().is_on() || self.is_paused() {
            return;
        }
        tauri::async_runtime::spawn(Arc::clone(self).run_auto());
    }

    pub fn toggle_pause(self: &Arc<Self>) {
        let paused = !self.is_paused();
        self.settings
            .set(AppConstants::AWS_PAUSED_KEY, if paused { "true" } else { "false" });
        if paused {
            self.cancel.notify_waiters();
        } else {
            self.auto_back_up();
        }
        self.observer.backup_changed();
    }

    /// The manual backup. Returns how many files are only in the backup. When that is not 0, nothing was
    /// uploaded yet: removing from the backup cannot be undone, so the user decides, and then
    /// `finish_backup` does the work.
    pub async fn back_up(&self) -> AppResult<usize> {
        let _guard = self.begin().ok_or(AppError::Busy)?;
        let planned = self.cancellable(self.service.plan(&self.config())).await;
        let Some(plan) = self.settle(planned)? else {
            return Ok(0);
        };
        if plan.extra_keys.is_empty() {
            self.finish(&plan, false).await?;
            return Ok(0);
        }
        let extras = plan.extra_keys.len();
        self.run.lock().pending_plan = Some(plan);
        Ok(extras)
    }

    /// Runs the plan that `back_up` made.
    pub async fn finish_backup(&self, remove_extras: bool) -> AppResult<()> {
        let _guard = self.begin().ok_or(AppError::Busy)?;
        let plan = self.run.lock().pending_plan.clone();
        match plan {
            Some(plan) => self.finish(&plan, remove_extras).await,
            None => Ok(()),
        }
    }

    /// Returns how many files were downloaded.
    pub async fn restore(&self) -> AppResult<usize> {
        let _guard = self.begin().ok_or(AppError::Busy)?;
        let restored = self.cancellable(self.service.restore(&self.config())).await;
        let count = self.settle(restored)?.unwrap_or(0);
        self.run.lock().last_result = format!("Restored {count} file(s)");
        Ok(count)
    }

    // MARK: - Settings

    /// Makes the settings from the Settings dialog the ones the backup uses. Until this is called,
    /// what the user types there changes nothing.
    pub fn save_settings(self: &Arc<Self>, config: &BackupConfig) {
        config.save(&self.settings);
        self.service.follow(&self.config().destination());
        if !config.is_on() {
            // The user turned the backup off. Stop the one that runs now.
            self.cancel.notify_waiters();
        }
        {
            let mut run = self.run.lock();
            // A plan that waits for an answer was made for the old place.
            run.pending_plan = None;
            run.last_result.clear();
        }
        self.observer.backup_changed();
        self.auto_back_up();
    }

    // MARK: - Access keys

    // The functions below take the settings that are on screen in the Settings dialog. They can be
    // not saved yet, so the user can set up and test a provider before the backup starts to use it.

    pub fn save_keys(self: &Arc<Self>, access_key: &str, secret_key: &str, provider: BackupProvider) -> AppResult<()> {
        self.service.save_keys(access_key, secret_key, provider)?;
        self.auto_back_up();
        Ok(())
    }

    pub fn remove_keys(&self, provider: BackupProvider) {
        self.service.remove_keys(provider);
    }

    /// The saved access key ID, mostly hidden. `None` when no keys are saved.
    pub fn key_hint(&self, provider: BackupProvider) -> Option<String> {
        self.service.saved_key_hint(provider)
    }

    /// Returns how many files are in the backup.
    pub async fn test_connection(&self, config: &BackupConfig) -> AppResult<usize> {
        self.service.test_connection(config).await
    }

    pub async fn begin_google_sign_in(&self, config: &BackupConfig) -> AppResult<GoogleSignIn> {
        self.service.begin_google_sign_in(config).await
    }

    pub async fn finish_google_sign_in(
        self: &Arc<Self>,
        sign_in: &GoogleSignIn,
        config: &BackupConfig,
    ) -> AppResult<()> {
        self.service.finish_google_sign_in(sign_in, config).await?;
        self.auto_back_up();
        Ok(())
    }

    // MARK: - Helpers

    async fn run_auto(self: Arc<Self>) {
        let Some(guard) = self.begin() else {
            self.run.lock().needs_another_run = true;
            return;
        };
        let result = self.cancellable(self.plan_and_run()).await;
        match result {
            Ok(()) => self.run.lock().last_result = Self::UP_TO_DATE.to_owned(),
            Err(AppError::Cancelled) => {}
            Err(_) => self.run.lock().last_result = "Backup failed. Choose Back Up Now to see why.".to_owned(),
        }
        drop(guard);
        let again = std::mem::take(&mut self.run.lock().needs_another_run);
        if again {
            self.auto_back_up();
        }
    }

    async fn plan_and_run(&self) -> AppResult<()> {
        let plan = self.service.plan(&self.config()).await?;
        self.service.run(&plan, false).await
    }

    async fn finish(&self, plan: &BackupPlan, remove_extras: bool) -> AppResult<()> {
        let ran = self.cancellable(self.service.run(plan, remove_extras)).await;
        if self.settle(ran)?.is_some() {
            let mut run = self.run.lock();
            run.pending_plan = None;
            run.last_result = Self::UP_TO_DATE.to_owned();
        }
        Ok(())
    }

    /// Marks the backup as running. `None` when it runs already.
    fn begin(&self) -> Option<RunGuard<'_>> {
        let mut run = self.run.lock();
        if run.is_running {
            return None;
        }
        run.is_running = true;
        drop(run);
        self.observer.backup_changed();
        Some(RunGuard { controller: self })
    }

    /// Runs the work until it is done or until the user pauses the backup.
    async fn cancellable<T>(&self, work: impl Future<Output = AppResult<T>>) -> AppResult<T> {
        tokio::select! {
            result = work => result,
            () = self.cancel.notified() => Err(AppError::Cancelled),
        }
    }

    /// Sorts out how a manual action ended. A stop by the user is not an error: it gives `None`.
    fn settle<T>(&self, result: AppResult<T>) -> AppResult<Option<T>> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(AppError::Cancelled) => Ok(None),
            Err(error) => {
                self.run.lock().last_result = "Last backup action failed".to_owned();
                Err(error)
            }
        }
    }

    fn config(&self) -> BackupConfig {
        BackupConfig::saved(&self.settings)
    }

    fn is_paused(&self) -> bool {
        self.settings.get(AppConstants::AWS_PAUSED_KEY).as_deref() == Some("true")
    }
}
