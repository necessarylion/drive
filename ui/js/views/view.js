import { AppConstants } from '../constants/app_constants.js';

/**
 * The base of all views. A view draws one part of the window into `root`.
 *
 * It listens with itself as the listener (`handleEvent`), so no callback functions are needed.
 * What a click does is written on the element, as `data-action`.
 */
export class View {
  constructor(root) {
    this.root = root;
  }

  /** Starts to listen for these DOM events on the root. */
  listen(...types) {
    for (const type of types) {
      this.root.addEventListener(type, this);
    }
  }

  /** Draws again each time one of these controllers changes. */
  watch(...controllers) {
    for (const controller of controllers) {
      controller.addEventListener(AppConstants.UPDATE_EVENT, this);
    }
  }

  handleEvent(event) {
    switch (event.type) {
      case AppConstants.UPDATE_EVENT:
        this.render();
        break;
      case 'click':
        this.onClick(event);
        break;
      case 'contextmenu':
        this.onContextMenu(event);
        break;
      case 'input':
        this.onInput(event);
        break;
      case 'change':
        this.onChange(event);
        break;
      case 'keydown':
        this.onKeyDown(event);
        break;
      default:
        break;
    }
  }

  /** The element with a `data-action` that the event happened in, or null. */
  actionOf(event) {
    const element = event.target instanceof Element ? event.target.closest('[data-action]') : null;
    return element && this.root.contains(element) ? element : null;
  }

  render() {}

  onClick(_event) {}

  onContextMenu(_event) {}

  onInput(_event) {}

  onChange(_event) {}

  onKeyDown(_event) {}
}
