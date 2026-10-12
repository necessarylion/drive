use std::collections::HashMap;

use parking_lot::Mutex;

use crate::models::backup::{BackupProvider, Credentials};
use crate::models::error::{AppError, AppResult};

/// Keeps the backup keys in a safe place. Each provider has its own keys.
pub trait SecretRepository: Send + Sync {
    fn load(&self, provider: BackupProvider) -> Option<Credentials>;
    fn save(&self, credentials: &Credentials, provider: BackupProvider) -> AppResult<()>;
    fn delete(&self, provider: BackupProvider);
}

/// Keeps the keys in the keychain of the system, so they are never in a plain file of ours:
/// the Keychain on a Mac, the Credential Manager on Windows, the Secret Service on Linux.
pub struct KeychainRepository {
    service: String,
    /// What we already read. macOS asks the user on every read of the Keychain when they
    /// press "Allow" and not "Always Allow", so we read each key only one time per launch.
    /// `None` is kept too: a read that gave nothing (no keys, or the user said no) is not tried again.
    cache: Mutex<HashMap<BackupProvider, Option<Credentials>>>,
}

impl KeychainRepository {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_owned(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn entry(&self, provider: BackupProvider) -> AppResult<keyring::Entry> {
        keyring::Entry::new(&self.service, provider.raw()).map_err(|error| Self::failed(&error))
    }

    fn read(&self, provider: BackupProvider) -> Option<Credentials> {
        let text = self.entry(provider).ok()?.get_password().ok()?;
        serde_json::from_str(&text).ok()
    }

    fn failed(error: &keyring::Error) -> AppError {
        AppError::KeychainFailed(error.to_string())
    }
}

impl SecretRepository for KeychainRepository {
    fn load(&self, provider: BackupProvider) -> Option<Credentials> {
        // The lock is held during the read on purpose. Two reads at the same time (the start of the
        // app does that) would each make macOS ask. Now the second one waits and gets the cached keys.
        let mut cache = self.cache.lock();
        if let Some(cached) = cache.get(&provider) {
            return cached.clone();
        }
        let credentials = self.read(provider);
        cache.insert(provider, credentials.clone());
        credentials
    }

    fn save(&self, credentials: &Credentials, provider: BackupProvider) -> AppResult<()> {
        let text = serde_json::to_string(credentials).map_err(|error| AppError::KeychainFailed(error.to_string()))?;
        self.entry(provider)?
            .set_password(&text)
            .map_err(|error| Self::failed(&error))?;
        self.cache.lock().insert(provider, Some(credentials.clone()));
        Ok(())
    }

    fn delete(&self, provider: BackupProvider) {
        self.cache.lock().insert(provider, None);
        if let Ok(entry) = self.entry(provider) {
            let _ = entry.delete_credential();
        }
    }
}
