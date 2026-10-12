import { BackupController } from './controllers/backup_controller.js';
import { DriveController } from './controllers/drive_controller.js';
import { BackupRepository } from './repositories/backup_repository.js';
import { DriveRepository } from './repositories/drive_repository.js';
import { SettingsRepository } from './repositories/settings_repository.js';
import { TauriGateway } from './repositories/tauri_gateway.js';
import { DialogView } from './views/dialog_view.js';
import { ViewerView } from './views/viewer_view.js';

/** Builds and wires the parts of a viewer window. It shows one file, the one the Rust side put in `DRIVE_VIEWED`. */
class ViewerContainer {
  constructor(tauri, storage, page) {
    const gateway = new TauriGateway(tauri);
    const dialogs = new DialogView(page.getElementById('dialog'));
    const backup = new BackupController(new BackupRepository(gateway), dialogs);
    // The controllers are not started here. This window reads no lists and starts no backup.
    this.drive = new DriveController(
      new DriveRepository(gateway),
      new SettingsRepository(gateway, storage),
      dialogs,
      backup,
    );
    this.viewer = new ViewerView(page.getElementById('viewer'), this.drive);
    page.body.classList.toggle('is-mac', navigator.userAgent.includes('Mac'));
  }

  start(item) {
    this.drive.view(item);
  }
}

new ViewerContainer(window.__TAURI__, window.localStorage, document).start(window.DRIVE_VIEWED);
