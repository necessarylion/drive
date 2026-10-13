/** All calls to the drive commands of the Rust side. */
export class DriveRepository {
  constructor(gateway) {
    this.gateway = gateway;
  }

  list(section, folder, query, sortField, ascending) {
    return this.gateway.invoke('drive_list', { section, folder, query, sortField, ascending });
  }

  columns(path) {
    return this.gateway.invoke('drive_columns', { path });
  }

  overview() {
    return this.gateway.invoke('drive_overview');
  }

  folderChain(folder) {
    return this.gateway.invoke('drive_folder_chain', { folder });
  }

  createFolder(name, folder, color) {
    return this.gateway.invoke('drive_create_folder', { name, folder, color });
  }

  createDocument(name, folder) {
    return this.gateway.invoke('drive_create_document', { name, folder });
  }

  add(sources, folder) {
    return this.gateway.invoke('drive_add', { sources, folder });
  }

  duplicate(paths) {
    return this.gateway.invoke('drive_duplicate', { paths });
  }

  export(paths, folder) {
    return this.gateway.invoke('drive_export', { paths, folder });
  }

  rename(path, name) {
    return this.gateway.invoke('drive_rename', { path, name });
  }

  setStarred(paths, starred) {
    return this.gateway.invoke('drive_set_starred', { paths, starred });
  }

  setPinned(paths, pinned) {
    return this.gateway.invoke('drive_set_pinned', { paths, pinned });
  }

  setColor(paths, color) {
    return this.gateway.invoke('drive_set_color', { paths, color });
  }

  trash(paths) {
    return this.gateway.invoke('drive_trash', { paths });
  }

  restore(paths) {
    return this.gateway.invoke('drive_restore', { paths });
  }

  deleteForever(paths) {
    return this.gateway.invoke('drive_delete_forever', { paths });
  }

  open(path) {
    return this.gateway.invoke('drive_open', { path });
  }

  /** Shows the item in the file manager of the system. Without a path, it shows the drive folder. */
  reveal(path = null) {
    return this.gateway.invoke('drive_reveal', { path });
  }

  fileUrl(path) {
    return this.gateway.fileUrl(path);
  }

  pickFiles() {
    return this.gateway.pick({ multiple: true, title: 'Upload' });
  }

  pickFolders() {
    return this.gateway.pick({ multiple: true, directory: true, title: 'Upload' });
  }

  /** Returns the picked folder, or null when the user cancels. */
  async pickFolder(title, startAt = null) {
    const picked = await this.gateway.pick({ directory: true, title, defaultPath: startAt ?? undefined });
    return picked.length > 0 ? picked[0] : null;
  }
}
