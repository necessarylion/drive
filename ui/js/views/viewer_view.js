import { Dom } from '../support/dom.js';
import { View } from './view.js';

/**
 * Shows one file over the window.
 */
// ponytail: the webview of the system does the showing. Pictures, video, audio and text work everywhere.
// A PDF works on macOS and Windows. Linux shows the "no preview" page for it; add pdf.js if that is needed.
export class ViewerView extends View {
  constructor(root, drive) {
    super(root);
    this.drive = drive;
    /** The path that is on screen, so the viewer is not built again on every update. */
    this.shownPath = null;
    this.listen('click');
    this.watch(drive);
  }

  get isOpen() {
    return this.drive.viewedItem !== null;
  }

  render() {
    const item = this.drive.viewedItem;
    if (!item) {
      this.root.hidden = true;
      this.root.replaceChildren();
      this.shownPath = null;
      return;
    }
    if (this.shownPath === item.path) return;
    this.shownPath = item.path;
    Dom.fill(this.root, [
      Dom.el('header', { class: 'viewer-bar', 'data-tauri-drag-region': true }, [
        Dom.button({ icon: 'x', class: 'tool', title: 'Close', data: { action: 'close' } }),
        Dom.el('div', { class: 'viewer-title' }, [
          Dom.el('div', { class: 'viewer-name', text: item.name }),
          Dom.el('div', { class: 'viewer-note', text: 'Saved to Drive' }),
        ]),
        Dom.el('div', { class: 'toolbar-space', 'data-tauri-drag-region': true }),
        Dom.button({ icon: 'download', class: 'tool', title: 'Download a Copy', data: { action: 'download' } }),
        Dom.button({ icon: 'folder', class: 'tool', title: 'Show in File Manager', data: { action: 'reveal' } }),
        Dom.button({ label: 'Open in Default App', class: 'button is-primary', data: { action: 'openDefault' } }),
      ]),
      Dom.el('div', { class: 'viewer-body' }, [this.content(item)]),
    ]);
    this.root.hidden = false;
  }

  content(item) {
    const url = this.drive.fileUrl(item);
    switch (item.kind) {
      case 'image':
        return Dom.el('img', { class: 'viewer-image', src: url, alt: item.name });
      case 'video':
        return Dom.el('video', { class: 'viewer-media', src: url, controls: true });
      case 'audio':
        return Dom.el('audio', { class: 'viewer-audio', src: url, controls: true });
      case 'pdf':
        return this.isLinux() ? this.noPreview(item) : Dom.el('iframe', { class: 'viewer-frame', src: url });
      case 'text':
      case 'sheet': {
        const page = Dom.el('pre', { class: 'viewer-text' });
        this.loadText(item, page);
        return page;
      }
      default:
        return this.noPreview(item);
    }
  }

  async loadText(item, page) {
    try {
      page.textContent = await this.drive.readText(item);
    } catch (error) {
      page.textContent = String(error);
    }
  }

  noPreview(item) {
    return Dom.el('div', { class: 'empty' }, [
      Dom.icon('docSearch', 'empty-icon'),
      Dom.el('div', { class: 'empty-title', text: 'No preview for this file' }),
      Dom.el('div', { class: 'empty-message', text: `Open “${item.name}” in another app to see it.` }),
    ]);
  }

  isLinux() {
    return navigator.userAgent.includes('Linux');
  }

  onClick(event) {
    const element = this.actionOf(event);
    const item = this.drive.viewedItem;
    if (!element || !item) return;
    switch (element.dataset.action) {
      case 'close':
        this.drive.closeViewer();
        break;
      case 'download':
        this.drive.askDownloadCopy([item.path]);
        break;
      case 'reveal':
        this.drive.showInFileManager([item.path]);
        break;
      case 'openDefault':
        this.drive.openInDefaultApp(item.path);
        break;
      default:
        break;
    }
  }
}
