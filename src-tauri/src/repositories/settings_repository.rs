use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use parking_lot::Mutex;

use crate::repositories::atomic_file::AtomicFile;

/// Keeps the settings as names and values in one JSON file.
#[derive(Debug)]
pub struct SettingsRepository {
    file: PathBuf,
    values: Mutex<BTreeMap<String, String>>,
}

impl SettingsRepository {
    pub fn new(file: PathBuf) -> Self {
        let text = fs::read_to_string(&file).unwrap_or_default();
        let values = serde_json::from_str(&text).unwrap_or_default();
        Self {
            file,
            values: Mutex::new(values),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.values.lock().get(key).cloned()
    }

    pub fn set(&self, key: &str, value: &str) {
        let mut values = self.values.lock();
        values.insert(key.to_owned(), value.to_owned());
        self.write(&values);
    }

    // ponytail: a failed write is not shown. The setting then works until the app closes, and is the old one after.
    fn write(&self, values: &BTreeMap<String, String>) {
        if let Some(folder) = self.file.parent() {
            let _ = fs::create_dir_all(folder);
        }
        if let Ok(text) = serde_json::to_string_pretty(values) {
            let _ = AtomicFile::write(&self.file, &text);
        }
    }
}
