use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;

use crate::config::app_config::AppConfig;
use crate::constants::AppConstants;
use crate::models::drive::{
    DriveItem, DriveMetadata, DriveSection, FolderColor, FolderRef, FolderSummary, ItemSorter, SortField, StorageGroup,
    StorageSlice,
};
use crate::models::error::{AppError, AppResult};
use crate::repositories::file_repository::FileRepository;
use crate::repositories::metadata_repository::MetadataRepository;

/// The rules of the drive: listing, adding, renaming, stars, pins and trash.
pub struct DriveService {
    config: AppConfig,
    files: Arc<FileRepository>,
    metadata_store: Arc<MetadataRepository>,
    metadata: Mutex<DriveMetadata>,
    files_root: PathBuf,
    trash_root: PathBuf,
}

impl DriveService {
    pub fn new(config: AppConfig, files: Arc<FileRepository>, metadata_store: Arc<MetadataRepository>) -> Self {
        let files_root = files.prepared_folder(&config.files_root());
        let trash_root = files.prepared_folder(&config.trash_root());
        let metadata = Mutex::new(metadata_store.load());
        Self {
            config,
            files,
            metadata_store,
            metadata,
            files_root,
            trash_root,
        }
    }

    pub fn root_folder(&self) -> &Path {
        &self.files_root
    }

    // MARK: - Reading

    pub fn items(&self, section: DriveSection, folder: &Path, query: &str, sorter: ItemSorter) -> Vec<DriveItem> {
        let metadata = self.metadata.lock();
        let mut found = Vec::new();
        match section {
            DriveSection::MyDrive if query.is_empty() => found = self.files.contents(folder),
            DriveSection::MyDrive => found = self.files.all_items(folder),
            DriveSection::Recent => found = self.recent_files(),
            DriveSection::Starred => {
                for path in &metadata.starred {
                    if let Some(item) = self.files.item(Path::new(path)) {
                        found.push(item);
                    }
                }
            }
            DriveSection::Trash => {
                for slot in self.files.contents(&self.trash_root) {
                    found.append(&mut self.files.contents(&slot.path));
                }
            }
        }

        let query = query.to_lowercase();
        let mut folders = Vec::new();
        let mut others = Vec::new();
        for mut item in found {
            if !query.is_empty() && !item.name.to_lowercase().contains(&query) {
                continue;
            }
            item.is_starred = metadata.starred.contains(&DriveMetadata::key(&item.path));
            if item.is_folder {
                folders.push(item);
            } else {
                others.push(item);
            }
        }
        // Recent keeps its newest-first order.
        if section == DriveSection::Recent {
            return others;
        }
        sorter.sort(&mut folders);
        sorter.sort(&mut others);
        folders.append(&mut others);
        folders
    }

    /// What is inside one folder, sorted by name. Used by the column view.
    pub fn children(&self, folder: &Path) -> Vec<DriveItem> {
        self.items(DriveSection::MyDrive, folder, "", ItemSorter::BY_NAME)
    }

    pub fn summary(&self, folder: &Path) -> FolderSummary {
        FolderSummary {
            count: self.files.contents(folder).len(),
            size: self.files.total_size(folder),
        }
    }

    // ponytail: walks the whole tree on every call. Fine for thousands of files; cache it if it gets slow.
    pub fn used_bytes(&self) -> u64 {
        self.files.total_size(&self.config.storage_root)
    }

    /// How much space each group of file types uses. For the storage bar.
    pub fn storage_slices(&self) -> Vec<StorageSlice> {
        let mut sizes: BTreeMap<StorageGroup, u64> = BTreeMap::new();
        for file in self.files.all_files(&self.files_root) {
            *sizes.entry(file.kind.storage_group()).or_default() += file.size;
        }
        let mut slices = Vec::new();
        for group in StorageGroup::ALL {
            let size = sizes.get(&group).copied().unwrap_or(0);
            if size > 0 {
                slices.push(StorageSlice { group, size });
            }
        }
        slices
    }

    pub fn color(&self, folder: &Path) -> FolderColor {
        let metadata = self.metadata.lock();
        metadata
            .folder_colors
            .get(&DriveMetadata::key(folder))
            .copied()
            .unwrap_or_default()
    }

    pub fn is_pinned(&self, folder: &Path) -> bool {
        self.metadata.lock().pinned.contains(&DriveMetadata::key(folder))
    }

    pub fn pinned_folders(&self) -> Vec<FolderRef> {
        let pinned = self.metadata.lock().pinned.clone();
        let mut folders = Vec::new();
        for key in pinned {
            let path = PathBuf::from(key);
            if self.files.exists(&path) {
                folders.push(self.folder_ref(&path));
            }
        }
        folders
    }

    pub fn is_in_drive(&self, path: &Path) -> bool {
        path.starts_with(&self.files_root)
    }

    /// True for anything the app keeps: the files and the trash.
    pub fn is_in_storage(&self, path: &Path) -> bool {
        self.is_in_drive(path) || path.starts_with(&self.trash_root)
    }

    /// The folders from the root down to the given folder. Used to jump to a folder.
    pub fn folder_chain(&self, folder: &Path) -> Vec<FolderRef> {
        let mut chain = Vec::new();
        let mut current = folder;
        while current != self.files_root && current.starts_with(&self.files_root) {
            chain.insert(0, self.folder_ref(current));
            let Some(parent) = current.parent() else {
                break;
            };
            current = parent;
        }
        chain
    }

    // MARK: - Changing

    pub fn create_folder(&self, name: &str, folder: &Path, color: FolderColor) -> AppResult<PathBuf> {
        self.require_in_drive(folder)?;
        let path = self.unique_path(&Self::validated(name)?, folder);
        self.files.create_folder(&path)?;
        if color != FolderColor::Blue {
            self.metadata
                .lock()
                .folder_colors
                .insert(DriveMetadata::key(&path), color);
            self.save();
        }
        Ok(path)
    }

    /// Makes a new empty file, like "Untitled.txt".
    pub fn create_document(&self, name: &str, folder: &Path) -> AppResult<PathBuf> {
        self.require_in_drive(folder)?;
        let path = self.unique_path(&Self::validated(name)?, folder);
        self.files.create_file(&path)?;
        Ok(path)
    }

    /// Adds items to a folder. Items from outside the drive are copied. Items already in the drive are moved.
    pub async fn add(&self, sources: &[PathBuf], folder: &Path) -> AppResult<()> {
        self.require_in_drive(folder)?;
        for source in sources {
            if source != &self.files_root && self.is_in_drive(source) {
                self.move_item(source, folder)?;
                continue;
            }
            // A folder that holds the drive cannot be copied into the drive. The copy would never end.
            if folder.starts_with(source) {
                return Err(AppError::IntoItself);
            }
            let name = Self::name_of(source);
            self.files.copy(source, &self.unique_path(&name, folder)).await?;
        }
        Ok(())
    }

    /// "Report.pdf" gets a twin named "Report copy.pdf" in the same folder.
    pub async fn duplicate(&self, paths: &[PathBuf]) -> AppResult<()> {
        for path in paths {
            self.require_item_in_drive(path)?;
            let folder = path.parent().unwrap_or(&self.files_root);
            let (base, extension) = Self::split_name(&Self::name_of(path));
            let name = Self::join_name(&format!("{base} copy"), &extension);
            self.files.copy(path, &self.unique_path(&name, folder)).await?;
        }
        Ok(())
    }

    /// Copies items to a folder outside the drive.
    pub async fn export(&self, paths: &[PathBuf], folder: &Path) -> AppResult<()> {
        for path in paths {
            if !self.is_in_storage(path) {
                return Err(AppError::OutsideDrive);
            }
            self.files
                .copy(path, &self.unique_path(&Self::name_of(path), folder))
                .await?;
        }
        Ok(())
    }

    pub fn rename(&self, path: &Path, new_name: &str) -> AppResult<()> {
        self.require_item_in_drive(path)?;
        let name = Self::validated(new_name)?;
        let old_name = Self::name_of(path);
        if name == old_name {
            return Ok(());
        }
        let destination = path.with_file_name(&name);
        // A change of letter case only still "exists" on a Mac or Windows disk, so let that one pass.
        if self.files.exists(&destination) && name.to_lowercase() != old_name.to_lowercase() {
            return Err(AppError::NameTaken(name));
        }
        self.files.move_item(path, &destination)?;
        self.metadata.lock().repath(path, &destination);
        self.save();
        Ok(())
    }

    pub fn set_starred(&self, paths: &[PathBuf], starred: bool) {
        let mut metadata = self.metadata.lock();
        for path in paths {
            let key = DriveMetadata::key(path);
            if starred {
                metadata.starred.insert(key);
            } else {
                metadata.starred.remove(&key);
            }
        }
        drop(metadata);
        self.save();
    }

    pub fn set_pinned(&self, paths: &[PathBuf], pinned: bool) {
        let mut metadata = self.metadata.lock();
        for path in paths {
            let key = DriveMetadata::key(path);
            metadata.pinned.retain(|other| other != &key);
            if pinned {
                metadata.pinned.push(key);
            }
        }
        drop(metadata);
        self.save();
    }

    pub fn set_color(&self, color: FolderColor, paths: &[PathBuf]) {
        let mut metadata = self.metadata.lock();
        for path in paths {
            metadata.folder_colors.insert(DriveMetadata::key(path), color);
        }
        drop(metadata);
        self.save();
    }

    pub fn trash(&self, paths: &[PathBuf]) -> AppResult<()> {
        let result = self.trash_each(paths);
        self.save();
        result
    }

    pub fn restore(&self, paths: &[PathBuf]) -> AppResult<()> {
        let result = self.restore_each(paths);
        self.save();
        result
    }

    pub fn delete_forever(&self, paths: &[PathBuf]) -> AppResult<()> {
        let result = self.delete_each(paths);
        self.save();
        result
    }

    /// Deletes what has been in the trash for longer than `AppConstants::TRASH_KEEP_DAYS`.
    /// `now` is in milliseconds since 1970. Returns how many items it deleted.
    // ponytail: the change date of the slot folder is the time the item went to the trash. If something else
    // writes into the slot, the item only stays longer. Save the date in the metadata if that is not exact enough.
    pub fn empty_old_trash(&self, now: i64) -> usize {
        let limit = now - AppConstants::TRASH_KEEP_DAYS * AppConstants::MILLIS_PER_DAY;
        let mut deleted = 0;
        for slot in self.files.contents(&self.trash_root) {
            // A date of 0 means the date could not be read. Such a slot is kept.
            if !slot.is_folder || slot.modified <= 0 || slot.modified > limit {
                continue;
            }
            if self.files.delete(&slot.path).is_ok() {
                self.metadata.lock().trash_origins.remove(&slot.name);
                deleted += 1;
            }
        }
        if deleted > 0 {
            self.save();
        }
        deleted
    }

    // MARK: - Helpers

    fn trash_each(&self, paths: &[PathBuf]) -> AppResult<()> {
        for path in paths {
            self.require_item_in_drive(path)?;
            // A parent and its child can both be picked. The child is gone once the parent moves.
            if !self.files.exists(path) {
                continue;
            }
            let slot_id = Self::new_slot_id()?;
            let slot = self.trash_root.join(&slot_id);
            self.files.create_folder(&slot)?;
            self.files.move_item(path, &slot.join(Self::name_of(path)))?;
            self.metadata
                .lock()
                .trash_origins
                .insert(slot_id, DriveMetadata::key(path));
        }
        Ok(())
    }

    fn restore_each(&self, paths: &[PathBuf]) -> AppResult<()> {
        for path in paths {
            let Some((slot, slot_id)) = self.trash_slot(path) else {
                continue;
            };
            let origin = self.metadata.lock().trash_origins.get(&slot_id).cloned();
            let mut folder = self.files_root.clone();
            if let Some(parent) = origin.as_deref().and_then(|origin| Path::new(origin).parent()) {
                // The saved place is outside data. Only use it when it is in the drive.
                if self.is_in_drive(parent) {
                    folder = parent.to_path_buf();
                }
            }
            self.files.create_folder(&folder)?;
            self.files
                .move_item(path, &self.unique_path(&Self::name_of(path), &folder))?;
            self.files.delete(&slot)?;
            self.metadata.lock().trash_origins.remove(&slot_id);
        }
        Ok(())
    }

    fn delete_each(&self, paths: &[PathBuf]) -> AppResult<()> {
        for path in paths {
            let Some((slot, slot_id)) = self.trash_slot(path) else {
                continue;
            };
            self.files.delete(&slot)?;
            self.metadata.lock().trash_origins.remove(&slot_id);
        }
        Ok(())
    }

    fn recent_files(&self) -> Vec<DriveItem> {
        let mut recent = self.files.all_files(&self.files_root);
        let newest_first = ItemSorter {
            field: SortField::Modified,
            ascending: false,
        };
        newest_first.sort(&mut recent);
        recent.truncate(AppConstants::RECENT_LIMIT);
        recent
    }

    fn move_item(&self, source: &Path, folder: &Path) -> AppResult<()> {
        // Dropped where it already is: nothing to do.
        if source.parent() == Some(folder) || folder == source {
            return Ok(());
        }
        if folder.starts_with(source) {
            return Err(AppError::IntoItself);
        }
        let destination = self.unique_path(&Self::name_of(source), folder);
        self.files.move_item(source, &destination)?;
        self.metadata.lock().repath(source, &destination);
        self.save();
        Ok(())
    }

    /// The trash slot folder of a trashed item, and its id. `None` when the item is not in the trash,
    /// so nothing else gets deleted.
    fn trash_slot(&self, path: &Path) -> Option<(PathBuf, String)> {
        let slot = path.parent()?;
        if slot.parent()? != self.trash_root {
            return None;
        }
        Some((slot.to_path_buf(), Self::name_of(slot)))
    }

    fn folder_ref(&self, path: &Path) -> FolderRef {
        FolderRef {
            path: path.to_path_buf(),
            name: Self::name_of(path),
            color: self.color(path),
        }
    }

    fn require_in_drive(&self, path: &Path) -> AppResult<()> {
        if self.is_in_drive(path) {
            Ok(())
        } else {
            Err(AppError::OutsideDrive)
        }
    }

    /// The path must be an item in the drive, not the drive itself. The window sends paths,
    /// and this keeps a wrong one from changing files that are not ours.
    fn require_item_in_drive(&self, path: &Path) -> AppResult<()> {
        if path == self.files_root {
            return Err(AppError::OutsideDrive);
        }
        self.require_in_drive(path)
    }

    fn validated(name: &str) -> AppResult<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.starts_with('.') || trimmed.contains(['/', '\\', ':']) {
            return Err(AppError::InvalidName);
        }
        Ok(trimmed.to_owned())
    }

    /// "Report.pdf" becomes "Report 2.pdf" when the name is taken.
    fn unique_path(&self, name: &str, folder: &Path) -> PathBuf {
        let mut candidate = folder.join(name);
        let (base, extension) = Self::split_name(name);
        let mut counter = 2;
        while self.files.exists(&candidate) {
            candidate = folder.join(Self::join_name(&format!("{base} {counter}"), &extension));
            counter += 1;
        }
        candidate
    }

    /// "Report.pdf" gives ("Report", "pdf"). A name without a dot gives an empty ending.
    fn split_name(name: &str) -> (String, String) {
        match name.rsplit_once('.') {
            Some((base, extension)) if !base.is_empty() => (base.to_owned(), extension.to_owned()),
            _ => (name.to_owned(), String::new()),
        }
    }

    fn join_name(base: &str, extension: &str) -> String {
        if extension.is_empty() {
            base.to_owned()
        } else {
            format!("{base}.{extension}")
        }
    }

    fn name_of(path: &Path) -> String {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// A random name for a trash slot.
    fn new_slot_id() -> AppResult<String> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| AppError::Io(std::io::Error::other(error.to_string())))?;
        let mut id = String::with_capacity(32);
        for byte in bytes {
            let _ = write!(id, "{byte:02X}");
        }
        Ok(id)
    }

    // ponytail: a failed save only loses stars, pins, colors and "put back" places, never files. Not shown to the user.
    fn save(&self) {
        let metadata = self.metadata.lock().clone();
        let _ = self.metadata_store.save(&metadata);
    }
}
