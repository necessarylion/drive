/** All calls to the settings commands of the Rust side, and the settings kept in the window itself. */
export class SettingsRepository {
  constructor(gateway, storage) {
    this.gateway = gateway;
    this.storage = storage;
  }

  load() {
    return this.gateway.invoke('settings_load');
  }

  /** The saved settings of one provider. Each provider has its own bucket and folder. */
  configFor(provider) {
    return this.gateway.invoke('settings_config_for', { provider });
  }

  /** An empty path means the default folder. */
  setStorageRoot(path) {
    return this.gateway.invoke('settings_set_storage_root', { path });
  }

  async pickStorageRoot() {
    const picked = await this.gateway.pick({ directory: true, title: 'Use This Folder' });
    return picked.length > 0 ? picked[0] : null;
  }

  /** A small value that only the window needs, like the view mode. */
  local(key, fallback) {
    return this.storage.getItem(key) ?? fallback;
  }

  setLocal(key, value) {
    this.storage.setItem(key, value);
  }
}
