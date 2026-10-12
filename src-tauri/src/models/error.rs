use serde::{Serialize, Serializer};

pub type AppResult<T> = Result<T, AppError>;

/// Every way an action can fail. The text is what the user reads.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("This name is not allowed. Do not use “/”, “\\” or “:”, and do not start with a dot.")]
    InvalidName,
    #[error("The name “{0}” is already used in this folder.")]
    NameTaken(String),
    #[error("A folder cannot be moved into itself.")]
    IntoItself,
    #[error("Choose a folder inside your Drive.")]
    OutsideDrive,
    #[error("Set the bucket or folder first. Open Settings.")]
    NotConfigured,
    #[error("{0}")]
    BadSettings(String),
    #[error("No access keys found. Add them in Settings.")]
    NoCredentials,
    #[error("Fill in the access keys first.")]
    MissingKeys,
    #[error("The keys could not be saved in the keychain of this computer. {0}")]
    KeychainFailed(String),
    #[error("The backup server said no ({0}). {1}")]
    RequestFailed(u16, String),
    #[error("The backup server could not be reached. {0}")]
    Network(String),
    #[error("A backup is running. Try again when it is done.")]
    Busy,
    /// The user paused the backup. It is not shown as an error.
    #[error("Stopped.")]
    Cancelled,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        // The address can hold a secret (the Azure SAS token), so it must not get into the message.
        Self::Network(error.without_url().to_string())
    }
}

/// Tauri sends an error to the window as its text.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
