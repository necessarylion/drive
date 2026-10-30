import { AppConstants } from '../constants/app_constants.js';

/**
 * Runs backup and restore for the window. The state itself lives on the Rust side.
 * This class asks for it again each time the Rust side says it changed.
 */
export class BackupController extends EventTarget {
  constructor(repository, dialogs) {
    super();
    this.repository = repository;
    this.dialogs = dialogs;
    this.status = {
      isConfigured: false,
      isRunning: false,
      isPaused: false,
      isUploading: false,
      fraction: 0,
      summary: 'Backup is off',
      destination: 'Backup is off',
    };
    this.runRefresh = this.refresh.bind(this);
  }

  async start() {
    await this.repository.onChanged(AppConstants.BACKUP_CHANGED_EVENT, this.runRefresh);
    await this.refresh();
    this.autoBackUp();
  }

  async refresh() {
    this.status = await this.repository.status();
    this.dispatchEvent(new Event(AppConstants.UPDATE_EVENT));
  }

  /** Syncs both ways without asking. A file deleted on one side is removed on the other side too. */
  autoBackUp() {
    return this.repository.auto();
  }

  togglePause() {
    return this.repository.togglePause();
  }

  /** The manual sync. It shows the error when it fails. */
  async backUp() {
    try {
      await this.repository.backUp();
    } catch (error) {
      this.fail(error);
    }
  }

  async restore() {
    try {
      await this.repository.restore();
    } catch (error) {
      this.fail(error);
    }
  }

  // MARK: - Settings

  // The functions below take the settings that are on screen in the Settings dialog. They can be
  // not saved yet, so the user can set up and test a provider before the backup starts to use it.

  saveSettings(config) {
    return this.repository.saveSettings(config);
  }

  saveKeys(accessKey, secretKey, provider) {
    return this.repository.saveKeys(accessKey, secretKey, provider);
  }

  removeKeys(provider) {
    return this.repository.removeKeys(provider);
  }

  keyHint(provider) {
    return this.repository.keyHint(provider);
  }

  testConnection(config) {
    return this.repository.testConnection(config);
  }

  googleSignIn(config) {
    return this.repository.googleSignIn(config);
  }

  fail(error) {
    this.dialogs.alert('Backup problem', String(error));
  }
}
