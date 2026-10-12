import { AppConstants } from '../constants/app_constants.js';

/** The menus of the app: what is in each one, and what a picked entry does. */
export class Menus {
  constructor(menu, drive, backup, settingsView) {
    this.menu = menu;
    this.drive = drive;
    this.backup = backup;
    this.settingsView = settingsView;
  }

  /** The right-click menu of one or more items. `at` is `{ x, y }` or `{ below: element }`. */
  async showItemMenu(at, paths) {
    const drive = this.drive;
    const command = await this.open(at, this.itemEntries(paths));
    if (command === null) return;
    if (command.startsWith('color:')) {
      await drive.setColor(command.slice('color:'.length), paths);
      return;
    }
    switch (command) {
      case 'open':
        drive.open(paths);
        break;
      case 'openDefault':
        await drive.openInDefaultApp(paths[0]);
        break;
      case 'rename':
        await drive.askRename(paths);
        break;
      case 'duplicate':
        await drive.duplicate(paths);
        break;
      case 'move':
        await drive.askMove(paths);
        break;
      case 'star':
        await drive.toggleStar(paths);
        break;
      case 'pin':
        await drive.togglePin(paths);
        break;
      case 'download':
        await drive.askDownloadCopy(paths);
        break;
      case 'reveal':
        await drive.showInFileManager(paths);
        break;
      case 'info':
        drive.showInfo(paths);
        break;
      case 'trash':
        await drive.trash(paths);
        break;
      case 'restore':
        await drive.restore(paths);
        break;
      case 'deleteForever':
        await drive.askDeleteForever(paths);
        break;
      default:
        break;
    }
  }

  /** The menu for making new things. Used by the New button and by a right click on empty space. */
  async showNewMenu(at) {
    const drive = this.drive;
    if (!drive.canAddFiles) return;
    const command = await this.open(at, [
      { command: 'newFolder', label: 'New Folder', icon: 'folderPlus' },
      { separator: true },
      { command: 'uploadFiles', label: 'Upload Files…', icon: 'upload' },
      { command: 'uploadFolder', label: 'Upload Folder…', icon: 'upload' },
      { separator: true },
      { command: 'newText', label: 'Text Document', icon: 'docText' },
      { command: 'newSheet', label: 'Spreadsheet', icon: 'table' },
    ]);
    switch (command) {
      case 'newFolder':
        await drive.askNewFolder();
        break;
      case 'uploadFiles':
        await drive.uploadFiles();
        break;
      case 'uploadFolder':
        await drive.uploadFolder();
        break;
      case 'newText':
        await drive.newTextDocument();
        break;
      case 'newSheet':
        await drive.newSpreadsheet();
        break;
      default:
        break;
    }
  }

  async showSortMenu(at) {
    const drive = this.drive;
    const entries = [];
    for (const field of AppConstants.SORT_FIELDS) {
      entries.push({ command: `field:${field.id}`, label: field.title, checked: drive.sortField === field.id });
    }
    entries.push({ separator: true });
    entries.push({ command: 'ascending', label: 'Ascending', checked: drive.isSortAscending });
    const command = await this.open(at, entries);
    if (command === null) return;
    if (command === 'ascending') {
      drive.setSortAscending(!drive.isSortAscending);
    } else if (drive.sortField !== command.slice('field:'.length)) {
      drive.sort(command.slice('field:'.length));
    }
  }

  async showBackupMenu(at) {
    const isRunning = this.backup.status.isRunning;
    const command = await this.open(at, [
      { command: 'backUp', label: 'Back Up Now', icon: 'cloudUp', disabled: isRunning },
      { command: 'restore', label: 'Restore from Backup', icon: 'download', disabled: isRunning },
      { separator: true },
      { command: 'settings', label: 'Settings…', icon: 'gear' },
    ]);
    switch (command) {
      case 'backUp':
        await this.backup.backUp();
        break;
      case 'restore':
        await this.backup.restore();
        break;
      case 'settings':
        await this.settingsView.open();
        break;
      default:
        break;
    }
  }

  openSettings() {
    return this.settingsView.open();
  }

  async showPinnedMenu(at, path) {
    const command = await this.open(at, [{ command: 'unpin', label: 'Unpin', icon: 'pin' }]);
    if (command === 'unpin') await this.drive.unpin(path);
  }

  itemEntries(paths) {
    const drive = this.drive;
    if (drive.section === 'trash') {
      return [
        { command: 'restore', label: 'Put Back', icon: 'undo' },
        { command: 'deleteForever', label: 'Delete Forever…', icon: 'trash', danger: true },
      ];
    }
    const areFolders = drive.areFolders(paths);
    const starred = drive.areStarred(paths);
    const entries = [{ command: 'open', label: 'Open', icon: 'open' }];
    if (paths.length === 1 && !areFolders) {
      entries.push({ command: 'openDefault', label: 'Open in Default App', icon: 'open' });
    }
    entries.push({ separator: true });
    entries.push({ command: 'rename', label: 'Rename…', icon: 'pencil', disabled: paths.length !== 1 });
    entries.push({ command: 'duplicate', label: 'Duplicate', icon: 'copy' });
    entries.push({ command: 'move', label: 'Move to…', icon: 'folder' });
    entries.push({ command: 'star', label: starred ? 'Remove Star' : 'Add Star', icon: 'star' });
    if (areFolders) {
      const pinned = drive.arePinned(paths);
      entries.push({ command: 'pin', label: pinned ? 'Unpin from Sidebar' : 'Pin to Sidebar', icon: 'pin' });
      entries.push({ colors: true });
    }
    entries.push({ separator: true });
    entries.push({ command: 'download', label: 'Download a Copy…', icon: 'download' });
    entries.push({ command: 'reveal', label: 'Show in File Manager', icon: 'folder' });
    entries.push({ command: 'info', label: 'Get Info', icon: 'info' });
    entries.push({ separator: true });
    entries.push({ command: 'trash', label: 'Move to Trash', icon: 'trash', danger: true });
    return entries;
  }

  open(at, entries) {
    return at.below ? this.menu.showBelow(at.below, entries) : this.menu.show(at.x, at.y, entries);
  }
}
