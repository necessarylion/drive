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

  /** Uploads new and changed files without asking. It never removes anything from the backup. */
  autoBackUp() {
    return this.repository.auto();
  }

  togglePause() {
    return this.repository.togglePause();
  }

  /** The manual backup. It also asks what to do with files that are only in the backup. */
  async backUp() {
    try {
      const extras = await this.repository.backUp();
      if (extras === 0) return;
      // Removing from the backup cannot be undone, so the user decides.
      const answer = await this.dialogs.choose({
        title: `${extras} file(s) in the backup are not on this computer`,
        message:
          'This happens when you deleted or renamed files here, or when this computer has not restored yet. ' +
          'Removing from the backup cannot be undone.',
        buttons: [
          { id: 'cancel', label: 'Cancel' },
          { id: 'remove', label: 'Remove Them from the Backup', kind: 'danger' },
          { id: 'keep', label: 'Keep Them in the Backup', kind: 'primary' },
        ],
      });
      if (answer !== 'cancel') await this.repository.finish(answer === 'remove');
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
