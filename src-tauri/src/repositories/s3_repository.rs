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
use crate::services::aws_signer::{AwsSigner, SigningRequest};

/// All network calls to S3. Google Cloud Storage speaks the same language, so it is used for that too.
pub struct S3Repository {
    signer: Arc<AwsSigner>,
    http: Arc<HttpTransport>,
}

impl S3Repository {
    pub fn new(signer: Arc<AwsSigner>, http: Arc<HttpTransport>) -> Self {
        Self { signer, http }
    }

    /// Reads one page of the object list. The token is `None` on the last page.
    pub fn page(body: &[u8]) -> AppResult<(Vec<RemoteObject>, Option<String>)> {
        let text = String::from_utf8_lossy(body);
        let document = roxmltree::Document::parse(&text)
            .map_err(|_| AppError::RequestFailed(0, "The answer of the server could not be read.".to_owned()))?;
        let root = document.root_element();
        let mut objects = Vec::new();
        for node in XmlReader::children(root, "Contents") {
            objects.push(RemoteObject {
                key: XmlReader::text(node, "Key").to_owned(),
                size: XmlReader::text(node, "Size").parse().unwrap_or(0),
                last_modified: DateReader::iso8601(XmlReader::text(node, "LastModified")),
            });
        }
        let token = XmlReader::text(root, "NextContinuationToken");
        Ok((objects, if token.is_empty() { None } else { Some(token.to_owned()) }))
    }

    fn request(
        &self,
        method: &str,
        key: &str,
        query: &BTreeMap<&str, String>,
        connection: &BackupConnection,
    ) -> AppResult<RequestBuilder> {
        let config = &connection.config;
        let path = format!("/{}", self.signer.uri_encode(key, true));
        let mut pairs = Vec::new();
        for (name, value) in query {
            pairs.push(format!(
                "{}={}",
                self.signer.uri_encode(name, false),
                self.signer.uri_encode(value, false)
            ));
        }
        let query_string = pairs.join("&");

        let mut headers = BTreeMap::new();
        headers.insert("host".to_owned(), config.host());
        headers.insert(
            "x-amz-content-sha256".to_owned(),
            AwsSigner::UNSIGNED_PAYLOAD.to_owned(),
        );
        headers.insert("x-amz-date".to_owned(), DateReader::amz_now());
        if let Some(token) = &connection.credentials.session_token {
            headers.insert("x-amz-security-token".to_owned(), token.clone());
        }
        let signing = SigningRequest {
            method,
            path: &path,
            query: &query_string,
            headers: &headers,
            payload_hash: AwsSigner::UNSIGNED_PAYLOAD,
            region: config.signing_region(),
        };
        let authorization = self.signer.authorization(&signing, &connection.credentials);

        let mut address = format!("https://{}{path}", config.host());
        if !query_string.is_empty() {
            address = format!("{address}?{query_string}");
        }
        let mut request = self.http.request(method, &address)?;
        // The HTTP client sets Host itself, from the address.
        for (name, value) in &headers {
            if name != "host" {
                request = request.header(name, value);
            }
        }
        Ok(request.header("Authorization", authorization))
    }
}

#[async_trait]
impl BackupRepository for S3Repository {
    /// Every object under the configured folder of the bucket.
    async fn list(&self, connection: &BackupConnection) -> AppResult<Vec<RemoteObject>> {
        let mut objects = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut query = BTreeMap::new();
            query.insert("list-type", "2".to_owned());
            query.insert("prefix", connection.config.key_prefix());
            if let Some(token) = &token {
                query.insert("continuation-token", token.clone());
            }
            let reply = self.http.send(self.request("GET", "", &query, connection)?).await?;
            reply.check_xml()?;
            let (mut page, next) = Self::page(&reply.body)?;
            objects.append(&mut page);
            token = next;
            if token.is_none() {
                return Ok(objects);
            }
        }
    }

    // ponytail: one PUT per file, so 5 GB is the biggest file. Use multipart upload if you need more.
    async fn upload(&self, file: &Path, key: &str, connection: &BackupConnection) -> AppResult<()> {
        let request = self.request("PUT", key, &BTreeMap::new(), connection)?;
        self.http.upload(request, file).await?.check_xml()
    }

    async fn download(&self, key: &str, destination: &Path, connection: &BackupConnection) -> AppResult<()> {
        let request = self.request("GET", key, &BTreeMap::new(), connection)?;
        self.http.download(request, destination).await?.check_xml()
    }

    // ponytail: one request per object. Use the batch delete call if removing thousands gets slow.
    async fn delete(&self, key: &str, connection: &BackupConnection) -> AppResult<()> {
        let request = self.request("DELETE", key, &BTreeMap::new(), connection)?;
        self.http.send(request).await?.check_xml()
    }
}
