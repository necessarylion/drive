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

## Architecture

The code goes in one direction: view → controller → service → repository → model. Both sides follow it, and both use dependency injection through the constructor.

### Rust side (`src-tauri/src/`)

- **`lib.rs`**: `AppContainer::install` is the one place where all parts are built and wired together. Add new services and repositories there. New commands must also be added to `generate_handler!` in `run`.
- **`commands/`**: the functions the window can call (`#[tauri::command]`). They are the "route" layer: each one only hands the call to a controller. All of them are `async`, so none blocks the main thread.
- **`controllers/`**: `DriveController` builds what the window shows (`ItemView` = an item plus color, summary, pin and backup state). `BackupController` keeps the state of a running backup. `SettingsController` reads and saves the storage folder.
- **`services/`**: the rules. `DriveService` (list, add, rename, star, pin, trash), `BackupService` (compare with the backup, upload, restore), `AwsSigner` (SigV4).
- **`repositories/`**: all outside access. `FileRepository` (disk), `MetadataRepository` and `BackupStateRepository` (JSON files), `SettingsRepository` (`settings.json`), `KeychainRepository` (the keychain of the system, through the `keyring` crate), `HttpTransport` (all HTTP), and one `BackupRepository` per provider.
- **`config/` and `constants/`**: `AppConfig` (where data is stored), `BackupConfig` (provider, bucket, region, prefix), `AppConstants` (all keys and limits).
- **`tray.rs`**: the tray icon and its menu.

### Window side (`ui/js/`)

- **`main.js`**: `AppContainer` builds and wires all parts of the window.
- **`views/`**: one class per part of the window. Every view extends `View`.
- **`controllers/`**: `DriveController` owns navigation, selection, sort, view mode and the item lists. `BackupController` holds the backup status.
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
- **The trash uses slots.** A trashed item moves to `Trash/<random id>/<name>`. `metadata.trash_origins` maps the slot id to the old path.
- **How the automatic backup starts.** Every action that changes files ends in `DriveController.didChange` (window side). It reads the lists again and calls `backup.autoBackUp()`. A new action that changes files must go through `didChange` too.
- **How the window learns about backup steps.** `BackupService` and `BackupController` call `BackupObserver::backup_changed`. `WindowObserver` (in `lib.rs`) turns that into the `backup-changed` event. The window then asks for `backup_status` again, and the drive lists are read again a moment later.
- **Backup rules.** `BackupService::plan` only compares; `run` does the work. A file uploads when its size differs or the local copy is newer (the same rule as `aws s3 sync`). The automatic backup never removes anything from the backup. Only the manual "Back Up Now" can, and it asks first. Restore never replaces a local file, and `local_path` refuses keys that could write outside the drive.
- **Stopping a backup.** Pause calls `Notify::notify_waiters`, and `BackupController::cancellable` drops the running work. `RunGuard` and `StepGuard` reset the state in `Drop`, so a stop in the middle leaves nothing behind. Never hold a `parking_lot::Mutex` guard across an `.await`.
- **There are four backup providers.** `BackupProvider` names them. Each one is a `BackupRepository` (list, upload, download, delete), and `BackupService::repository` picks one. `S3Repository` also serves Google Cloud Storage. `AzureRepository` signs in with a SAS token. `GoogleDriveRepository` has no paths, only file IDs, so its `list` builds the paths and the other calls use the IDs from the last `list`.
- **The Google sign-in uses the browser.** `GoogleAuthRepository::begin` opens a port on `127.0.0.1`, the command opens the Google page in the browser, and `finish` waits for Google to send the browser back to that port (OAuth 2 with PKCE, scope `drive.file`). It needs a "Desktop app" client, with its client ID and client secret.
- **Backup settings wait for Save.** The Settings dialog edits a draft `BackupConfig`. Only `BackupController::save_settings` makes it the one in use. Each provider has its own bucket and folder (`AppConstants::bucket_key`).
- **The backup state is saved.** `BackupService` keeps which files are in the backup in `backup-state.json`, so the icons are right at the start. The state is for one destination. `BackupService::follow` drops it when the destination changes.
- **Keys live only in the keychain.** The bucket, region and prefix are in `settings.json`. `BackupConfig::validate` limits the bucket, region and account to safe letters because they become part of the host name. `Credentials` has no `Debug` on purpose. `AppError` from `reqwest` drops the URL, because the Azure URL holds the SAS token.
- **Dates are milliseconds since 1970** (`i64`) everywhere: in the models, in the JSON files and in the window.
- **`// ponytail:` comments** mark a simple choice made on purpose, with its limit and how to grow it (for example one `PUT` per file, so 5 GB is the biggest file). Read them before you "fix" such code.

### Tests

Tests are in `src-tauri/tests/`. `drive_service_tests.rs` runs the real `DriveService` and `FileRepository` on a new temp folder for each test. `backup_tests.rs` checks the signer against the examples in the AWS documentation, and runs a full backup and restore against a fake provider that is a folder on disk. Keys are kept in memory there (`MemorySecrets`), so the tests never touch the keychain. There are no tests for the window code.
