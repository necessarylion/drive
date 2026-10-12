import { Icons } from '../constants/icons.js';

/** Small helpers to build the page without a framework. */
export class Dom {
  /**
   * Makes one element. `attributes` can hold `class`, `text`, `data` (an object), `style`,
   * and any HTML attribute. A value of null or false leaves the attribute out.
   */
  static el(tag, attributes = {}, children = []) {
    const element = document.createElement(tag);
    for (const [name, value] of Object.entries(attributes)) {
      if (value === null || value === undefined || value === false) continue;
      if (name === 'class') element.className = value;
      else if (name === 'text') element.textContent = value;
      else if (name === 'data') Dom.setData(element, value);
      else if (name === 'style') element.style.cssText = value;
      else element.setAttribute(name, value === true ? '' : value);
    }
    for (const child of children) {
      if (child) element.append(child);
    }
    return element;
  }

  /** An icon from our own icon list. The SVG text is ours, so it is safe to put in as HTML. */
  static icon(name, className = '', filled = false) {
    const span = Dom.el('span', { class: `icon ${className}`.trim() });
    span.innerHTML = Icons.svg(name, filled);
    return span;
  }

  /** A button with an icon, a text, or both. `data.action` says what a click does. */
  static button(options) {
    const children = [];
    if (options.icon) children.push(Dom.icon(options.icon));
    if (options.label) children.push(Dom.el('span', { text: options.label }));
    return Dom.el(
      'button',
      {
        type: 'button',
        class: options.class ?? 'button',
        title: options.title ?? null,
        disabled: options.disabled === true,
        data: options.data ?? {},
      },
      children,
    );
  }

  /** Puts new children in place of the old ones. Empty entries are left out. */
  static fill(parent, children) {
    const kept = [];
    for (const child of children) {
      if (child) kept.push(child);
    }
    parent.replaceChildren(...kept);
  }

  static setData(element, values) {
    for (const [name, value] of Object.entries(values)) {
      if (value !== null && value !== undefined) element.dataset[name] = String(value);
    }
  }
}
