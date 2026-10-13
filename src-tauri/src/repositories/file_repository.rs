use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::drive::{DriveItem, FileKind};
use crate::models::error::{AppError, AppResult};

/// All access to files and folders on disk.
#[derive(Debug, Default)]
pub struct FileRepository;

impl FileRepository {
    pub fn new() -> Self {
        Self
    }

    /// Creates the folder and returns its real path.
    /// Listings return real paths ("/private/var", not "/var"), and path checks need both sides to match.
    pub fn prepared_folder(&self, path: &Path) -> PathBuf {
        // If this fails, the first real action fails too and shows the error.
        let _ = self.create_folder(path);
        dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    pub fn item(&self, path: &Path) -> Option<DriveItem> {
        let metadata = fs::metadata(path).ok()?;
        // A name that is not valid text cannot be sent to the window, so such a file is not shown.
        let name = path.file_name()?.to_str()?.to_owned();
        path.to_str()?;
        let modified = Self::millis(metadata.modified().ok());
        // Some Linux file systems keep no creation date. The change date is the best guess then.
        let created = metadata
            .created()
            .ok()
            .map_or(modified, |time| Self::millis(Some(time)));
        Some(DriveItem {
            path: path.to_path_buf(),
            name,
            is_folder: metadata.is_dir(),
            kind: FileKind::of(path, metadata.is_dir()),
            size: if metadata.is_dir() { 0 } else { metadata.len() },
            modified,
            created,
            is_starred: false,
        })
    }

    /// What is directly inside the folder. Hidden items are left out.
    pub fn contents(&self, folder: &Path) -> Vec<DriveItem> {
        let mut items = Vec::new();
        for path in Self::visible_children(folder) {
            if let Some(item) = self.item(&path) {
                items.push(item);
            }
        }
        items
    }

    /// Every item under the folder, at any depth.
    pub fn all_items(&self, folder: &Path) -> Vec<DriveItem> {
        let mut items = Vec::new();
        self.collect(folder, true, &mut items);
        items
    }

    /// Every real file under the folder, at any depth.
    pub fn all_files(&self, folder: &Path) -> Vec<DriveItem> {
        let mut items = Vec::new();
        self.collect(folder, false, &mut items);
        items
    }

    /// The size of all files under the folder, hidden ones too.
    pub fn total_size(&self, folder: &Path) -> u64 {
        Self::size_under(folder)
    }

    fn size_under(folder: &Path) -> u64 {
        let Ok(entries) = fs::read_dir(folder) else {
            return 0;
        };
        let mut total = 0;
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                total += Self::size_under(&entry.path());
            } else {
                total += metadata.len();
            }
        }
        total
    }

    pub fn exists(&self, path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok()
    }

    pub fn create_folder(&self, path: &Path) -> AppResult<()> {
        Ok(fs::create_dir_all(path)?)
    }

    /// Makes a new empty file. Fails when the file is there already.
    pub fn create_file(&self, path: &Path) -> AppResult<()> {
        fs::File::create_new(path)?;
        Ok(())
    }

    pub fn move_item(&self, source: &Path, destination: &Path) -> AppResult<()> {
        Ok(fs::rename(source, destination)?)
    }

    pub fn delete(&self, path: &Path) -> AppResult<()> {
        // A link to a folder is removed as a file, so the folder it points to stays.
        if fs::symlink_metadata(path)?.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn set_modified(&self, millis: i64, path: &Path) -> AppResult<()> {
        let time = UNIX_EPOCH + Duration::from_millis(u64::try_from(millis).unwrap_or(0));
        fs::File::options().write(true).open(path)?.set_modified(time)?;
        Ok(())
    }

    /// Copies a file, or a folder with all that is in it. Runs off the async threads, so a big file blocks nothing.
    pub async fn copy(&self, source: &Path, destination: &Path) -> AppResult<()> {
        let (source, destination) = (source.to_path_buf(), destination.to_path_buf());
        let copied = tokio::task::spawn_blocking(move || Self::copy_tree(&source, &destination)).await;
        copied.map_err(|error| AppError::Io(std::io::Error::other(error)))?
    }

    fn copy_tree(source: &Path, destination: &Path) -> AppResult<()> {
        if !fs::metadata(source)?.is_dir() {
            fs::copy(source, destination)?;
            return Ok(());
        }
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            Self::copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
        Ok(())
    }

    fn collect(&self, folder: &Path, with_folders: bool, items: &mut Vec<DriveItem>) {
        for path in Self::visible_children(folder) {
            // Do not go into a link to a folder. It could point back up and never end.
            let is_real_folder = fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_dir());
            if let Some(item) = self.item(&path) {
                if with_folders || !item.is_folder {
                    items.push(item);
                }
            }
            if is_real_folder {
                self.collect(&path, with_folders, items);
            }
        }
    }

    // ponytail: "hidden" means the name starts with a dot. Files that Windows marks as hidden are shown.
    // Read the file attributes on Windows if that gets annoying.
    fn visible_children(folder: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let Ok(entries) = fs::read_dir(folder) else {
            return paths;
        };
        for entry in entries.flatten() {
            if !entry.file_name().to_string_lossy().starts_with('.') {
                paths.push(entry.path());
            }
        }
        paths
    }

    fn millis(time: Option<SystemTime>) -> i64 {
        let since = time.and_then(|time| time.duration_since(UNIX_EPOCH).ok());
        since.map_or(0, |duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
    }
}
