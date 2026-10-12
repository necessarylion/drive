import { AppConstants } from '../constants/app_constants.js';
import { Dom } from '../support/dom.js';
import { Format } from '../support/format.js';

/**
 * Moves items of the drive by dragging them onto a folder. A folder is anything with `data-folder`:
 * a folder card, a folder in a column, a pinned folder, "My Drive" in the sidebar, and the path at the top.
 */
// ponytail: this uses pointer events, not HTML drag and drop. HTML drag and drop does not work on Windows
// while the window takes file drops from the system. The list does not scroll by itself during a drag.
// Add that if the folder to drop on is often out of view.
export class ItemDrag {
  constructor(drive, page) {
    this.drive = drive;
    this.page = page;
    /** The item the mouse button went down on. Null when no button is down. */
    this.pressed = null;
    /** The paths that are dragged. */
    this.paths = [];
    /** The small label that follows the mouse. It is only there during a drag. */
    this.ghost = null;
    /** The folder under the mouse. */
    this.target = null;
    this.isClickSwallowed = false;
    this.allowClicks = this.keepClicks.bind(this);
  }

  start() {
    const page = this.page;
    page.addEventListener('pointerdown', this);
    page.addEventListener('pointermove', this);
    page.addEventListener('pointerup', this);
    page.addEventListener('pointercancel', this);
    page.addEventListener('keydown', this);
    // In the capture phase, so the click that ends a drag does not reach a view.
    page.addEventListener('click', this, true);
    window.addEventListener('blur', this);
  }

  handleEvent(event) {
    switch (event.type) {
      case 'pointerdown':
        this.onPointerDown(event);
        break;
      case 'pointermove':
        this.onPointerMove(event);
        break;
      case 'pointerup':
        this.onPointerUp(event);
        break;
      case 'click':
        this.onClick(event);
        break;
      case 'keydown':
        if (event.key === 'Escape') this.end();
        break;
      default:
        // The pointer was taken away, or the window went to the back.
        this.end();
        break;
    }
  }

  onPointerDown(event) {
    // Items in the trash are put back with "Put Back", not by dragging.
    if (event.button !== 0 || this.drive.section === 'trash') return;
    const element = event.target instanceof Element ? event.target.closest('[data-action="item"]') : null;
    if (!element) return;
    const data = element.dataset;
    this.pressed = { path: data.path, depth: data.depth, x: event.clientX, y: event.clientY };
  }

  onPointerMove(event) {
    const pressed = this.pressed;
    if (!pressed) return;
    // The button was let go outside the window.
    if (event.buttons === 0) {
      this.end();
      return;
    }
    if (!this.ghost) {
      const distance = Math.hypot(event.clientX - pressed.x, event.clientY - pressed.y);
      // A small move during a click is not a drag.
      if (distance < AppConstants.DRAG_START_PIXELS) return;
      this.begin(pressed);
    }
    this.ghost.style.transform = `translate(${event.clientX + 12}px, ${event.clientY + 14}px)`;
    this.aim(this.folderAt(event));
  }

  onPointerUp(event) {
    if (!this.ghost) {
      this.pressed = null;
      return;
    }
    const folder = this.folderAt(event);
    const paths = this.paths;
    this.end();
    // The browser sends a click after the button goes up. It must not change the selection.
    this.isClickSwallowed = true;
    setTimeout(this.allowClicks, 0);
    if (folder) this.drive.move(paths, folder.dataset.path);
  }

  onClick(event) {
    if (!this.isClickSwallowed) return;
    this.isClickSwallowed = false;
    event.stopPropagation();
    event.preventDefault();
  }

  /** Runs right after a drag, for the case that no click came. */
  keepClicks() {
    this.isClickSwallowed = false;
  }

  begin(pressed) {
    // In column view the drag is for the one item under the mouse, like its menu.
    this.paths = pressed.depth === undefined ? this.drive.targets(pressed.path) : [pressed.path];
    const item = this.drive.find(pressed.path);
    const label = this.paths.length === 1 && item ? item.name : Format.items(this.paths.length);
    this.ghost = Dom.el('div', { class: 'drag-ghost', text: label });
    this.page.body.append(this.ghost);
    this.page.body.classList.add('is-dragging');
  }

  /** The folder under the mouse, or null. A dragged folder cannot be dropped on itself. */
  folderAt(event) {
    const under = this.page.elementFromPoint(event.clientX, event.clientY);
    const folder = under ? under.closest('[data-folder]') : null;
    return folder && !this.paths.includes(folder.dataset.path) ? folder : null;
  }

  /** Marks the folder the items would go into. */
  aim(folder) {
    if (folder === this.target) return;
    if (this.target) this.target.classList.remove('is-drop-target');
    if (folder) folder.classList.add('is-drop-target');
    this.target = folder;
  }

  end() {
    this.aim(null);
    if (this.ghost) this.ghost.remove();
    this.ghost = null;
    this.pressed = null;
    this.paths = [];
    this.page.body.classList.remove('is-dragging');
  }
}
