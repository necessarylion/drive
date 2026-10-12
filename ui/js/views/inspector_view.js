import { Dom } from '../support/dom.js';
import { Format } from '../support/format.js';
import { View } from './view.js';

/** The details pane on the right side of the window. */
export class InspectorView extends View {
  constructor(root, drive, backup, renderer, menus) {
    super(root);
    this.drive = drive;
    this.backup = backup;
    this.renderer = renderer;
    this.menus = menus;
    this.listen('click');
    this.watch(drive, backup);
  }

  render() {
    const drive = this.drive;
    this.root.hidden = !drive.isInspectorVisible;
    if (this.root.hidden) return;
    const item = drive.selectedItem;
    if (item) {
      Dom.fill(this.root, [this.details(item)]);
      return;
    }
    Dom.fill(this.root, [
      Dom.el('div', { class: 'empty' }, [
        Dom.icon('docSearch', 'empty-icon'),
        Dom.el('div', { class: 'empty-title', text: 'Nothing selected' }),
        Dom.el('div', { class: 'empty-message', text: 'Select a file or folder to see its details.' }),
      ]),
    ]);
  }

  /** Preview, name, actions and facts of one item. Used by the inspector and by column view. */
  details(item) {
    const drive = this.drive;
    const inTrash = drive.section === 'trash';
    const parts = [
      this.renderer.thumbnail(item, 'details-preview'),
      Dom.el('div', {}, [
        Dom.el('div', { class: 'details-name', text: item.name }),
        Dom.el('div', { class: 'details-kind', text: drive.details(item) }),
      ]),
      Dom.el('div', { class: 'details-actions' }, [
        Dom.button({
          label: inTrash ? 'In Trash' : 'Open',
          class: 'button is-primary is-wide',
          disabled: inTrash,
          data: { action: 'open', path: item.path },
        }),
        Dom.button({ icon: 'ellipsis', title: 'More actions', data: { action: 'more', path: item.path } }),
      ]),
    ];
    if (item.sync) {
      parts.push(Dom.el('hr', { class: 'divider' }));
      parts.push(
        Dom.el('div', { class: 'details-sync' }, [
          this.renderer.syncBadge(item),
          Dom.el('div', {}, [
            Dom.el('div', { class: 'details-sync-title', text: this.renderer.syncTitle(item) }),
            Dom.el('div', { class: 'details-sync-place', text: this.backup.status.destination }),
          ]),
        ]),
      );
    }
    parts.push(Dom.el('hr', { class: 'divider' }));
    parts.push(Dom.el('div', { class: 'section-title', text: 'Information' }));
    parts.push(this.infoRow('Where', drive.whereText(item)));
    parts.push(this.infoRow('Created', Format.day(item.created)));
    parts.push(this.infoRow('Modified', Format.modified(item.modified)));
    return Dom.el('div', { class: 'details' }, parts);
  }

  infoRow(label, value) {
    return Dom.el('div', { class: 'info-row' }, [
      Dom.el('span', { class: 'info-label', text: label }),
      Dom.el('span', { class: 'info-value', text: value }),
    ]);
  }

  onClick(event) {
    const element = this.actionOf(event);
    if (element) this.runAction(element);
  }

  /** Runs a button of the details. Column view shows the same buttons, so it calls this too. */
  runAction(element) {
    const path = element.dataset.path;
    if (element.dataset.action === 'open') {
      this.drive.open([path]);
    } else if (element.dataset.action === 'more') {
      this.menus.showItemMenu({ below: element }, [path]);
    }
  }
}
