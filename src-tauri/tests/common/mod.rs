use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A new empty folder for one test. It is removed when the test ends.
pub struct TempFolder {
    path: PathBuf,
}

impl TempFolder {
    pub fn new() -> Self {
        let number = COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!("DriveTests-{}-{number}", std::process::id());
        let path = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the temp folder can be made");
        Self {
            path: dunce::canonicalize(&path).expect("the temp folder has a real path"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
