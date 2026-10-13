import { AppConstants } from '../constants/app_constants.js';
import { Format } from '../support/format.js';

/**
 * Holds what the window shows, and turns user actions into calls to the Rust side.
 * It sends an "update" event after each change, and the views draw again.
 */
export class DriveController extends EventTarget {
  constructor(repository, settings, dialogs, backup) {
    super();
    this.repository = repository;
    this.settings = settings;
    this.dialogs = dialogs;
    this.backup = backup;

    /** A place in the app: a sidebar list, the open folders under it, and the pinned folder the user came in through. */
    this.location = { section: 'myDrive', path: [], pinned: null };
    this.searchText = '';
    this.sortField = 'name';
    this.isSortAscending = true;
    this.viewMode = settings.local(AppConstants.VIEW_MODE_KEY, 'list');
    this.selection = new Set();
    this.isInspectorShown = true;
    this.isSidebarShown = true;
    /** The file a viewer window shows. The main window never sets it. */
    this.viewedItem = null;

    this.items = [];
    /** One list per open folder, from the root down. Only filled in column view. */
    this.columns = [];
    this.overview = { root: '', pinned: [], slices: [], usedBytes: 0, recent: [] };
    this.isBusy = false;

    this.backStack = [];
    this.forwardStack = [];
    /** Goes up with each read, so the answer of an older read is dropped. */
    this.ticket = 0;
    this.refreshTimer = null;
    this.runRefresh = this.refresh.bind(this);
  }

  get section() {
    return this.location.section;
  }

  get path() {
    return this.location.path;
  }

  get currentFolder() {
    return this.path.length > 0 ? this.path[this.path.length - 1].path : this.overview.root;
  }

  get canAddFiles() {
    return this.section === 'myDrive';
  }

  get canGoBack() {
    return this.backStack.length > 0;
  }

  get canGoForward() {
    return this.forwardStack.length > 0;
  }

  get isColumnMode() {
    return this.viewMode === 'columns' && this.section === 'myDrive' && this.searchText === '';
  }

  /** The inspector is hidden in column view, because that view has its own preview. */
  get isInspectorVisible() {
    return this.isInspectorShown && !this.isColumnMode;
  }

  get title() {
    if (this.section === 'myDrive' && this.path.length > 0) return this.path[this.path.length - 1].name;
    return AppConstants.section(this.section).title;
  }

  get statusText() {
    const count = Format.items(this.items.length);
    return this.selection.size === 0 ? count : `${count}, ${this.selection.size} selected`;
  }

  get selectedItem() {
    for (const item of this.items) {
      if (this.selection.has(item.path)) return item;
    }
    return null;
  }

  /** "8 items · 846 kB · Edited 1 second ago" */
  get headerSubtitle() {
    let totalSize = 0;
    let latest = 0;
    for (const item of this.items) {
      totalSize += item.summary ? item.summary.size : item.size;
      latest = Math.max(latest, item.modified);
    }
    const parts = [Format.items(this.items.length), Format.bytes(totalSize)];
    if (this.section === 'trash') parts.push(`Deleted forever after ${AppConstants.TRASH_KEEP_DAYS} days`);
    else if (this.items.length > 0) parts.push(`Edited ${Format.relative(latest)}`);
    return parts.join(' · ');
  }

  /** The views listen with an object that has `handleEvent`, so no callback is needed. */
  handleEvent() {
    // The backup made a step. Wait a moment for more steps, then read the lists again.
    this.scheduleRefresh();
  }

  // MARK: - Reading

  async start() {
    await this.refresh();
  }

  /** Reads the current list again. */
  async reload() {
    this.ticket += 1;
    const ticket = this.ticket;
    try {
      const folder = this.path.length > 0 ? this.currentFolder : null;
      const items = await this.repository.list(
        this.section,
        folder,
        this.searchText,
        this.sortField,
        this.isSortAscending,
      );
      const columns = this.isColumnMode ? await this.repository.columns(this.pathStrings()) : [];
      if (ticket !== this.ticket) return;
      this.items = items;
      this.columns = columns;
      this.keepSelection();
    } catch (error) {
      this.fail(error);
    }
    this.emit();
  }

  /** Reads everything again, also the sidebar and the storage numbers. */
  async refresh() {
    try {
      this.overview = await this.repository.overview();
    } catch (error) {
      this.fail(error);
    }
    await this.reload();
  }

  scheduleRefresh() {
    clearTimeout(this.refreshTimer);
    this.refreshTimer = setTimeout(this.runRefresh, AppConstants.REFRESH_DELAY_MS);
  }

  /** Looks for an item in the list and in the columns. */
  find(path) {
    for (const item of this.items) {
      if (item.path === path) return item;
    }
    for (const column of this.columns) {
      for (const item of column) {
        if (item.path === path) return item;
      }
    }
    return null;
  }

  areStarred(paths) {
    return this.all(paths, 'isStarred');
  }

  arePinned(paths) {
    return this.all(paths, 'isPinned');
  }

  areFolders(paths) {
    return this.all(paths, 'isFolder');
  }

  /** "PDF document · 18.4 MB" */
  details(item) {
    if (item.isFolder) {
      const summary = item.summary ?? { count: 0, size: 0 };
      return `Folder · ${Format.items(summary.count)} · ${Format.bytes(summary.size)}`;
    }
    return `${AppConstants.KINDS[item.kind].label} · ${Format.bytes(item.size)}`;
  }

  /** "My Drive / Clients / Northwind" */
  whereText(item) {
    if (this.section === 'trash') return AppConstants.section('trash').title;
    return [AppConstants.section('myDrive').title, ...item.parents].join(' / ');
  }

  fileUrl(item) {
    return this.repository.fileUrl(item.path);
  }

  readText(item) {
    return this.repository.readText(item.path);
  }

  // MARK: - Navigation

  navigate(target) {
    const next = { section: target.section, path: target.path ?? [], pinned: target.pinned ?? null };
    // Stay "inside" the pinned folder while the new place is still under it.
    const pinned = this.location.pinned;
    if (next.pinned === null && pinned !== null && this.holds(next.path, pinned)) next.pinned = pinned;
    if (this.key(next) === this.key(this.location)) return;
    this.backStack.push(this.location);
    this.forwardStack = [];
    this.show(next);
  }

  goBack() {
    const previous = this.backStack.pop();
    if (!previous) return;
    this.forwardStack.push(this.location);
    this.show(previous);
  }

  goForward() {
    const next = this.forwardStack.pop();
    if (!next) return;
    this.backStack.push(this.location);
    this.show(next);
  }

  goToPathLevel(count) {
    this.navigate({ section: 'myDrive', path: this.path.slice(0, count) });
  }

  /** A click on a list in the sidebar. It always goes to the top of that list. */
  showSection(section) {
    this.searchText = '';
    this.navigate({ section });
    // A click on the list that is open already still clears the search.
    this.reload();
  }

  /** A click on a pinned folder in the sidebar. */
  async showPinned(path) {
    this.searchText = '';
    this.navigate({ section: 'myDrive', path: await this.repository.folderChain(path), pinned: path });
  }

  async showFolder(path) {
    this.searchText = '';
    this.navigate({ section: 'myDrive', path: await this.repository.folderChain(path) });
  }

  // MARK: - Selection

  /** `additive` is true when the Command or Control key is down. */
  select(path, additive) {
    if (!additive) {
      this.selection = new Set([path]);
    } else if (this.selection.has(path)) {
      this.selection.delete(path);
    } else {
      this.selection.add(path);
    }
    this.emit();
  }

  /** A click in column view. A folder opens the next column. A file is picked. */
  async selectInColumn(path, depth) {
    const item = this.find(path);
    if (!item) return;
    const base = this.path.slice(0, depth);
    if (item.isFolder) {
      base.push({ path: item.path, name: item.name, color: item.color });
      this.navigate({ section: 'myDrive', path: base });
      return;
    }
    this.navigate({ section: 'myDrive', path: base });
    await this.reload();
    this.selection = new Set([item.path]);
    this.emit();
  }

  selectAll() {
    this.selection = new Set(this.pathsOf(this.items));
    this.emit();
  }

  clearSelection() {
    if (this.selection.size === 0) return;
    this.selection = new Set();
    this.emit();
  }

  /** A right click on a selected item acts on the whole selection. */
  targets(path) {
    return this.selection.has(path) ? [...this.selection] : [path];
  }

  sort(field) {
    if (this.sortField === field) {
      this.isSortAscending = !this.isSortAscending;
    } else {
      this.sortField = field;
    }
    this.reload();
  }

  setSortAscending(ascending) {
    this.isSortAscending = ascending;
    this.reload();
  }

  /** Remembered between runs of the app. */
  setViewMode(mode) {
    this.viewMode = mode;
    this.settings.setLocal(AppConstants.VIEW_MODE_KEY, mode);
    this.reload();
  }

  setSearch(text) {
    this.searchText = text;
    this.reload();
  }

  toggleInspector() {
    this.isInspectorShown = !this.isInspectorShown;
    this.emit();
  }

  toggleSidebar() {
    this.isSidebarShown = !this.isSidebarShown;
    this.emit();
  }

  showInfo(paths) {
    this.selection = new Set(paths);
    this.isInspectorShown = true;
    this.emit();
  }

  // MARK: - Opening

  /** Opens a folder in place, or a file in a viewer window of its own. */
  async open(paths) {
    if (this.section === 'trash' || paths.length === 0) return;
    const item = this.find(paths[0]);
    if (!item) return;
    if (item.isFolder) {
      this.showFolder(item.path);
    } else {
      await this.guard(this.repository.view(item.path));
    }
  }

  openSelection() {
    this.open([...this.selection]);
  }

  /** Used by a viewer window: this is the file to show. */
  view(item) {
    this.viewedItem = item;
    this.emit();
  }

  async openInDefaultApp(path) {
    await this.guard(this.repository.open(path));
  }

  async showInFileManager(paths) {
    if (paths.length > 0) await this.guard(this.repository.reveal(paths[0]));
  }

  async showDriveInFileManager() {
    await this.guard(this.repository.reveal());
  }

  // MARK: - Adding

  async uploadFiles() {
    await this.addFiles(await this.repository.pickFiles(), this.currentFolder);
  }

  async uploadFolder() {
    await this.addFiles(await this.repository.pickFolders(), this.currentFolder);
  }

  /** Adds files that were dropped on the window or picked in a dialog. */
  async addFiles(paths, folder) {
    if (!this.canAddFiles || paths.length === 0) return;
    await this.busy(this.repository.add(paths, folder));
    await this.didChange();
  }

  async askNewFolder() {
    if (!this.canAddFiles) return;
    const answer = await this.dialogs.askName({
      title: 'New folder',
      subtitle: `It will be created in ${this.title}.`,
      icon: 'folderPlus',
      value: AppConstants.DEFAULT_FOLDER_NAME,
      confirmLabel: 'Create',
      withColor: true,
    });
    if (!answer) return;
    await this.guard(this.repository.createFolder(answer.name, this.currentFolder, answer.color));
    await this.didChange();
  }

  async askRename(paths) {
    const item = paths.length === 1 ? this.find(paths[0]) : null;
    if (!item) return;
    const answer = await this.dialogs.askName({
      title: 'Rename',
      subtitle: 'Type the new name.',
      icon: 'pencil',
      value: item.name,
      confirmLabel: 'Rename',
      withColor: false,
    });
    if (!answer) return;
    await this.guard(this.repository.rename(item.path, answer.name));
    await this.didChange();
  }

  newTextDocument() {
    return this.newDocument('Untitled.txt');
  }

  newSpreadsheet() {
    return this.newDocument('Untitled.csv');
  }

  // MARK: - Changing

  async duplicate(paths) {
    if (!this.canAddFiles || paths.length === 0) return;
    await this.busy(this.repository.duplicate(paths));
    await this.didChange();
  }

  async askMove(paths) {
    const folder = await this.repository.pickFolder('Move', this.currentFolder);
    if (folder) await this.move(paths, folder);
  }

  /** Moves items of the drive into a folder of the drive. */
  async move(paths, folder) {
    if (paths.length === 0) return;
    await this.busy(this.repository.add(paths, folder));
    await this.didChange();
  }

  async askDownloadCopy(paths) {
    const folder = await this.repository.pickFolder('Save Here');
    if (!folder) return;
    await this.busy(this.repository.export(paths, folder));
  }

  async toggleStar(paths) {
    await this.guard(this.repository.setStarred(paths, !this.areStarred(paths)));
    await this.didChange();
  }

  async togglePin(paths) {
    await this.guard(this.repository.setPinned(paths, !this.arePinned(paths)));
    await this.didChange();
  }

  async unpin(path) {
    await this.guard(this.repository.setPinned([path], false));
    await this.didChange();
  }

  async setColor(color, paths) {
    await this.guard(this.repository.setColor(paths, color));
    await this.didChange();
  }

  async askTrash(paths) {
    if (paths.length === 0) return;
    const count = paths.length === 1 ? '1 item' : `${paths.length} items`;
    const answer = await this.dialogs.choose({
      title: `Move ${count} to the trash?`,
      message: `You can put it back from the trash for ${AppConstants.TRASH_KEEP_DAYS} days.`,
      buttons: [
        { id: 'cancel', label: 'Cancel' },
        { id: 'trash', label: 'Move to Trash', kind: 'danger' },
      ],
    });
    if (answer !== 'trash') return;
    await this.guard(this.repository.trash(paths));
    await this.didChange();
  }

  trashSelection() {
    const paths = [...this.selection];
    return this.section === 'trash' ? this.askDeleteForever(paths) : this.askTrash(paths);
  }

  async restore(paths) {
    await this.guard(this.repository.restore(paths));
    await this.didChange();
  }

  async askDeleteForever(paths) {
    if (paths.length === 0) return;
    const answer = await this.dialogs.choose({
      title: `Delete ${paths.length} item(s) forever?`,
      message: 'You cannot undo this.',
      buttons: [
        { id: 'cancel', label: 'Cancel' },
        { id: 'delete', label: 'Delete Forever', kind: 'danger' },
      ],
    });
    if (answer !== 'delete') return;
    await this.guard(this.repository.deleteForever(paths));
    await this.didChange();
  }

  askEmptyTrash() {
    return this.askDeleteForever(this.pathsOf(this.items));
  }

  // MARK: - Helpers

  emit() {
    this.dispatchEvent(new Event(AppConstants.UPDATE_EVENT));
  }

  show(location) {
    this.location = location;
    this.selection = new Set();
    this.reload();
  }

  /** Every change to the files goes through here, so the lists are read again and the backup starts. */
  async didChange() {
    await this.refresh();
    this.backup.autoBackUp();
  }

  async newDocument(name) {
    if (!this.canAddFiles) return;
    const path = await this.guard(this.repository.createDocument(name, this.currentFolder));
    await this.didChange();
    if (path) {
      this.selection = new Set([path]);
      this.emit();
    }
  }

  /** Waits for the work. Shows the error when it fails, and gives null then. */
  async guard(work) {
    try {
      return await work;
    } catch (error) {
      this.fail(error);
      return null;
    }
  }

  /** Like `guard`, and shows the spinner in the status bar while the work runs. */
  async busy(work) {
    this.isBusy = true;
    this.emit();
    const result = await this.guard(work);
    this.isBusy = false;
    this.emit();
    return result;
  }

  fail(error) {
    this.dialogs.alert('Something went wrong', String(error));
  }

  keepSelection() {
    const kept = new Set();
    for (const item of this.items) {
      if (this.selection.has(item.path)) kept.add(item.path);
    }
    this.selection = kept;
  }

  /** True when the item of every path has the flag. Paths that are not on screen do not count. */
  all(paths, flag) {
    for (const path of paths) {
      const item = this.find(path);
      if (item && !item[flag]) return false;
    }
    return true;
  }

  holds(path, folderPath) {
    for (const folder of path) {
      if (folder.path === folderPath) return true;
    }
    return false;
  }

  pathStrings() {
    return this.pathsOf(this.path);
  }

  pathsOf(entries) {
    const paths = [];
    for (const entry of entries) {
      paths.push(entry.path);
    }
    return paths;
  }

  /** A text that is the same for two locations that are the same place. */
  key(location) {
    return JSON.stringify([location.section, this.pathsOf(location.path), location.pinned]);
  }
}
