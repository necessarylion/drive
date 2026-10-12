use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::config::backup_config::BackupConfig;
use crate::constants::AppConstants;
use crate::models::backup::BackupConnection;
use crate::models::error::{AppError, AppResult};
use crate::models::google::GoogleToken;
use crate::repositories::http_transport::HttpTransport;
use crate::services::aws_signer::AwsSigner;

/// One sign-in that waits for the user. It is made new for each sign-in.
pub struct GoogleSignIn {
    /// The Google page where the user signs in. Open it in the browser.
    pub url: String,
    /// Google sends the browser back to this address on this computer when the user is done.
    pub redirect_uri: String,
    /// Google checks the verifier, and we check the state.
    pub verifier: String,
    pub state: String,
    listener: Option<TcpListener>,
}

/// The last access token, and the refresh token it was made from.
struct TokenCache {
    access_token: String,
    owner: String,
    end: Instant,
}

/// Signs in to Google with OAuth 2, and gets access tokens for the Drive calls.
///
/// The sign-in runs in the browser of the user. Google then sends the browser to
/// "http://127.0.0.1:port", where this app listens for a moment. This works the same on every system.
/// It needs a client of the type "Desktop app" in the Google Cloud project.
pub struct GoogleAuthRepository {
    /// Only used for its URL encoding.
    signer: Arc<AwsSigner>,
    http: Arc<HttpTransport>,
    cache: Mutex<Option<TokenCache>>,
}

impl GoogleAuthRepository {
    /// With this scope, the app only sees the files it made itself. Google needs no review for it.
    const SCOPE: &'static str = "https://www.googleapis.com/auth/drive.file";
    const SIGN_IN_ADDRESS: &'static str = "https://accounts.google.com/o/oauth2/v2/auth";
    const TOKEN_ADDRESS: &'static str = "https://oauth2.googleapis.com/token";
    const DONE_PAGE: &'static str = "<html><body style=\"font-family:sans-serif;padding:40px\">\
        <h3>Drive</h3><p>You can close this page and go back to Drive.</p></body></html>";

    pub fn new(signer: Arc<AwsSigner>, http: Arc<HttpTransport>) -> Self {
        Self {
            signer,
            http,
            cache: Mutex::new(None),
        }
    }

    /// Starts a sign-in: opens a port on this computer and builds the address of the Google page.
    pub async fn begin(&self, config: &BackupConfig) -> AppResult<GoogleSignIn> {
        config.validate()?;
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let mut sign_in = self.prepare(config, &format!("http://127.0.0.1:{port}"))?;
        sign_in.listener = Some(listener);
        Ok(sign_in)
    }

    /// Builds the sign-in without the open port. `begin` adds the port.
    pub fn prepare(&self, config: &BackupConfig, redirect_uri: &str) -> AppResult<GoogleSignIn> {
        config.validate()?;
        let verifier = Self::random_text()?;
        let state = Self::random_text()?;
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let mut fields = BTreeMap::new();
        fields.insert("client_id", config.client_id.clone());
        fields.insert("code_challenge", challenge);
        fields.insert("code_challenge_method", "S256".to_owned());
        fields.insert("redirect_uri", redirect_uri.to_owned());
        fields.insert("response_type", "code".to_owned());
        fields.insert("scope", Self::SCOPE.to_owned());
        fields.insert("state", state.clone());
        Ok(GoogleSignIn {
            url: format!("{}?{}", Self::SIGN_IN_ADDRESS, self.form(&fields)),
            redirect_uri: redirect_uri.to_owned(),
            verifier,
            state,
            listener: None,
        })
    }

    /// Waits until Google sends the browser back, then gets the tokens.
    /// Returns the refresh token, which is what gets saved.
    pub async fn finish(&self, sign_in: &GoogleSignIn, config: &BackupConfig) -> AppResult<String> {
        let Some(listener) = &sign_in.listener else {
            return Err(Self::bad("The sign-in was not started. Try again."));
        };
        let limit = Duration::from_secs(AppConstants::GOOGLE_SIGN_IN_SECONDS);
        let Ok(answer) = tokio::time::timeout(limit, Self::wait_for_answer(listener)).await else {
            return Err(Self::bad("The Google sign-in took too long. Try again."));
        };
        self.exchange(&answer?, sign_in, config).await
    }

    /// Takes the values Google put in the address it sent the browser to, and gets the tokens for them.
    pub async fn exchange(
        &self,
        answer: &BTreeMap<String, String>,
        sign_in: &GoogleSignIn,
        config: &BackupConfig,
    ) -> AppResult<String> {
        // The state proves that this answer belongs to the sign-in we started.
        let state_fits = !sign_in.state.is_empty() && answer.get("state") == Some(&sign_in.state);
        let Some(code) = answer.get("code").filter(|_| state_fits) else {
            let reason = answer.get("error").map_or("Try again.", String::as_str);
            return Err(Self::bad(&format!("Google did not finish the sign-in. {reason}")));
        };
        let mut fields = BTreeMap::new();
        fields.insert("client_id", config.client_id.clone());
        fields.insert("code", code.clone());
        fields.insert("code_verifier", sign_in.verifier.clone());
        fields.insert("grant_type", "authorization_code".to_owned());
        fields.insert("redirect_uri", sign_in.redirect_uri.clone());
        let token = self.request_token(fields, config).await?;
        let Some(refresh_token) = token.refresh_token.clone() else {
            return Err(Self::bad("Google did not send a refresh token. Try to sign in again."));
        };
        self.remember(&token, &refresh_token);
        Ok(refresh_token)
    }

    /// A token that works now. It asks Google for a new one when the old one is about to end.
    pub async fn access_token(&self, connection: &BackupConnection) -> AppResult<String> {
        let refresh_token = &connection.credentials.secret_key;
        if let Some(cache) = self.cache.lock().as_ref() {
            if &cache.owner == refresh_token && cache.end > Instant::now() {
                return Ok(cache.access_token.clone());
            }
        }
        let mut fields = BTreeMap::new();
        fields.insert("client_id", connection.config.client_id.clone());
        fields.insert("grant_type", "refresh_token".to_owned());
        fields.insert("refresh_token", refresh_token.clone());
        let token = self.request_token(fields, &connection.config).await?;
        self.remember(&token, refresh_token);
        Ok(token.access_token)
    }

    fn remember(&self, token: &GoogleToken, owner: &str) {
        *self.cache.lock() = Some(TokenCache {
            access_token: token.access_token.clone(),
            owner: owner.to_owned(),
            // Stop using it one minute early, so it does not end in the middle of a request.
            end: Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(60)),
        });
    }

    async fn request_token(
        &self,
        mut fields: BTreeMap<&'static str, String>,
        config: &BackupConfig,
    ) -> AppResult<GoogleToken> {
        if !config.client_secret.is_empty() {
            fields.insert("client_secret", config.client_secret.clone());
        }
        let request = self
            .http
            .request("POST", Self::TOKEN_ADDRESS)?
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(self.form(&fields));
        let reply = self.http.send(request).await?;
        if !reply.is_ok() {
            let json: serde_json::Value = serde_json::from_slice(&reply.body).unwrap_or_default();
            let reason = json["error_description"]
                .as_str()
                .or(json["error"].as_str())
                .unwrap_or("No details.");
            return Err(Self::bad(&format!(
                "Google refused the sign-in. Sign in again in Settings. ({reason})"
            )));
        }
        serde_json::from_slice(&reply.body).map_err(|_| Self::bad("The answer of Google could not be read."))
    }

    /// Takes browser requests on the port until one has the answer of Google in its address.
    async fn wait_for_answer(listener: &TcpListener) -> AppResult<BTreeMap<String, String>> {
        loop {
            let (mut stream, _) = listener.accept().await?;
            let mut buffer = vec![0; 8192];
            let count = stream.read(&mut buffer).await?;
            let head = String::from_utf8_lossy(&buffer[..count]);
            // The first line looks like "GET /?code=abc&state=xyz HTTP/1.1".
            let target = head.split_whitespace().nth(1).unwrap_or("");
            let answer = Self::query_values(target);
            let page = Self::DONE_PAGE;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{page}",
                page.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            // A browser also asks for other things, like the page icon. Those have no answer in them.
            if answer.contains_key("code") || answer.contains_key("error") {
                return Ok(answer);
            }
        }
    }

    /// Reads "a=1&b=2" after the "?" of an address.
    pub fn query_values(target: &str) -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();
        let query = target.split_once('?').map_or("", |(_, query)| query);
        for pair in query.split('&') {
            if let Some((name, value)) = pair.split_once('=') {
                values.insert(Self::decode(name), Self::decode(value));
            }
        }
        values
    }

    /// Turns "%2F" back into "/" and "+" into a space.
    fn decode(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut decoded = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            let hex = text
                .get(index + 1..index + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            match (bytes[index], hex) {
                (b'%', Some(byte)) => {
                    decoded.push(byte);
                    index += 3;
                }
                (b'+', _) => {
                    decoded.push(b' ');
                    index += 1;
                }
                (byte, _) => {
                    decoded.push(byte);
                    index += 1;
                }
            }
        }
        String::from_utf8_lossy(&decoded).into_owned()
    }

    /// "a=1&b=2", with the names in order and the values encoded.
    fn form(&self, fields: &BTreeMap<&'static str, String>) -> String {
        let mut pairs = Vec::new();
        for (name, value) in fields {
            pairs.push(format!("{name}={}", self.signer.uri_encode(value, false)));
        }
        pairs.join("&")
    }

    fn random_text() -> AppResult<String> {
        let mut bytes = [0u8; 32];
        // The random numbers of the system are safe to use for secrets.
        getrandom::fill(&mut bytes).map_err(|error| AppError::Io(std::io::Error::other(error.to_string())))?;
        Ok(URL_SAFE_NO_PAD.encode(bytes))
    }

    fn bad(message: &str) -> AppError {
        AppError::BadSettings(message.to_owned())
    }
}
