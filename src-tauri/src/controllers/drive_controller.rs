use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;

use crate::config::backup_config::BackupConfig;
use crate::constants::AppConstants;
use crate::models::drive::{DriveItem, DriveSection, FolderColor, FolderRef, ItemSorter, ItemView, Overview};
use crate::models::error::AppResult;
use crate::repositories::settings_repository::SettingsRepository;
use crate::services::backup_service::BackupService;
use crate::services::drive_service::DriveService;

/// Turns the requests of the window into service calls, and builds what the window shows.
pub struct DriveController {
    service: Arc<DriveService>,
    backup: Arc<BackupService>,
    settings: Arc<SettingsRepository>,
}

impl DriveController {
    pub fn new(service: Arc<DriveService>, backup: Arc<BackupService>, settings: Arc<SettingsRepository>) -> Self {
        Self {
            service,
            backup,
            settings,
        }
    }

    // MARK: - Reading

    pub fn list(&self, section: DriveSection, folder: Option<&Path>, query: &str, sorter: ItemSorter) -> Vec<ItemView> {
        let folder = folder.unwrap_or(self.service.root_folder());
        self.views(self.service.items(section, folder, query, sorter), section)
    }

    /// One list per open folder, from the root down. For the column view.
    pub fn columns(&self, path: &[PathBuf]) -> Vec<Vec<ItemView>> {
        let mut columns = vec![self.views(self.service.children(self.service.root_folder()), DriveSection::MyDrive)];
        for folder in path {
            columns.push(self.views(self.service.children(folder), DriveSection::MyDrive));
        }
        columns
    }

    pub fn overview(&self) -> Overview {
        let root = self.service.root_folder();
        let mut recent = self.service.items(DriveSection::Recent, root, "", ItemSorter::BY_NAME);
        recent.truncate(AppConstants::MENU_RECENT_LIMIT);
        Overview {
            root: root.to_path_buf(),
            pinned: self.service.pinned_folders(),
            slices: self.service.storage_slices(),
            used_bytes: self.service.used_bytes(),
            recent: self.views(recent, DriveSection::Recent),
        }
    }

    pub fn folder_chain(&self, folder: &Path) -> Vec<FolderRef> {
        self.service.folder_chain(folder)
    }

    pub fn root_folder(&self) -> &Path {
        self.service.root_folder()
    }

    pub fn is_in_storage(&self, path: &Path) -> bool {
        self.service.is_in_storage(path)
    }

    // MARK: - Changing

    pub fn create_folder(&self, name: &str, folder: &Path, color: FolderColor) -> AppResult<PathBuf> {
        self.service.create_folder(name, folder, color)
    }

    pub fn create_document(&self, name: &str, folder: &Path) -> AppResult<PathBuf> {
        self.service.create_document(name, folder)
    }

    pub async fn add(&self, sources: &[PathBuf], folder: &Path) -> AppResult<()> {
        self.service.add(sources, folder).await
    }

    pub async fn duplicate(&self, paths: &[PathBuf]) -> AppResult<()> {
        self.service.duplicate(paths).await
    }

    pub async fn export(&self, paths: &[PathBuf], folder: &Path) -> AppResult<()> {
        self.service.export(paths, folder).await
    }

    pub fn rename(&self, path: &Path, name: &str) -> AppResult<()> {
        self.service.rename(path, name)
    }

    pub fn set_starred(&self, paths: &[PathBuf], starred: bool) {
        self.service.set_starred(paths, starred);
    }

    pub fn set_pinned(&self, paths: &[PathBuf], pinned: bool) {
        self.service.set_pinned(paths, pinned);
    }

    pub fn set_color(&self, color: FolderColor, paths: &[PathBuf]) {
        self.service.set_color(color, paths);
    }

    pub fn trash(&self, paths: &[PathBuf]) -> AppResult<()> {
        self.service.trash(paths)
    }

    pub fn restore(&self, paths: &[PathBuf]) -> AppResult<()> {
        self.service.restore(paths)
    }

    pub fn delete_forever(&self, paths: &[PathBuf]) -> AppResult<()> {
        self.service.delete_forever(paths)
    }

    /// Deletes what has been in the trash for too long.
    pub fn empty_old_trash(&self) -> usize {
        self.service.empty_old_trash(Utc::now().timestamp_millis())
    }

    // MARK: - Helpers

    fn views(&self, items: Vec<DriveItem>, section: DriveSection) -> Vec<ItemView> {
        let is_configured = BackupConfig::saved(&self.settings).is_on();
        let mut views = Vec::with_capacity(items.len());
        for item in items {
            views.push(self.view(item, section, is_configured));
        }
        views
    }

    fn view(&self, item: DriveItem, section: DriveSection, is_configured: bool) -> ItemView {
        let is_folder = item.is_folder;
        let in_trash = section == DriveSection::Trash;
        // No backup icon for folders, for the trash, and when the backup is off.
        let shows_sync = is_configured && !is_folder && !in_trash;
        ItemView {
            color: is_folder.then(|| self.service.color(&item.path)),
            summary: is_folder.then(|| self.service.summary(&item.path)),
            is_pinned: is_folder && self.service.is_pinned(&item.path),
            sync: shows_sync.then(|| self.backup.state_of(&item.path, item.modified)),
            parents: if in_trash { Vec::new() } else { self.parents(&item.path) },
            item,
        }
    }

    /// The names of the folders from the top of the drive down to the item.
    fn parents(&self, path: &Path) -> Vec<String> {
        let mut names = Vec::new();
        let folder = path.parent().unwrap_or(path);
        for folder in self.service.folder_chain(folder) {
            names.push(folder.name);
        }
        names
    }
}
