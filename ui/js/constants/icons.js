/**
 * The icons of the app, as the inside of a 24 x 24 SVG. They are drawn with lines,
 * so they look the same on every system.
 */
export class Icons {
  static CLOUD = '<path d="M7 18a4 4 0 0 1-.6-7.95A6 6 0 0 1 18 9.5a4.25 4.25 0 0 1-.5 8.5z"/>';
  static DOC = '<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v4h4"/>';
  static FOLDER = '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>';
  static FRAME = '<rect x="3" y="4" width="18" height="16" rx="2"/>';

  static PATHS = {
    clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
    star: '<path d="M12 3.5l2.6 5.3 5.9.9-4.2 4.1 1 5.8L12 16.9 6.7 19.6l1-5.8L3.5 9.7l5.9-.9z"/>',
    trash: '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>',
    folder: Icons.FOLDER,
    folderPlus: `${Icons.FOLDER}<path d="M12 11v5M9.5 13.5h5"/>`,
    doc: Icons.DOC,
    docText: `${Icons.DOC}<path d="M9 12h6M9 16h6"/>`,
    docSearch: `${Icons.DOC}<circle cx="11.5" cy="14" r="2.5"/><path d="M13.5 16l2 2"/>`,
    pdf: `${Icons.DOC}<rect x="9" y="11" width="6" height="4"/><path d="M9 18h4"/>`,
    image:
      '<rect x="3" y="5" width="18" height="14" rx="2"/><circle cx="8.5" cy="10" r="1.5"/><path d="M21 16l-5-5-8 8"/>',
    film: `${Icons.FRAME}<path d="M8 4v16M16 4v16M3 9h5M3 15h5M16 9h5M16 15h5"/>`,
    waveform: '<path d="M4 10v4M8 6v12M12 3v18M16 7v10M20 10v4"/>',
    table: '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 10h18M3 14.5h18M9 5v14M15 5v14"/>',
    slides: '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M12 16v4M8 20h8"/>',
    archive: '<path d="M4 4h16v4H4zM5 8v12h14V8M10 12h4"/>',
    pencil: '<path d="M4 20l1-4L16 5l3 3L8 19zM14 7l3 3"/>',
    list: '<path d="M8 6h13M8 12h13M8 18h13M3.5 6h.01M3.5 12h.01M3.5 18h.01"/>',
    grid: '<rect x="4" y="4" width="6.5" height="6.5" rx="1"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1"/>',
    columns: `${Icons.FRAME}<path d="M9 4v16M15 4v16"/>`,
    sort: '<path d="M7 20V4M7 4L4 7M7 4l3 3M17 4v16M17 20l-3-3M17 20l3-3"/>',
    sidebarLeft: `${Icons.FRAME}<path d="M9 4v16"/>`,
    sidebarRight: `${Icons.FRAME}<path d="M15 4v16"/>`,
    plus: '<path d="M12 5v14M5 12h14"/>',
    search: '<circle cx="11" cy="11" r="6.5"/><path d="M20 20l-4-4"/>',
    x: '<path d="M6 6l12 12M18 6L6 18"/>',
    xCircle: '<circle cx="12" cy="12" r="9"/><path d="M9 9l6 6M15 9l-6 6"/>',
    chevronLeft: '<path d="M15 5l-7 7 7 7"/>',
    chevronRight: '<path d="M9 5l7 7-7 7"/>',
    chevronUp: '<path d="M5 15l7-7 7 7"/>',
    chevronDown: '<path d="M5 9l7 7 7-7"/>',
    check: '<path d="M5 12.5l4.5 4.5L19 7"/>',
    cloud: Icons.CLOUD,
    cloudUp: `${Icons.CLOUD}<path d="M12 16v-5M9.5 13l2.5-2.5 2.5 2.5"/>`,
    cloudCheck: `${Icons.CLOUD}<path d="M9.5 13.5l2 2 3.5-3.5"/>`,
    checkCircle: '<circle cx="12" cy="12" r="9"/><path d="M8 12.5l3 3 5-6"/>',
    sync: '<path d="M20 12a8 8 0 0 1-14 5.3M4 12a8 8 0 0 1 14-5.3M18 3v4h-4M6 21v-4h4"/>',
    pause: '<rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/>',
    play: '<path d="M7 4l13 8-13 8z"/>',
    gear: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>',
    ellipsis: '<path d="M5 12h.01M12 12h.01M19 12h.01" stroke-width="3"/>',
    pin: '<path d="M9 3h6l-1 6 3 3H7l3-3zM12 12v9"/>',
    copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/>',
    download: '<path d="M12 4v11M7 11l5 5 5-5M5 20h14"/>',
    upload: '<path d="M12 16V5M7 9l5-5 5 5M5 20h14"/>',
    info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7.5h.01"/>',
    open: '<path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>',
    undo: '<path d="M9 7l-5 5 5 5M4 12h11a4 4 0 0 1 0 8h-3"/>',
  };

  /** The full SVG text of one icon. `filled` paints the inside too, like the yellow star. */
  static svg(name, filled = false) {
    const body = Icons.PATHS[name] ?? Icons.PATHS.doc;
    const fill = filled ? 'currentColor' : 'none';
    return `<svg viewBox="0 0 24 24" fill="${fill}" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
  }
}
