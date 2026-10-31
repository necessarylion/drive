# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Drive is a desktop app for macOS, Windows and Linux, built with Tauri 2. It manages the files in one folder on the computer and can back them up to AWS S3, Google Cloud Storage, Azure Blob Storage or Google Drive.

- `src-tauri/` is the Rust side (edition 2024). It holds all the rules and all outside access.
- `ui/` is the window: plain HTML, CSS and JavaScript modules. There is no framework, no bundler and no build step.

Keep the dependencies few. S3 calls and AWS Signature Version 4 signing are written by hand on top of `reqwest`, `sha2` and `hmac`. Do not add an AWS SDK or a frontend framework.

## Commands

| Command | What it does |
| --- | --- |
| `make setup` | Install the dev tools (Tauri CLI and Biome) with Bun |
| `make run` | Run the app (`bun tauri dev`) |
| `make app` | Build the app and its installers for this computer |
| `make test` | Run all Rust tests (`cargo test` in `src-tauri`) |
| `make format` | `cargo fmt` and `biome format` |
| `make lint` | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `biome check ui` |
| `make icon` | Rebuild the icons from `Resources/Logo.png` |

Run one test or one test file:

```sh
cd src-tauri
cargo test --test drive_service_tests
cargo test --test drive_service_tests full_life_of_a_file
```

Run the app without touching the real files:

```sh
DRIVE_STORAGE_ROOT=/tmp/drive-demo make run
```

Run `make format`, `make lint` and `make test` before a pull request. Clippy runs with `pedantic` and warnings are errors. The style is in `src-tauri/rustfmt.toml` (120 columns) and `biome.json` (2 spaces, 120 columns, single quotes).

A release is made by pushing a tag that starts with `v` and has three plain numbers, like `v1.2.0`. `.github/workflows/build.yml` then builds for macOS (Apple silicon, Intel), Linux (x64, arm64) and Windows (x64, arm64, 32-bit) in parallel, and adds the installers to the GitHub release. On a pull request and on a push to `main`, the workflow only runs the checks (format, Clippy, tests, Biome). Start it by hand on the Actions page to try the builds without a release. The app is only signed ad hoc.

A release build also makes the signed update files and `latest.json` (the workflow sets `bundle.createUpdaterArtifacts` for a tag only). It needs the repository secret `TAURI_SIGNING_PRIVATE_KEY`. Other builds, also `make app`, need no key.

## Architecture

The code goes in one direction: view → controller → service → repository → model. Both sides follow it, and both use dependency injection through the constructor.

### Rust side (`src-tauri/src/`)

- **`lib.rs`**: `AppContainer::install` is the one place where all parts are built and wired together. Add new services and repositories there. New commands must also be added to `generate_handler!` in `run`.
- **`commands/`**: the functions the window can call (`#[tauri::command]`). They are the "route" layer: each one only hands the call to a controller. All of them are `async`, so none blocks the main thread.
- **`controllers/`**: `DriveController` builds what the window shows (`ItemView` = an item plus color, summary, pin and backup state). `BackupController` keeps the state of a running backup. `SettingsController` reads and saves the storage folder. `UpdateController` asks `tauri-plugin-updater` for a newer release and installs it.
- **`services/`**: the rules. `DriveService` (list, add, rename, star, pin, trash), `BackupService` (compare with the backup, upload, restore), `AwsSigner` (SigV4).
- **`repositories/`**: all outside access. `FileRepository` (disk), `MetadataRepository` and `BackupStateRepository` (JSON files), `SettingsRepository` (`settings.json`), `KeychainRepository` (the keychain of the system, through the `keyring` crate), `HttpTransport` (all HTTP), and one `BackupRepository` per provider.
- **`config/` and `constants/`**: `AppConfig` (where data is stored), `BackupConfig` (provider, bucket, region, prefix), `AppConstants` (all keys and limits).
- **`tray.rs`**: the tray icon and its menu.

### Window side (`ui/js/`)

- **`main.js`**: `AppContainer` builds and wires all parts of the window.
- **`views/`**: one class per part of the window. Every view extends `View`.
- **`controllers/`**: `DriveController` owns navigation, selection, sort, view mode and the item lists. `BackupController` holds the backup status. `UpdateController` looks for a newer version and asks before it installs one.
- **`views/item_drag.js`**: `ItemDrag` is not a view. Like `Shortcuts`, it listens on the whole page.
- **`repositories/`**: `TauriGateway` is the only class that touches `window.__TAURI__`. The other repositories call commands through it.

### Things to know before you change code

- **No arrow functions and no callbacks in the window code.** Views listen with themselves as the listener (`addEventListener(type, this)` and `handleEvent`). What a click does is written on the element as `data-action`, and the view runs it in a `switch`. Where an API must have a function, pass a bound method. Menus and dialogs give back a promise (`Promise.withResolvers`), so the caller uses `await`.
- **Views draw from state.** A controller sends an `update` event after each change, and the views build their part again. `ToolbarView` and `SettingsView` are the exceptions: they change their elements in place, so a text field keeps the cursor while the user types.
- **Never put outside text into the page as HTML.** `Dom.el` sets text with `textContent`. Only `Dom.icon` uses `innerHTML`, and only with our own SVG from `Icons`.
- **The design is fixed.** The sizes and colors in `ui/css/app.css` are the ones of the first (Swift) version of the app; see `docs/screenshot.png`. Keep them when you change the layout. Light and dark follow the system.
- **Files are normal files on disk.** The storage root has `Files/`, `Trash/`, `metadata.json` and `backup-state.json`. The root comes from `DRIVE_STORAGE_ROOT`, then the folder saved in Settings, then `<data dir>/Drive` (`~/Library/Application Support/Drive` on a Mac). The settings are in `<config dir>/Drive/settings.json`.
- **Metadata is keyed by full path.** Stars, pins and folder colors are stored by path in `DriveMetadata`. After any move or rename, call `metadata.repath(old, new)`, or the item loses them. Every field has `#[serde(default)]`, so a file from an older version (also from the Swift app) still loads. Do the same for new fields.
- **Paths must be canonical.** `FileRepository::prepared_folder` returns the real path (`/private/var`, not `/var`). Path checks use `Path::starts_with` on these. Never check paths with string prefixes; that breaks on Windows.
- **The window sends paths, so the services check them.** `DriveService::require_item_in_drive` stops rename, trash and duplicate for anything outside `Files/`. `trash_slot` is the guard that stops restore and delete from touching anything outside the trash. `drive_open` and `drive_reveal` only take paths inside the storage root. The asset protocol (used to show pictures and PDFs) is only allowed for the storage root; see `AppContainer::install`.
- **The close button hides the main window.** `AppContainer::keep_alive` stops the close and hides it, so the window comes back as it was (Dock icon or tray). Only "Quit Drive" stops the app. Drive adds itself to the login items once (`AppContainer::start_at_login`, `tauri-plugin-autostart`). It then starts with `--hidden`: the window stays hidden (`"visible": false` in `tauri.conf.json`) and only the sync runs.
- **The trash uses slots.** A trashed item moves to `Trash/<random id>/<name>`. `metadata.trash_origins` maps the slot id to the old path.
- **The trash empties itself.** `DriveService::empty_old_trash` deletes a slot when the change date of the slot folder is older than `AppConstants::TRASH_KEEP_DAYS` (30). `AppContainer::keep_trash_clean` runs it at the start and then once per hour.
- **Items are dragged with pointer events.** `ItemDrag` (window side) moves items onto any element with `data-folder` and `data-path`. It does not use HTML drag and drop, because that does not work on Windows while the window takes file drops from the system. Files from outside the app still come in through `Shortcuts.onFileDrop`, and use the same `data-folder` elements.
- **The app updates itself.** The window calls `update_check` at the start and every 6 hours (`UpdateController` on both sides). It asks the user, then `update_install` downloads, installs and restarts. The address of `latest.json` and the public key are in `tauri.conf.json` under `plugins.updater`. A debug build never looks for updates.
- **How the automatic backup starts.** Every action that changes files ends in `DriveController.didChange` (window side). It reads the lists again and calls `backup.autoBackUp()`. A new action that changes files must go through `didChange` too.
- **How the window learns about backup steps.** `BackupService` and `BackupController` call `BackupObserver::backup_changed`. `WindowObserver` (in `lib.rs`) turns that into the `backup-changed` event. The window then asks for `backup_status` again, and the drive lists are read again a moment later.
- **The backup syncs both ways.** `BackupService::plan` only compares; `run` does the work. `BackupState.synced_dates` holds the files that were in both places after the last sync. So a file that is on one side only was deleted on the other side when it is in that list, and is new when it is not. A rename is a delete plus a new file. A file deleted here is removed from the backup; a file removed from the backup goes to the trash here. A file changed on both sides: this computer wins. `remote_dates` tells when a file changed in the backup. The first time both sides have a file, it uploads when its size differs or the local copy is newer (the same rule as `aws s3 sync`). The sync runs after each change and every `AppConstants::SYNC_CHECK_SECONDS`. Restore never replaces a local file, and `local_path` refuses keys that could write outside the drive.
- **Stopping a backup.** Pause calls `Notify::notify_waiters`, and `BackupController::cancellable` drops the running work. `RunGuard` and `StepGuard` reset the state in `Drop`, so a stop in the middle leaves nothing behind. Never hold a `parking_lot::Mutex` guard across an `.await`.
- **There are four backup providers.** `BackupProvider` names them. Each one is a `BackupRepository` (list, upload, download, delete), and `BackupService::repository` picks one. `S3Repository` also serves Google Cloud Storage. `AzureRepository` signs in with a SAS token. `GoogleDriveRepository` has no paths, only file IDs, so its `list` builds the paths and the other calls use the IDs from the last `list`.
- **Big files go up in parts.** A file over `AppConstants::UPLOAD_PART_BYTES` (8 MB) is sent in parts, and a part that fails is sent again. `PartUpload` sends four parts at the same time for S3 (multipart upload) and Azure (blocks). Google Drive only takes the parts in order (`send_in_parts`). `BackupService::run` uploads four files at the same time, so a `BackupRepository` must be safe to call from many uploads at once.
- **The Google sign-in uses the browser.** `GoogleAuthRepository::begin` opens a port on `127.0.0.1`, the command opens the Google page in the browser, and `finish` waits for Google to send the browser back to that port (OAuth 2 with PKCE, scope `drive.file`). It needs a "Desktop app" client, with its client ID and client secret.
- **Backup settings wait for Save.** The Settings dialog edits a draft `BackupConfig`. Only `BackupController::save_settings` makes it the one in use. Each provider has its own bucket and folder (`AppConstants::bucket_key`).
- **The backup state is saved.** `BackupService` keeps which files are in the backup in `backup-state.json`, so the icons are right at the start. The state is for one destination. `BackupService::follow` drops it when the destination changes.
- **Keys live only in the keychain.** The bucket, region and prefix are in `settings.json`. `BackupConfig::validate` limits the bucket, region and account to safe letters because they become part of the host name. `Credentials` has no `Debug` on purpose. `AppError` from `reqwest` drops the URL, because the Azure URL holds the SAS token.
- **Dates are milliseconds since 1970** (`i64`) everywhere: in the models, in the JSON files and in the window.
- **`// ponytail:` comments** mark a simple choice made on purpose, with its limit and how to grow it (for example one `PUT` per file, so 5 GB is the biggest file). Read them before you "fix" such code.

### Tests

Tests are in `src-tauri/tests/`. `drive_service_tests.rs` runs the real `DriveService` and `FileRepository` on a new temp folder for each test. `backup_tests.rs` checks the signer against the examples in the AWS documentation, and runs a full backup and restore against a fake provider that is a folder on disk. Keys are kept in memory there (`MemorySecrets`), so the tests never touch the keychain. There are no tests for the window code.
