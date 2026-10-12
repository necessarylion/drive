/** The one place that talks to Tauri. Everything else goes through it. */
export class TauriGateway {
  constructor(tauri) {
    this.tauri = tauri;
  }

  /** Calls a command of the Rust side. It fails with the error text as a string. */
  invoke(command, args = {}) {
    return this.tauri.core.invoke(command, args);
  }

  listen(event, handler) {
    return this.tauri.event.listen(event, handler);
  }

  /** An address the window can load a file of the drive from, for pictures and PDFs. */
  fileUrl(path) {
    return this.tauri.core.convertFileSrc(path);
  }

  /** Shows the file dialog of the system. Returns the picked paths. The list is empty when the user cancels. */
  async pick(options) {
    const picked = await this.tauri.dialog.open(options);
    if (picked === null || picked === undefined) return [];
    return Array.isArray(picked) ? picked : [picked];
  }

  /** Tells the handler when files from outside the app are dropped on the window. */
  onFileDrop(handler) {
    return this.tauri.webview.getCurrentWebview().onDragDropEvent(handler);
  }
}
