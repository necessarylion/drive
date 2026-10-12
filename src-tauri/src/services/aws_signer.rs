use std::collections::BTreeMap;
use std::fmt::Write;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::models::backup::Credentials;

/// One request to sign.
pub struct SigningRequest<'a> {
    pub method: &'a str,
    /// The URL path, already encoded with `uri_encode`.
    pub path: &'a str,
    /// The query string, already encoded and sorted by name.
    pub query: &'a str,
    /// Every header to sign, with names in small letters. Needs "host" and "x-amz-date".
    pub headers: &'a BTreeMap<String, String>,
    pub payload_hash: &'a str,
    pub region: &'a str,
}

/// Signs S3 requests with AWS Signature Version 4.
#[derive(Debug, Default)]
pub struct AwsSigner;

impl AwsSigner {
    /// Tells S3 the body is not part of the signature. Allowed over HTTPS, and saves hashing big files.
    pub const UNSIGNED_PAYLOAD: &'static str = "UNSIGNED-PAYLOAD";

    pub fn new() -> Self {
        Self
    }

    /// Returns the value of the Authorization header.
    pub fn authorization(&self, request: &SigningRequest, credentials: &Credentials) -> String {
        let SigningRequest {
            method,
            path,
            query,
            headers,
            payload_hash,
            region,
        } = *request;
        let mut canonical_headers = String::new();
        let mut names = Vec::new();
        for (name, value) in headers {
            let _ = writeln!(canonical_headers, "{name}:{}", value.trim());
            names.push(name.as_str());
        }
        let signed_headers = names.join(";");
        let canonical_request = [method, path, query, &canonical_headers, &signed_headers, payload_hash].join("\n");

        let amz_date = headers.get("x-amz-date").map_or("", String::as_str);
        let day = amz_date.get(..8).unwrap_or("");
        let scope = format!("{day}/{region}/s3/aws4_request");
        let request_hash = Self::hex(&Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{request_hash}");

        let mut key = Self::hmac(format!("AWS4{}", credentials.secret_key).as_bytes(), day);
        key = Self::hmac(&key, region);
        key = Self::hmac(&key, "s3");
        key = Self::hmac(&key, "aws4_request");
        let signature = Self::hex(&Self::hmac(&key, &string_to_sign));

        format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope},SignedHeaders={signed_headers},Signature={signature}",
            credentials.access_key
        )
    }

    /// Percent-encodes the way AWS wants: all bytes but letters, numbers and "-_.~".
    pub fn uri_encode(&self, text: &str, keep_slash: bool) -> String {
        let mut encoded = String::with_capacity(text.len());
        for byte in text.bytes() {
            let is_plain = byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte);
            if is_plain || (keep_slash && byte == b'/') {
                encoded.push(char::from(byte));
            } else {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
        encoded
    }

    fn hmac(key: &[u8], message: &str) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes a key of any size");
        mac.update(message.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }

    fn hex(bytes: &[u8]) -> String {
        let mut text = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            let _ = write!(text, "{byte:02x}");
        }
        text
    }
}
