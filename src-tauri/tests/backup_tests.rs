mod common;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use common::TempFolder;
use drive_lib::config::backup_config::BackupConfig;
use drive_lib::constants::AppConstants;
use drive_lib::models::backup::{BackupConnection, BackupProvider, BackupState, Credentials, RemoteObject};
use drive_lib::models::drive::{DriveMetadata, SyncState};
use drive_lib::models::error::{AppError, AppResult};
use drive_lib::models::google::GoogleFile;
use drive_lib::repositories::azure_repository::AzureRepository;
use drive_lib::repositories::backup_repository::BackupRepository;
use drive_lib::repositories::backup_state_repository::BackupStateRepository;
use drive_lib::repositories::file_repository::FileRepository;
use drive_lib::repositories::google_auth_repository::GoogleAuthRepository;
use drive_lib::repositories::google_drive_repository::GoogleDriveRepository;
use drive_lib::repositories::http_transport::{HttpReply, HttpTransport};
use drive_lib::repositories::part_upload::{PartTarget, PartUpload};
use drive_lib::repositories::s3_repository::S3Repository;
use drive_lib::repositories::secret_repository::SecretRepository;
use drive_lib::repositories::settings_repository::SettingsRepository;
use drive_lib::services::aws_signer::{AwsSigner, SigningRequest};
use drive_lib::services::backup_service::{BackupObserver, BackupRepositories, BackupService};
use parking_lot::Mutex;

// The example keys and results below come from the AWS Signature Version 4 documentation for S3.
const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// 2009-10-09 21:04:30 UTC, in milliseconds.
const OCTOBER_2009: i64 = 1_255_122_270_000;

struct SilentObserver;

impl BackupObserver for SilentObserver {
    fn backup_changed(&self) {}
}

/// Keeps the keys in memory, so the tests do not touch the keychain of the computer.
#[derive(Default)]
struct MemorySecrets {
    saved: Mutex<HashMap<BackupProvider, Credentials>>,
}

impl SecretRepository for MemorySecrets {
    fn load(&self, provider: BackupProvider) -> Option<Credentials> {
        self.saved.lock().get(&provider).cloned()
    }

    fn save(&self, credentials: &Credentials, provider: BackupProvider) -> AppResult<()> {
        self.saved.lock().insert(provider, credentials.clone());
        Ok(())
    }

    fn delete(&self, provider: BackupProvider) {
        self.saved.lock().remove(&provider);
    }
}

/// A backup that lives in a folder on disk. It stands in for S3.
struct FolderBackup {
    folder: TempFolder,
}

impl FolderBackup {
    fn file(&self, key: &str) -> std::path::PathBuf {
        // Keys can have "/" in them. One flat file per key keeps this simple.
        self.folder.path().join(key.replace('/', "__"))
    }
}

#[async_trait]
impl BackupRepository for FolderBackup {
    async fn list(&self, _connection: &BackupConnection) -> AppResult<Vec<RemoteObject>> {
        let mut objects = Vec::new();
        for entry in std::fs::read_dir(self.folder.path())? {
            let entry = entry?;
            let modified = entry
                .metadata()?
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap();
            objects.push(RemoteObject {
                key: entry.file_name().to_string_lossy().replace("__", "/"),
                size: entry.metadata()?.len(),
                last_modified: i64::try_from(modified.as_millis()).unwrap(),
            });
        }
        Ok(objects)
    }

    async fn upload(&self, file: &Path, key: &str, _connection: &BackupConnection) -> AppResult<()> {
        std::fs::copy(file, self.file(key))?;
        Ok(())
    }

    async fn download(&self, key: &str, destination: &Path, _connection: &BackupConnection) -> AppResult<()> {
        std::fs::copy(self.file(key), destination)?;
        Ok(())
    }

    async fn delete(&self, key: &str, _connection: &BackupConnection) -> AppResult<()> {
        Ok(std::fs::remove_file(self.file(key))?)
    }
}

fn signer() -> Arc<AwsSigner> {
    Arc::new(AwsSigner::new())
}

fn google_auth() -> Arc<GoogleAuthRepository> {
    Arc::new(GoogleAuthRepository::new(signer(), Arc::new(HttpTransport::new())))
}

fn config(provider: BackupProvider, bucket: &str) -> BackupConfig {
    BackupConfig {
        provider,
        bucket: bucket.to_owned(),
        ..BackupConfig::default()
    }
}

fn backup_service(root: &Path, s3: Arc<dyn BackupRepository>, secrets: Arc<MemorySecrets>) -> BackupService {
    let http = Arc::new(HttpTransport::new());
    let auth = google_auth();
    let repositories = BackupRepositories {
        s3,
        azure: Arc::new(AzureRepository::new(signer(), http.clone())),
        google: Arc::new(GoogleDriveRepository::new(auth.clone(), signer(), http)),
    };
    BackupService::new(
        root.join("Files"),
        Arc::new(FileRepository::new()),
        repositories,
        auth,
        secrets,
        Arc::new(BackupStateRepository::new(root.join("backup-state.json"))),
        Arc::new(SilentObserver),
    )
}

fn signing<'a>(path: &'a str, query: &'a str, headers: &'a BTreeMap<String, String>) -> SigningRequest<'a> {
    SigningRequest {
        method: "GET",
        path,
        query,
        headers,
        payload_hash: EMPTY_HASH,
        region: "us-east-1",
    }
}

#[test]
fn signs_like_the_aws_examples() {
    let credentials = Credentials::new("AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
    let mut headers = BTreeMap::new();
    headers.insert("host".to_owned(), "examplebucket.s3.amazonaws.com".to_owned());
    headers.insert("x-amz-content-sha256".to_owned(), EMPTY_HASH.to_owned());
    headers.insert("x-amz-date".to_owned(), "20130524T000000Z".to_owned());
    let signer = AwsSigner::new();
    let list = signer.authorization(&signing("/", "max-keys=2&prefix=J", &headers), &credentials);
    assert!(list.ends_with("Signature=34b48302e7b5fa45bde8084f4b7868a86f0a534bc59db6670ed5711ef69dc6f7"));

    headers.insert("range".to_owned(), "bytes=0-9".to_owned());
    let get = signer.authorization(&signing("/test.txt", "", &headers), &credentials);
    assert_eq!(
        get,
        "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,\
         SignedHeaders=host;range;x-amz-content-sha256;x-amz-date,\
         Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
    );
}

#[test]
fn encodes_keys() {
    let signer = AwsSigner::new();
    assert_eq!(
        signer.uri_encode("Docs/My file (1)+é.txt", true),
        "Docs/My%20file%20%281%29%2B%C3%A9.txt"
    );
    assert_eq!(signer.uri_encode("a/b", false), "a%2Fb");
}

#[test]
fn each_provider_keeps_its_own_bucket() {
    let temp = TempFolder::new();
    let file = temp.path().join("settings.json");
    let settings = SettingsRepository::new(file.clone());

    let mut s3 = config(BackupProvider::S3, "my-s3-bucket");
    s3.region = "eu-west-1".to_owned();
    s3.prefix = "mac".to_owned();
    s3.save(&settings);
    let mut azure = config(BackupProvider::Azure, "my-container");
    azure.account = "mydrive".to_owned();
    azure.save(&settings);

    // The provider that was saved last is the one in use. The settings are read back from the file.
    let settings = SettingsRepository::new(file);
    let mut expected = azure.clone();
    expected.region = "eu-west-1".to_owned();
    assert_eq!(BackupConfig::saved(&settings), expected);
    let saved_s3 = BackupConfig::saved_for(BackupProvider::S3, &settings);
    assert_eq!(saved_s3.bucket, "my-s3-bucket");
    assert_eq!(saved_s3.prefix, "mac");
    assert_eq!(BackupConfig::saved_for(BackupProvider::Gcs, &settings).bucket, "");
}

#[test]
fn remembers_what_is_in_the_backup() {
    let temp = TempFolder::new();
    let store = BackupStateRepository::new(temp.path().join("backup-state.json"));
    let file = temp.path().join("Files").join("a.txt");
    let date = 1_700_000_000_000;
    let mut state = BackupState {
        destination: "s3://one/".to_owned(),
        ..BackupState::default()
    };
    state.synced_dates.insert(DriveMetadata::key(&file), date);
    store.save(&state).unwrap();

    // A new launch knows the file is in the backup, without asking the provider.
    let remote = Arc::new(FolderBackup {
        folder: TempFolder::new(),
    });
    let service = backup_service(temp.path(), remote, Arc::new(MemorySecrets::default()));
    service.follow("s3://one/");
    assert_eq!(service.state_of(&file, date), SyncState::Synced);
    // A file that changed after the backup is not in it.
    assert_eq!(service.state_of(&file, date + 1), SyncState::Pending);

    // Another backup place does not have the file. This is also saved.
    service.follow("s3://two/");
    assert_eq!(service.state_of(&file, date), SyncState::Pending);
    let expected = BackupState {
        destination: "s3://two/".to_owned(),
        ..BackupState::default()
    };
    assert_eq!(store.load(), expected);
}

#[tokio::test]
async fn backs_up_and_restores() {
    let temp = TempFolder::new();
    let root = temp.path().join("Files");
    std::fs::create_dir_all(root.join("Docs")).unwrap();
    std::fs::write(root.join("Docs").join("a.txt"), "hello").unwrap();
    std::fs::write(root.join(".hidden"), "x").unwrap();

    let remote = Arc::new(FolderBackup {
        folder: TempFolder::new(),
    });
    let secrets = Arc::new(MemorySecrets::default());
    let service = backup_service(temp.path(), remote.clone(), secrets);
    let mut config = config(BackupProvider::S3, "my-backup");
    config.prefix = "mac".to_owned();

    // Without keys there is no backup.
    assert!(matches!(service.plan(&config).await, Err(AppError::NoCredentials)));
    assert!(matches!(
        service.save_keys("", "secret", BackupProvider::S3),
        Err(AppError::MissingKeys)
    ));
    service
        .save_keys(" AKIAEXAMPLE1234 ", "secret", BackupProvider::S3)
        .unwrap();
    assert_eq!(service.saved_key_hint(BackupProvider::S3).as_deref(), Some("AKIA…1234"));

    // A file that is only in the backup, and keys that try to leave the drive.
    std::fs::write(remote.file("mac/only-remote.txt"), "remote").unwrap();
    std::fs::write(remote.file("mac/../evil.txt"), "evil").unwrap();
    std::fs::write(remote.file("mac//empty-part.txt"), "evil").unwrap();

    let plan = service.plan(&config).await.unwrap();
    assert_eq!(plan.uploads.len(), 1);
    assert_eq!(plan.uploads[0].key, "mac/Docs/a.txt");
    assert_eq!(plan.extra_keys.len(), 3);

    // The automatic backup uploads, and removes nothing.
    service.run(&plan, false).await.unwrap();
    assert!(remote.file("mac/Docs/a.txt").exists());
    assert!(remote.file("mac/only-remote.txt").exists());
    let local = root.join("Docs").join("a.txt");
    let modified = FileRepository::new().item(&local).unwrap().modified;
    assert_eq!(service.state_of(&local, modified), SyncState::Synced);
    assert_eq!(service.test_connection(&config).await.unwrap(), 4);

    // Restore brings back the missing file, and nothing from outside the drive.
    assert_eq!(service.restore(&config).await.unwrap(), 1);
    assert_eq!(std::fs::read_to_string(root.join("only-remote.txt")).unwrap(), "remote");
    assert!(!temp.path().join("evil.txt").exists());
    assert!(!root.join("evil.txt").exists());
    assert!(!root.join("empty-part.txt").exists());

    // Now only the two bad keys are extra. The manual backup may remove them.
    let plan = service.plan(&config).await.unwrap();
    assert_eq!(plan.uploads.len(), 0);
    assert_eq!(plan.extra_keys.len(), 2);
    service.run(&plan, true).await.unwrap();
    assert_eq!(service.test_connection(&config).await.unwrap(), 2);
}

#[test]
fn refuses_unsafe_bucket_names() {
    config(BackupProvider::S3, "my-drive-backup").validate().unwrap();
    assert!(matches!(
        config(BackupProvider::S3, "").validate(),
        Err(AppError::NotConfigured)
    ));
    assert!(config(BackupProvider::S3, "evil.com/x").validate().is_err());
    assert!(config(BackupProvider::S3, "-bad-").validate().is_err());
    let mut spaced = config(BackupProvider::S3, "good-name");
    spaced.region = "us east".to_owned();
    assert!(spaced.validate().is_err());
}

#[test]
fn builds_the_address_of_each_provider() {
    let mut config = config(BackupProvider::S3, "my-backup");
    config.region = "eu-west-1".to_owned();
    config.prefix = "mac".to_owned();
    assert_eq!(config.host(), "my-backup.s3.eu-west-1.amazonaws.com");
    assert_eq!(config.signing_region(), "eu-west-1");
    assert_eq!(config.destination(), "s3://my-backup/mac/");

    // Google does not use the region, so a bad one must not stop the backup.
    config.provider = BackupProvider::Gcs;
    config.region = "not used".to_owned();
    config.validate().unwrap();
    assert_eq!(config.host(), "my-backup.storage.googleapis.com");
    assert_eq!(config.signing_region(), "auto");
    assert_eq!(config.destination(), "gs://my-backup/mac/");

    config.provider = BackupProvider::Azure;
    assert!(config.validate().is_err());
    config.account = "evil.com/x".to_owned();
    assert!(config.validate().is_err());
    config.account = "mydrive".to_owned();
    config.validate().unwrap();
    assert_eq!(
        config.destination(),
        "https://mydrive.blob.core.windows.net/my-backup/mac/"
    );
}

#[tokio::test]
async fn builds_the_google_sign_in_address() {
    let auth = google_auth();
    let mut config = config(BackupProvider::GoogleDrive, "Drive Backup");
    // Without a good client ID there is no sign-in.
    assert!(auth.prepare(&config, "http://127.0.0.1:1234").is_err());
    config.client_id = "evil.com/x".to_owned();
    assert!(config.validate().is_err());

    config.client_id = "123-abc.apps.googleusercontent.com".to_owned();
    assert_eq!(config.destination(), "Google Drive/Drive Backup/");
    let sign_in = auth.prepare(&config, "http://127.0.0.1:1234").unwrap();
    assert!(
        sign_in
            .url
            .starts_with("https://accounts.google.com/o/oauth2/v2/auth?client_id=123-abc.apps.")
    );
    assert!(sign_in.url.contains("&redirect_uri=http%3A%2F%2F127.0.0.1%3A1234&"));
    assert!(
        sign_in
            .url
            .contains("&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.file&")
    );
    assert!(sign_in.url.contains("&code_challenge_method=S256&"));

    // The answer of Google is read from the address the browser asks for.
    let answer = GoogleAuthRepository::query_values("/?code=4%2Fabc&state=wrong+one");
    assert_eq!(answer.get("code").map(String::as_str), Some("4/abc"));
    assert_eq!(answer.get("state").map(String::as_str), Some("wrong one"));
    // The state is wrong, so it stops before any network call.
    assert!(matches!(
        auth.exchange(&answer, &sign_in, &config).await,
        Err(AppError::BadSettings(_))
    ));
}

#[test]
fn builds_paths_from_the_google_drive_list() {
    let file = |id: &str, name: &str, parent: &str, size: Option<&str>| GoogleFile {
        id: id.to_owned(),
        name: name.to_owned(),
        parents: Some(vec![parent.to_owned()]),
        mime_type: Some(
            if size.is_some() {
                "text/plain"
            } else {
                GoogleFile::FOLDER_TYPE
            }
            .to_owned(),
        ),
        size: size.map(str::to_owned),
        modified_time: None,
    };
    let mut files = vec![
        file("root", "Drive Backup", "my-drive", None),
        file("docs", "Docs", "root", None),
        file("a", "a.txt", "docs", Some("42")),
        file("b", "b.txt", "root", Some("7")),
        // A file of the app that is in another folder. It is not part of this backup.
        file("other", "Old Backup", "my-drive", None),
        file("c", "c.txt", "other", Some("1")),
    ];
    files[2].modified_time = Some("2009-10-09T21:04:30.000Z".to_owned());
    let drive = GoogleDriveRepository::new(google_auth(), signer(), Arc::new(HttpTransport::new()));

    let mut config = config(BackupProvider::GoogleDrive, "Drive Backup");
    let objects = drive.index(&files, &config);
    let mut keys = Vec::new();
    for object in &objects {
        keys.push(object.key.as_str());
    }
    keys.sort_unstable();
    assert_eq!(keys, ["Docs/a.txt", "b.txt"]);
    assert_eq!(objects[0].size, 42);
    assert_eq!(objects[0].last_modified, OCTOBER_2009);

    // Only the configured folder in the backup folder counts, like the prefix of a bucket.
    config.prefix = "Docs".to_owned();
    assert_eq!(drive.index(&files, &config).len(), 1);

    // No backup folder yet means an empty backup.
    config.bucket = "Not There".to_owned();
    assert_eq!(drive.index(&files, &config).len(), 0);
}

#[test]
fn builds_azure_requests() {
    let azure = AzureRepository::new(signer(), Arc::new(HttpTransport::new()));
    let mut config = config(BackupProvider::Azure, "my-backup");
    config.account = "mydrive".to_owned();
    let connection = BackupConnection {
        config,
        credentials: Credentials::new("", "sv=2022-11-02&sig=a%2Fb"),
    };

    assert_eq!(
        azure.address("Docs/My file+1.txt", &BTreeMap::new(), &connection),
        "https://mydrive.blob.core.windows.net/my-backup/Docs/My%20file%2B1.txt?sv=2022-11-02&sig=a%2Fb"
    );
    let mut query = BTreeMap::new();
    query.insert("comp", "list".to_owned());
    query.insert("prefix", "a b/".to_owned());
    assert_eq!(
        azure.address("", &query, &connection),
        "https://mydrive.blob.core.windows.net/my-backup?comp=list&prefix=a%20b%2F&sv=2022-11-02&sig=a%2Fb"
    );
}

#[test]
fn reads_the_azure_and_s3_lists() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <EnumerationResults ServiceEndpoint="https://mydrive.blob.core.windows.net/" ContainerName="my-backup">
          <Blobs>
            <Blob>
              <Name>Docs/a.txt</Name>
              <Properties>
                <Last-Modified>Fri, 09 Oct 2009 21:04:30 GMT</Last-Modified>
                <Content-Length>42</Content-Length>
              </Properties>
            </Blob>
            <Blob>
              <Name>Docs</Name>
              <Properties>
                <Last-Modified>Fri, 09 Oct 2009 21:04:30 GMT</Last-Modified>
                <Content-Length>0</Content-Length>
                <ResourceType>directory</ResourceType>
              </Properties>
            </Blob>
          </Blobs>
          <NextMarker>next-page</NextMarker>
        </EnumerationResults>"#;
    let (objects, marker) = AzureRepository::page(xml.as_bytes()).unwrap();
    assert_eq!(marker.as_deref(), Some("next-page"));
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].key, "Docs/a.txt");
    assert_eq!(objects[0].size, 42);
    assert_eq!(objects[0].last_modified, OCTOBER_2009);

    let (objects, marker) = AzureRepository::page(b"<EnumerationResults><NextMarker /></EnumerationResults>").unwrap();
    assert!(marker.is_none());
    assert_eq!(objects.len(), 0);

    // S3 puts a namespace on its answer, and escapes "&" in keys.
    let s3 = r#"<?xml version="1.0" encoding="UTF-8"?>
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
          <Contents>
            <Key>Docs/a &amp; b.txt</Key>
            <LastModified>2009-10-09T21:04:30.000Z</LastModified>
            <Size>42</Size>
          </Contents>
          <NextContinuationToken>token-2</NextContinuationToken>
        </ListBucketResult>"#;
    let (objects, token) = S3Repository::page(s3.as_bytes()).unwrap();
    assert_eq!(token.as_deref(), Some("token-2"));
    assert_eq!(objects[0].key, "Docs/a & b.txt");
    assert_eq!(objects[0].size, 42);
    assert_eq!(objects[0].last_modified, OCTOBER_2009);
}

/// Plays the upload server of Google on this computer, until the file is whole. It breaks one time
/// in the middle of a part, the way a bad network does. Returns the bytes it got.
async fn fake_google_upload(listener: tokio::net::TcpListener, size: usize) -> Vec<u8> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stored = Vec::new();
    let mut has_failed = false;
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let head_end = loop {
            if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                break end + 4;
            }
            let mut buffer = [0; 8 * 1024];
            let count = stream.read(&mut buffer).await.unwrap();
            assert!(count > 0, "the request stopped before its headers were done");
            request.extend_from_slice(&buffer[..count]);
        };
        let head = String::from_utf8_lossy(&request[..head_end]).to_lowercase();
        let header = |name: &str| {
            let line = head.lines().find(|line| line.starts_with(name));
            line.map_or("", |line| line[name.len()..].trim()).to_owned()
        };
        let mut body = request[head_end..].to_vec();
        body.resize(header("content-length:").parse().unwrap(), 0);
        let already_read = request.len() - head_end;
        stream.read_exact(&mut body[already_read..]).await.unwrap();

        let range = header("content-range:");
        let mut status = "308 Resume Incomplete";
        if !range.starts_with("bytes */") {
            let start: usize = range["bytes ".len()..].split('-').next().unwrap().parse().unwrap();
            assert_eq!(start, stored.len(), "a part must start where the last one ended");
            if has_failed || start == 0 {
                stored.extend_from_slice(&body);
            } else {
                // Keep only half of this part, and say the server broke.
                has_failed = true;
                stored.extend_from_slice(&body[..body.len() / 2]);
                status = "503 Service Unavailable";
            }
        }
        let is_done = stored.len() == size;
        let (status, answer) = if is_done {
            ("200 OK", "{\"id\":\"file-1\"}")
        } else {
            (status, "")
        };
        let have = if stored.is_empty() {
            String::new()
        } else {
            format!("Range: bytes=0-{}\r\n", stored.len() - 1)
        };
        let reply = format!(
            "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n{have}\r\n{answer}",
            answer.len()
        );
        stream.write_all(reply.as_bytes()).await.unwrap();
        if is_done {
            return stored;
        }
    }
}

#[tokio::test]
async fn sends_a_big_file_to_google_in_parts() {
    assert_eq!(GoogleDriveRepository::next_offset(None), 0);
    assert_eq!(GoogleDriveRepository::next_offset(Some("bytes=0-99")), 100);

    // Two and a half parts, so the last part is a short one.
    let size = usize::try_from(AppConstants::UPLOAD_PART_BYTES).unwrap() * 5 / 2;
    let mut content = Vec::with_capacity(size);
    for index in 0..size {
        content.push(u8::try_from(index % 251).unwrap());
    }
    let folder = TempFolder::new();
    let file = folder.path().join("big.bin");
    std::fs::write(&file, &content).unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let location = format!("http://{}/upload", listener.local_addr().unwrap());
    let server = tokio::spawn(fake_google_upload(listener, size));

    let google = GoogleDriveRepository::new(google_auth(), signer(), Arc::new(HttpTransport::new()));
    let sent = google.send_in_parts(&location, &file).await.unwrap();

    assert_eq!(sent.id, "file-1");
    assert!(
        server.await.unwrap() == content,
        "Google must get the same bytes, in the same order"
    );
}

/// Stands in for S3 or Azure. It keeps the parts in the memory, and the network breaks one time.
#[derive(Default)]
struct MemoryParts {
    parts: Mutex<BTreeMap<u64, Vec<u8>>>,
    has_failed: Mutex<bool>,
}

#[async_trait]
impl PartTarget for MemoryParts {
    async fn send_part(&self, number: u64, bytes: Vec<u8>) -> AppResult<HttpReply> {
        if number == 2 && !std::mem::replace(&mut *self.has_failed.lock(), true) {
            return Err(AppError::Network("The line broke.".to_owned()));
        }
        self.parts.lock().insert(number, bytes);
        Ok(HttpReply {
            status: 200,
            location: None,
            range: None,
            etag: Some(format!("tag-{number}")),
            body: Vec::new(),
        })
    }
}

#[tokio::test]
async fn sends_a_big_file_in_parts() {
    // Five and a half parts: more parts than workers, and the last part is a short one.
    let size = AppConstants::UPLOAD_PART_BYTES * 11 / 2;
    let mut content = Vec::new();
    for index in 0..size {
        content.push(u8::try_from(index % 251).unwrap());
    }
    let folder = TempFolder::new();
    let file = folder.path().join("big.bin");
    std::fs::write(&file, &content).unwrap();

    let target = MemoryParts::default();
    let tags = PartUpload::run(&target, &file, size).await.unwrap();

    assert_eq!(tags, ["tag-1", "tag-2", "tag-3", "tag-4", "tag-5", "tag-6"]);
    let joined = target.parts.lock().values().flatten().copied().collect::<Vec<u8>>();
    assert!(
        joined == content,
        "the parts must give the same file when they are joined"
    );

    // A very big file gets bigger parts, so S3 never sees too many.
    assert_eq!(PartUpload::part_size(size), AppConstants::UPLOAD_PART_BYTES);
    let huge = AppConstants::UPLOAD_PART_BYTES * AppConstants::UPLOAD_MAX_PARTS * 3;
    assert_eq!(PartUpload::part_size(huge), AppConstants::UPLOAD_PART_BYTES * 3);
    assert!(!PartUpload::is_needed(AppConstants::UPLOAD_PART_BYTES));
}

#[test]
fn builds_the_calls_that_join_the_parts() {
    assert_eq!(
        S3Repository::complete_body(&["\"a\"".to_owned(), "\"b\"".to_owned()]),
        "<CompleteMultipartUpload><Part><PartNumber>1</PartNumber><ETag>\"a\"</ETag></Part>\
         <Part><PartNumber>2</PartNumber><ETag>\"b\"</ETag></Part></CompleteMultipartUpload>"
    );
    // "00000001" and "00000002" in base64.
    assert_eq!(
        AzureRepository::block_list(2),
        "<BlockList><Latest>MDAwMDAwMDE=</Latest><Latest>MDAwMDAwMDI=</Latest></BlockList>"
    );
}
