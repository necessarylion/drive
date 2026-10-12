use std::path::Path;

use reqwest::header::{CONTENT_LENGTH, LOCATION};
use reqwest::{Body, Client, Method, RequestBuilder, Response, Url};
use tokio::io::AsyncWriteExt;

use crate::models::error::{AppError, AppResult};
use crate::repositories::xml_reader::XmlReader;

/// What a server answered.
#[derive(Debug, Clone)]
pub struct HttpReply {
    pub status: u16,
    /// The "Location" header. Google uses it to say where a file must be sent.
    pub location: Option<String>,
    pub body: Vec<u8>,
}

impl HttpReply {
    pub fn is_ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Fails when the server did not say OK. S3 and Azure put the reason in a "Message" tag.
    pub fn check_xml(&self) -> AppResult<()> {
        if self.is_ok() {
            return Ok(());
        }
        let text = String::from_utf8_lossy(&self.body);
        let message = match roxmltree::Document::parse(&text) {
            Ok(document) => XmlReader::text(document.root_element(), "Message").to_owned(),
            Err(_) => String::new(),
        };
        Err(self.failed(message))
    }

    /// Google sends its errors as JSON, not as XML like the other providers.
    pub fn check_json(&self) -> AppResult<()> {
        if self.is_ok() {
            return Ok(());
        }
        let json: serde_json::Value = serde_json::from_slice(&self.body).unwrap_or_default();
        Err(self.failed(json["error"]["message"].as_str().unwrap_or("").to_owned()))
    }

    fn failed(&self, message: String) -> AppError {
        let message = if message.is_empty() {
            "No details.".to_owned()
        } else {
            message
        };
        AppError::RequestFailed(self.status, message)
    }
}

/// Sends the HTTP requests of all backup providers.
#[derive(Debug, Default)]
pub struct HttpTransport {
    client: Client,
}

impl HttpTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request(&self, method: &str, address: &str) -> AppResult<RequestBuilder> {
        let bad = || AppError::BadSettings("The address could not be built. Check the backup settings.".to_owned());
        let method = Method::from_bytes(method.as_bytes()).map_err(|_| bad())?;
        let url = Url::parse(address).map_err(|_| bad())?;
        Ok(self.client.request(method, url))
    }

    pub async fn send(&self, request: RequestBuilder) -> AppResult<HttpReply> {
        Self::reply(request.send().await?).await
    }

    /// Sends the file as the body. It is read piece by piece, so a big file does not fill the memory.
    pub async fn upload(&self, request: RequestBuilder, file: &Path) -> AppResult<HttpReply> {
        let handle = tokio::fs::File::open(file).await?;
        let size = handle.metadata().await?.len();
        self.send(request.header(CONTENT_LENGTH, size).body(Body::from(handle)))
            .await
    }

    /// Writes the body of the answer into the file. When the server says no, no file is made
    /// and the answer holds the reason.
    pub async fn download(&self, request: RequestBuilder, destination: &Path) -> AppResult<HttpReply> {
        let mut response = request.send().await?;
        if !response.status().is_success() {
            return Self::reply(response).await;
        }
        let status = response.status().as_u16();
        let mut file = tokio::fs::File::create(destination).await?;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(HttpReply {
            status,
            location: None,
            body: Vec::new(),
        })
    }

    async fn reply(response: Response) -> AppResult<HttpReply> {
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response.bytes().await?.to_vec();
        Ok(HttpReply { status, location, body })
    }
}
