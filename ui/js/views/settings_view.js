import { AppConstants } from '../constants/app_constants.js';
import { Dom } from '../support/dom.js';
import { View } from './view.js';

/**
 * The Settings dialog: where the files are kept, where the backup goes, and the access keys.
 * The backup settings on screen are a draft. The backup does not use them until the user chooses Save.
 */
export class SettingsView extends View {
  constructor(root, settings, backup) {
    super(root);
    this.settings = settings;
    this.backup = backup;
    this.storageRoot = '';
    this.defaultStorageRoot = '';
    this.draft = null;
    this.saved = null;
    this.keyHint = null;
    /** The result of the last action with the keys, in plain words. */
    this.result = '';
    this.isTesting = false;
    this.listen('click', 'input', 'change', 'keydown');
  }

  get isOpen() {
    return !this.root.hidden;
  }

  get isDirty() {
    return JSON.stringify(this.draft) !== JSON.stringify(this.saved);
  }

  get provider() {
    return AppConstants.provider(this.draft.provider);
  }

  async open() {
    await this.revert();
    this.root.hidden = false;
  }

  close() {
    this.root.hidden = true;
    this.root.replaceChildren();
  }

  /** Drops the changes that are not saved. Also runs when the dialog opens. */
  async revert() {
    const loaded = await this.settings.load();
    this.storageRoot = loaded.storageRoot;
    this.defaultStorageRoot = loaded.defaultStorageRoot;
    this.saved = loaded.config;
    this.draft = { ...loaded.config };
    await this.refreshKeys();
  }

  /** Each provider has its own keys, so show the ones of the provider on screen. */
  async refreshKeys() {
    this.keyHint = await this.backup.keyHint(this.draft.provider);
    this.result = '';
    this.render();
  }

  render() {
    Dom.fill(this.root, [
      Dom.el('div', { class: 'sheet settings' }, [
        Dom.el('div', { class: 'settings-head' }, [
          Dom.el('div', { class: 'sheet-title', text: 'Settings' }),
          Dom.button({ label: 'Done', class: 'button is-primary', data: { action: 'close' } }),
        ]),
        this.storageSection(),
        this.backupSection(),
        this.keysSection(),
      ]),
    ]);
    this.syncButtons();
  }

  storageSection() {
    return this.section(
      'Storage',
      [
        Dom.el('div', { class: 'form-row' }, [
          Dom.el('span', { class: 'form-label', text: 'Folder' }),
          Dom.el('span', { class: 'form-value', text: this.storageRoot || this.defaultStorageRoot }),
        ]),
        Dom.el('div', { class: 'form-buttons' }, [
          Dom.button({ label: 'Use Default', disabled: this.storageRoot === '', data: { action: 'useDefault' } }),
          Dom.button({ label: 'Choose…', data: { action: 'chooseFolder' } }),
        ]),
      ],
      'This is where Drive keeps your files. Drive opens again right after a change, to show the new folder. ' +
        'Files in the old folder stay there. Drive does not move them.',
    );
  }

  backupSection() {
    const id = this.draft.provider;
    const options = [];
    for (const provider of AppConstants.PROVIDERS) {
      options.push(Dom.el('option', { value: provider.id, text: provider.title, selected: provider.id === id }));
    }
    const rows = [
      Dom.el('label', { class: 'form-row' }, [
        Dom.el('span', { class: 'form-label', text: 'Back up my files' }),
        Dom.el('input', { type: 'checkbox', checked: this.draft.enabled, data: { field: 'enabled' } }),
      ]),
      Dom.el('label', { class: 'form-row' }, [
        Dom.el('span', { class: 'form-label', text: 'Provider' }),
        Dom.el('select', { class: 'field', data: { field: 'provider' } }, options),
      ]),
    ];
    if (id === 'azure') rows.push(this.field('Storage account', 'account', 'mydriveaccount'));
    if (id === 'gdrive') {
      rows.push(this.field('Client ID', 'clientId', '123-abc.apps.googleusercontent.com'));
      rows.push(this.field('Client secret', 'clientSecret', 'GOCSPX-…'));
    }
    rows.push(this.field(this.provider.bucketLabel, 'bucket', 'my-drive-backup'));
    if (id === 'aws') rows.push(this.field('Region', 'region', AppConstants.AWS_DEFAULT_REGION));
    rows.push(this.field('Folder in it', 'prefix', 'Optional'));
    rows.push(
      Dom.el('div', { class: 'form-buttons' }, [
        Dom.el('span', { class: 'form-note', data: { role: 'dirty' } }),
        Dom.button({ label: 'Revert', data: { action: 'revert' } }),
        Dom.button({ label: 'Save', data: { action: 'save' } }),
      ]),
    );
    return this.section(
      'Backup',
      rows,
      'Drive uploads new and changed files by itself when the backup is on and a bucket, container or folder is set. ' +
        'Turning it off removes nothing from the backup. ' +
        'A change here takes effect after you choose Save.',
    );
  }

  keysSection() {
    const id = this.draft.provider;
    const rows = [];
    if (this.keyHint) {
      rows.push(
        Dom.el('div', { class: 'form-row' }, [
          Dom.el('span', { class: 'form-label', text: 'Saved key' }),
          Dom.el('span', { class: 'form-value is-mono', text: this.keyHint }),
        ]),
      );
    }
    // Google Drive has no keys to type. The user signs in with the button below.
    if (id === 'azure') {
      rows.push(this.keyField('SAS token', 'secretKey', 'Paste it here', true));
    } else if (id !== 'gdrive') {
      rows.push(this.keyField('Access key ID', 'accessKey', this.provider.keyHint, false));
      rows.push(this.keyField('Secret access key', 'secretKey', 'Paste it here', true));
    }
    const lastButton =
      id === 'gdrive'
        ? Dom.button({ label: 'Sign In with Google…', data: { action: 'signIn' } })
        : Dom.button({ label: 'Save Keys', data: { action: 'saveKeys' } });
    rows.push(
      Dom.el('div', { class: 'form-buttons' }, [
        Dom.el('span', { class: 'form-note', text: this.result }),
        this.isTesting ? Dom.el('span', { class: 'spinner' }) : null,
        Dom.button({ label: 'Test Connection', data: { action: 'test' } }),
        Dom.button({ label: 'Remove', class: 'button is-danger-text', data: { action: 'removeKeys' } }),
        lastButton,
      ]),
    );
    return this.section(
      id === 'gdrive' ? 'Google Account' : 'Access Keys',
      rows,
      `${this.provider.keysHelp} The keys are kept in the keychain of this computer.`,
    );
  }

  section(title, rows, footer) {
    return Dom.el('section', { class: 'form-section' }, [
      Dom.el('div', { class: 'form-title', text: title }),
      Dom.el('div', { class: 'form-box' }, rows),
      Dom.el('div', { class: 'form-footer', text: footer }),
    ]);
  }

  field(label, name, placeholder) {
    return Dom.el('label', { class: 'form-row' }, [
      Dom.el('span', { class: 'form-label', text: label }),
      Dom.el('input', {
        class: 'field',
        type: 'text',
        value: this.draft[name],
        placeholder,
        spellcheck: 'false',
        autocomplete: 'off',
        data: { field: name },
      }),
    ]);
  }

  keyField(label, name, placeholder, isSecret) {
    return Dom.el('label', { class: 'form-row' }, [
      Dom.el('span', { class: 'form-label', text: label }),
      Dom.el('input', {
        class: 'field',
        type: isSecret ? 'password' : 'text',
        placeholder,
        spellcheck: 'false',
        autocomplete: 'off',
        data: { key: name },
      }),
    ]);
  }

  /** Turns the buttons on and off for what is typed now. It does not build the form again, so typing is not disturbed. */
  syncButtons() {
    const dirty = this.isDirty;
    const draft = this.draft;
    this.root.querySelector('[data-role="dirty"]').textContent = dirty ? 'These changes are not saved yet.' : '';
    this.setDisabled('revert', !dirty);
    this.setDisabled('save', !dirty);
    this.setDisabled('test', this.isTesting || draft.bucket === '');
    this.setDisabled('removeKeys', this.keyHint === null);
    this.setDisabled('signIn', draft.clientId === '' || draft.bucket === '');
    this.setDisabled(
      'saveKeys',
      this.keyValue('secretKey') === '' || (this.keyValue('accessKey') === '' && draft.provider !== 'azure'),
    );
  }

  /** Draws the form again, and keeps what is typed in the key fields. */
  redraw() {
    const typed = [this.keyValue('accessKey'), this.keyValue('secretKey')];
    this.render();
    this.setKeyValue('accessKey', typed[0]);
    this.setKeyValue('secretKey', typed[1]);
    this.syncButtons();
  }

  setKeyValue(name, value) {
    const input = this.root.querySelector(`[data-key="${name}"]`);
    if (input) input.value = value;
  }

  setDisabled(action, disabled) {
    const button = this.root.querySelector(`[data-action="${action}"]`);
    if (button) button.disabled = disabled;
  }

  keyValue(name) {
    const input = this.root.querySelector(`[data-key="${name}"]`);
    return input ? input.value : '';
  }

  onInput(event) {
    const field = event.target.dataset.field;
    if (field === 'enabled') this.draft.enabled = event.target.checked;
    else if (field && field !== 'provider') this.draft[field] = event.target.value;
    this.syncButtons();
  }

  /** Each provider has its own bucket and folder, so show the saved ones of the picked provider. */
  async onChange(event) {
    if (event.target.dataset.field !== 'provider') return;
    // The on/off switch is one for all providers, so keep what the user set.
    this.draft = { ...(await this.settings.configFor(event.target.value)), enabled: this.draft.enabled };
    await this.refreshKeys();
  }

  onKeyDown(event) {
    if (event.key === 'Escape') this.close();
    // Keys in the dialog must not reach the shortcuts of the window behind it.
    event.stopPropagation();
  }

  async onClick(event) {
    const element = this.actionOf(event);
    if (!element) return;
    switch (element.dataset.action) {
      case 'close':
        this.close();
        break;
      case 'useDefault':
        await this.setStorageRoot('');
        break;
      case 'chooseFolder':
        await this.chooseFolder();
        break;
      case 'revert':
        await this.revert();
        break;
      case 'save':
        await this.backup.saveSettings(this.draft);
        // Read them back, so the screen shows what was saved (spaces around the text are cut off).
        await this.revert();
        break;
      case 'test':
        await this.testConnection();
        break;
      case 'removeKeys':
        await this.backup.removeKeys(this.draft.provider);
        await this.showResult('Keys removed.');
        break;
      case 'saveKeys':
        await this.saveKeys();
        break;
      case 'signIn':
        await this.signIn();
        break;
      default:
        break;
    }
  }

  async chooseFolder() {
    const folder = await this.settings.pickStorageRoot();
    if (folder) await this.setStorageRoot(folder);
  }

  async setStorageRoot(path) {
    await this.settings.setStorageRoot(path);
    this.storageRoot = path;
    this.render();
  }

  async testConnection() {
    this.isTesting = true;
    this.result = '';
    this.redraw();
    try {
      const count = await this.backup.testConnection(this.draft);
      this.result = `It works. ${count} file(s) are in the backup.`;
    } catch (error) {
      this.result = String(error);
    }
    this.isTesting = false;
    this.redraw();
  }

  async saveKeys() {
    try {
      await this.backup.saveKeys(this.keyValue('accessKey'), this.keyValue('secretKey'), this.draft.provider);
      // The form is built again, so the secret does not stay on screen after it is saved.
      await this.showResult('Keys saved.');
    } catch (error) {
      this.result = String(error);
      this.redraw();
    }
  }

  async signIn() {
    this.result = 'Finish the sign-in in your browser.';
    this.redraw();
    try {
      await this.backup.googleSignIn(this.draft);
      await this.showResult('Signed in.');
    } catch (error) {
      this.result = String(error);
      this.redraw();
    }
  }

  /** Reads the saved key again, then shows the message. */
  async showResult(message) {
    await this.refreshKeys();
    this.result = message;
    this.render();
  }
}
