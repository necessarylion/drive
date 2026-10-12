use serde::Deserialize;

/// One file or folder, the way the Google Drive API sends it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GoogleFile {
    pub id: String,
    pub name: String,
    pub parents: Option<Vec<String>>,
    pub mime_type: Option<String>,
    /// The size in bytes, as text. Folders have none.
    pub size: Option<String>,
    pub modified_time: Option<String>,
}

impl GoogleFile {
    pub const FOLDER_TYPE: &'static str = "application/vnd.google-apps.folder";

    pub fn is_folder(&self) -> bool {
        self.mime_type.as_deref() == Some(Self::FOLDER_TYPE)
    }

    pub fn parent(&self) -> &str {
        match &self.parents {
            Some(parents) if !parents.is_empty() => &parents[0],
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GoogleFileList {
    pub files: Vec<GoogleFile>,
    pub next_page_token: Option<String>,
}

/// The answer of the Google token address.
#[derive(Clone, Deserialize)]
pub struct GoogleToken {
    pub access_token: String,
    /// Seconds until the access token stops working.
    pub expires_in: u64,
    /// Only sent after a sign-in, not after a refresh.
    #[serde(default)]
    pub refresh_token: Option<String>,
}
