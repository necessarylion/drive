import { AppConstants } from '../constants/app_constants.js';
import { Dom } from '../support/dom.js';
import { Format } from '../support/format.js';
import { View } from './view.js';

/** The middle of the window: the title, the folders and the files. */
export class BrowserView extends View {
  constructor(root, drive, renderer, inspector, menus) {
    super(root);
    this.drive = drive;
    this.renderer = renderer;
    this.inspector = inspector;
    this.menus = menus;
    this.listen('click', 'contextmenu');
    this.watch(drive);
  }

  render() {
    const drive = this.drive;
    this.root.classList.toggle('is-columns', drive.isColumnMode);
    if (drive.isColumnMode) {
      Dom.fill(this.root, [this.columns()]);
      return;
    }
    const parts = [this.header()];
    if (drive.items.length === 0) {
      parts.push(this.emptyState());
    } else {
      parts.push(this.folders(), this.files());
    }
    Dom.fill(this.root, [Dom.el('div', { class: 'browser-content' }, parts)]);
  }

  header() {
    return Dom.el('div', { class: 'browser-header' }, [
      Dom.el('h1', { class: 'browser-title', text: this.drive.title }),
      Dom.el('div', { class: 'browser-subtitle', text: this.drive.headerSubtitle }),
    ]);
  }

  folders() {
    const cards = [];
    for (const item of this.drive.items) {
      if (item.isFolder) cards.push(this.folderCard(item));
    }
    if (cards.length === 0) return null;
    return Dom.el('section', { class: 'browser-section' }, [
      Dom.el('div', { class: 'section-title', text: 'Folders' }),
      Dom.el('div', { class: 'folder-grid' }, cards),
    ]);
  }

  files() {
    const files = [];
    for (const item of this.drive.items) {
      if (!item.isFolder) files.push(item);
    }
    if (files.length === 0) return null;
    const isGrid = this.drive.viewMode === 'grid';
    const rows = isGrid ? [] : [this.listHeader()];
    for (const item of files) {
      rows.push(isGrid ? this.fileTile(item) : this.fileRow(item));
    }
    return Dom.el('section', { class: 'browser-section' }, [
      Dom.el('div', { class: 'section-title', text: 'Files' }),
      Dom.el('div', { class: isGrid ? 'tile-grid' : 'file-list' }, rows),
    ]);
  }

  /** A folder shown as a card: a colored icon, the name, and what is inside. */
  folderCard(item) {
    const summary = item.summary
      ? `${Format.items(item.summary.count)} · ${Format.bytes(item.summary.size)}`
      : 'Folder';
    return Dom.el('div', this.renderer.itemAttributes(item, 'folder-card'), [
      Dom.el('div', { class: `folder-icon tint-${item.color ?? 'blue'}` }, [Dom.icon('folder')]),
      Dom.el('div', { class: 'folder-text' }, [
        Dom.el('div', { class: 'folder-name' }, [Dom.el('span', { text: item.name }), this.renderer.star(item)]),
        Dom.el('div', { class: 'folder-summary', text: summary }),
      ]),
    ]);
  }

  /** The titles above the file list. A click sorts by that column. */
  listHeader() {
    const titles = [];
    for (const field of AppConstants.SORT_FIELDS) {
      const isActive = this.drive.sortField === field.id;
      const arrow = isActive ? Dom.icon(this.drive.isSortAscending ? 'chevronUp' : 'chevronDown', 'sort-arrow') : null;
      titles.push(
        Dom.el(
          'button',
          {
            type: 'button',
            class: `list-title col-${field.id}${isActive ? ' is-active' : ''}`,
            data: { action: 'sortBy', field: field.id },
          },
          [Dom.el('span', { text: field.title }), arrow],
        ),
      );
    }
    titles.push(Dom.el('span', { class: 'col-badge' }));
    return Dom.el('div', { class: 'list-header' }, titles);
  }

  /** One file in list view. */
  fileRow(item) {
    return Dom.el('div', this.renderer.itemAttributes(item, 'file-row'), [
      Dom.el('div', { class: 'col-name' }, [
        this.renderer.kindIcon(item, 'file-icon'),
        Dom.el('span', { class: 'file-name', text: item.name }),
        this.renderer.star(item),
      ]),
      Dom.el('div', { class: 'col-modified', text: Format.modified(item.modified) }),
      Dom.el('div', { class: 'col-size', text: Format.bytes(item.size) }),
      Dom.el('div', { class: 'col-badge' }, [this.renderer.syncBadge(item)]),
    ]);
  }

  /** One file in grid view. */
  fileTile(item) {
    return Dom.el('div', this.renderer.itemAttributes(item, 'file-tile'), [
      this.renderer.thumbnail(item, 'tile-thumbnail'),
      Dom.el('div', { class: 'tile-foot' }, [
        Dom.el('div', { class: 'tile-text' }, [
          Dom.el('div', { class: 'tile-name', text: item.name }),
          Dom.el('div', { class: 'tile-size', text: Format.bytes(item.size) }),
        ]),
        this.renderer.syncBadge(item),
      ]),
    ]);
  }

  /** Column view: one column per open folder, and a preview of the picked file at the end. */
  columns() {
    const drive = this.drive;
    const parts = [];
    let depth = 0;
    for (const items of drive.columns) {
      const rows = [];
      for (const item of items) {
        rows.push(this.columnRow(item, depth));
      }
      parts.push(Dom.el('div', { class: 'column' }, rows));
      depth += 1;
    }
    const picked = drive.selectedItem;
    if (picked) parts.push(Dom.el('div', { class: 'column-preview' }, [this.inspector.details(picked)]));
    return Dom.el('div', { class: 'columns' }, parts);
  }

  columnRow(item, depth) {
    const drive = this.drive;
    const isOpen = drive.path.length > depth && drive.path[depth].path === item.path;
    const attributes = this.renderer.itemAttributes(item, `column-row${isOpen ? ' is-open' : ''}`, { depth });
    return Dom.el('div', attributes, [
      this.renderer.kindIcon(item, 'column-icon'),
      Dom.el('span', { class: 'column-name', text: item.name }),
      item.isFolder ? Dom.icon('chevronRight', 'column-arrow') : null,
    ]);
  }

  emptyState() {
    const drive = this.drive;
    if (drive.searchText !== '') {
      return Dom.el('div', { class: 'empty' }, [
        Dom.icon('search', 'empty-icon'),
        Dom.el('div', { class: 'empty-title', text: `No results for “${drive.searchText}”` }),
        Dom.el('div', { class: 'empty-message', text: 'Check the spelling or try a new search.' }),
      ]);
    }
    const section = AppConstants.section(drive.section);
    return Dom.el('div', { class: 'empty' }, [
      Dom.icon(section.icon === 'drive' ? 'folder' : section.icon, 'empty-icon'),
      Dom.el('div', { class: 'empty-title', text: section.emptyTitle }),
      Dom.el('div', { class: 'empty-message', text: section.emptyMessage }),
      drive.canAddFiles ? Dom.button({ label: 'Upload Files…', data: { action: 'upload' } }) : null,
    ]);
  }

  onClick(event) {
    const element = this.actionOf(event);
    const drive = this.drive;
    if (!element) {
      drive.clearSelection();
      return;
    }
    const data = element.dataset;
    switch (data.action) {
      case 'item':
        this.clickItem(event, data);
        break;
      case 'sortBy':
        drive.sort(data.field);
        break;
      case 'upload':
        drive.uploadFiles();
        break;
      default:
        // The buttons of the preview in column view.
        this.inspector.runAction(element);
        break;
    }
  }

  /** One click picks the item. A second click right after opens it. */
  clickItem(event, data) {
    const drive = this.drive;
    if (event.detail >= 2) {
      drive.open([data.path]);
    } else if (data.depth !== undefined) {
      drive.selectInColumn(data.path, Number(data.depth));
    } else {
      drive.select(data.path, event.metaKey || event.ctrlKey);
    }
  }

  onContextMenu(event) {
    event.preventDefault();
    const element = this.actionOf(event);
    const at = { x: event.clientX, y: event.clientY };
    if (element && element.dataset.action === 'item') {
      const path = element.dataset.path;
      // In column view the menu is for the one item under the mouse.
      const paths = element.dataset.depth === undefined ? this.drive.targets(path) : [path];
      this.menus.showItemMenu(at, paths);
    } else {
      this.menus.showNewMenu(at);
    }
  }
}
