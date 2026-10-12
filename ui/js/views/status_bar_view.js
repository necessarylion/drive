import { Dom } from '../support/dom.js';
import { View } from './view.js';

/** The bar at the bottom: the item count on the left, the backup on the right. */
export class StatusBarView extends View {
  constructor(root, drive, backup, menus) {
    super(root);
    this.drive = drive;
    this.backup = backup;
    this.menus = menus;
    this.listen('click');
    this.watch(drive, backup);
  }

  render() {
    const status = this.backup.status;
    const right = [
      Dom.button({
        icon: status.isRunning ? 'cloudUp' : 'cloud',
        class: 'status-cloud',
        title: 'Backup',
        data: { action: 'menu' },
      }),
      Dom.el('span', { class: 'status-summary', text: status.summary }),
    ];
    if (status.isUploading) {
      const percent = Math.round(status.fraction * 100);
      right.push(Dom.el('progress', { class: 'status-progress', max: '100', value: String(percent) }));
      right.push(Dom.el('span', { class: 'status-percent', text: `${percent}%` }));
    }
    if (status.isConfigured) {
      right.push(
        Dom.button({
          icon: status.isPaused ? 'play' : 'pause',
          class: 'status-pause',
          title: status.isPaused ? 'Resume the backup' : 'Pause the backup',
          data: { action: 'pause' },
        }),
      );
    }
    right.push(
      Dom.button({ icon: 'gear', class: 'status-pause status-gear', title: 'Settings', data: { action: 'settings' } }),
    );
    Dom.fill(this.root, [
      Dom.el('span', { text: this.drive.statusText }),
      this.drive.isBusy ? Dom.el('span', { class: 'spinner' }) : null,
      Dom.el('span', { class: 'status-space' }),
      Dom.el('div', { class: 'status-backup' }, right),
    ]);
  }

  onClick(event) {
    const element = this.actionOf(event);
    if (!element) return;
    if (element.dataset.action === 'menu') {
      // The menu opens above the button, because the bar is at the bottom of the window.
      const box = element.getBoundingClientRect();
      this.menus.showBackupMenu({ x: box.left, y: box.top - 130 });
    } else if (element.dataset.action === 'pause') {
      this.backup.togglePause();
    } else if (element.dataset.action === 'settings') {
      this.menus.openSettings();
    }
  }
}
