import { AppConstants } from './constants/app_constants.js';
import { BackupController } from './controllers/backup_controller.js';
import { DriveController } from './controllers/drive_controller.js';
import { BackupRepository } from './repositories/backup_repository.js';
import { DriveRepository } from './repositories/drive_repository.js';
import { SettingsRepository } from './repositories/settings_repository.js';
import { TauriGateway } from './repositories/tauri_gateway.js';
import { BrowserView } from './views/browser_view.js';
import { DialogView } from './views/dialog_view.js';
import { InspectorView } from './views/inspector_view.js';
import { ItemRenderer } from './views/item_renderer.js';
import { MenuView } from './views/menu_view.js';
import { Menus } from './views/menus.js';
import { SettingsView } from './views/settings_view.js';
import { Shortcuts } from './views/shortcuts.js';
import { SidebarView } from './views/sidebar_view.js';
import { StatusBarView } from './views/status_bar_view.js';
import { ToolbarView } from './views/toolbar_view.js';
import { ViewerView } from './views/viewer_view.js';

/** The one place where the parts of the window are built and wired together. */
class AppContainer {
  constructor(tauri, storage, page) {
    const gateway = new TauriGateway(tauri);
    const settings = new SettingsRepository(gateway, storage);
    const dialogs = new DialogView(page.getElementById('dialog'));
    const menu = new MenuView(page.getElementById('menu'));

    this.backup = new BackupController(new BackupRepository(gateway), dialogs);
    this.drive = new DriveController(new DriveRepository(gateway), settings, dialogs, this.backup);
    // A backup step can change the backup icons and, after a restore, the files. So the drive reads again.
    this.backup.addEventListener(AppConstants.UPDATE_EVENT, this.drive);

    const settingsView = new SettingsView(page.getElementById('settings'), settings, this.backup);
    const menus = new Menus(menu, this.drive, this.backup, settingsView);
    const renderer = new ItemRenderer(this.drive);
    const inspector = new InspectorView(page.getElementById('inspector'), this.drive, this.backup, renderer, menus);
    const toolbar = new ToolbarView(page.getElementById('toolbar'), this.drive, menus);
    const viewer = new ViewerView(page.getElementById('viewer'), this.drive);
    this.views = [
      new SidebarView(page.getElementById('sidebar'), this.drive, this.backup, menus),
      new BrowserView(page.getElementById('browser'), this.drive, renderer, inspector, menus),
      new StatusBarView(page.getElementById('status'), this.drive, this.backup, menus),
      inspector,
      toolbar,
      viewer,
    ];
    this.shortcuts = new Shortcuts(gateway, this.drive, this.backup, { toolbar, dialogs, menu, settingsView, viewer });
  }

  async start() {
    await this.shortcuts.start();
    await this.drive.start();
    await this.backup.start();
  }
}

new AppContainer(window.__TAURI__, window.localStorage, document).start();
