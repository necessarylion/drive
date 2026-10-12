import { AppConstants } from '../constants/app_constants.js';
import { Dom } from '../support/dom.js';
import { View } from './view.js';

/**
 * A popup menu, used for right clicks and for the menus of buttons.
 * `show` gives back the command of the picked entry, or null when the menu was closed.
 */
export class MenuView extends View {
  constructor(root) {
    super(root);
    this.pending = null;
    this.listen('click', 'contextmenu');
    document.addEventListener('pointerdown', this, true);
    window.addEventListener('blur', this);
  }

  /**
   * Entries are `{ command, label, icon, danger, disabled, checked }`, `{ separator: true }`,
   * or `{ colors: true }` for the row of folder colors (its command is "color:<name>").
   */
  show(x, y, entries) {
    this.close(null);
    const rows = [];
    for (const entry of entries) {
      rows.push(this.row(entry));
    }
    Dom.fill(this.root, rows);
    this.root.hidden = false;
    // Keep the menu inside the window.
    const box = this.root.getBoundingClientRect();
    const left = Math.max(8, Math.min(x, window.innerWidth - box.width - 8));
    const top = Math.max(8, Math.min(y, window.innerHeight - box.height - 8));
    this.root.style.left = `${left}px`;
    this.root.style.top = `${top}px`;
    this.pending = Promise.withResolvers();
    return this.pending.promise;
  }

  /** Shows the menu under a button. */
  showBelow(element, entries) {
    const box = element.getBoundingClientRect();
    return this.show(box.left, box.bottom + 6, entries);
  }

  get isOpen() {
    return !this.root.hidden;
  }

  close(command) {
    this.root.hidden = true;
    if (this.pending) this.pending.resolve(command);
    this.pending = null;
  }

  handleEvent(event) {
    // A press outside the menu, or a switch to another app, closes it.
    if (event.type === 'blur') {
      this.close(null);
    } else if (event.type === 'pointerdown') {
      if (this.isOpen && !this.root.contains(event.target)) this.close(null);
    } else {
      super.handleEvent(event);
    }
  }

  onClick(event) {
    const element = this.actionOf(event);
    if (element && !element.disabled) this.close(element.dataset.action);
  }

  onContextMenu(event) {
    event.preventDefault();
  }

  row(entry) {
    if (entry.separator) return Dom.el('div', { class: 'menu-separator' });
    if (entry.colors) return this.colorRow();
    let check = null;
    // An entry that can be ticked keeps the space of the tick, also when it is not ticked.
    if (entry.checked === true) check = Dom.icon('check', 'menu-check');
    else if (entry.checked === false) check = Dom.el('span', { class: 'icon menu-check' });
    return Dom.el(
      'button',
      {
        type: 'button',
        class: `menu-item${entry.danger ? ' is-danger' : ''}`,
        disabled: entry.disabled === true,
        data: { action: entry.command },
      },
      [check, entry.icon ? Dom.icon(entry.icon) : null, Dom.el('span', { text: entry.label })],
    );
  }

  colorRow() {
    const dots = [];
    for (const color of AppConstants.FOLDER_COLORS) {
      dots.push(
        Dom.el('button', {
          type: 'button',
          class: `color-dot bg-${color}`,
          title: color,
          data: { action: `color:${color}` },
        }),
      );
    }
    return Dom.el('div', { class: 'menu-colors' }, dots);
  }
}
