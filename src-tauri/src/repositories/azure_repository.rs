use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::RequestBuilder;

use crate::constants::AppConstants;
use crate::models::backup::{BackupConnection, RemoteObject};
use crate::models::error::{AppError, AppResult};
use crate::repositories::backup_repository::BackupRepository;
use crate::repositories::date_reader::DateReader;
use crate::repositories::http_transport::{HttpReply, HttpTransport};
use crate::repositories::part_upload::{PartTarget, PartUpload};
use crate::repositories::xml_reader::XmlReader;
use crate::services::aws_signer::AwsSigner;

/// One big file on its way to Azure, in parts. Azure calls a part a "block".
struct AzureBlocks<'a> {
    repository: &'a AzureRepository,
    key: &'a str,
    connection: &'a BackupConnection,
}

#[async_trait]
impl PartTarget for AzureBlocks<'_> {
    async fn send_part(&self, number: u64, bytes: Vec<u8>) -> AppResult<HttpReply> {
        let mut query = BTreeMap::new();
        query.insert("comp", "block".to_owned());
        query.insert("blockid", AzureRepository::block_id(number));
        let request = self.repository.request("PUT", self.key, &query, self.connection)?;
        let limit = Duration::from_secs(AppConstants::UPLOAD_PART_SECONDS);
        self.repository.http.send(request.timeout(limit).body(bytes)).await
    }
}

/// All network calls to Azure Blob Storage.
// ponytail: it signs in with a SAS token, and that token has an end date.
// Sign each request with the account key if making new tokens gets annoying.
pub struct AzureRepository {
    /// Only used for its URL encoding.
    signer: Arc<AwsSigner>,
    http: Arc<HttpTransport>,
}

impl AzureRepository {
    /// The version of the Azure API these calls are written for.
    const API_VERSION: &'static str = "2021-08-06";

    pub fn new(signer: Arc<AwsSigner>, http: Arc<HttpTransport>) -> Self {
        Self { signer, http }
    }

    /// Reads one page of the blob list. The marker is `None` on the last page.
    pub fn page(body: &[u8]) -> AppResult<(Vec<RemoteObject>, Option<String>)> {
        let text = String::from_utf8_lossy(body);
        let document = roxmltree::Document::parse(&text)
            .map_err(|_| AppError::RequestFailed(0, "The answer of the server could not be read.".to_owned()))?;
        let root = document.root_element();
        let mut objects = Vec::new();
        let blobs = XmlReader::child(root, "Blobs").map(|node| XmlReader::children(node, "Blob"));
        for blob in blobs.unwrap_or_default() {
            let Some(properties) = XmlReader::child(blob, "Properties") else {
                continue;
            };
            // A storage account with real folders also lists the folders. They are not files.
            if XmlReader::text(properties, "ResourceType") == "directory" {
                continue;
            }
            objects.push(RemoteObject {
                key: XmlReader::text(blob, "Name").to_owned(),
                size: XmlReader::text(properties, "Content-Length").parse().unwrap_or(0),
                last_modified: DateReader::rfc1123(XmlReader::text(properties, "Last-Modified")),
            });
        }
        let marker = XmlReader::text(root, "NextMarker");
        Ok((
            objects,
            if marker.is_empty() {
                None
            } else {
                Some(marker.to_owned())
            },
        ))
    }

    /// The name of a block. Azure wants base64, and the same length for every block of a file.
    pub fn block_id(number: u64) -> String {
        STANDARD.encode(format!("{number:08}"))
    }

    /// The body of the call that joins the blocks into one file.
    pub fn block_list(count: u64) -> String {
        let mut body = String::from("<BlockList>");
        for number in 1..=count {
            let _ = write!(body, "<Latest>{}</Latest>", Self::block_id(number));
        }
        body + "</BlockList>"
    }

    /// Sends a big file in blocks, and tells Azure to join them.
    /// Azure removes the blocks of a file that was never joined by itself, after one week.
    async fn upload_in_blocks(
        &self,
        file: &Path,
        size: u64,
        key: &str,
        connection: &BackupConnection,
    ) -> AppResult<()> {
        let blocks = AzureBlocks {
            repository: self,
            key,
            connection,
        };
        let count = PartUpload::run(&blocks, file, size).await?.len();
        let mut query = BTreeMap::new();
        query.insert("comp", "blocklist".to_owned());
        let request = self.request("PUT", key, &query, connection)?;
        let body = Self::block_list(count as u64);
        self.http.send(request.body(body)).await?.check_xml()
    }

    /// The address of a request. The SAS token goes at the end. It is the sign-in, so no header is signed.
    pub fn address(&self, key: &str, query: &BTreeMap<&str, String>, connection: &BackupConnection) -> String {
        let config = &connection.config;
        let mut path = format!("/{}", config.bucket);
        if !key.is_empty() {
            path = format!("{path}/{}", self.signer.uri_encode(key, true));
        }
        let mut pairs = Vec::new();
        for (name, value) in query {
            pairs.push(format!("{name}={}", self.signer.uri_encode(value, false)));
        }
        // The token is already encoded by Azure.
        pairs.push(connection.credentials.secret_key.clone());
        format!("https://{}{path}?{}", config.host(), pairs.join("&"))
    }

    fn request(
        &self,
        method: &str,
        key: &str,
        query: &BTreeMap<&str, String>,
        connection: &BackupConnection,
    ) -> AppResult<RequestBuilder> {
        let request = self.http.request(method, &self.address(key, query, connection))?;
        Ok(request.header("x-ms-version", Self::API_VERSION))
    }
}

#[async_trait]
impl BackupRepository for AzureRepository {
    async fn list(&self, connection: &BackupConnection) -> AppResult<Vec<RemoteObject>> {
        let mut objects = Vec::new();
        let mut marker: Option<String> = None;
        loop {
            let mut query = BTreeMap::new();
            query.insert("restype", "container".to_owned());
            query.insert("comp", "list".to_owned());
            query.insert("prefix", connection.config.key_prefix());
            if let Some(marker) = &marker {
                query.insert("marker", marker.clone());
            }
            let reply = self.http.send(self.request("GET", "", &query, connection)?).await?;
            reply.check_xml()?;
            let (mut page, next) = Self::page(&reply.body)?;
            objects.append(&mut page);
            marker = next;
            if marker.is_none() {
                return Ok(objects);
            }
        }
    }

    async fn upload(&self, file: &Path, key: &str, connection: &BackupConnection) -> AppResult<()> {
        let size = tokio::fs::metadata(file).await?.len();
        if PartUpload::is_needed(size) {
            return self.upload_in_blocks(file, size, key, connection).await;
        }
        let request = self.request("PUT", key, &BTreeMap::new(), connection)?;
        self.http
            .upload(request.header("x-ms-blob-type", "BlockBlob"), file)
            .await?
            .check_xml()
    }

    async fn download(&self, key: &str, destination: &Path, connection: &BackupConnection) -> AppResult<()> {
        let request = self.request("GET", key, &BTreeMap::new(), connection)?;
        self.http.download(request, destination).await?.check_xml()
    }

    async fn delete(&self, key: &str, connection: &BackupConnection) -> AppResult<()> {
        let request = self.request("DELETE", key, &BTreeMap::new(), connection)?;
        self.http.send(request).await?.check_xml()
    }
}
