/** All calls to the backup commands of the Rust side. */
export class BackupRepository {
  constructor(gateway) {
    this.gateway = gateway;
  }

  status() {
    return this.gateway.invoke('backup_status');
  }

  auto() {
    return this.gateway.invoke('backup_auto');
  }

  togglePause() {
    return this.gateway.invoke('backup_toggle_pause');
  }

  backUp() {
    return this.gateway.invoke('backup_now');
  }

  restore() {
    return this.gateway.invoke('backup_restore');
  }

  saveSettings(config) {
    return this.gateway.invoke('backup_save_settings', { config });
  }

  saveKeys(accessKey, secretKey, provider) {
    return this.gateway.invoke('backup_save_keys', { accessKey, secretKey, provider });
  }

  removeKeys(provider) {
    return this.gateway.invoke('backup_remove_keys', { provider });
  }

  keyHint(provider) {
    return this.gateway.invoke('backup_key_hint', { provider });
  }

  testConnection(config) {
    return this.gateway.invoke('backup_test_connection', { config });
  }

  googleSignIn(config) {
    return this.gateway.invoke('backup_google_sign_in', { config });
  }

  /** Calls the handler each time the backup made a step. */
  onChanged(event, handler) {
    return this.gateway.listen(event, handler);
  }
}
