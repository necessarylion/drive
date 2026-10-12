/** Turns numbers and dates into the words the window shows. */
export class Format {
  static UNITS = ['kB', 'MB', 'GB', 'TB'];
  static DIGITS = [0, 1, 2, 2];
  static DAY_MS = 24 * 60 * 60 * 1000;
  /** The steps of "5 minutes ago": the name of the unit, and how many seconds it has. */
  static STEPS = [
    ['year', 365 * 24 * 3600],
    ['month', 30 * 24 * 3600],
    ['day', 24 * 3600],
    ['hour', 3600],
    ['minute', 60],
    ['second', 1],
  ];

  /** "846 kB", "18.4 MB". Like a file manager, 1 kB is 1000 bytes. */
  static bytes(size) {
    if (size === 0) return 'Zero kB';
    if (size === 1) return '1 byte';
    if (size < 1000) return `${size} bytes`;
    let value = size / 1000;
    let unit = 0;
    while (value >= 1000 && unit < Format.UNITS.length - 1) {
      value /= 1000;
      unit += 1;
    }
    return `${Number(value.toFixed(Format.DIGITS[unit]))} ${Format.UNITS[unit]}`;
  }

  static items(count) {
    return count === 1 ? '1 item' : `${count} items`;
  }

  /** "Oct 8, 2026" */
  static day(millis) {
    return new Date(millis).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }

  /** "Today, 10:42", "Yesterday, 17:30" or "Oct 8, 2026". */
  static modified(millis) {
    const date = new Date(millis);
    const time = date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    if (millis >= today.getTime()) return `Today, ${time}`;
    if (millis >= today.getTime() - Format.DAY_MS) return `Yesterday, ${time}`;
    return Format.day(millis);
  }

  /** "5 minutes ago", "yesterday", "now". */
  static relative(millis) {
    const seconds = Math.round((millis - Date.now()) / 1000);
    const formatter = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });
    for (const [unit, size] of Format.STEPS) {
      if (Math.abs(seconds) >= size || unit === 'second') {
        return formatter.format(Math.round(seconds / size), unit);
      }
    }
    return '';
  }
}
