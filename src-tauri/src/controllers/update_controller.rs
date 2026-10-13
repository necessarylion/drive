use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::models::error::{AppError, AppResult};

/// Looks for a newer version of the app, and installs it.
/// Where to look and the key that checks the download are in `tauri.conf.json`, under `plugins.updater`.
pub struct UpdateController {
    app: AppHandle,
}

impl UpdateController {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// The version of the update, or `None` when the app is up to date.
    pub async fn check(&self) -> AppResult<Option<String>> {
        Ok(self.newer().await?.map(|update| update.version))
    }

    /// Downloads the update, installs it and starts the app again.
    // ponytail: no progress is shown while it downloads; the window only shows its spinner.
    // Send the progress to the window if the download gets big.
    pub async fn install(&self) -> AppResult<()> {
        let Some(update) = self.newer().await? else {
            return Ok(());
        };
        let installed = update.download_and_install(|_, _| {}, || {}).await;
        installed.map_err(Self::failed)?;
        self.app.restart()
    }

    async fn newer(&self) -> AppResult<Option<Update>> {
        // A build made for development is never replaced by a release.
        if cfg!(debug_assertions) {
            return Ok(None);
        }
        let updater = self.app.updater().map_err(Self::failed)?;
        updater.check().await.map_err(Self::failed)
    }

    // The shape is fixed by `map_err`, which gives the error by value.
    #[allow(clippy::needless_pass_by_value)]
    fn failed(error: tauri_plugin_updater::Error) -> AppError {
        AppError::Network(error.to_string())
    }
}
