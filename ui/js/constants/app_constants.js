/** All global names and limits of the window code. */
export class AppConstants {
  static DEFAULT_FOLDER_NAME = 'Untitled Folder';
  static VIEW_MODE_KEY = 'viewMode';
  static BACKUP_CHANGED_EVENT = 'backup-changed';
  /** The Rust side sends this when "Settings…" is picked in the menu bar or in the tray menu. */
  static OPEN_SETTINGS_EVENT = 'open-settings';
  /** The event our own controllers send when their state changed. */
  static UPDATE_EVENT = 'update';
  /** How long to wait for more backup steps before the lists are read again. */
  static REFRESH_DELAY_MS = 400;
  static AWS_DEFAULT_REGION = 'us-east-1';
  /** The mouse must move this far with the button down before a drag starts. */
  static DRAG_START_PIXELS = 5;
  /** How often the window looks for a newer version of the app: every 6 hours. */
  static UPDATE_CHECK_MS = 6 * 60 * 60 * 1000;
  /** An item stays in the trash this long. The Rust side has the same number, and does the deleting. */
  static TRASH_KEEP_DAYS = 30;

  static SECTIONS = [
    {
      id: 'myDrive',
      title: 'My Drive',
      icon: 'drive',
      emptyTitle: 'This folder is empty',
      emptyMessage: 'Drop files here, or upload them.',
    },
    {
      id: 'recent',
      title: 'Recents',
      icon: 'clock',
      emptyTitle: 'No recent files',
      emptyMessage: 'Files you add or change show up here.',
    },
    {
      id: 'starred',
      title: 'Starred',
      icon: 'star',
      emptyTitle: 'No starred items',
      emptyMessage: 'Add a star to an item to find it fast.',
    },
    {
      id: 'trash',
      title: 'Trash',
      icon: 'trash',
      emptyTitle: 'Trash is empty',
      emptyMessage: `Items you move to the trash show up here. Drive deletes them forever after ${AppConstants.TRASH_KEEP_DAYS} days.`,
    },
  ];

  static SORT_FIELDS = [
    { id: 'name', title: 'Name' },
    { id: 'modified', title: 'Last modified' },
    { id: 'size', title: 'Size' },
  ];

  static FOLDER_COLORS = ['blue', 'purple', 'green', 'orange', 'pink', 'gray'];

  /** The icon and the words for each file kind. The color is in the CSS, as `kind-<name>`. */
  static KINDS = {
    folder: { icon: 'folder', label: 'Folder' },
    pdf: { icon: 'pdf', label: 'PDF document' },
    image: { icon: 'image', label: 'Image' },
    video: { icon: 'film', label: 'Video' },
    audio: { icon: 'waveform', label: 'Audio' },
    sheet: { icon: 'table', label: 'Spreadsheet' },
    slides: { icon: 'slides', label: 'Presentation' },
    text: { icon: 'docText', label: 'Text document' },
    archive: { icon: 'archive', label: 'Archive' },
    design: { icon: 'pencil', label: 'Design file' },
    other: { icon: 'doc', label: 'Document' },
  };

  static PROVIDERS = [
    {
      id: 'aws',
      title: 'AWS S3',
      bucketLabel: 'Bucket',
      keysHelp: 'Use the access key of an IAM user.',
      keyHint: 'AKIA…',
    },
    {
      id: 'gcs',
      title: 'Google Cloud Storage',
      bucketLabel: 'Bucket',
      keysHelp: 'Use an HMAC key. Make one in Cloud Storage > Settings > Interoperability.',
      keyHint: 'GOOG…',
    },
    {
      id: 'azure',
      title: 'Azure Blob Storage',
      bucketLabel: 'Container',
      keysHelp: 'Make a SAS token for the container. Allow Read, Add, Create, Write, Delete and List.',
      keyHint: '',
    },
    {
      id: 'gdrive',
      title: 'Google Drive',
      bucketLabel: 'Folder in Google Drive',
      keysHelp: 'Drive can only see the files it uploaded itself, not your other Google Drive files.',
      keyHint: '',
    },
  ];

  static section(id) {
    for (const section of AppConstants.SECTIONS) {
      if (section.id === id) return section;
    }
    return AppConstants.SECTIONS[0];
  }

  static provider(id) {
    for (const provider of AppConstants.PROVIDERS) {
      if (provider.id === id) return provider;
    }
    return AppConstants.PROVIDERS[0];
  }
}
