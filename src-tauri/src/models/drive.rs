use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The lists in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DriveSection {
    MyDrive,
    Recent,
    Starred,
    Trash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FolderColor {
    #[default]
    Blue,
    Purple,
    Green,
    Orange,
    Pink,
    Gray,
}

/// The parts of the storage bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageGroup {
    Documents,
    Media,
    Other,
}

impl StorageGroup {
    pub const ALL: [Self; 3] = [Self::Documents, Self::Media, Self::Other];
}

/// Where a file stands with the backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncState {
    Synced,
    Pending,
    Uploading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortField {
    Name,
    Modified,
    Size,
}

/// The rough type of a file. It picks the icon and its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Folder,
    Pdf,
    Image,
    Video,
    Audio,
    Sheet,
    Slides,
    Text,
    Archive,
    Design,
    Other,
}

impl FileKind {
    const DESIGN: [&'static str; 5] = ["fig", "sketch", "psd", "ai", "xd"];
    const SHEET: [&'static str; 6] = ["xlsx", "xls", "numbers", "csv", "tsv", "ods"];
    const SLIDES: [&'static str; 4] = ["key", "pptx", "ppt", "odp"];
    const IMAGE: [&'static str; 12] = [
        "png", "jpg", "jpeg", "gif", "webp", "heic", "bmp", "tiff", "tif", "svg", "ico", "avif",
    ];
    const VIDEO: [&'static str; 6] = ["mp4", "mov", "mkv", "avi", "webm", "m4v"];
    const AUDIO: [&'static str; 7] = ["mp3", "wav", "m4a", "aac", "flac", "ogg", "opus"];
    const ARCHIVE: [&'static str; 10] = ["zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "dmg", "iso"];
    const TEXT: [&'static str; 26] = [
        "doc", "docx", "pages", "rtf", "odt", "txt", "md", "log", "json", "xml", "html", "css", "js", "ts", "rs", "py",
        "swift", "yaml", "yml", "toml", "sh", "c", "h", "cpp", "java", "go",
    ];

    // ponytail: the type comes from the file ending only. Read the first bytes of the file if endings lie too often.
    pub fn of(path: &Path, is_folder: bool) -> Self {
        if is_folder {
            return Self::Folder;
        }
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        let ext = extension.as_str();
        if Self::DESIGN.contains(&ext) {
            Self::Design
        } else if Self::SHEET.contains(&ext) {
            Self::Sheet
        } else if Self::SLIDES.contains(&ext) {
            Self::Slides
        } else if ext == "pdf" {
            Self::Pdf
        } else if Self::IMAGE.contains(&ext) {
            Self::Image
        } else if Self::VIDEO.contains(&ext) {
            Self::Video
        } else if Self::AUDIO.contains(&ext) {
            Self::Audio
        } else if Self::ARCHIVE.contains(&ext) {
            Self::Archive
        } else if Self::TEXT.contains(&ext) {
            Self::Text
        } else {
            Self::Other
        }
    }

    pub fn storage_group(self) -> StorageGroup {
        match self {
            Self::Pdf | Self::Sheet | Self::Slides | Self::Text => StorageGroup::Documents,
            Self::Image | Self::Video | Self::Audio | Self::Design => StorageGroup::Media,
            Self::Folder | Self::Archive | Self::Other => StorageGroup::Other,
        }
    }
}

/// One file or folder in the drive. Dates are milliseconds since 1970.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveItem {
    pub path: PathBuf,
    pub name: String,
    pub is_folder: bool,
    pub kind: FileKind,
    pub size: u64,
    pub modified: i64,
    pub created: i64,
    pub is_starred: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FolderSummary {
    pub count: usize,
    pub size: u64,
}

/// A folder with its name. Used for the pinned folders and for the path bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FolderRef {
    pub path: PathBuf,
    pub name: String,
    pub color: FolderColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StorageSlice {
    pub group: StorageGroup,
    pub size: u64,
}

/// One item the way the window shows it: the item, plus what the other parts of the app know about it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    #[serde(flatten)]
    pub item: DriveItem,
    /// Only folders have these three.
    pub color: Option<FolderColor>,
    pub summary: Option<FolderSummary>,
    pub is_pinned: bool,
    /// None means: show no backup icon for this item.
    pub sync: Option<SyncState>,
    /// The names of the folders from the top of the drive down to this item.
    pub parents: Vec<String>,
}

/// What the sidebar and the tray show.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub root: PathBuf,
    pub pinned: Vec<FolderRef>,
    pub slices: Vec<StorageSlice>,
    pub used_bytes: u64,
    pub recent: Vec<ItemView>,
}

/// Sorts items by one field. Ties are sorted by name.
#[derive(Debug, Clone, Copy)]
pub struct ItemSorter {
    pub field: SortField,
    pub ascending: bool,
}

impl ItemSorter {
    pub const BY_NAME: Self = Self {
        field: SortField::Name,
        ascending: true,
    };

    pub fn sort(self, items: &mut [DriveItem]) {
        items.sort_by(|left, right| self.compare(left, right));
    }

    fn compare(self, left: &DriveItem, right: &DriveItem) -> Ordering {
        let by_name = NaturalOrder::compare(&left.name, &right.name);
        let order = match self.field {
            SortField::Name => by_name,
            SortField::Modified => left.modified.cmp(&right.modified).then(by_name),
            SortField::Size => left.size.cmp(&right.size).then(by_name),
        };
        if self.ascending { order } else { order.reverse() }
    }
}

/// Compares names the way a file manager does: no difference between big and small letters,
/// and "file 2" comes before "file 10".
pub struct NaturalOrder;

impl NaturalOrder {
    pub fn compare(left: &str, right: &str) -> Ordering {
        let a: Vec<char> = left.to_lowercase().chars().collect();
        let b: Vec<char> = right.to_lowercase().chars().collect();
        let (mut i, mut j) = (0, 0);
        while i < a.len() && j < b.len() {
            if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
                let (left_number, next_i) = Self::number(&a, i);
                let (right_number, next_j) = Self::number(&b, j);
                if left_number != right_number {
                    return left_number.cmp(&right_number);
                }
                i = next_i;
                j = next_j;
            } else {
                if a[i] != b[j] {
                    return a[i].cmp(&b[j]);
                }
                i += 1;
                j += 1;
            }
        }
        (a.len() - i).cmp(&(b.len() - j)).then_with(|| left.cmp(right))
    }

    /// Reads the number that starts at `start`. Returns it and the place after it.
    fn number(chars: &[char], start: usize) -> (u128, usize) {
        let mut value: u128 = 0;
        let mut index = start;
        while index < chars.len() && chars[index].is_ascii_digit() {
            let digit = u128::from(chars[index].to_digit(10).unwrap_or(0));
            value = value.saturating_mul(10).saturating_add(digit);
            index += 1;
        }
        (value, index)
    }
}

/// Extra facts the file system does not keep. Saved as one JSON file.
/// Every field has a default, so a file saved by an older version still loads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DriveMetadata {
    /// Paths of starred items.
    pub starred: BTreeSet<String>,
    /// Trash slot id -> the path the item had before it was trashed.
    pub trash_origins: BTreeMap<String, String>,
    /// Paths of folders pinned to the sidebar, in the order they were pinned.
    pub pinned: Vec<String>,
    /// Folder path -> its color.
    pub folder_colors: BTreeMap<String, FolderColor>,
}

impl DriveMetadata {
    /// The text that stands for a path in the metadata.
    pub fn key(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    /// Keeps stars, pins and colors on an item, and on everything inside it, after a move or rename.
    pub fn repath(&mut self, old_path: &Path, new_path: &Path) {
        let mut starred = BTreeSet::new();
        for key in &self.starred {
            starred.insert(Self::moved(key, old_path, new_path));
        }
        self.starred = starred;

        for key in &mut self.pinned {
            *key = Self::moved(key, old_path, new_path);
        }

        let mut colors = BTreeMap::new();
        for (key, color) in &self.folder_colors {
            colors.insert(Self::moved(key, old_path, new_path), *color);
        }
        self.folder_colors = colors;
    }

    fn moved(key: &str, old_path: &Path, new_path: &Path) -> String {
        match Path::new(key).strip_prefix(old_path) {
            Ok(rest) if rest.as_os_str().is_empty() => Self::key(new_path),
            Ok(rest) => Self::key(&new_path.join(rest)),
            Err(_) => key.to_owned(),
        }
    }
}
