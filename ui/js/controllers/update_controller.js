import { AppConstants } from '../constants/app_constants.js';

/** Looks for a newer version of Drive, and asks the user before it installs one. */
export class UpdateController {
  constructor(repository, dialogs, drive) {
    this.repository = repository;
    this.dialogs = dialogs;
    this.drive = drive;
    /** The version the user said "Later" to. Drive does not ask for it again until it starts the next time. */
    this.declined = null;
    this.runCheck = this.check.bind(this);
  }

  /** Looks now, and then again and again, because the app can run for days. */
  start() {
    setInterval(this.runCheck, AppConstants.UPDATE_CHECK_MS);
    this.check();
  }

  async check() {
    // Do not put a question on top of a dialog the user is in.
    if (this.dialogs.isOpen) return;
    let version = null;
    try {
      version = await this.repository.check();
    } catch {
      // No internet, or no release yet. The next check tries again.
      return;
    }
    if (!version || version === this.declined || this.dialogs.isOpen) return;
    const answer = await this.dialogs.choose({
      title: `Drive ${version} is ready`,
      message: 'Drive downloads it, installs it and starts again. Your files stay as they are.',
      buttons: [
        { id: 'later', label: 'Later' },
        { id: 'install', label: 'Install and Restart', kind: 'primary' },
      ],
    });
    if (answer !== 'install') {
      this.declined = version;
      return;
    }
    // The spinner in the status bar turns while it downloads. An error is shown by the drive.
    await this.drive.busy(this.repository.install());
  }
}
