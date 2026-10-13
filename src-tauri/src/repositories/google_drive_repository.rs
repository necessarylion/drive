use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use reqwest::RequestBuilder;
use reqwest::header::{CONTENT_LENGTH, CONTENT_RANGE};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::config::backup_config::BackupConfig;
use crate::constants::AppConstants;
use crate::models::backup::{BackupConnection, RemoteObject};
use crate::models::error::{AppError, AppResult};
use crate::models::google::{GoogleFile, GoogleFileList};
use crate::repositories::backup_repository::BackupRepository;
use crate::repositories::date_reader::DateReader;
use crate::repositories::google_auth_repository::GoogleAuthRepository;
use crate::repositories::http_transport::HttpTransport;
use crate::services::aws_signer::AwsSigner;

/// What the last `list` found.
#[derive(Default)]
struct DriveIndex {
    /// Path -> Drive ID. The path "" is the backup folder itself.
    folder_ids: HashMap<String, String>,
    file_ids: HashMap<String, String>,
    /// The account and backup folder that the two maps above belong to.
    listed_for: String,
}

/// All network calls to Google Drive.
///
/// Google Drive has no paths. Each file has an ID and a parent folder. So `list` reads all files once
/// and builds the paths, and the other calls use the IDs it found.
pub struct GoogleDriveRepository {
    auth: Arc<GoogleAuthRepository>,
    /// Only used for its URL encoding.
    signer: Arc<AwsSigner>,
    http: Arc<HttpTransport>,
    index: Mutex<DriveIndex>,
    /// Held while a folder is looked up or made. Files go up at the same time, and two of them
    /// must not make the same folder twice.
    folder_lock: tokio::sync::Mutex<()>,
}

impl GoogleDriveRepository {
    const API: &'static str = "https://www.googleapis.com/drive/v3/files";
    const UPLOAD_API: &'static str = "https://www.googleapis.com/upload/drive/v3/files";
    const LIST_FIELDS: &'static str = "nextPageToken,files(id,name,parents,mimeType,size,modifiedTime)";
    const JSON_TYPE: &'static str = "application/json; charset=UTF-8";

    pub fn new(auth: Arc<GoogleAuthRepository>, signer: Arc<AwsSigner>, http: Arc<HttpTransport>) -> Self {
        Self {
            auth,
            signer,
            http,
            index: Mutex::new(DriveIndex::default()),
            folder_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Builds the paths from the flat file list of Google, and remembers the ID of each path.
    /// Returns the files that are in the backup folder, under the configured folder in it.
    pub fn index(&self, files: &[GoogleFile], config: &BackupConfig) -> Vec<RemoteObject> {
        let mut index = self.index.lock();
        index.folder_ids.clear();
        index.file_ids.clear();
        let mut by_id = HashMap::new();
        for file in files {
            by_id.insert(file.id.as_str(), file);
        }
        // The backup folder is the folder with the right name that is not inside another folder of ours.
        // ponytail: when two folders match, the one with the smaller ID wins. Rename one if that happens.
        let mut root_id = "";
        for file in files {
            let is_root = file.is_folder() && file.name == config.bucket && !by_id.contains_key(file.parent());
            if is_root && (root_id.is_empty() || file.id.as_str() < root_id) {
                root_id = &file.id;
            }
        }
        if root_id.is_empty() {
            return Vec::new();
        }
        index.folder_ids.insert(String::new(), root_id.to_owned());

        let prefix = config.key_prefix();
        let mut objects = Vec::new();
        for file in files {
            if file.id == root_id {
                continue;
            }
            let Some(path) = Self::path_of(file, &by_id, root_id) else {
                continue;
            };
            if file.is_folder() {
                index.folder_ids.insert(path, file.id.clone());
                continue;
            }
            index.file_ids.insert(path.clone(), file.id.clone());
            if !path.starts_with(&prefix) {
                continue;
            }
            objects.push(RemoteObject {
                key: path,
                size: file.size.as_deref().unwrap_or("").parse().unwrap_or(0),
                last_modified: DateReader::iso8601(file.modified_time.as_deref().unwrap_or("")),
            });
        }
        objects
    }

    /// "Docs/a.txt" for a file in the folder Docs of the backup folder. `None` when the file is not in the backup folder.
    fn path_of(file: &GoogleFile, by_id: &HashMap<&str, &GoogleFile>, root_id: &str) -> Option<String> {
        let mut names = vec![file.name.as_str()];
        let mut parent = file.parent();
        while parent != root_id {
            let folder = by_id.get(parent)?;
            names.insert(0, &folder.name);
            parent = folder.parent();
            // Guards against a loop in bad data.
            if names.len() > by_id.len() + 1 {
                return None;
            }
        }
        Some(names.join("/"))
    }

    /// The ID of the folder at this path. Makes the folder, and the folders above it, when they are missing.
    async fn folder_id(&self, path: &str, connection: &BackupConnection) -> AppResult<String> {
        let _lock = self.folder_lock.lock().await;
        // Walk from the backup folder ("") down to the wanted folder, one name at a time.
        let mut current = String::new();
        let mut parent_id = self
            .ensure_folder("", &connection.config.bucket, None, connection)
            .await?;
        for name in path.split('/') {
            if name.is_empty() {
                continue;
            }
            current = if current.is_empty() {
                name.to_owned()
            } else {
                format!("{current}/{name}")
            };
            parent_id = self.ensure_folder(&current, name, Some(&parent_id), connection).await?;
        }
        Ok(parent_id)
    }

    async fn ensure_folder(
        &self,
        path: &str,
        name: &str,
        parent_id: Option<&str>,
        connection: &BackupConnection,
    ) -> AppResult<String> {
        if let Some(id) = self.index.lock().folder_ids.get(path) {
            return Ok(id.clone());
        }
        let mut body = json!({ "mimeType": GoogleFile::FOLDER_TYPE, "name": name });
        if let Some(parent_id) = parent_id {
            body["parents"] = json!([parent_id]);
        }
        let request = self.request("POST", Self::API, connection).await?;
        let reply = self.http.send(Self::with_json(request, &body)).await?;
        reply.check_json()?;
        let id = Self::file_in(&reply.body)?.id;
        self.index.lock().folder_ids.insert(path.to_owned(), id.clone());
        Ok(id)
    }

    async fn file_id(&self, key: &str, connection: &BackupConnection) -> AppResult<String> {
        self.prepare(connection).await?;
        match self.index.lock().file_ids.get(key) {
            Some(id) => Ok(id.clone()),
            None => Err(AppError::RequestFailed(
                404,
                format!("“{key}” is not in Google Drive any more."),
            )),
        }
    }

    /// The IDs are only right for the account and folder that were listed. List again when they changed.
    async fn prepare(&self, connection: &BackupConnection) -> AppResult<()> {
        let is_listed = self.index.lock().listed_for == Self::tag(connection);
        if !is_listed {
            self.list(connection).await?;
        }
        Ok(())
    }

    fn tag(connection: &BackupConnection) -> String {
        format!("{}\n{}", connection.credentials.secret_key, connection.config.bucket)
    }

    async fn request(&self, method: &str, address: &str, connection: &BackupConnection) -> AppResult<RequestBuilder> {
        let token = self.auth.access_token(connection).await?;
        Ok(self.http.request(method, address)?.bearer_auth(token))
    }

    /// Sends the file to the address Google gave, one part after the other. Google joins the parts.
    /// When a part fails, it asks Google how much it has and goes on from there. So a short break
    /// in the network does not start a big file from zero.
    // ponytail: the parts go one by one, because Google does not take the parts of one file side by side.
    // A backup that is stopped starts the file again. Save `location` if it must go on after a restart.
    pub async fn send_in_parts(&self, location: &str, file: &Path) -> AppResult<GoogleFile> {
        let mut handle = tokio::fs::File::open(file).await?;
        let size = handle.metadata().await?.len();
        let mut offset = 0;
        let mut failures = 0;
        let mut must_ask = false;
        loop {
            let length = if must_ask {
                0
            } else {
                (size.saturating_sub(offset)).min(AppConstants::UPLOAD_PART_BYTES)
            };
            let mut part = vec![0; usize::try_from(length).expect("a part fits in the memory")];
            handle.seek(SeekFrom::Start(offset)).await?;
            handle.read_exact(&mut part).await?;
            let mut request = self
                .http
                .request("PUT", location)?
                .timeout(Duration::from_secs(AppConstants::UPLOAD_PART_SECONDS))
                // Google wants the length also when there are no bytes.
                .header(CONTENT_LENGTH, length)
                .body(part);
            if length > 0 {
                let range = format!("bytes {offset}-{}/{size}", offset + length - 1);
                request = request.header(CONTENT_RANGE, range);
            } else if size > 0 {
                // No bytes and a "*" asks Google how much of the file it has.
                request = request.header(CONTENT_RANGE, format!("bytes */{size}"));
            }

            let reply = match self.http.send(request).await {
                Ok(reply) if reply.status < 500 => reply,
                failed => {
                    failures += 1;
                    if failures >= AppConstants::UPLOAD_PART_TRIES {
                        // Give the reason of the last try: the network error, or what the server said.
                        failed?.check_json()?;
                    }
                    tokio::time::sleep(Duration::from_secs(failures)).await;
                    must_ask = true;
                    continue;
                }
            };
            must_ask = false;
            // 308 means "I have this much, send the rest".
            if reply.status != 308 {
                reply.check_json()?;
                return Self::file_in(&reply.body);
            }
            let next = Self::next_offset(reply.range.as_deref());
            if next > offset {
                failures = 0;
            } else {
                failures += 1;
                if failures >= AppConstants::UPLOAD_PART_TRIES {
                    return Err(AppError::RequestFailed(
                        308,
                        "Google does not take the file.".to_owned(),
                    ));
                }
            }
            offset = next;
        }
    }

    /// Where the next part starts. Google answers "bytes=0-99" when it has the first 100 bytes,
    /// and sends no range when it has nothing.
    pub fn next_offset(range: Option<&str>) -> u64 {
        let last = range.and_then(|range| range.rsplit_once('-')?.1.trim().parse::<u64>().ok());
        last.map_or(0, |last| last + 1)
    }

    fn with_json(request: RequestBuilder, body: &serde_json::Value) -> RequestBuilder {
        request.header("Content-Type", Self::JSON_TYPE).body(body.to_string())
    }

    fn file_in(body: &[u8]) -> AppResult<GoogleFile> {
        serde_json::from_slice(body)
            .map_err(|_| AppError::RequestFailed(0, "The answer of Google could not be read.".to_owned()))
    }
}

#[async_trait]
impl BackupRepository for GoogleDriveRepository {
    // ponytail: this reads every file the app ever made in this Google account, also the ones in another
    // backup folder. Ask Google for one folder at a time if that list gets slow.
    async fn list(&self, connection: &BackupConnection) -> AppResult<Vec<RemoteObject>> {
        let mut files = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut address = format!(
                "{}?pageSize=1000&q={}&fields={}",
                Self::API,
                self.signer.uri_encode("trashed = false", false),
                self.signer.uri_encode(Self::LIST_FIELDS, false)
            );
            if let Some(token) = &page_token {
                address = format!("{address}&pageToken={}", self.signer.uri_encode(token, false));
            }
            let reply = self.http.send(self.request("GET", &address, connection).await?).await?;
            reply.check_json()?;
            let mut page: GoogleFileList = serde_json::from_slice(&reply.body)
                .map_err(|_| AppError::RequestFailed(0, "The answer of Google could not be read.".to_owned()))?;
            files.append(&mut page.files);
            page_token = page.next_page_token;
            if page_token.is_none() {
                break;
            }
        }
        let objects = self.index(&files, &connection.config);
        self.index.lock().listed_for = Self::tag(connection);
        Ok(objects)
    }

    async fn upload(&self, file: &Path, key: &str, connection: &BackupConnection) -> AppResult<()> {
        self.prepare(connection).await?;
        // Step 1: tell Google what is coming. It answers with the address to send the file to.
        let known_id = self.index.lock().file_ids.get(key).cloned();
        let start = if let Some(id) = known_id {
            let address = format!("{}/{id}?uploadType=resumable", Self::UPLOAD_API);
            Self::with_json(self.request("PATCH", &address, connection).await?, &json!({}))
        } else {
            let (folder, name) = key.rsplit_once('/').unwrap_or(("", key));
            let folder_id = self.folder_id(folder, connection).await?;
            let address = format!("{}?uploadType=resumable", Self::UPLOAD_API);
            let body = json!({ "name": name, "parents": [folder_id] });
            Self::with_json(self.request("POST", &address, connection).await?, &body)
        };
        let reply = self.http.send(start).await?;
        reply.check_json()?;
        let Some(location) = reply.location else {
            return Err(AppError::RequestFailed(
                0,
                "Google did not say where to send the file.".to_owned(),
            ));
        };

        // Step 2: send the file.
        let id = self.send_in_parts(&location, file).await?.id;
        self.index.lock().file_ids.insert(key.to_owned(), id);
        Ok(())
    }

    async fn download(&self, key: &str, destination: &Path, connection: &BackupConnection) -> AppResult<()> {
        let id = self.file_id(key, connection).await?;
        let request = self
            .request("GET", &format!("{}/{id}?alt=media", Self::API), connection)
            .await?;
        self.http.download(request, destination).await?.check_json()
    }

    /// Moves the file to the trash of Google Drive. Google removes it for good after 30 days.
    async fn delete(&self, key: &str, connection: &BackupConnection) -> AppResult<()> {
        let id = self.file_id(key, connection).await?;
        let request = self
            .request("PATCH", &format!("{}/{id}", Self::API), connection)
            .await?;
        self.http
            .send(Self::with_json(request, &json!({ "trashed": true })))
            .await?
            .check_json()?;
        self.index.lock().file_ids.remove(key);
        Ok(())
    }
}
