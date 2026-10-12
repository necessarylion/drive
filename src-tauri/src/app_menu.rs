use tauri::menu::{Menu, MenuEvent, MenuItem, MenuItemKind};
use tauri::{App, AppHandle, Emitter, Manager};

use crate::constants::AppConstants;

/// The menu bar at the top of the screen on a Mac. It is the standard one, plus "Settings…" in the Drive menu.
/// Windows and Linux have no menu bar. There, Settings opens from the status bar and from the tray.
pub struct AppMenu;

impl AppMenu {
    const SETTINGS: &'static str = "app-settings";

    pub fn install(app: &App) -> tauri::Result<()> {
        if !cfg!(target_os = "macos") {
            return Ok(());
        }
        let menu = Menu::default(app.handle())?;
        let settings = MenuItem::with_id(app, Self::SETTINGS, "Settings…", true, Some("CmdOrCtrl+,"))?;
        // The first menu is the one with the name of the app. Settings goes under "About Drive".
        if let Some(MenuItemKind::Submenu(drive_menu)) = menu.items()?.into_iter().next() {
            drive_menu.insert(&settings, 1)?;
        }
        app.set_menu(menu)?;
        app.on_menu_event(Self::handle);
        Ok(())
    }

    /// Brings the window to the front and shows the Settings dialog in it.
    pub fn open_settings(app: &AppHandle) {
        if let Some(window) = app.get_webview_window(AppConstants::MAIN_WINDOW) {
            let _ = window.show();
            let _ = window.set_focus();
        }
        let _ = app.emit(AppConstants::OPEN_SETTINGS_EVENT, ());
    }

    // Tauri gives the event by value, so the shape of this function is fixed.
    #[allow(clippy::needless_pass_by_value)]
    fn handle(app: &AppHandle, event: MenuEvent) {
        if event.id().as_ref() == Self::SETTINGS {
            Self::open_settings(app);
        }
    }
}
