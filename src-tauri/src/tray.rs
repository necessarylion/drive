use std::sync::Arc;

use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::app_menu::AppMenu;
use crate::controllers::backup_controller::BackupController;
use crate::controllers::drive_controller::DriveController;

/// The Drive icon in the menu bar (Mac) or in the system tray (Windows, Linux), with its menu.
// ponytail: a plain menu, not the small window with the recent files that the Swift app had.
// Build a second webview window for it if the menu is not enough.
pub struct TrayMenu;

impl TrayMenu {
    const OPEN: &'static str = "open";
    const BACK_UP: &'static str = "back-up";
    const PAUSE: &'static str = "pause";
    const FILES: &'static str = "files";
    const SETTINGS: &'static str = "tray-settings";
    const QUIT: &'static str = "quit";

    pub fn install(app: &App) -> tauri::Result<()> {
        let open = MenuItem::with_id(app, Self::OPEN, "Open Drive", true, None::<&str>)?;
        let back_up = MenuItem::with_id(app, Self::BACK_UP, "Back Up Now", true, None::<&str>)?;
        let pause = MenuItem::with_id(app, Self::PAUSE, "Pause or Resume Backup", true, None::<&str>)?;
        let files = MenuItem::with_id(app, Self::FILES, "Show Files", true, None::<&str>)?;
        let settings = MenuItem::with_id(app, Self::SETTINGS, "Settings…", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, Self::QUIT, "Quit Drive", true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&open, &back_up, &pause, &files, &settings, &quit])?;
        let mut tray = TrayIconBuilder::new()
            .tooltip("Drive")
            .menu(&menu)
            .on_menu_event(Self::handle);
        if let Some(icon) = app.default_window_icon() {
            tray = tray.icon(icon.clone());
        }
        tray.build(app)?;
        Ok(())
    }

    // Tauri gives the event by value, so the shape of this function is fixed.
    #[allow(clippy::needless_pass_by_value)]
    fn handle(app: &AppHandle, event: MenuEvent) {
        let backup = app.state::<Arc<BackupController>>();
        match event.id().as_ref() {
            Self::OPEN => AppMenu::show_window(app),
            // This one uploads new and changed files. It never removes anything from the backup.
            Self::BACK_UP => backup.auto_back_up(),
            Self::PAUSE => backup.toggle_pause(),
            Self::FILES => {
                let root = app
                    .state::<Arc<DriveController>>()
                    .root_folder()
                    .to_string_lossy()
                    .into_owned();
                let _ = app.opener().open_path(root, None::<&str>);
            }
            Self::SETTINGS => AppMenu::open_settings(app),
            Self::QUIT => app.exit(0),
            _ => {}
        }
    }
}
