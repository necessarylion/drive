use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::RequestBuilder;
use reqwest::header::CONTENT_LENGTH;

use crate::constants::AppConstants;
use crate::models::backup::{BackupConnection, RemoteObject};
use crate::models::error::{AppError, AppResult};
use crate::repositories::backup_repository::BackupRepository;
use crate::repositories::date_reader::DateReader;
use crate::repositories::http_transport::{HttpReply, HttpTransport};
use crate::repositories::part_upload::{PartTarget, PartUpload};
use crate::repositories::xml_reader::XmlReader;
use crate::services::aws_signer::{AwsSigner, SigningRequest};

/// One big file on its way to S3, in parts.
struct S3Parts<'a> {
    repository: &'a S3Repository,
    key: &'a str,
    upload_id: &'a str,
    connection: &'a BackupConnection,
}

#[async_trait]
impl PartTarget for S3Parts<'_> {
    async fn send_part(&self, number: u64, bytes: Vec<u8>) -> AppResult<HttpReply> {
        let mut query = BTreeMap::new();
        query.insert("partNumber", number.to_string());
        query.insert("uploadId", self.upload_id.to_owned());
        let request = self.repository.request("PUT", self.key, &query, self.connection)?;
        let limit = Duration::from_secs(AppConstants::UPLOAD_PART_SECONDS);
        self.repository.http.send(request.timeout(limit).body(bytes)).await
    }
}

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

    /// The body of the call that joins the parts. S3 wants the tag of each part back.
    pub fn complete_body(tags: &[String]) -> String {
        let mut body = String::from("<CompleteMultipartUpload>");
        for (index, tag) in tags.iter().enumerate() {
            let number = index + 1;
            let _ = write!(body, "<Part><PartNumber>{number}</PartNumber><ETag>{tag}</ETag></Part>");
        }
        body + "</CompleteMultipartUpload>"
    }

    /// The text of one tag at the top of an XML answer, or "".
    fn top_text(body: &[u8], name: &str) -> String {
        let text = String::from_utf8_lossy(body);
        match roxmltree::Document::parse(&text) {
            Ok(document) => XmlReader::text(document.root_element(), name).to_owned(),
            Err(_) => String::new(),
        }
    }

    /// Sends a big file in parts, and tells S3 to join them (a "multipart upload").
    // ponytail: when the backup is stopped in the middle, the parts stay in the bucket, hidden, and cost
    // money. Add a lifecycle rule to the bucket that removes unfinished multipart uploads after some days.
    async fn upload_in_parts(&self, file: &Path, size: u64, key: &str, connection: &BackupConnection) -> AppResult<()> {
        let mut query = BTreeMap::new();
        query.insert("uploads", String::new());
        let start = self.request("POST", key, &query, connection)?;
        let reply = self.http.send(start.header(CONTENT_LENGTH, 0)).await?;
        reply.check_xml()?;
        let upload_id = Self::top_text(&reply.body, "UploadId");
        if upload_id.is_empty() {
            return Err(AppError::RequestFailed(0, "The server gave no upload ID.".to_owned()));
        }

        let mut query = BTreeMap::new();
        query.insert("uploadId", upload_id.clone());
        let parts = S3Parts {
            repository: self,
            key,
            upload_id: &upload_id,
            connection,
        };
        let done = match PartUpload::run(&parts, file, size).await {
            Ok(tags) => self.complete(&tags, key, &query, connection).await,
            Err(error) => Err(error),
        };
        if done.is_err() {
            // Tell S3 to drop the parts it has. If this fails too, the first error is the one to show.
            if let Ok(abort) = self.request("DELETE", key, &query, connection) {
                let _ = self.http.send(abort).await;
            }
        }
        done
    }

    async fn complete(
        &self,
        tags: &[String],
        key: &str,
        query: &BTreeMap<&str, String>,
        connection: &BackupConnection,
    ) -> AppResult<()> {
        let request = self.request("POST", key, query, connection)?;
        let reply = self.http.send(request.body(Self::complete_body(tags))).await?;
        reply.check_xml()?;
        // S3 can say OK first and still fail. Then the answer holds an error in place of the result.
        let code = Self::top_text(&reply.body, "Code");
        if code.is_empty() {
            return Ok(());
        }
        let message = Self::top_text(&reply.body, "Message");
        Err(AppError::RequestFailed(reply.status, format!("{code}. {message}")))
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

    async fn upload(&self, file: &Path, key: &str, connection: &BackupConnection) -> AppResult<()> {
        let size = tokio::fs::metadata(file).await?.len();
        if PartUpload::is_needed(size) {
            return self.upload_in_parts(file, size, key, connection).await;
        }
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
