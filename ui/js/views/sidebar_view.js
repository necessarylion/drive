import { AppConstants } from '../constants/app_constants.js';
import { Dom } from '../support/dom.js';
import { Format } from '../support/format.js';
import { View } from './view.js';

/** The left side of the window: the lists, the pinned folders, the storage bar and the backup line. */
export class SidebarView extends View {
  constructor(root, drive, backup, menus) {
    super(root);
    this.drive = drive;
    this.backup = backup;
    this.menus = menus;
    this.listen('click', 'contextmenu');
    this.watch(drive, backup);
  }

  render() {
    const drive = this.drive;
    document.body.classList.toggle('sidebar-hidden', !drive.isSidebarShown);
    const rows = [Dom.el('div', { class: 'sidebar-title', text: 'Drive' })];
    for (const section of AppConstants.SECTIONS) {
      rows.push(this.sectionRow(section));
    }
    if (drive.overview.pinned.length > 0) {
      rows.push(Dom.el('div', { class: 'sidebar-title', text: 'Pinned folders' }));
      for (const folder of drive.overview.pinned) {
        rows.push(this.pinnedRow(folder));
      }
    }
    Dom.fill(this.root, [
      Dom.el('div', { class: 'sidebar-top', 'data-tauri-drag-region': true }, [
        Dom.button({
          icon: 'sidebarLeft',
          class: 'tool',
          title: 'Hide the sidebar',
          data: { action: 'toggleSidebar' },
        }),
      ]),
      Dom.el('nav', { class: 'sidebar-list' }, rows),
      this.footer(),
    ]);
  }

  sectionRow(section) {
    const drive = this.drive;
    const isPicked = drive.location.pinned === null && drive.section === section.id;
    // My Drive shows the app logo.
    const icon =
      section.id === 'myDrive'
        ? Dom.el('img', { class: 'sidebar-logo', src: 'assets/logo.png', alt: '' })
        : Dom.icon(section.icon, 'sidebar-icon');
    return Dom.el(
      'button',
      {
        type: 'button',
        class: `sidebar-row${isPicked ? ' is-picked' : ''}`,
        data: { action: 'section', section: section.id },
      },
      [icon, Dom.el('span', { text: section.title })],
    );
  }

  pinnedRow(folder) {
    const isPicked = this.drive.location.pinned === folder.path;
    return Dom.el(
      'button',
      {
        type: 'button',
        class: `sidebar-row${isPicked ? ' is-picked' : ''}`,
        title: folder.name,
        data: { action: 'pinned', path: folder.path },
      },
      [
        Dom.el('span', { class: `pin-dot bg-${folder.color}` }),
        Dom.el('span', { class: 'sidebar-name', text: folder.name }),
      ],
    );
  }

  /** The storage bar and the backup line at the bottom. */
  footer() {
    const drive = this.drive;
    const status = this.backup.status;
    let total = 0;
    for (const slice of drive.overview.slices) {
      total += slice.size;
    }
    const parts = [];
    for (const slice of drive.overview.slices) {
      // Each slice gets its share of the bar. The bar is full when the drive has files.
      const share = total === 0 ? 0 : (slice.size / total) * 100;
      parts.push(Dom.el('span', { class: `storage-part group-${slice.group}`, style: `flex: ${share} 0 3px` }));
    }
    if (parts.length === 0) parts.push(Dom.el('span', { class: 'storage-part is-empty', style: 'flex: 1' }));
    const cloud = Dom.icon(status.isRunning ? 'sync' : 'cloudCheck', status.isConfigured ? 'is-success' : '');
    return Dom.el('div', { class: 'sidebar-footer' }, [
      Dom.el('div', { class: 'storage-title', text: 'Storage' }),
      Dom.el('div', { class: 'storage-bar' }, parts),
      Dom.el('div', { class: 'storage-text', text: `${Format.bytes(drive.overview.usedBytes)} used` }),
      Dom.el('div', { class: 'storage-backup' }, [cloud, Dom.el('span', { text: status.summary })]),
    ]);
  }

  onClick(event) {
    const element = this.actionOf(event);
    if (!element) return;
    switch (element.dataset.action) {
      case 'section':
        this.drive.showSection(element.dataset.section);
        break;
      case 'pinned':
        this.drive.showPinned(element.dataset.path);
        break;
      case 'toggleSidebar':
        this.drive.toggleSidebar();
        break;
      default:
        break;
    }
  }

  onContextMenu(event) {
    event.preventDefault();
    const element = this.actionOf(event);
    if (element && element.dataset.action === 'pinned') {
      this.menus.showPinnedMenu({ x: event.clientX, y: event.clientY }, element.dataset.path);
    }
  }
}
