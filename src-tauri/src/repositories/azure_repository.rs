use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::RequestBuilder;

use crate::models::backup::{BackupConnection, RemoteObject};
use crate::models::error::{AppError, AppResult};
use crate::repositories::backup_repository::BackupRepository;
use crate::repositories::date_reader::DateReader;
use crate::repositories::http_transport::HttpTransport;
use crate::repositories::xml_reader::XmlReader;
use crate::services::aws_signer::AwsSigner;

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

    // ponytail: one PUT per file, so about 5 GB is the biggest file. Upload in blocks if you need more.
    async fn upload(&self, file: &Path, key: &str, connection: &BackupConnection) -> AppResult<()> {
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
