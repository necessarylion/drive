use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::constants::AppConstants;
use crate::models::drive::FileKind;

/// A window that shows one file. Each file gets its own window, so closing it leaves the main window alone.
pub struct ViewerWindow;

impl ViewerWindow {
    const PAGE: &'static str = "viewer.html";

    /// Opens the window for this file. A file that is already open only comes to the front.
    pub fn open(app: &AppHandle, path: &Path) -> tauri::Result<()> {
        // The label of a window may only hold plain letters, so the path goes in as a number.
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        let label = format!("{}{:x}", AppConstants::VIEWER_WINDOW_PREFIX, hasher.finish());
        if let Some(window) = app.get_webview_window(&label) {
            return window.set_focus();
        }

        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let item = serde_json::json!({ "path": path, "name": name, "kind": FileKind::of(path, false) });
        // Start from the main window, so the title bar and the sizes are the same on every system.
        let mut config = app.config().app.windows.first().cloned().unwrap_or_default();
        config.label = label;
        config.title = name;
        config.url = WebviewUrl::App(Self::PAGE.into());
        WebviewWindowBuilder::from_config(app, &config)?
            // The page reads this to know which file to show.
            .initialization_script(format!("window.DRIVE_VIEWED = {item};"))
            .build()?;
        Ok(())
    }
}
