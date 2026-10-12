/** All calls to the update commands of the Rust side. */
export class UpdateRepository {
  constructor(gateway) {
    this.gateway = gateway;
  }

  /** The version of a newer release, or null when the app is up to date. */
  check() {
    return this.gateway.invoke('update_check');
  }

  /** Installs the newer release. The app starts again, so this only comes back when it failed. */
  install() {
    return this.gateway.invoke('update_install');
  }
}
