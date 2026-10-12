import { AppConstants } from '../constants/app_constants.js';
import { Dom } from '../support/dom.js';

/** Builds the small parts that cards, rows, tiles and the inspector share. */
export class ItemRenderer {
  constructor(drive) {
    this.drive = drive;
  }

  /** The icon of a file kind, in the color of that kind. A folder gets the color the user gave it. */
  kindIcon(item, className = '') {
    const tint = item.isFolder ? `color-${item.color ?? 'blue'}` : `kind-${item.kind}`;
    return Dom.icon(AppConstants.KINDS[item.kind].icon, `${className} ${tint}`.trim());
  }

  star(item) {
    return item.isStarred ? Dom.icon('star', 'star-mark', true) : null;
  }

  /** The small icon that tells if a file is in the backup. Nothing for an item without a backup state. */
  syncBadge(item) {
    switch (item.sync) {
      case 'synced':
        return Dom.el('span', { class: 'sync-badge sync-synced', title: 'Backed up' }, [Dom.icon('checkCircle')]);
      case 'uploading':
        return Dom.el('span', { class: 'sync-badge sync-uploading', title: 'Uploading' }, [Dom.icon('sync')]);
      case 'pending':
        return Dom.el('span', { class: 'sync-badge sync-pending', title: 'Not backed up yet' }, [Dom.icon('cloudUp')]);
      default:
        return null;
    }
  }

  syncTitle(item) {
    switch (item.sync) {
      case 'synced':
        return 'Backed up';
      case 'uploading':
        return 'Uploading';
      default:
        return 'Not backed up yet';
    }
  }

  /** Shows the picture itself for an image. Shows the type icon for everything else. */
  // ponytail: only pictures get a real preview, and the full file is loaded for it.
  // Make small preview files on the Rust side if big pictures make the grid slow.
  thumbnail(item, className) {
    const box = Dom.el('div', { class: `thumbnail kind-bg-${item.isFolder ? 'folder' : item.kind} ${className}` });
    if (item.kind === 'image' && this.drive.section !== 'trash') {
      box.append(Dom.el('img', { src: this.drive.fileUrl(item), alt: '', loading: 'lazy', draggable: 'false' }));
    } else {
      box.append(this.kindIcon(item, 'thumbnail-icon'));
    }
    return box;
  }

  /** What every clickable item carries: the action, the path, and the "selected" look. */
  itemAttributes(item, className, extra = {}) {
    const selected = this.drive.selection.has(item.path) ? ' is-selected' : '';
    return {
      class: `${className}${selected}`,
      title: item.name,
      data: { action: 'item', path: item.path, folder: item.isFolder ? '1' : null, ...extra },
    };
  }
}
