import { AppConstants } from '../constants/app_constants.js';
import { Dom } from '../support/dom.js';
import { View } from './view.js';

/**
 * The bar at the top: back and forward, the path, the view mode, sort, the inspector button,
 * New, and the search field.
 *
 * It is built one time and then only changed, so the search field keeps the cursor while the user types.
 */
export class ToolbarView extends View {
  static VIEW_MODES = [
    { id: 'list', icon: 'list', title: 'List' },
    { id: 'grid', icon: 'grid', title: 'Grid' },
    { id: 'columns', icon: 'columns', title: 'Columns' },
  ];

  constructor(root, drive, menus) {
    super(root);
    this.drive = drive;
    this.menus = menus;
    /** The search field is hidden until the user asks for it with Command+F or Control+F. */
    this.isSearchShown = false;
    this.build();
    this.listen('click', 'input', 'keydown', 'focusout');
    this.watch(drive);
  }

  build() {
    this.sidebarButton = Dom.button({
      icon: 'sidebarLeft',
      class: 'tool show-sidebar',
      title: 'Show the sidebar',
      data: { action: 'toggleSidebar' },
    });
    this.backButton = Dom.button({ icon: 'chevronLeft', class: 'tool', title: 'Back', data: { action: 'back' } });
    this.forwardButton = Dom.button({
      icon: 'chevronRight',
      class: 'tool',
      title: 'Forward',
      data: { action: 'forward' },
    });
    this.crumbs = Dom.el('div', { class: 'crumbs' });
    this.modeButtons = [];
    for (const mode of ToolbarView.VIEW_MODES) {
      this.modeButtons.push(
        Dom.button({ icon: mode.icon, class: 'segment', title: mode.title, data: { action: 'mode', mode: mode.id } }),
      );
    }
    this.sortButton = Dom.el(
      'button',
      { type: 'button', class: 'tool tool-wide', title: 'Sort', data: { action: 'sort' } },
      [Dom.icon('sort'), Dom.icon('chevronDown', 'tool-caret')],
    );
    this.inspectorButton = Dom.button({
      icon: 'sidebarRight',
      class: 'tool',
      title: 'Show or hide the details',
      data: { action: 'inspector' },
    });
    this.newButton = Dom.button({ icon: 'plus', label: 'New', class: 'tool-new', data: { action: 'new' } });
    this.emptyTrashButton = Dom.button({
      icon: 'trash',
      label: 'Empty Trash',
      class: 'tool-new',
      data: { action: 'emptyTrash' },
    });
    this.searchInput = Dom.el('input', {
      class: 'search-input',
      type: 'text',
      placeholder: 'Search in Drive',
      spellcheck: 'false',
      autocomplete: 'off',
    });
    this.clearButton = Dom.button({
      icon: 'xCircle',
      class: 'search-clear',
      title: 'Clear',
      data: { action: 'clear' },
    });
    this.searchBox = Dom.el('div', { class: 'search' }, [Dom.icon('search'), this.searchInput, this.clearButton]);
    Dom.fill(this.root, [
      this.sidebarButton,
      this.backButton,
      this.forwardButton,
      this.crumbs,
      Dom.el('div', { class: 'segments' }, this.modeButtons),
      this.sortButton,
      Dom.el('div', { class: 'toolbar-space', 'data-tauri-drag-region': true }),
      this.inspectorButton,
      this.newButton,
      this.emptyTrashButton,
      this.searchBox,
    ]);
  }

  render() {
    const drive = this.drive;
    this.backButton.disabled = !drive.canGoBack;
    this.forwardButton.disabled = !drive.canGoForward;
    for (const button of this.modeButtons) {
      button.classList.toggle('is-picked', button.dataset.mode === drive.viewMode);
    }
    this.inspectorButton.classList.toggle('is-on', drive.isInspectorVisible);
    const inTrash = drive.section === 'trash';
    this.newButton.hidden = inTrash;
    this.newButton.disabled = !drive.canAddFiles;
    this.emptyTrashButton.hidden = !inTrash;
    this.emptyTrashButton.disabled = drive.items.length === 0;
    // Do not touch the field while the user types in it.
    if (this.searchInput.value !== drive.searchText) this.searchInput.value = drive.searchText;
    this.clearButton.hidden = drive.searchText === '';
    // A search that is in use stays on screen, so the user sees why the list is short.
    this.searchBox.hidden = !this.isSearchShown && drive.searchText === '';
    Dom.fill(this.crumbs, this.crumbParts());
  }

  /** Shows the search field and puts the cursor in it. */
  focusSearch() {
    this.isSearchShown = true;
    this.searchBox.hidden = false;
    this.searchInput.focus();
    this.searchInput.select();
  }

  crumbParts() {
    const drive = this.drive;
    if (drive.section !== 'myDrive') {
      return [Dom.el('span', { class: 'crumb is-current', text: AppConstants.section(drive.section).title })];
    }
    const parts = [this.crumb(AppConstants.section('myDrive').title, 0)];
    let level = 1;
    for (const folder of drive.path) {
      parts.push(Dom.icon('chevronRight', 'crumb-arrow'));
      parts.push(this.crumb(folder.name, level));
      level += 1;
    }
    return parts;
  }

  crumb(title, level) {
    const isCurrent = level === this.drive.path.length;
    return Dom.el('button', {
      type: 'button',
      class: `crumb${isCurrent ? ' is-current' : ''}`,
      text: title,
      data: { action: 'crumb', level },
    });
  }

  onClick(event) {
    const element = this.actionOf(event);
    if (!element) return;
    const drive = this.drive;
    switch (element.dataset.action) {
      case 'toggleSidebar':
        drive.toggleSidebar();
        break;
      case 'back':
        drive.goBack();
        break;
      case 'forward':
        drive.goForward();
        break;
      case 'crumb':
        drive.goToPathLevel(Number(element.dataset.level));
        break;
      case 'mode':
        drive.setViewMode(element.dataset.mode);
        break;
      case 'sort':
        this.menus.showSortMenu({ below: element });
        break;
      case 'inspector':
        drive.toggleInspector();
        break;
      case 'new':
        this.menus.showNewMenu({ below: element });
        break;
      case 'emptyTrash':
        drive.askEmptyTrash();
        break;
      case 'clear':
        drive.setSearch('');
        this.searchInput.focus();
        break;
      default:
        break;
    }
  }

  onInput(event) {
    if (event.target === this.searchInput) this.drive.setSearch(this.searchInput.value);
  }

  onKeyDown(event) {
    if (event.target === this.searchInput && event.key === 'Escape') {
      this.hideSearch();
      this.drive.setSearch('');
      this.searchInput.blur();
    }
  }

  handleEvent(event) {
    // The empty search field goes away when the user clicks somewhere else.
    if (event.type === 'focusout') {
      if (event.target === this.searchInput && this.searchInput.value === '') this.hideSearch();
    } else {
      super.handleEvent(event);
    }
  }

  hideSearch() {
    this.isSearchShown = false;
    this.searchBox.hidden = true;
  }
}
