mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::TempFolder;
use drive_lib::config::app_config::AppConfig;
use drive_lib::models::drive::{DriveMetadata, DriveSection, FolderColor, ItemSorter, NaturalOrder, StorageGroup};
use drive_lib::models::error::AppError;
use drive_lib::repositories::file_repository::FileRepository;
use drive_lib::repositories::metadata_repository::MetadataRepository;
use drive_lib::services::drive_service::DriveService;

/// The real service and the real file repository, on a new temp folder.
struct Fixture {
    temp: TempFolder,
    service: DriveService,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempFolder::new();
        let config = AppConfig::new(temp.path().to_path_buf());
        let store = Arc::new(MetadataRepository::new(config.metadata_file()));
        let service = DriveService::new(config, Arc::new(FileRepository::new()), store);
        Self { temp, service }
    }

    fn root(&self) -> PathBuf {
        self.service.root_folder().to_path_buf()
    }

    fn names(&self, section: DriveSection, folder: &Path) -> Vec<String> {
        let mut names = Vec::new();
        for item in self.service.items(section, folder, "", ItemSorter::BY_NAME) {
            names.push(item.name);
        }
        names
    }
}

#[tokio::test]
async fn full_life_of_a_file() {
    let fixture = Fixture::new();
    let service = &fixture.service;
    let root = fixture.root();

    // Import a file from outside, two times. The second one gets a new name.
    let outside = fixture.temp.path().join("note.txt");
    std::fs::write(&outside, "hello").unwrap();
    service.add(&[outside.clone(), outside], &root).await.unwrap();
    assert_eq!(fixture.names(DriveSection::MyDrive, &root), ["note 2.txt", "note.txt"]);

    // Folders come first. A starred file keeps its star after a move and a rename.
    let folder = service.create_folder("Docs", &root, FolderColor::Blue).unwrap();
    let note = root.join("note.txt");
    service.set_starred(std::slice::from_ref(&note), true);
    service.add(&[note], &folder).await.unwrap();
    service.rename(&folder.join("note.txt"), "todo.txt").unwrap();
    assert_eq!(fixture.names(DriveSection::MyDrive, &root), ["Docs", "note 2.txt"]);
    assert_eq!(fixture.names(DriveSection::Starred, &root), ["todo.txt"]);

    // Search looks in all folders.
    let found = service.items(DriveSection::MyDrive, &root, "TODO", ItemSorter::BY_NAME);
    assert_eq!(found.len(), 1);
    assert!(found[0].is_starred);

    // Bad names and bad moves are refused.
    assert!(matches!(service.rename(&folder, "a/b"), Err(AppError::InvalidName)));
    assert!(matches!(service.rename(&folder, "note 2.txt"), Err(AppError::NameTaken(name)) if name == "note 2.txt"));
    let inner = service.create_folder("Inner", &folder, FolderColor::Blue).unwrap();
    assert!(matches!(
        service.add(std::slice::from_ref(&folder), &inner).await,
        Err(AppError::IntoItself)
    ));
    // A path that is not in the drive is never renamed or trashed.
    let stranger = fixture.temp.path().join("metadata.json");
    assert!(matches!(
        service.rename(&stranger, "x.json"),
        Err(AppError::OutsideDrive)
    ));
    assert!(matches!(service.trash(&[stranger]), Err(AppError::OutsideDrive)));

    // Trash, put back, then delete forever.
    service.trash(std::slice::from_ref(&folder)).unwrap();
    assert_eq!(fixture.names(DriveSection::MyDrive, &root), ["note 2.txt"]);
    assert_eq!(fixture.names(DriveSection::Starred, &root).len(), 0);
    let trashed = service.items(DriveSection::Trash, &root, "", ItemSorter::BY_NAME);
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].name, "Docs");

    service.restore(&[trashed[0].path.clone()]).unwrap();
    assert_eq!(fixture.names(DriveSection::MyDrive, &folder), ["Inner", "todo.txt"]);
    assert_eq!(fixture.names(DriveSection::Starred, &root), ["todo.txt"]);

    service.trash(&[folder]).unwrap();
    // A path outside the trash must never be deleted by "delete forever".
    service.delete_forever(&[root.join("note 2.txt")]).unwrap();
    assert_eq!(fixture.names(DriveSection::MyDrive, &root), ["note 2.txt"]);
    let mut in_trash = Vec::new();
    for item in service.items(DriveSection::Trash, &root, "", ItemSorter::BY_NAME) {
        in_trash.push(item.path);
    }
    service.delete_forever(&in_trash).unwrap();
    assert_eq!(fixture.names(DriveSection::Trash, &root).len(), 0);
}

#[test]
fn old_items_leave_the_trash_by_themselves() {
    const DAY: i64 = 24 * 60 * 60 * 1000;
    let fixture = Fixture::new();
    let service = &fixture.service;
    let root = fixture.root();
    let note = root.join("note.txt");
    std::fs::write(&note, "hello").unwrap();
    service.trash(&[note]).unwrap();
    let trashed_at = service.items(DriveSection::Trash, &root, "", ItemSorter::BY_NAME)[0].modified;

    // After 29 days it is still there. After 31 days it is gone, and so is its "put back" place.
    assert_eq!(service.empty_old_trash(trashed_at + 29 * DAY), 0);
    assert_eq!(fixture.names(DriveSection::Trash, &root), ["note.txt"]);
    assert_eq!(service.empty_old_trash(trashed_at + 31 * DAY), 1);
    assert_eq!(fixture.names(DriveSection::Trash, &root).len(), 0);
    let saved = std::fs::read_to_string(fixture.temp.path().join("metadata.json")).unwrap();
    assert!(saved.contains(r#""trashOrigins":{}"#), "{saved}");
    // Files in the drive are never touched, however old they are.
    std::fs::write(root.join("keep.txt"), "hello").unwrap();
    assert_eq!(service.empty_old_trash(trashed_at + 1000 * DAY), 0);
    assert_eq!(fixture.names(DriveSection::MyDrive, &root), ["keep.txt"]);
}

#[test]
fn folders_keep_pin_and_color_after_rename() {
    let fixture = Fixture::new();
    let service = &fixture.service;
    let root = fixture.root();
    let folder = service.create_folder("Clients", &root, FolderColor::Green).unwrap();
    service.set_pinned(std::slice::from_ref(&folder), true);
    service.rename(&folder, "Customers").unwrap();
    let renamed = root.join("Customers");

    let pinned = service.pinned_folders();
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].name, "Customers");
    assert_eq!(pinned[0].color, FolderColor::Green);
    assert!(service.is_pinned(&renamed));
    assert_eq!(service.folder_chain(&renamed).len(), 1);
    service.set_pinned(&[renamed], false);
    assert_eq!(service.pinned_folders().len(), 0);
}

#[tokio::test]
async fn duplicates_exports_and_counts_files() {
    let fixture = Fixture::new();
    let service = &fixture.service;
    let root = fixture.root();
    let note = service.create_document("Untitled.txt", &root).unwrap();
    std::fs::write(&note, "hello").unwrap();
    service.duplicate(std::slice::from_ref(&note)).await.unwrap();
    service.duplicate(std::slice::from_ref(&note)).await.unwrap();
    assert_eq!(
        fixture.names(DriveSection::MyDrive, &root),
        ["Untitled copy 2.txt", "Untitled copy.txt", "Untitled.txt"]
    );

    let summary = service.summary(&root);
    assert_eq!(summary.count, 3);
    assert_eq!(summary.size, 15);
    let slices = service.storage_slices();
    assert_eq!(slices.len(), 1);
    assert_eq!(slices[0].group, StorageGroup::Documents);

    let outside = fixture.temp.path().join("Exported");
    std::fs::create_dir_all(&outside).unwrap();
    service.export(std::slice::from_ref(&note), &outside).await.unwrap();
    assert!(outside.join("Untitled.txt").exists());
    assert!(service.is_in_drive(&note));
    assert!(!service.is_in_drive(&outside));
}

#[test]
fn loads_metadata_saved_by_an_older_version() {
    let old = r#"{"starred":["/a/b.txt"],"trashOrigins":{}}"#;
    let metadata: DriveMetadata = serde_json::from_str(old).unwrap();
    assert!(metadata.starred.contains("/a/b.txt"));
    assert_eq!(metadata.pinned.len(), 0);

    // The Swift app wrote the colors like this.
    let swift = r#"{"starred":[],"trashOrigins":{"ABC":"/a/b"},"pinned":["/a"],"folderColors":{"/a":"green"}}"#;
    let metadata: DriveMetadata = serde_json::from_str(swift).unwrap();
    assert_eq!(metadata.folder_colors.get("/a"), Some(&FolderColor::Green));
}

#[test]
fn sorts_names_like_a_file_manager() {
    let mut names = vec!["file 10.txt", "File 2.txt", "apple", "file 1.txt"];
    names.sort_by(|left, right| NaturalOrder::compare(left, right));
    assert_eq!(names, ["apple", "file 1.txt", "File 2.txt", "file 10.txt"]);
}
