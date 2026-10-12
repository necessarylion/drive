import { AppConstants } from '../constants/app_constants.js';
import { Dom } from '../support/dom.js';
import { View } from './view.js';

/**
 * The small dialogs of the app: a name (with a color for a new folder), a question with buttons,
 * and a message. Each function waits until the user answers.
 */
export class DialogView extends View {
  constructor(root) {
    super(root);
    this.pending = null;
    this.color = 'blue';
    this.listen('click', 'keydown');
  }

  get isOpen() {
    return !this.root.hidden;
  }

  /** Gives `{ name, color }`, or null when the user cancels. */
  askName(options) {
    this.color = 'blue';
    const input = Dom.el('input', { class: 'field', type: 'text', value: options.value, spellcheck: 'false' });
    const parts = [
      Dom.el('div', { class: 'sheet-head' }, [
        Dom.icon(options.icon, 'sheet-icon'),
        Dom.el('div', {}, [
          Dom.el('div', { class: 'sheet-title', text: options.title }),
          Dom.el('div', { class: 'sheet-subtitle', text: options.subtitle }),
        ]),
      ]),
      Dom.el('label', { class: 'sheet-label', text: 'Name' }),
      input,
    ];
    if (options.withColor) {
      parts.push(Dom.el('label', { class: 'sheet-label', text: 'Color' }));
      parts.push(Dom.el('div', { class: 'color-row', data: { colors: '1' } }, this.colorDots()));
    }
    parts.push(
      Dom.el('div', { class: 'sheet-buttons' }, [
        Dom.button({ label: 'Cancel', data: { action: 'cancel' } }),
        Dom.button({ label: options.confirmLabel, class: 'button is-primary', data: { action: 'confirm' } }),
      ]),
    );
    const waiting = this.present('name', null, parts);
    input.focus();
    // Pick the name without the ending, so typing replaces "Report" and keeps ".pdf".
    const dot = options.value.lastIndexOf('.');
    input.setSelectionRange(0, dot > 0 ? dot : options.value.length);
    return waiting;
  }

  /** Gives the id of the picked button. The first button is the one for Escape. */
  choose(options) {
    const buttons = [];
    for (const button of options.buttons) {
      const kind = button.kind ? ` is-${button.kind}` : '';
      buttons.push(Dom.button({ label: button.label, class: `button${kind}`, data: { action: `pick:${button.id}` } }));
    }
    return this.present('choose', options.buttons[0].id, [
      Dom.el('div', { class: 'sheet-title', text: options.title }),
      Dom.el('div', { class: 'sheet-message', text: options.message }),
      Dom.el('div', { class: 'sheet-buttons' }, buttons),
    ]);
  }

  alert(title, message) {
    return this.choose({ title, message, buttons: [{ id: 'ok', label: 'OK', kind: 'primary' }] });
  }

  /** `cancelAnswer` is what the dialog gives when the user presses Escape. */
  present(mode, cancelAnswer, parts) {
    // A new dialog takes the place of one that is still open.
    this.cancel();
    this.mode = mode;
    this.cancelAnswer = cancelAnswer;
    Dom.fill(this.root, [Dom.el('div', { class: 'sheet' }, parts)]);
    this.root.hidden = false;
    this.pending = Promise.withResolvers();
    const first = this.root.querySelector('.is-primary');
    if (first) first.focus();
    return this.pending.promise;
  }

  finish(answer) {
    if (!this.pending) return;
    this.root.hidden = true;
    this.pending.resolve(answer);
    this.pending = null;
  }

  confirm() {
    if (this.mode === 'name') {
      this.finish({ name: this.root.querySelector('input').value, color: this.color });
    } else {
      const primary = this.root.querySelector('.is-primary');
      if (primary) primary.click();
    }
  }

  cancel() {
    this.finish(this.cancelAnswer);
  }

  onClick(event) {
    const element = this.actionOf(event);
    if (!element) return;
    const action = element.dataset.action;
    if (action === 'cancel') this.cancel();
    else if (action === 'confirm') this.confirm();
    else if (action.startsWith('pick:')) this.finish(action.slice('pick:'.length));
    else if (action.startsWith('color:')) this.pickColor(action.slice('color:'.length));
  }

  onKeyDown(event) {
    if (event.key === 'Enter') {
      event.preventDefault();
      this.confirm();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      this.cancel();
    }
    // Keys in a dialog must not reach the shortcuts of the window behind it.
    event.stopPropagation();
  }

  pickColor(color) {
    this.color = color;
    Dom.fill(this.root.querySelector('[data-colors]'), this.colorDots());
  }

  colorDots() {
    const dots = [];
    for (const color of AppConstants.FOLDER_COLORS) {
      const picked = color === this.color ? ' is-picked' : '';
      dots.push(
        Dom.el('button', {
          type: 'button',
          class: `color-dot bg-${color}${picked}`,
          title: color,
          data: { action: `color:${color}` },
        }),
      );
    }
    return dots;
  }
}
