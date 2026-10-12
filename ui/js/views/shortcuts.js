import { AppConstants } from '../constants/app_constants.js';

/**
 * The things that are not tied to one part of the window: keyboard shortcuts,
 * files dropped from outside, and the window coming back to the front.
 */
export class Shortcuts {
  constructor(gateway, drive, backup, views) {
    this.gateway = gateway;
    this.drive = drive;
    this.backup = backup;
    this.toolbar = views.toolbar;
    this.dialogs = views.dialogs;
    this.menu = views.menu;
    this.settingsView = views.settingsView;
    this.isMac = navigator.userAgent.includes('Mac');
    this.dropFiles = this.onFileDrop.bind(this);
    this.openSettings = this.settings.bind(this);
  }

  async start() {
    document.body.classList.toggle('is-mac', this.isMac);
    document.addEventListener('keydown', this);
    document.addEventListener('contextmenu', this);
    window.addEventListener('focus', this);
    await this.gateway.onFileDrop(this.dropFiles);
    await this.gateway.listen(AppConstants.OPEN_SETTINGS_EVENT, this.openSettings);
  }

  handleEvent(event) {
    switch (event.type) {
      case 'keydown':
        this.onKeyDown(event);
        break;
      case 'contextmenu':
        // Our own menus are used. The menu of the webview only stays for text fields, for copy and paste.
        if (!this.isTyping(event)) event.preventDefault();
        break;
      case 'focus':
        // Pick up changes made in the file manager while the app was in the back.
        this.drive.refresh();
        this.backup.autoBackUp();
        break;
      default:
        break;
    }
  }

  onKeyDown(event) {
    if (this.dialogs.isOpen || this.settingsView.isOpen) return;
    const drive = this.drive;
    const command = this.isMac ? event.metaKey : event.ctrlKey;
    const key = event.key.toLowerCase();
    if (key === 'escape') {
      if (this.menu.isOpen) this.menu.close(null);
      else if (!this.isTyping(event)) drive.clearSelection();
      return;
    }
    if (command && key === 'f') {
      event.preventDefault();
      this.toolbar.focusSearch();
      return;
    }
    if (this.isTyping(event)) return;

    const action = command ? this.commandAction(key, event.shiftKey) : this.plainAction(key);
    if (action) {
      event.preventDefault();
      action.call(this);
    }
  }

  /** What a key does together with Command (Mac) or Control (other systems). */
  commandAction(key, shift) {
    switch (key) {
      case 'n':
        return shift ? this.newFolder : null;
      case 'u':
        return this.upload;
      case 'o':
      case 'y':
        return this.open;
      case 'd':
        return this.duplicate;
      case 'i':
        return this.inspector;
      case 'a':
        return this.selectAll;
      case '[':
        return this.back;
      case ']':
        return this.forward;
      case ',':
        return this.settings;
      case 'backspace':
      case 'delete':
        return this.trash;
      default:
        return null;
    }
  }

  plainAction(key) {
    switch (key) {
      case 'enter':
      case ' ':
        return this.open;
      case 'delete':
        return this.trash;
      default:
        return null;
    }
  }

  newFolder() {
    this.drive.askNewFolder();
  }

  upload() {
    this.drive.uploadFiles();
  }

  open() {
    this.drive.openSelection();
  }

  duplicate() {
    this.drive.duplicate([...this.drive.selection]);
  }

  inspector() {
    this.drive.toggleInspector();
  }

  selectAll() {
    this.drive.selectAll();
  }

  back() {
    this.drive.goBack();
  }

  forward() {
    this.drive.goForward();
  }

  settings() {
    this.settingsView.open();
  }

  trash() {
    this.drive.trashSelection();
  }

  /** Files from outside the app. Dropped on a folder, they go into it. Dropped elsewhere, they go into the open folder. */
  // Items of the drive itself are dragged with `ItemDrag`.
  onFileDrop(event) {
    const payload = event.payload;
    document.body.classList.toggle('is-dropping', payload.type === 'enter' || payload.type === 'over');
    if (payload.type !== 'drop') return;
    // The position comes in real pixels of the screen. The page counts in CSS pixels.
    const x = payload.position.x / window.devicePixelRatio;
    const y = payload.position.y / window.devicePixelRatio;
    const under = document.elementFromPoint(x, y);
    const folder = under ? under.closest('[data-folder]') : null;
    this.drive.addFiles(payload.paths, folder ? folder.dataset.path : this.drive.currentFolder);
  }

  isTyping(event) {
    return event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement;
  }
}
